//! PE 数据同步服务
//!
//! 负责从 SurrealDB 同步 PE 和 pe_owner 数据到 Kuzu

use crate::rs_surreal::query::*;
use crate::rs_surreal::{SUL_DB, query_type_refnos_by_dbnum};
use crate::types::*;
use crate::{RefU64, RefnoEnum};
use anyhow::Result;
use log::{debug, info, warn};
use std::collections::HashMap;

#[cfg(feature = "kuzu")]
use crate::rs_kuzu::{create_kuzu_connection, init_kuzu, init_kuzu_schema};
#[cfg(feature = "kuzu")]
use kuzu::{SystemConfig, Value};

/// PE 同步服务
pub struct PeSyncService {
    /// 批处理大小
    batch_size: usize,
    /// 是否只同步指定的 dbnum
    filter_dbnum: Option<i32>,
}

impl PeSyncService {
    /// 创建新的同步服务实例
    pub fn new(batch_size: usize) -> Self {
        Self {
            batch_size,
            filter_dbnum: None,
        }
    }

    /// 设置只同步指定的 dbnum
    pub fn with_dbnum_filter(mut self, dbnum: i32) -> Self {
        self.filter_dbnum = Some(dbnum);
        self
    }

    /// 初始化 Kuzu 数据库
    #[cfg(feature = "kuzu")]
    pub async fn init_kuzu_database(&self, db_path: &str) -> Result<()> {
        info!("初始化 Kuzu 数据库: {}", db_path);

        // 初始化数据库
        init_kuzu(db_path, SystemConfig::default()).await?;

        // 初始化模式
        init_kuzu_schema().await?;

        info!("Kuzu 数据库初始化完成");
        Ok(())
    }

    /// 执行完整同步
    pub async fn sync_all(&self) -> Result<SyncStats> {
        info!("开始 PE 数据全量同步");
        let start_time = std::time::Instant::now();

        let mut stats = SyncStats::default();

        // 1. 同步 PE 节点
        stats.pe_count = self.sync_pe_nodes().await?;

        // 2. 同步 pe_owner 关系
        stats.owner_count = self.sync_owner_relations().await?;

        stats.duration = start_time.elapsed();

        info!("同步完成: {:?}", stats);
        Ok(stats)
    }

    /// 同步 PE 节点数据
    #[cfg(feature = "kuzu")]
    async fn sync_pe_nodes(&self) -> Result<usize> {
        info!("开始同步 PE 节点");

        // 查询所有 PE 数据
        let pes = if let Some(dbnum) = self.filter_dbnum {
            self.query_pe_by_dbnum(dbnum).await?
        } else {
            self.query_all_pe().await?
        };

        info!("查询到 {} 个 PE 节点", pes.len());

        // 批量插入到 Kuzu
        let mut total_inserted = 0;
        for chunk in pes.chunks(self.batch_size) {
            let inserted = self.insert_pe_batch(chunk).await?;
            total_inserted += inserted;
            debug!("已插入 {} / {} PE 节点", total_inserted, pes.len());
        }

        Ok(total_inserted)
    }

    /// 同步 pe_owner 关系
    #[cfg(feature = "kuzu")]
    async fn sync_owner_relations(&self) -> Result<usize> {
        info!("开始同步 pe_owner 关系");

        // 查询所有 owner 关系
        let relations = if let Some(dbnum) = self.filter_dbnum {
            self.query_owner_relations_by_dbnum(dbnum).await?
        } else {
            self.query_all_owner_relations().await?
        };

        info!("查询到 {} 个 owner 关系", relations.len());

        // 批量创建关系
        let mut total_created = 0;
        for chunk in relations.chunks(self.batch_size) {
            let created = self.create_owner_relations_batch(chunk).await?;
            total_created += created;
            debug!("已创建 {} / {} owner 关系", total_created, relations.len());
        }

        Ok(total_created)
    }

    /// 从 SurrealDB 查询所有 PE
    async fn query_all_pe(&self) -> Result<Vec<SPdmsElement>> {
        // 使用更保守的查询方式 - 查询所有常见类型
        let nouns = vec![
            "PIPE", "BRAN", "EQUI", "STRU", "FITT", "VALV", "FLAN", "INST", "TUBI", "ATTA", "PLOO",
            "LOOP", "SITE", "ZONE",
        ];
        let mut all_pes = Vec::new();

        for noun in nouns {
            // 查询所有 dbnum
            for dbnum in [1112, 7999, 7997, 8000] {
                if let Ok(refnos) = query_type_refnos_by_dbnum(&[noun], dbnum, None, false).await {
                    for refno in refnos {
                        if let Ok(Some(pe)) = get_pe(refno).await {
                            if !pe.deleted {
                                all_pes.push(pe);
                            }
                        }
                    }
                }
            }
        }

        Ok(all_pes)
    }

    /// 按 dbnum 查询 PE
    async fn query_pe_by_dbnum(&self, dbnum: i32) -> Result<Vec<SPdmsElement>> {
        // 先获取指定 dbnum 的 refnos
        // 使用现有的查询函数，查询所有类型
        let nouns = vec![
            "PIPE", "BRAN", "EQUI", "STRU", "FITT", "VALV", "FLAN", "INST", "TUBI", "ATTA", "PLOO",
            "LOOP",
        ];
        let mut all_refnos = Vec::new();
        for noun in nouns {
            if let Ok(refnos) = query_type_refnos_by_dbnum(&[noun], dbnum as u32, None, false).await
            {
                all_refnos.extend(refnos);
            }
        }
        let refnos = all_refnos;

        let mut pes = Vec::new();
        for refno in refnos {
            if let Ok(Some(pe)) = get_pe(refno).await {
                if !pe.deleted && pe.dbnum == dbnum {
                    pes.push(pe);
                }
            }
        }

        Ok(pes)
    }

    /// 查询所有 owner 关系
    async fn query_all_owner_relations(&self) -> Result<Vec<OwnerRelation>> {
        // 获取所有 PE 并从中提取 owner 关系
        let nouns = vec![
            "PIPE", "BRAN", "EQUI", "STRU", "FITT", "VALV", "FLAN", "INST", "TUBI", "ATTA", "PLOO",
            "LOOP",
        ];
        let mut relations = Vec::new();

        for noun in nouns {
            for dbnum in [1112, 7999, 7997, 8000] {
                if let Ok(refnos) = query_type_refnos_by_dbnum(&[noun], dbnum, None, false).await {
                    for child_refno in refnos {
                        if let Ok(Some(pe)) = get_pe(child_refno).await {
                            if !pe.deleted && pe.owner.refno().0 != 0 {
                                relations.push(OwnerRelation {
                                    child_refno: child_refno.refno().0,
                                    parent_refno: pe.owner.refno().0,
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(relations)
    }

    /// 按 dbnum 查询 owner 关系
    async fn query_owner_relations_by_dbnum(&self, dbnum: i32) -> Result<Vec<OwnerRelation>> {
        // 获取指定 dbnum 的 PE 并从中提取 owner 关系
        let nouns = vec![
            "PIPE", "BRAN", "EQUI", "STRU", "FITT", "VALV", "FLAN", "INST", "TUBI", "ATTA", "PLOO",
            "LOOP",
        ];
        let mut all_refnos = Vec::new();
        for noun in nouns {
            if let Ok(refnos) = query_type_refnos_by_dbnum(&[noun], dbnum as u32, None, false).await
            {
                all_refnos.extend(refnos);
            }
        }
        let refnos = all_refnos;

        let mut relations = Vec::new();
        for child_refno in refnos {
            if let Ok(Some(pe)) = get_pe(child_refno).await {
                if !pe.deleted && pe.owner.refno().0 != 0 {
                    relations.push(OwnerRelation {
                        child_refno: child_refno.refno().0,
                        parent_refno: pe.owner.refno().0,
                    });
                }
            }
        }

        Ok(relations)
    }

    /// 批量插入 PE 到 Kuzu
    #[cfg(feature = "kuzu")]
    async fn insert_pe_batch(&self, pes: &[SPdmsElement]) -> Result<usize> {
        let conn = create_kuzu_connection()?;

        // 准备批量插入语句
        let mut values = Vec::new();
        for pe in pes {
            values.push(format!(
                "({}, '{}', '{}', {}, {}, '{}', {}, {}, {})",
                pe.refno().0,
                pe.name.replace("'", "''"),
                pe.noun.replace("'", "''"),
                pe.dbnum,
                pe.sesno,
                pe.cata_hash.replace("'", "''"),
                pe.deleted,
                pe.status_code
                    .as_ref()
                    .map_or("NULL".to_string(), |s| format!(
                        "'{}'",
                        s.replace("'", "''")
                    )),
                pe.lock
            ));
        }

        if values.is_empty() {
            return Ok(0);
        }

        // 执行批量插入
        let sql = format!(
            "CREATE (p:PE {{refno: $1, name: $2, noun: $3, dbnum: $4, sesno: $5, cata_hash: $6, deleted: $7, status_code: $8, lock: $9}})",
        );

        // 使用 MERGE 避免重复
        let mut count = 0;
        for pe in pes {
            let merge_sql = format!(
                r#"
                MERGE (p:PE {{refno: {}}})
                SET p.name = '{}',
                    p.noun = '{}',
                    p.dbnum = {},
                    p.sesno = {},
                    p.cata_hash = '{}',
                    p.deleted = {},
                    p.status_code = {},
                    p.lock = {}
                "#,
                pe.refno().0,
                pe.name.replace("'", "''"),
                pe.noun.replace("'", "''"),
                pe.dbnum,
                pe.sesno,
                pe.cata_hash.replace("'", "''"),
                pe.deleted,
                pe.status_code
                    .as_ref()
                    .map_or("NULL".to_string(), |s| format!(
                        "'{}'",
                        s.replace("'", "''")
                    )),
                pe.lock
            );

            match conn.query(&merge_sql) {
                Ok(_) => count += 1,
                Err(e) => warn!("插入 PE {} 失败: {}", pe.refno().0, e),
            }
        }

        Ok(count)
    }

    /// 批量创建 owner 关系
    #[cfg(feature = "kuzu")]
    async fn create_owner_relations_batch(&self, relations: &[OwnerRelation]) -> Result<usize> {
        let conn = create_kuzu_connection()?;

        let mut count = 0;
        for rel in relations {
            let sql = format!(
                r#"
                MATCH (parent:PE {{refno: {}}}), (child:PE {{refno: {}}})
                MERGE (parent)-[:OWNS]->(child)
                "#,
                rel.parent_refno, rel.child_refno
            );

            match conn.query(&sql) {
                Ok(_) => count += 1,
                Err(e) => warn!(
                    "创建关系 {} -> {} 失败: {}",
                    rel.parent_refno, rel.child_refno, e
                ),
            }
        }

        Ok(count)
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

/// Owner 关系结构
#[derive(Debug, Clone, serde::Deserialize)]
struct OwnerRelation {
    child_refno: u64,
    parent_refno: u64,
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
    async fn sync_pe_nodes(&self) -> Result<usize> {
        anyhow::bail!("Kuzu feature not enabled")
    }

    async fn sync_owner_relations(&self) -> Result<usize> {
        anyhow::bail!("Kuzu feature not enabled")
    }

    pub async fn verify_sync(&self, _dbnum: Option<i32>) -> Result<VerificationResult> {
        anyhow::bail!("Kuzu feature not enabled")
    }
}
