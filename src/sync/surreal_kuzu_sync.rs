//! SurrealDB 到 Kuzu 的数据同步方案
//!
//! 基于新的架构设计，实现从 SurrealDB 到 Kuzu 的数据同步

use anyhow::{Context, Result};
use kuzu::Connection;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

use crate::rs_kuzu::json_schema::{load_attr_info_json, pdms_type_to_kuzu};
use crate::rs_kuzu::{create_kuzu_connection, init_kuzu_schema};
use crate::rs_surreal::SUL_DB;
use crate::types::{NamedAttrMap, RefnoEnum, RefU64, AttrVal};
use crate::pdms_types::AttrInfo;

/// 同步配置
#[derive(Debug, Clone)]
pub struct SyncConfig {
    /// 批次大小
    pub batch_size: usize,
    /// 是否启用并行处理
    pub parallel: bool,
    /// 并行线程数
    pub thread_count: usize,
    /// 是否跳过错误继续同步
    pub skip_errors: bool,
    /// 指定同步的 noun 类型（空则同步所有）
    pub target_nouns: Vec<String>,
    /// 是否执行增量同步
    pub incremental: bool,
    /// 增量同步的起始 sesno
    pub from_sesno: Option<i32>,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            batch_size: 1000,
            parallel: true,
            thread_count: 4,
            skip_errors: true,
            target_nouns: Vec::new(),
            incremental: false,
            from_sesno: None,
        }
    }
}

/// 同步统计信息
#[derive(Debug, Default)]
pub struct SyncStats {
    pub total_pe_records: usize,
    pub synced_pe_records: usize,
    pub total_attr_records: usize,
    pub synced_attr_records: usize,
    pub total_relations: usize,
    pub synced_relations: usize,
    pub errors: Vec<String>,
    pub duration_ms: u128,
}

/// SurrealDB 到 Kuzu 的数据同步器
pub struct SurrealKuzuSync<'a> {
    config: SyncConfig,
    kuzu_conn: Connection<'a>,
    attr_info: HashMap<String, HashMap<String, AttrInfo>>,
    stats: SyncStats,
}

impl<'a> SurrealKuzuSync<'a> {
    /// 创建新的同步器实例
    pub async fn new(config: SyncConfig) -> Result<Self> {
        // 确保 Kuzu 已初始化
        let kuzu_conn = create_kuzu_connection()
            .context("创建 Kuzu 连接失败")?;

        // 加载属性信息映射
        let attr_info_json = load_attr_info_json()
            .context("加载属性信息失败")?;

        Ok(Self {
            config,
            kuzu_conn,
            attr_info: attr_info_json.named_attr_info_map,
            stats: SyncStats::default(),
        })
    }

    /// 执行全量同步
    pub async fn sync_full(&mut self) -> Result<SyncStats> {
        let start = std::time::Instant::now();
        log::info!("开始全量同步 SurrealDB -> Kuzu");

        // 1. 确保 Kuzu schema 已创建
        self.ensure_kuzu_schema().await?;

        // 2. 获取所有需要同步的 PE 记录
        let pe_list = self.fetch_pe_records().await?;
        self.stats.total_pe_records = pe_list.len();
        log::info!("需要同步 {} 条 PE 记录", pe_list.len());

        // 3. 批量同步 PE 记录
        self.sync_pe_batch(pe_list).await?;

        // 4. 同步属性数据
        self.sync_attributes().await?;

        // 5. 同步关系数据
        self.sync_relations().await?;

        self.stats.duration_ms = start.elapsed().as_millis();
        log::info!("同步完成，耗时 {} ms", self.stats.duration_ms);

        Ok(self.stats.clone())
    }

    /// 执行增量同步
    pub async fn sync_incremental(&mut self, from_sesno: i32) -> Result<SyncStats> {
        let start = std::time::Instant::now();
        log::info!("开始增量同步 SurrealDB -> Kuzu (from sesno: {})", from_sesno);

        // 1. 获取增量变更记录
        let changes = self.fetch_incremental_changes(from_sesno).await?;
        log::info!("发现 {} 条增量变更", changes.len());

        // 2. 应用变更到 Kuzu
        self.apply_changes(changes).await?;

        self.stats.duration_ms = start.elapsed().as_millis();
        log::info!("增量同步完成，耗时 {} ms", self.stats.duration_ms);

        Ok(self.stats.clone())
    }

    /// 确保 Kuzu schema 已创建
    async fn ensure_kuzu_schema(&self) -> Result<()> {
        // 检查是否需要初始化 schema
        match self.kuzu_conn.query("MATCH (p:PE) RETURN COUNT(*) LIMIT 1;") {
            Ok(_) => {
                log::debug!("Kuzu schema 已存在");
            }
            Err(_) => {
                log::info!("初始化 Kuzu schema...");
                init_kuzu_schema().await?;
            }
        }
        Ok(())
    }

    /// 从 SurrealDB 获取 PE 记录
    async fn fetch_pe_records(&self) -> Result<Vec<PERecord>> {
        let db = SUL_DB.read().await;
        let surreal = db.as_ref()
            .ok_or_else(|| anyhow::anyhow!("SurrealDB 未初始化"))?;

        let mut pe_records = Vec::new();

        // 根据配置决定查询范围
        let query = if self.config.target_nouns.is_empty() {
            "SELECT * FROM pe LIMIT 100000".to_string()
        } else {
            let nouns = self.config.target_nouns
                .iter()
                .map(|n| format!("'{}'", n))
                .collect::<Vec<_>>()
                .join(",");
            format!("SELECT * FROM pe WHERE noun IN [{}]", nouns)
        };

        // 执行查询
        let result: Vec<Value> = surreal.query(&query)
            .await?
            .take(0)?;

        for record in result {
            if let Some(pe) = PERecord::from_surreal_value(record) {
                pe_records.push(pe);
            }
        }

        Ok(pe_records)
    }

    /// 批量同步 PE 记录到 Kuzu
    async fn sync_pe_batch(&mut self, pe_list: Vec<PERecord>) -> Result<()> {
        let batch_size = self.config.batch_size;
        let total_batches = (pe_list.len() + batch_size - 1) / batch_size;

        for (batch_idx, chunk) in pe_list.chunks(batch_size).enumerate() {
            log::debug!("同步批次 {}/{}", batch_idx + 1, total_batches);

            // 构建批量插入语句
            let mut statements = Vec::new();

            for pe in chunk {
                // 插入 PE 主表
                statements.push(format!(
                    "CREATE (p:PE {{refno: {}, name: '{}', noun: '{}', dbnum: {}, sesno: {}, deleted: {}, lock: {}}})",
                    pe.refno, pe.name, pe.noun, pe.dbnum, pe.sesno, pe.deleted, pe.lock
                ));

                self.stats.synced_pe_records += 1;
            }

            // 执行批量插入
            for stmt in statements {
                if let Err(e) = self.kuzu_conn.query(&stmt) {
                    if self.config.skip_errors {
                        self.stats.errors.push(format!("PE插入错误: {}", e));
                    } else {
                        return Err(e.into());
                    }
                }
            }
        }

        Ok(())
    }

    /// 同步属性数据
    async fn sync_attributes(&mut self) -> Result<()> {
        log::info!("开始同步属性数据...");

        let db = SUL_DB.read().await;
        let surreal = db.as_ref()
            .ok_or_else(|| anyhow::anyhow!("SurrealDB 未初始化"))?;

        // 按 noun 分组同步
        for (noun, attr_info_map) in &self.attr_info {
            let table_name = format!("Attr_{}", noun.to_uppercase());

            // 查询该 noun 的所有记录
            let query = format!("SELECT * FROM pe WHERE noun = '{}' LIMIT 10000", noun);
            let records: Vec<Value> = surreal.query(&query)
                .await?
                .take(0)?;

            for record in records {
                if let Some(refno) = record.get("refno").and_then(|v| v.as_i64()) {
                    if let Some(attrs) = record.get("attrs").and_then(|v| v.as_object()) {
                        // 转换属性并插入到对应的 Attr_<NOUN> 表
                        self.insert_attr_record(&table_name, refno, attrs, attr_info_map)?;
                        self.stats.synced_attr_records += 1;
                    }
                }
            }
        }

        Ok(())
    }

    /// 插入属性记录到指定的 Attr_<NOUN> 表
    fn insert_attr_record(
        &mut self,
        table_name: &str,
        refno: i64,
        attrs: &serde_json::Map<String, Value>,
        attr_info_map: &HashMap<String, AttrInfo>
    ) -> Result<()> {
        let mut fields = vec![format!("refno: {}", refno)];

        for (attr_name, attr_value) in attrs {
            if let Some(attr_info) = attr_info_map.get(&attr_name.to_uppercase()) {
                let field_value = self.convert_value_for_kuzu(attr_value, &attr_info.att_type)?;
                fields.push(format!("{}: {}", attr_name.to_uppercase(), field_value));
            }
        }

        let stmt = format!("CREATE (a:{} {{{}}})", table_name, fields.join(", "));

        if let Err(e) = self.kuzu_conn.query(&stmt) {
            if self.config.skip_errors {
                self.stats.errors.push(format!("属性插入错误: {}", e));
            } else {
                return Err(e.into());
            }
        }

        Ok(())
    }

    /// 同步关系数据
    async fn sync_relations(&mut self) -> Result<()> {
        log::info!("开始同步关系数据...");

        // 1. 同步 OWNS 关系（层次关系）
        self.sync_owner_relations().await?;

        // 2. 同步 TO_<NOUN> 关系（PE 到属性表的关系）
        self.sync_attr_relations().await?;

        // 3. 同步引用关系（REFERS_TO）
        self.sync_reference_relations().await?;

        Ok(())
    }

    /// 同步 owner 关系
    async fn sync_owner_relations(&mut self) -> Result<()> {
        let db = SUL_DB.read().await;
        let surreal = db.as_ref()
            .ok_or_else(|| anyhow::anyhow!("SurrealDB 未初始化"))?;

        let query = "SELECT id, owner FROM pe WHERE owner != null LIMIT 100000";
        let records: Vec<Value> = surreal.query(query)
            .await?
            .take(0)?;

        for record in records {
            if let (Some(child_refno), Some(owner_refno)) = (
                record.get("id").and_then(|v| v.as_str()),
                record.get("owner").and_then(|v| v.as_i64())
            ) {
                let stmt = format!(
                    "MATCH (child:PE {{refno: {}}}), (parent:PE {{refno: {}}}) \
                     CREATE (parent)-[:OWNS]->(child)",
                    child_refno.replace("pe:", ""), owner_refno
                );

                if let Err(e) = self.kuzu_conn.query(&stmt) {
                    if self.config.skip_errors {
                        self.stats.errors.push(format!("OWNS关系错误: {}", e));
                    } else {
                        return Err(e.into());
                    }
                }
                self.stats.synced_relations += 1;
            }
        }

        Ok(())
    }

    /// 同步 PE 到属性表的关系
    async fn sync_attr_relations(&mut self) -> Result<()> {
        for noun in self.attr_info.keys() {
            let rel_name = format!("TO_{}", noun.to_uppercase());
            let table_name = format!("Attr_{}", noun.to_uppercase());

            let stmt = format!(
                "MATCH (p:PE), (a:{}) WHERE p.refno = a.refno AND p.noun = '{}' \
                 CREATE (p)-[:{}]->(a)",
                table_name, noun, rel_name
            );

            if let Err(e) = self.kuzu_conn.query(&stmt) {
                if self.config.skip_errors {
                    self.stats.errors.push(format!("{}关系错误: {}", rel_name, e));
                }
            }
        }

        Ok(())
    }

    /// 同步引用关系
    async fn sync_reference_relations(&mut self) -> Result<()> {
        // 实现引用关系同步逻辑
        // 例如：SPRE_REFNO, CREF 等字段的引用关系
        Ok(())
    }

    /// 获取增量变更
    async fn fetch_incremental_changes(&self, from_sesno: i32) -> Result<Vec<ChangeRecord>> {
        let db = SUL_DB.read().await;
        let surreal = db.as_ref()
            .ok_or_else(|| anyhow::anyhow!("SurrealDB 未初始化"))?;

        let query = format!("SELECT * FROM pe WHERE sesno > {} ORDER BY sesno", from_sesno);
        let records: Vec<Value> = surreal.query(&query)
            .await?
            .take(0)?;

        let mut changes = Vec::new();
        for record in records {
            if let Some(change) = ChangeRecord::from_surreal_value(record) {
                changes.push(change);
            }
        }

        Ok(changes)
    }

    /// 应用变更到 Kuzu
    async fn apply_changes(&mut self, changes: Vec<ChangeRecord>) -> Result<()> {
        for change in changes {
            match change.operation {
                Operation::Create => self.apply_create(change).await?,
                Operation::Update => self.apply_update(change).await?,
                Operation::Delete => self.apply_delete(change).await?,
            }
        }
        Ok(())
    }

    /// 应用创建操作
    async fn apply_create(&mut self, change: ChangeRecord) -> Result<()> {
        // 实现创建逻辑
        Ok(())
    }

    /// 应用更新操作
    async fn apply_update(&mut self, change: ChangeRecord) -> Result<()> {
        // 实现更新逻辑
        Ok(())
    }

    /// 应用删除操作
    async fn apply_delete(&mut self, change: ChangeRecord) -> Result<()> {
        // 实现删除逻辑
        Ok(())
    }

    /// 转换值为 Kuzu 格式
    fn convert_value_for_kuzu(&self, value: &Value, attr_type: &crate::pdms_types::DbAttributeType) -> Result<String> {
        match value {
            Value::Null => Ok("NULL".to_string()),
            Value::Bool(b) => Ok(b.to_string()),
            Value::Number(n) => Ok(n.to_string()),
            Value::String(s) => Ok(format!("'{}'", s.replace('\'', "''"))),
            Value::Array(arr) => {
                let values = arr.iter()
                    .map(|v| self.convert_value_for_kuzu(v, attr_type))
                    .collect::<Result<Vec<_>>>()?;
                Ok(format!("[{}]", values.join(", ")))
            }
            _ => Ok("NULL".to_string()),
        }
    }
}

/// PE 记录
#[derive(Debug, Clone)]
struct PERecord {
    refno: i64,
    name: String,
    noun: String,
    dbnum: i32,
    sesno: i32,
    deleted: bool,
    lock: bool,
}

impl PERecord {
    fn from_surreal_value(value: Value) -> Option<Self> {
        Some(PERecord {
            refno: value.get("refno")?.as_i64()? as i64,
            name: value.get("name")?.as_str()?.to_string(),
            noun: value.get("noun")?.as_str()?.to_string(),
            dbnum: value.get("dbnum")?.as_i64()? as i32,
            sesno: value.get("sesno")?.as_i64()? as i32,
            deleted: value.get("deleted")?.as_bool()?,
            lock: value.get("lock")?.as_bool()?,
        })
    }
}

/// 变更记录
#[derive(Debug, Clone)]
struct ChangeRecord {
    refno: i64,
    operation: Operation,
    data: Value,
}

impl ChangeRecord {
    fn from_surreal_value(value: Value) -> Option<Self> {
        // 实现从 SurrealDB 记录到变更记录的转换
        None
    }
}

/// 操作类型
#[derive(Debug, Clone)]
enum Operation {
    Create,
    Update,
    Delete,
}

/// 批量同步任务
pub async fn batch_sync_surreal_to_kuzu(config: SyncConfig) -> Result<SyncStats> {
    let mut syncer = SurrealKuzuSync::new(config).await?;
    syncer.sync_full().await
}

/// 增量同步任务
pub async fn incremental_sync_surreal_to_kuzu(from_sesno: i32) -> Result<SyncStats> {
    let config = SyncConfig {
        incremental: true,
        from_sesno: Some(from_sesno),
        ..Default::default()
    };

    let mut syncer = SurrealKuzuSync::new(config).await?;
    syncer.sync_incremental(from_sesno).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sync_config() {
        let config = SyncConfig::default();
        assert_eq!(config.batch_size, 1000);
        assert!(config.parallel);
    }
}