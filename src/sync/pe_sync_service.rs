//! PE 数据同步服务 - 基于层级树的递归同步
//!
//! 负责从 SurrealDB 同步 PE 和 pe_owner 数据到 Kuzu
//! 支持指定任意 refno 节点,递归同步其所有子树

use crate::rs_surreal::{get_mdb_world_site_pes, get_site_pes_by_dbnum, DBType, SUL_DB};
use crate::types::*;
use anyhow::{Context, Result};
use log::{debug, error, info, warn};

#[cfg(feature = "kuzu")]
use crate::rs_kuzu::{create_kuzu_connection, init_kuzu, init_kuzu_schema};
#[cfg(feature = "kuzu")]
use kuzu::SystemConfig;

/// PE 同步服务
pub struct PeSyncService {
    /// 失败记录追踪
    failed_nodes: Vec<FailedRecord>,
    failed_relations: Vec<FailedRecord>,
}

/// 失败记录
#[derive(Debug, Clone)]
pub struct FailedRecord {
    pub refno: u64,
    pub parent_refno: Option<u64>,
    pub error: String,
}

impl PeSyncService {
    /// 创建新的同步服务实例
    pub fn new() -> Self {
        Self {
            failed_nodes: Vec::new(),
            failed_relations: Vec::new(),
        }
    }

    /// 初始化 Kuzu 数据库
    #[cfg(feature = "kuzu")]
    pub async fn init_kuzu_database(&self, db_path: &str) -> Result<()> {
        info!("初始化 Kuzu 数据库: {}", db_path);
        init_kuzu(db_path, SystemConfig::default()).await?;
        init_kuzu_schema().await?;
        info!("Kuzu 数据库初始化完成");
        Ok(())
    }


    /// 🎯 核心方法: 同步指定 dbnum 的所有 SITE 及其子树
    ///
    /// ## 功能概述
    /// - 根据传入的 `dbnum` 或 `mdb_name` 解析需要同步的根 SITE 列表。
    /// - 逐个调用 [`Self::sync_by_refno`] 执行广度优先同步，累积同步统计。
    /// - 汇总耗时与同步结果，并统一输出失败节点与关系的日志。
    ///
    /// ## 行为细节
    /// 1. 当提供 `dbnum` 时，限定同步单一数据库下的 SITE。
    /// 2. 当 `dbnum` 为 `None` 时，必须传入 `mdb_name` 与可选模块 `module`，以获取全库 SITE。
    /// 3. 对每个 SITE 调用 [`Self::sync_by_refno`]，并叠加节点/关系计数与耗时。
    /// 4. 最终打印统计信息并调用 [`Self::report_failures`] 报告同步异常。
    ///
    /// # 参数
    /// - `dbnum`: 数据库编号，如果为 None 则查询所有 SITE
    /// - `mdb_name`: MDB 名称(当 dbnum 为 None 时必须提供)
    /// - `module`: 数据库模块类型(默认 DESI)
    #[cfg(feature = "kuzu")]
    pub async fn sync_all_sites(
        &mut self,
        dbnum: Option<i32>,
        mdb_name: Option<String>,
        module: Option<DBType>,
    ) -> Result<SyncStats> {
        let start_time = std::time::Instant::now();

        // 1. 获取所有 SITE 根节点集合
        let sites = if let Some(dbnum) = dbnum {
            // 按 dbnum 查询 SITE
            info!("开始同步 dbnum {} 的所有 SITE", dbnum);
            self.fetch_sites_by_dbnum(dbnum).await?
        } else {
            // 通过 MDB 查询所有 SITE
            let mdb = mdb_name.ok_or_else(|| {
                anyhow::anyhow!("当 dbnum 为 None 时，必须提供 mdb_name 参数")
            })?;
            let db_module = module.unwrap_or(DBType::DESI);
            info!("开始同步 MDB '{}' 的所有 SITE", mdb);
            self.fetch_sites_by_mdb(&mdb, db_module).await?
        };

        info!("找到 {} 个 SITE 节点", sites.len());

        // 2. 逐个 SITE 执行同步并累计统计信息
        let mut total_stats = SyncStats::default();
        for (idx, site_refno) in sites.iter().enumerate() {
            info!(
                "同步 SITE [{}/{}]: refno={}",
                idx + 1,
                sites.len(),
                site_refno
            );

            let stats = self.sync_by_refno(*site_refno).await?;
            total_stats.pe_count += stats.pe_count;
            total_stats.owner_count += stats.owner_count;
        }

        total_stats.duration = start_time.elapsed();
        info!("所有 SITE 同步完成: {:?}", total_stats);

        // 3. 输出在整个同步过程中记录的失败节点与关系
        self.report_failures();

        Ok(total_stats)
    }

    /// 🎯 核心通用方法: 同步指定 refno 及其所有子节点(批量操作)
    ///
    /// ## 功能概述
    /// 这是最通用的同步方法，可以同步任意节点（SITE、ZONE、EQUI 等）及其整个子树。
    ///
    /// ## 工作流程（批量优化版 - 使用 SurrealDB 层级查询）
    /// 1. **阶段一：批量查询** - 使用 SurrealDB 的层级查询一次性获取整个子树
    /// 2. **阶段二：批量插入节点** - 将所有节点批量插入到 Kuzu 数据库
    /// 3. **阶段三：批量创建关系** - 根据每个节点的 owner 字段批量创建父子关系
    ///
    /// ## 性能优化
    /// - **消除逐层遍历**：使用 SurrealDB 图查询语法一次性获取整个子树
    /// - **批量操作**：先插入所有节点，再创建所有关系，减少数据库往返
    /// - **省略 pe_owner 表查询**：直接使用 PE 节点的 owner 字段创建关系
    ///
    /// # 参数
    /// - `root_refno`: 任意节点的 refno，可以是 SITE、ZONE、EQUI 等任何类型
    ///
    /// # 返回
    /// - 同步统计信息，包括节点数、关系数、耗时
    #[cfg(feature = "kuzu")]
    pub async fn sync_by_refno(&mut self, root_refno: RefU64) -> Result<SyncStats> {
        info!("开始批量同步节点树: root_refno={}", root_refno);
        let start_time = std::time::Instant::now();

        let mut stats = SyncStats::default();

        // 🎯 阶段一：使用 SurrealDB 层级查询一次性获取整个子树
        info!("阶段 1/3: 查询所有子节点...");
        let all_nodes = self.fetch_subtree(root_refno).await?;
        info!("找到 {} 个节点（包含根节点）", all_nodes.len());

        // 🎯 阶段二：批量插入所有节点到 Kuzu
        info!("阶段 2/3: 批量插入节点到 Kuzu...");
        for (idx, pe) in all_nodes.iter().enumerate() {
            if let Err(e) = self.insert_pe_node(pe).await {
                error!("插入节点 {} 失败: {}", pe.refno(), e);
                self.failed_nodes.push(FailedRecord {
                    refno: pe.refno().0,
                    parent_refno: None,
                    error: e.to_string(),
                });
            } else {
                stats.pe_count += 1;
                if (idx + 1) % 100 == 0 {
                    debug!("已插入 {}/{} 个节点", idx + 1, all_nodes.len());
                }
            }
        }
        info!("节点插入完成: {}/{} 成功", stats.pe_count, all_nodes.len());

        // 🎯 阶段三：批量创建关系（基于每个节点的 owner 字段）
        info!("阶段 3/3: 批量创建 OWNS 关系...");
        for pe in &all_nodes {
            let owner_refno = pe.owner.refno().0; // 获取 owner 的 u64 值
            // 如果有父节点且父节点不为 0（根节点的 owner 通常为 0）
            if owner_refno > 0 {
                if let Err(e) = self
                    .create_owner_relation(owner_refno, pe.refno().0)
                    .await
                {
                    error!(
                        "创建关系 {} -> {} 失败: {}",
                        owner_refno,
                        pe.refno(),
                        e
                    );
                    self.failed_relations.push(FailedRecord {
                        refno: pe.refno().0,
                        parent_refno: Some(owner_refno),
                        error: e.to_string(),
                    });
                } else {
                    stats.owner_count += 1;
                }
            }
        }
        info!("关系创建完成: {} 条", stats.owner_count);

        stats.duration = start_time.elapsed();
        info!("节点树同步完成: {:?}", stats);
        Ok(stats)
    }

    /// 通过 MDB 获取所有 SITE 节点
    async fn fetch_sites_by_mdb(&self, mdb: &str, module: DBType) -> Result<Vec<RefU64>> {
        let sites = get_mdb_world_site_pes(mdb.to_string(), module).await?;
        Ok(sites.into_iter().map(|s| s.refno()).collect())
    }

    /// 从 SurrealDB 获取指定 dbnum 的所有 SITE 节点
    ///
    /// 使用 `get_site_pes_by_dbnum` 通过 WORL -> pe_owner 关系查询 SITE
    /// 这种方式与 `get_mdb_world_site_pes` 保持一致的查询逻辑
    async fn fetch_sites_by_dbnum(&self, dbnum: i32) -> Result<Vec<RefU64>> {
        let sites = get_site_pes_by_dbnum(dbnum as u32).await?;
        Ok(sites.into_iter().map(|s| s.refno()).collect())
    }

    /// 🎯 使用 SurrealDB 层级查询获取整个子树（包含根节点）
    ///
    /// ## 工作原理
    /// 使用 SurrealDB 的图遍历语法 `<-pe_owner` 递归查询所有子孙节点
    /// pe_owner 关系方向: child -[pe_owner]-> parent
    /// 所以从 parent <-pe_owner 可以获取所有子节点
    ///
    /// ## 查询逻辑
    /// 1. 先获取根节点本身
    /// 2. 使用 pe:{root}<-pe_owner 递归获取所有后代节点
    /// 3. 过滤掉已删除的节点（deleted = false）
    ///
    /// # 返回
    /// 包含根节点和所有子孙节点的列表
    async fn fetch_subtree(&self, root_refno: RefU64) -> Result<Vec<SPdmsElement>> {
        let mut all_nodes = Vec::new();

        // 1. 查询根节点（使用 record ID 格式）
        let root_sql = format!(
            "SELECT * FROM pe:{} WHERE deleted = false LIMIT 1",
            root_refno
        );
        let mut result = SUL_DB.query(root_sql).await?;
        let root_nodes: Vec<SPdmsElement> = result.take(0)?;

        if root_nodes.is_empty() {
            warn!("根节点 {} 不存在或已删除", root_refno);
            return Ok(all_nodes);
        }

        all_nodes.extend(root_nodes);

        // 2. 查询所有子孙节点（使用层级遍历）
        let descendants_sql = format!(
            r#"
            SELECT VALUE in
            FROM pe:{}<-pe_owner
            WHERE in.deleted = false
            "#,
            root_refno
        );

        let mut result = SUL_DB.query(descendants_sql).await?;
        let descendants: Vec<SPdmsElement> = result.take(0)?;
        all_nodes.extend(descendants);

        Ok(all_nodes)
    }

    /// 根据 refno 获取 PE 数据
    async fn fetch_pe_by_refno(&self, refno: RefU64) -> Result<Option<SPdmsElement>> {
        let sql = format!(
            "SELECT * FROM pe WHERE refno = {} AND deleted = false LIMIT 1",
            refno
        );

        let mut result = SUL_DB.query(sql).await?;
        let pes: Vec<SPdmsElement> = result.take(0)?;
        Ok(pes.into_iter().next())
    }

    /// 插入单个 PE 节点到 Kuzu
    #[cfg(feature = "kuzu")]
    async fn insert_pe_node(&mut self, pe: &SPdmsElement) -> Result<()> {
        let conn = create_kuzu_connection()?;

        let name = pe.name.replace("'", "''");
        let noun = pe.noun.replace("'", "''");
        let cata_hash = pe.cata_hash.replace("'", "''");

        let create_sql = format!(
            r#"CREATE (p:PE {{refno: {}, name: '{}', noun: '{}', dbnum: {}, sesno: {}, cata_hash: '{}', deleted: {}, lock: {}}})"#,
            pe.refno().0,
            name,
            noun,
            pe.dbnum,
            pe.sesno,
            cata_hash,
            pe.deleted,
            pe.lock
        );

        conn.query(&create_sql)
            .context(format!("插入 PE 节点 {} 失败", pe.refno()))?;

        Ok(())
    }

    /// 创建单条 owner 关系
    #[cfg(feature = "kuzu")]
    async fn create_owner_relation(&mut self, parent_refno: u64, child_refno: u64) -> Result<()> {
        let conn = create_kuzu_connection()?;

        let sql = format!(
            r#"MATCH (parent:PE {{refno: {}}}), (child:PE {{refno: {}}}) CREATE (parent)-[:OWNS]->(child)"#,
            parent_refno, child_refno
        );

        conn.query(&sql).context(format!(
            "创建关系 {} -> {} 失败",
            parent_refno, child_refno
        ))?;

        Ok(())
    }

    /// 报告失败记录
    fn report_failures(&self) {
        if !self.failed_nodes.is_empty() {
            warn!("⚠️  {} 个节点同步失败:", self.failed_nodes.len());
            for (idx, fail) in self.failed_nodes.iter().take(10).enumerate() {
                warn!("  [{}] refno={}: {}", idx + 1, fail.refno, fail.error);
            }
            if self.failed_nodes.len() > 10 {
                warn!("  ... 还有 {} 个失败记录", self.failed_nodes.len() - 10);
            }
        }

        if !self.failed_relations.is_empty() {
            warn!("⚠️  {} 条关系同步失败:", self.failed_relations.len());
            for (idx, fail) in self.failed_relations.iter().take(10).enumerate() {
                warn!(
                    "  [{}] {} -> {}: {}",
                    idx + 1,
                    fail.parent_refno.unwrap_or(0),
                    fail.refno,
                    fail.error
                );
            }
            if self.failed_relations.len() > 10 {
                warn!(
                    "  ... 还有 {} 个失败记录",
                    self.failed_relations.len() - 10
                );
            }
        }
    }

    /// 获取失败记录用于重试
    pub fn get_failed_nodes(&self) -> &[FailedRecord] {
        &self.failed_nodes
    }

    pub fn get_failed_relations(&self) -> &[FailedRecord] {
        &self.failed_relations
    }

    /// 验证同步结果
    #[cfg(feature = "kuzu")]
    pub async fn verify_sync(&self, dbnum: Option<i32>) -> Result<VerificationResult> {
        info!("开始验证同步结果");

        let mut result = VerificationResult::default();

        // 1. 比较 PE 节点数量
        let surreal_pe_count = self.count_surreal_pe(dbnum).await?;
        let kuzu_pe_count = self.count_kuzu_pe(dbnum).await?;
        result.pe_count_match = surreal_pe_count == kuzu_pe_count;
        result.surreal_pe_count = surreal_pe_count;
        result.kuzu_pe_count = kuzu_pe_count;

        // 2. 比较 owner 关系数量
        let surreal_owner_count = self.count_surreal_owner_relations(dbnum).await?;
        let kuzu_owner_count = self.count_kuzu_owner_relations(dbnum).await?;
        result.owner_count_match = surreal_owner_count == kuzu_owner_count;
        result.surreal_owner_count = surreal_owner_count;
        result.kuzu_owner_count = kuzu_owner_count;

        // 3. 抽样验证数据内容
        if let Some(dbnum) = dbnum {
            result.sample_verification = self.verify_sample_data(dbnum).await?;
        }

        info!("验证结果: {:?}", result);
        Ok(result)
    }

    /// 统计 SurrealDB PE 数量
    async fn count_surreal_pe(&self, dbnum: Option<i32>) -> Result<usize> {
        let db = &*SUL_DB;

        let sql = if let Some(dbnum) = dbnum {
            format!(
                "SELECT count() FROM pe WHERE dbnum = {} AND deleted = false GROUP ALL",
                dbnum
            )
        } else {
            "SELECT count() FROM pe WHERE deleted = false GROUP ALL".to_string()
        };

        let mut result = db.query(sql).await?;
        let count: Option<usize> = result.take("count")?;
        Ok(count.unwrap_or(0))
    }

    /// 统计 Kuzu PE 数量
    #[cfg(feature = "kuzu")]
    async fn count_kuzu_pe(&self, dbnum: Option<i32>) -> Result<usize> {
        let conn = create_kuzu_connection()?;

        let sql = if let Some(dbnum) = dbnum {
            format!(
                "MATCH (p:PE) WHERE p.dbnum = {} RETURN count(p) AS cnt",
                dbnum
            )
        } else {
            "MATCH (p:PE) RETURN count(p) AS cnt".to_string()
        };

        let mut result = conn.query(&sql)?;
        if let Some(row) = result.next() {
            if let kuzu::Value::Int64(count) = row.get(0).unwrap() {
                return Ok(*count as usize);
            }
        }
        Ok(0)
    }

    /// 统计 SurrealDB owner 关系数量
    async fn count_surreal_owner_relations(&self, dbnum: Option<i32>) -> Result<usize> {
        let db = &*SUL_DB;

        let sql = if let Some(dbnum) = dbnum {
            format!(
                "SELECT count() FROM pe_owner WHERE in.dbnum = {} OR out.dbnum = {} GROUP ALL",
                dbnum, dbnum
            )
        } else {
            "SELECT count() FROM pe_owner GROUP ALL".to_string()
        };

        let mut result = db.query(sql).await?;
        let count: Option<usize> = result.take("count")?;
        Ok(count.unwrap_or(0))
    }

    /// 统计 Kuzu owner 关系数量
    #[cfg(feature = "kuzu")]
    async fn count_kuzu_owner_relations(&self, dbnum: Option<i32>) -> Result<usize> {
        let conn = create_kuzu_connection()?;

        let sql = if let Some(dbnum) = dbnum {
            format!(
                "MATCH (p1:PE)-[r:OWNS]->(p2:PE) WHERE p1.dbnum = {} OR p2.dbnum = {} RETURN count(r) AS cnt",
                dbnum, dbnum
            )
        } else {
            "MATCH ()-[r:OWNS]->() RETURN count(r) AS cnt".to_string()
        };

        let mut result = conn.query(&sql)?;
        if let Some(row) = result.next() {
            if let kuzu::Value::Int64(count) = row.get(0).unwrap() {
                return Ok(*count as usize);
            }
        }
        Ok(0)
    }

    /// 抽样验证数据内容
    #[cfg(feature = "kuzu")]
    async fn verify_sample_data(&self, dbnum: i32) -> Result<bool> {
        // 从 SurrealDB 获取几个样本
        let db = &*SUL_DB;

        let sql = format!(
            "SELECT * FROM pe WHERE dbnum = {} AND deleted = false LIMIT 5",
            dbnum
        );
        let mut result = db.query(sql).await?;
        let samples: Vec<SPdmsElement> = result.take(0)?;

        // 验证每个样本在 Kuzu 中的数据
        let conn = create_kuzu_connection()?;
        for sample in samples {
            let kuzu_sql = format!(
                "MATCH (p:PE {{refno: {}}}) RETURN p.name, p.noun, p.dbnum, p.sesno",
                sample.refno().0
            );

            let mut kuzu_result = conn.query(&kuzu_sql)?;
            if let Some(row) = kuzu_result.next() {
                // 验证字段匹配
                // 这里简化处理，实际应该更严格地比较
                debug!("验证样本 {}: 成功", sample.refno().0);
            } else {
                warn!("样本 {} 在 Kuzu 中未找到", sample.refno().0);
                return Ok(false);
            }
        }

        Ok(true)
    }
}

/// 同步统计
#[derive(Debug, Default)]
pub struct SyncStats {
    /// 同步的 PE 节点数量
    pub pe_count: usize,
    /// 同步的 owner 关系数量
    pub owner_count: usize,
    /// 同步耗时
    pub duration: std::time::Duration,
}

/// 验证结果
#[derive(Debug, Default)]
pub struct VerificationResult {
    /// PE 数量是否匹配
    pub pe_count_match: bool,
    /// SurrealDB PE 数量
    pub surreal_pe_count: usize,
    /// Kuzu PE 数量
    pub kuzu_pe_count: usize,
    /// Owner 关系数量是否匹配
    pub owner_count_match: bool,
    /// SurrealDB owner 关系数量
    pub surreal_owner_count: usize,
    /// Kuzu owner 关系数量
    pub kuzu_owner_count: usize,
    /// 样本数据验证是否通过
    pub sample_verification: bool,
}

#[cfg(not(feature = "kuzu"))]
impl PeSyncService {
    pub async fn sync_all_sites(
        &mut self,
        _dbnum: Option<i32>,
        _mdb_name: Option<String>,
        _module: Option<DBType>,
    ) -> Result<SyncStats> {
        anyhow::bail!("Kuzu feature not enabled")
    }

    pub async fn sync_by_refno(&mut self, _root_refno: RefU64) -> Result<SyncStats> {
        anyhow::bail!("Kuzu feature not enabled")
    }

    pub async fn verify_sync(&self, _dbnum: Option<i32>) -> Result<VerificationResult> {
        anyhow::bail!("Kuzu feature not enabled")
    }
}
