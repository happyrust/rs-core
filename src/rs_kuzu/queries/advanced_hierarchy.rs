//! Kuzu 高级层级查询模块
//!
//! 提供返回完整对象（PE、NamedAttrMap等）的高级查询功能

use crate::rs_kuzu::{create_kuzu_connection, error::KuzuQueryError};
use crate::types::{RefnoEnum, RefU64, NamedAttrMap};
use crate::pe::SPdmsElement;
use anyhow::Result;
use kuzu::Value;

/// 获取子节点的 PE 完整信息
///
/// # 参数
/// * `refno` - 父节点refno
///
/// # 返回
/// * `Result<Vec<SPdmsElement>>` - 子节点PE列表
pub async fn kuzu_get_children_pes(refno: RefnoEnum) -> Result<Vec<SPdmsElement>> {
    let query = format!(
        "MATCH (parent:PE {{refno: {}}})-[:OWNS]->(child:PE)
         WHERE child.deleted = false
         RETURN child.refno, child.name, child.noun, child.dbnum, child.sesno, child.deleted, child.lock",
        refno.refno().0
    );

    log::debug!("Kuzu query: {}", query);

    let conn = create_kuzu_connection()
        .map_err(|e| KuzuQueryError::ConnectionError(e.to_string()))?;

    let mut result = conn.query(&query)
        .map_err(|e| KuzuQueryError::QueryExecutionError {
            query: query.clone(),
            error: e.to_string(),
        })?;

    let mut pes = Vec::new();

    while let Some(row) = result.next() {
        if let (
            Some(Value::Int64(refno_val)),
            Some(Value::String(name)),
            Some(Value::String(noun)),
            Some(Value::Int64(dbnum)),
            Some(Value::Int64(sesno)),
            Some(Value::Bool(deleted)),
            Some(Value::Bool(lock)),
        ) = (
            row.get(0),
            row.get(1),
            row.get(2),
            row.get(3),
            row.get(4),
            row.get(5),
            row.get(6),
        ) {
            let pe = SPdmsElement {
                refno: RefnoEnum::from(RefU64(*refno_val as u64)),
                name: name.clone(),
                noun: noun.clone(),
                dbnum: *dbnum as i32,
                sesno: *sesno as i32,
                deleted: *deleted,
                lock: *lock,
                owner: RefnoEnum::default(), // 需要单独查询
            };
            pes.push(pe);
        }
    }

    log::debug!("Found {} children PEs for refno {:?}", pes.len(), refno);
    Ok(pes)
}

/// 获取子节点的属性映射
///
/// # 参数
/// * `refno` - 父节点refno
///
/// # 返回
/// * `Result<Vec<NamedAttrMap>>` - 子节点属性映射列表
///
/// 注意: 当前版本仅返回基础PE属性，完整属性需要进一步查询属性表
pub async fn kuzu_get_children_named_attmaps(refno: RefnoEnum) -> Result<Vec<NamedAttrMap>> {
    let query = format!(
        "MATCH (parent:PE {{refno: {}}})-[:OWNS]->(child:PE)
         WHERE child.deleted = false
         RETURN child.refno, child.name, child.noun, child.dbnum, child.sesno",
        refno.refno().0
    );

    log::debug!("Kuzu query: {}", query);

    let conn = create_kuzu_connection()
        .map_err(|e| KuzuQueryError::ConnectionError(e.to_string()))?;

    let mut result = conn.query(&query)
        .map_err(|e| KuzuQueryError::QueryExecutionError {
            query: query.clone(),
            error: e.to_string(),
        })?;

    let mut attmaps = Vec::new();

    while let Some(row) = result.next() {
        if let (
            Some(Value::Int64(refno_val)),
            Some(Value::String(name)),
            Some(Value::String(noun)),
            Some(Value::Int64(dbnum)),
            Some(Value::Int64(sesno)),
        ) = (
            row.get(0),
            row.get(1),
            row.get(2),
            row.get(3),
            row.get(4),
        ) {
            let mut attmap = NamedAttrMap::default();

            // 设置基础PE属性
            attmap.insert_i32(":REFNO", *refno_val as i32);
            attmap.insert_string(":NAME", name.clone());
            attmap.insert_string(":NOUN", noun.clone());
            attmap.insert_i32(":DBNUM", *dbnum as i32);
            attmap.insert_i32(":SESNO", *sesno as i32);

            attmaps.push(attmap);
        }
    }

    log::debug!("Found {} children attmaps for refno {:?}", attmaps.len(), refno);
    Ok(attmaps)
}

/// 获取祖先的属性映射列表
///
/// # 参数
/// * `refno` - 子节点refno
///
/// # 返回
/// * `Result<Vec<NamedAttrMap>>` - 祖先属性映射列表（从近到远）
pub async fn kuzu_get_ancestor_attmaps(refno: RefnoEnum) -> Result<Vec<NamedAttrMap>> {
    let query = format!(
        "MATCH (child:PE {{refno: {}}})<-[:OWNS*]-(ancestor:PE)
         WHERE ancestor.deleted = false
         RETURN ancestor.refno, ancestor.name, ancestor.noun, ancestor.dbnum, ancestor.sesno",
        refno.refno().0
    );

    log::debug!("Kuzu query: {}", query);

    let conn = create_kuzu_connection()
        .map_err(|e| KuzuQueryError::ConnectionError(e.to_string()))?;

    let mut result = conn.query(&query)
        .map_err(|e| KuzuQueryError::QueryExecutionError {
            query: query.clone(),
            error: e.to_string(),
        })?;

    let mut attmaps = Vec::new();

    while let Some(row) = result.next() {
        if let (
            Some(Value::Int64(refno_val)),
            Some(Value::String(name)),
            Some(Value::String(noun)),
            Some(Value::Int64(dbnum)),
            Some(Value::Int64(sesno)),
        ) = (
            row.get(0),
            row.get(1),
            row.get(2),
            row.get(3),
            row.get(4),
        ) {
            let mut attmap = NamedAttrMap::default();

            attmap.insert_i32(":REFNO", *refno_val as i32);
            attmap.insert_string(":NAME", name.clone());
            attmap.insert_string(":NOUN", noun.clone());
            attmap.insert_i32(":DBNUM", *dbnum as i32);
            attmap.insert_i32(":SESNO", *sesno as i32);

            attmaps.push(attmap);
        }
    }

    log::debug!("Found {} ancestor attmaps for refno {:?}", attmaps.len(), refno);
    Ok(attmaps)
}

/// 按类型过滤子节点并返回属性映射
///
/// # 参数
/// * `refno` - 父节点refno
/// * `types` - 类型过滤列表
///
/// # 返回
/// * `Result<Vec<NamedAttrMap>>` - 匹配的子节点属性映射
pub async fn kuzu_query_filter_children_atts(
    refno: RefnoEnum,
    types: &[&str],
) -> Result<Vec<NamedAttrMap>> {
    let noun_filter = if types.is_empty() {
        String::new()
    } else {
        let nouns = types.iter().map(|n| format!("'{}'", n)).collect::<Vec<_>>().join(", ");
        format!("AND child.noun IN [{}]", nouns)
    };

    let query = format!(
        "MATCH (parent:PE {{refno: {}}})-[:OWNS]->(child:PE)
         WHERE child.deleted = false {}
         RETURN child.refno, child.name, child.noun, child.dbnum, child.sesno",
        refno.refno().0,
        noun_filter
    );

    log::debug!("Kuzu query: {}", query);

    let conn = create_kuzu_connection()
        .map_err(|e| KuzuQueryError::ConnectionError(e.to_string()))?;

    let mut result = conn.query(&query)
        .map_err(|e| KuzuQueryError::QueryExecutionError {
            query: query.clone(),
            error: e.to_string(),
        })?;

    let mut attmaps = Vec::new();

    while let Some(row) = result.next() {
        if let (
            Some(Value::Int64(refno_val)),
            Some(Value::String(name)),
            Some(Value::String(noun)),
            Some(Value::Int64(dbnum)),
            Some(Value::Int64(sesno)),
        ) = (
            row.get(0),
            row.get(1),
            row.get(2),
            row.get(3),
            row.get(4),
        ) {
            let mut attmap = NamedAttrMap::default();

            attmap.insert_i32(":REFNO", *refno_val as i32);
            attmap.insert_string(":NAME", name.clone());
            attmap.insert_string(":NOUN", noun.clone());
            attmap.insert_i32(":DBNUM", *dbnum as i32);
            attmap.insert_i32(":SESNO", *sesno as i32);

            attmaps.push(attmap);
        }
    }

    log::debug!("Found {} filtered children attmaps for refno {:?}", attmaps.len(), refno);
    Ok(attmaps)
}

/// 按类型过滤深层子孙并返回属性映射
///
/// # 参数
/// * `refno` - 父节点refno
/// * `nouns` - 类型过滤列表
///
/// # 返回
/// * `Result<Vec<NamedAttrMap>>` - 匹配的子孙属性映射
pub async fn kuzu_query_filter_deep_children_atts(
    refno: RefnoEnum,
    nouns: &[&str],
) -> Result<Vec<NamedAttrMap>> {
    let noun_filter = if nouns.is_empty() {
        String::new()
    } else {
        let nouns_str = nouns.iter().map(|n| format!("'{}'", n)).collect::<Vec<_>>().join(", ");
        format!("AND descendant.noun IN [{}]", nouns_str)
    };

    let query = format!(
        "MATCH (parent:PE {{refno: {}}})-[:OWNS*1..12]->(descendant:PE)
         WHERE descendant.deleted = false {}
         RETURN DISTINCT descendant.refno, descendant.name, descendant.noun, descendant.dbnum, descendant.sesno",
        refno.refno().0,
        noun_filter
    );

    log::debug!("Kuzu query: {}", query);

    let conn = create_kuzu_connection()
        .map_err(|e| KuzuQueryError::ConnectionError(e.to_string()))?;

    let mut result = conn.query(&query)
        .map_err(|e| KuzuQueryError::QueryExecutionError {
            query: query.clone(),
            error: e.to_string(),
        })?;

    let mut attmaps = Vec::new();

    while let Some(row) = result.next() {
        if let (
            Some(Value::Int64(refno_val)),
            Some(Value::String(name)),
            Some(Value::String(noun)),
            Some(Value::Int64(dbnum)),
            Some(Value::Int64(sesno)),
        ) = (
            row.get(0),
            row.get(1),
            row.get(2),
            row.get(3),
            row.get(4),
        ) {
            let mut attmap = NamedAttrMap::default();

            attmap.insert_i32(":REFNO", *refno_val as i32);
            attmap.insert_string(":NAME", name.clone());
            attmap.insert_string(":NOUN", noun.clone());
            attmap.insert_i32(":DBNUM", *dbnum as i32);
            attmap.insert_i32(":SESNO", *sesno as i32);

            attmaps.push(attmap);
        }
    }

    log::debug!("Found {} filtered deep children attmaps for refno {:?}", attmaps.len(), refno);
    Ok(attmaps)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // 需要数据库环境
    async fn test_get_children_pes() {
        let refno = RefnoEnum::from(RefU64(123));
        let result = kuzu_get_children_pes(refno).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore]
    async fn test_get_children_named_attmaps() {
        let refno = RefnoEnum::from(RefU64(123));
        let result = kuzu_get_children_named_attmaps(refno).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore]
    async fn test_get_ancestor_attmaps() {
        let refno = RefnoEnum::from(RefU64(456));
        let result = kuzu_get_ancestor_attmaps(refno).await;
        assert!(result.is_ok());
    }
}
