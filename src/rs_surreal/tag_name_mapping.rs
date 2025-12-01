//! Tag name mapping for PE elements
//!
//! This module provides mapping between PE elements and their tag names

use crate::types::RefnoEnum;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use surrealdb::engine::any::Any;
use surrealdb::types::Value;
use surrealdb::Surreal;

/// 位号映射表结构
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TagNameMapping {
    /// 映射记录ID
    pub id: String,
    /// PE元素引用
    #[serde(rename = "in")]
    pub pe_refno: RefnoEnum,
    /// 位号
    pub tag_name: String,
    /// 节点全名（用于Excel导入时匹配）
    pub full_name: String,
    /// 创建时间
    pub created_at: Option<chrono::NaiveDateTime>,
    /// 更新时间
    pub updated_at: Option<chrono::NaiveDateTime>,
}

impl TagNameMapping {
    /// 创建新的位号映射
    pub fn new(id: String, pe_refno: RefnoEnum, tag_name: String, full_name: String) -> Self {
        Self {
            id,
            pe_refno,
            tag_name,
            full_name,
            created_at: None,
            updated_at: None,
        }
    }

    /// 生成 SurQL 插入语句
    pub fn to_surql(&self) -> String {
        let created_at_str = match &self.created_at {
            Some(dt) => format!("d'{}'", dt.format("%Y-%m-%dT%H:%M:%S")),
            None => "time::now()".to_string(),
        };

        let updated_at_str = match &self.updated_at {
            Some(dt) => format!("d'{}'", dt.format("%Y-%m-%dT%H:%M:%S")),
            None => "NONE".to_string(),
        };

        format!(
            r#"CREATE tag_name_mapping:{} SET
                in = {},
                tag_name = '{}',
                full_name = '{}',
                created_at = {},
                updated_at = {};"#,
            self.id,
            self.pe_refno.to_pe_key(),
            self.tag_name.replace("'", "\\'"),
            self.full_name.replace("'", "\\'"),
            created_at_str,
            updated_at_str
        )
    }
}

/// 简单查询结果结构，用于获取位号
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TagNameQueryResult {
    pub tag_name: String,
}

/// 简单查询结果结构，用于获取full_name
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TagNameFullNameResult {
    pub full_name: String,
}

/// 根据 PE refno 查询位号
pub async fn get_tag_name_by_refno(
    conn: &Surreal<Any>,
    refno: RefnoEnum,
) -> Result<Option<String>> {
    let sql = format!(
        r#"SELECT tag_name FROM tag_name_mapping WHERE in = {} LIMIT 1"#,
        refno.to_pe_key()
    );

    let mut result = conn.query(&sql).await?;
    let row: Option<Value> = result.take(0).ok().flatten();

    Ok(row.and_then(|v| {
        if let Value::Object(obj) = v {
            obj.get("tag_name").and_then(|tag_v| {
                if let Value::String(s) = tag_v {
                    Some(s.clone())
                } else {
                    None
                }
            })
        } else {
            None
        }
    }))
}

/// 批量查询位号
pub async fn get_tag_names_by_refnos(
    conn: &Surreal<Any>,
    refnos: &[RefnoEnum],
) -> Result<Vec<(RefnoEnum, String, String)>> {
    if refnos.is_empty() {
        return Ok(Vec::new());
    }

    let batch_size = 100;
    let mut results = Vec::new();

    for chunk in refnos.chunks(batch_size) {
        let keys = chunk
            .iter()
            .map(|r| r.to_pe_key())
            .collect::<Vec<_>>()
            .join(", ");

        let sql = format!(
            r#"SELECT in, tag_name, full_name
               FROM tag_name_mapping
               WHERE in IN [{}]"#,
            keys
        );

        let mut result = conn.query(&sql).await?;
        let rows: Vec<Value> = result.take(0).unwrap_or_default();

        for row in rows {
            if let Value::Object(obj) = row {
                let tag_name_opt = obj.get("tag_name").and_then(|v| {
                    if let Value::String(s) = v {
                        Some(s.clone())
                    } else {
                        None
                    }
                });

                let full_name_opt = obj.get("full_name").and_then(|v| {
                    if let Value::String(s) = v {
                        Some(s.clone())
                    } else {
                        None
                    }
                });

                if let (Some(tag_name), Some(full_name)) = (tag_name_opt, full_name_opt) {
                    // 从 in 字段获取 RecordId 并转换为 RefnoEnum
                    if let Some(Value::RecordId(rid)) = obj.get("in") {
                        let refno: crate::types::RefU64 = rid.clone().into();
                        let refno_enum = RefnoEnum::Refno(refno);
                        results.push((refno_enum, tag_name, full_name));
                    }
                }
            }
        }
    }

    Ok(results)
}

/// 根据节点全名查询 refno（用于Excel导入时匹配）
pub async fn get_refno_by_full_name(
    conn: &Surreal<Any>,
    full_name: &str,
) -> Result<Option<RefnoEnum>> {
    let sql = format!(
        r#"SELECT in FROM tag_name_mapping WHERE full_name = '{}' LIMIT 1"#,
        full_name.replace("'", "\\'")
    );

    let mut result = conn.query(&sql).await?;
    let row: Option<Value> = result.take(0).ok().flatten();

    Ok(row.and_then(|v| {
        if let Value::Object(obj) = v {
            obj.get("in").and_then(|v| {
                if let Value::RecordId(rid) = v {
                    let refno: crate::types::RefU64 = rid.clone().into();
                    Some(RefnoEnum::Refno(refno))
                } else {
                    None
                }
            })
        } else {
            None
        }
    }))
}

/// 创建或更新位号映射
pub async fn upsert_tag_name_mapping(
    conn: &Surreal<Any>,
    mapping: &TagNameMapping,
) -> Result<()> {
    let sql = mapping.to_surql();
    conn.query(&sql).await?;
    Ok(())
}

/// 批量创建或更新位号映射
pub async fn upsert_tag_name_mappings_batch(
    conn: &Surreal<Any>,
    mappings: &[TagNameMapping],
) -> Result<()> {
    for mapping in mappings {
        upsert_tag_name_mapping(conn, mapping).await?;
    }
    Ok(())
}

/// 删除位号映射
pub async fn delete_tag_name_mapping(conn: &Surreal<Any>, refno: RefnoEnum) -> Result<()> {
    let sql = format!(
        r#"DELETE tag_name_mapping WHERE in = {}"#,
        refno.to_pe_key()
    );
    conn.query(&sql).await?;
    Ok(())
}
