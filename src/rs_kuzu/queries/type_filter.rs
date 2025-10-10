//! Kuzu 类型过滤查询模块
//!
//! 提供基于 dbnum 和 noun 的类型过滤查询功能

use crate::rs_kuzu::{create_kuzu_connection, error::KuzuQueryError, query_builder::TypeFilterQueryBuilder};
use crate::types::{RefnoEnum, RefU64};
use anyhow::Result;
use kuzu::Value;

/// 按 dbnum 和 noun 查询 refno 列表
///
/// # 参数
/// * `nouns` - noun 类型列表
/// * `dbnum` - 数据库编号
/// * `has_children` - 是否有子节点的过滤条件（None表示不过滤）
///
/// # 返回
/// * `Result<Vec<RefnoEnum>>` - 匹配的refno列表
///
/// # 示例
/// ```no_run
/// # use aios_core::rs_kuzu::queries::type_filter::kuzu_query_type_refnos_by_dbnum;
/// # tokio_test::block_on(async {
/// let refnos = kuzu_query_type_refnos_by_dbnum(&["PIPE", "EQUI"], 1112, None).await.unwrap();
/// println!("找到 {} 个元素", refnos.len());
/// # });
/// ```
pub async fn kuzu_query_type_refnos_by_dbnum(
    nouns: &[&str],
    dbnum: i32,
    has_children: Option<bool>,
) -> Result<Vec<RefnoEnum>> {
    let query = TypeFilterQueryBuilder::new()
        .dbnum(dbnum as u32)
        .nouns(nouns)
        .with_children(has_children)
        .build();

    log::debug!("Kuzu query: {}", query);

    let conn = create_kuzu_connection()
        .map_err(|e| KuzuQueryError::ConnectionError(e.to_string()))?;

    let mut result = conn.query(&query)
        .map_err(|e| KuzuQueryError::QueryExecutionError {
            query: query.clone(),
            error: e.to_string(),
        })?;

    let mut refnos = Vec::new();

    while let Some(row) = result.next() {
        if let Some(Value::Int64(refno_val)) = row.get(0) {
            refnos.push(RefnoEnum::from(RefU64(*refno_val as u64)));
        }
    }

    log::debug!("Found {} elements for dbnum={} nouns={:?}", refnos.len(), dbnum, nouns);
    Ok(refnos)
}

/// 按多个 dbnum 和 noun 查询 refno 列表
///
/// # 参数
/// * `nouns` - noun 类型列表
/// * `dbnums` - 数据库编号列表
///
/// # 返回
/// * `Result<Vec<RefnoEnum>>` - 匹配的refno列表
pub async fn kuzu_query_type_refnos_by_dbnums(
    nouns: &[&str],
    dbnums: &[i32],
) -> Result<Vec<RefnoEnum>> {
    let dbnums_u32: Vec<u32> = dbnums.iter().map(|&d| d as u32).collect();
    let query = TypeFilterQueryBuilder::new()
        .dbnums(&dbnums_u32)
        .nouns(nouns)
        .build();

    log::debug!("Kuzu query: {}", query);

    let conn = create_kuzu_connection()
        .map_err(|e| KuzuQueryError::ConnectionError(e.to_string()))?;

    let mut result = conn.query(&query)
        .map_err(|e| KuzuQueryError::QueryExecutionError {
            query: query.clone(),
            error: e.to_string(),
        })?;

    let mut refnos = Vec::new();

    while let Some(row) = result.next() {
        if let Some(Value::Int64(refno_val)) = row.get(0) {
            refnos.push(RefnoEnum::from(RefU64(*refno_val as u64)));
        }
    }

    log::debug!("Found {} elements for dbnums={:?} nouns={:?}", refnos.len(), dbnums, nouns);
    Ok(refnos)
}

/// 获取指定 dbnum 的 WORLD 节点
///
/// # 参数
/// * `dbnum` - 数据库编号
///
/// # 返回
/// * `Result<Option<RefnoEnum>>` - WORLD 节点的 refno（如果存在）
pub async fn kuzu_get_world_by_dbnum(dbnum: i32) -> Result<Option<RefnoEnum>> {
    let query = TypeFilterQueryBuilder::new()
        .dbnum(dbnum as u32)
        .nouns(&["WORLD"])
        .limit(1)
        .build();

    log::debug!("Kuzu query: {}", query);

    let conn = create_kuzu_connection()
        .map_err(|e| KuzuQueryError::ConnectionError(e.to_string()))?;

    let mut result = conn.query(&query)
        .map_err(|e| KuzuQueryError::QueryExecutionError {
            query: query.clone(),
            error: e.to_string(),
        })?;

    if let Some(row) = result.next() {
        if let Some(Value::Int64(refno_val)) = row.get(0) {
            return Ok(Some(RefnoEnum::from(RefU64(*refno_val as u64))));
        }
    }

    Ok(None)
}

/// 获取指定 dbnum 的所有 SITE 节点
///
/// # 参数
/// * `dbnum` - 数据库编号
///
/// # 返回
/// * `Result<Vec<RefnoEnum>>` - SITE 节点的 refno 列表
pub async fn kuzu_get_sites_of_dbnum(dbnum: i32) -> Result<Vec<RefnoEnum>> {
    kuzu_query_type_refnos_by_dbnum(&["SITE"], dbnum, None).await
}

/// 按 dbnum 和使用类别查询 refno 列表
///
/// # 参数
/// * `nouns` - noun 类型列表
/// * `dbnum` - 数据库编号
///
/// # 返回
/// * `Result<Vec<RefnoEnum>>` - 匹配的refno列表
///
/// 注意：这个方法需要额外的类别信息，当前简化实现
pub async fn kuzu_query_use_cate_refnos_by_dbnum(
    nouns: &[&str],
    dbnum: i32,
) -> Result<Vec<RefnoEnum>> {
    // TODO: 实现类别过滤逻辑
    // 当前版本与 query_type_refnos_by_dbnum 相同
    kuzu_query_type_refnos_by_dbnum(nouns, dbnum, None).await
}

/// 统计指定类型在数据库中的数量
///
/// # 参数
/// * `noun` - noun 类型
/// * `dbnum` - 数据库编号
///
/// # 返回
/// * `Result<usize>` - 匹配元素的数量
pub async fn kuzu_count_by_type(noun: &str, dbnum: i32) -> Result<usize> {
    let dbnum = dbnum as u32;
    let query = format!(
        "MATCH (pe:PE {{noun: '{}', dbnum: {}}})
         RETURN COUNT(*) AS count",
        noun, dbnum
    );

    log::debug!("Kuzu count query: {}", query);

    let conn = create_kuzu_connection()
        .map_err(|e| KuzuQueryError::ConnectionError(e.to_string()))?;

    let mut result = conn.query(&query)
        .map_err(|e| KuzuQueryError::QueryExecutionError {
            query: query.clone(),
            error: e.to_string(),
        })?;

    if let Some(row) = result.next() {
        if let Some(Value::Int64(count)) = row.get(0) {
            log::debug!("Count for noun={} dbnum={}: {}", noun, dbnum, count);
            return Ok(*count as usize);
        }
    }

    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // 需要数据库环境
    async fn test_query_type_refnos_by_dbnum() {
        let result = kuzu_query_type_refnos_by_dbnum(&["PIPE", "EQUI"], 1112, None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore]
    async fn test_get_world_by_dbnum() {
        let result = kuzu_get_world_by_dbnum(1112).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore]
    async fn test_get_sites_of_dbnum() {
        let result = kuzu_get_sites_of_dbnum(1112).await;
        assert!(result.is_ok());
    }
}
