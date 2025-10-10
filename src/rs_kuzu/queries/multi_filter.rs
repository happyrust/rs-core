//! Kuzu 多条件组合查询模块
//!
//! 提供复杂的多条件组合查询功能

use crate::rs_kuzu::{create_kuzu_connection, error::KuzuQueryError};
use crate::types::{RefnoEnum, RefU64};
use anyhow::Result;
use itertools::Itertools;
use kuzu::Value;

/// 多refno + 类型过滤的深层子孙查询
///
/// # 参数
/// * `refnos` - 父节点refno列表
/// * `nouns` - 要过滤的noun类型列表
/// * `max_depth` - 最大递归深度（默认 12）
///
/// # 返回
/// * `Result<Vec<RefnoEnum>>` - 匹配的子孙refno列表
pub async fn kuzu_query_multi_filter_deep_children(
    refnos: &[RefnoEnum],
    nouns: &[&str],
    max_depth: usize,
) -> Result<Vec<RefnoEnum>> {
    if refnos.is_empty() {
        return Ok(Vec::new());
    }

    let refno_list = refnos.iter().map(|r| r.refno().0).join(", ");
    let nouns_str = nouns.iter().map(|n| format!("'{}'", n)).join(", ");

    let noun_filter = if nouns.is_empty() {
        String::new()
    } else {
        format!("\n       AND descendant.noun IN [{}]", nouns_str)
    };

    let query = format!(
        "MATCH (parent:PE)-[:OWNS*1..{}]->(descendant:PE)
         WHERE parent.refno IN [{}]{}
               AND descendant.deleted = false
         RETURN DISTINCT descendant.refno",
        max_depth, refno_list, noun_filter
    );

    log::debug!("Kuzu query: {}", query);

    let conn = create_kuzu_connection()
        .map_err(|e| KuzuQueryError::ConnectionError(e.to_string()))?;

    let mut result = conn.query(&query)
        .map_err(|e| KuzuQueryError::QueryExecutionError {
            query: query.clone(),
            error: e.to_string(),
        })?;

    let mut descendants = Vec::new();

    while let Some(row) = result.next() {
        if let Some(Value::Int64(refno_val)) = row.get(0) {
            descendants.push(RefnoEnum::from(RefU64(*refno_val as u64)));
        }
    }

    log::debug!("Found {} descendants for {} parents with noun filter", descendants.len(), refnos.len());
    Ok(descendants)
}

/// SPRE过滤的深层子孙查询
///
/// # 参数
/// * `refno` - 父节点refno
/// * `max_level` - 最大递归深度（None表示12层）
///
/// # 返回
/// * `Result<Vec<RefnoEnum>>` - 不是SPRE实例的子孙refno列表
///
/// 注意：需要 TO_SPRE 关系已在 Kuzu 中创建
pub async fn kuzu_query_deep_children_filter_spre(
    refno: RefnoEnum,
    max_level: Option<usize>,
) -> Result<Vec<RefnoEnum>> {
    let depth_limit = max_level.unwrap_or(12);

    let query = format!(
        "MATCH (parent:PE {{refno: {}}})-[:OWNS*1..{}]->(descendant:PE)
         WHERE descendant.deleted = false
               AND NOT EXISTS {{ MATCH (descendant)-[:TO_SPRE]->() }}
         RETURN DISTINCT descendant.refno",
        refno.refno().0,
        depth_limit
    );

    log::debug!("Kuzu query: {}", query);

    let conn = create_kuzu_connection()
        .map_err(|e| KuzuQueryError::ConnectionError(e.to_string()))?;

    let mut result = conn.query(&query)
        .map_err(|e| KuzuQueryError::QueryExecutionError {
            query: query.clone(),
            error: e.to_string(),
        })?;

    let mut descendants = Vec::new();

    while let Some(row) = result.next() {
        if let Some(Value::Int64(refno_val)) = row.get(0) {
            descendants.push(RefnoEnum::from(RefU64(*refno_val as u64)));
        }
    }

    log::debug!("Found {} non-SPRE descendants for refno {:?}", descendants.len(), refno);
    Ok(descendants)
}

/// 多refno + SPRE过滤的深层子孙查询
///
/// # 参数
/// * `refnos` - 父节点refno列表
/// * `nouns` - 要过滤的noun类型列表
/// * `max_level` - 最大递归深度
///
/// # 返回
/// * `Result<Vec<RefnoEnum>>` - 匹配且不是SPRE实例的子孙refno列表
pub async fn kuzu_query_multi_deep_children_filter_spre(
    refnos: &[RefnoEnum],
    nouns: &[&str],
    max_level: Option<usize>,
) -> Result<Vec<RefnoEnum>> {
    if refnos.is_empty() {
        return Ok(Vec::new());
    }

    let depth_limit = max_level.unwrap_or(12);
    let refno_list = refnos.iter().map(|r| r.refno().0).join(", ");
    let nouns_str = nouns.iter().map(|n| format!("'{}'", n)).join(", ");

    let noun_filter = if nouns.is_empty() {
        String::new()
    } else {
        format!("\n       AND descendant.noun IN [{}]", nouns_str)
    };

    let query = format!(
        "MATCH (parent:PE)-[:OWNS*1..{}]->(descendant:PE)
         WHERE parent.refno IN [{}]{}
               AND descendant.deleted = false
               AND NOT EXISTS {{ MATCH (descendant)-[:TO_SPRE]->() }}
         RETURN DISTINCT descendant.refno",
        depth_limit, refno_list, noun_filter
    );

    log::debug!("Kuzu query: {}", query);

    let conn = create_kuzu_connection()
        .map_err(|e| KuzuQueryError::ConnectionError(e.to_string()))?;

    let mut result = conn.query(&query)
        .map_err(|e| KuzuQueryError::QueryExecutionError {
            query: query.clone(),
            error: e.to_string(),
        })?;

    let mut descendants = Vec::new();

    while let Some(row) = result.next() {
        if let Some(Value::Int64(refno_val)) = row.get(0) {
            descendants.push(RefnoEnum::from(RefU64(*refno_val as u64)));
        }
    }

    log::debug!("Found {} non-SPRE descendants for {} parents", descendants.len(), refnos.len());
    Ok(descendants)
}

/// 多refno + 实例化过滤的深层子孙查询
///
/// # 参数
/// * `refnos` - 父节点refno列表
/// * `nouns` - 要过滤的noun类型列表
/// * `include_spre` - 是否包含SPRE实例
///
/// # 返回
/// * `Result<Vec<RefnoEnum>>` - 匹配的子孙refno列表
pub async fn kuzu_query_multi_deep_children_filter_inst(
    refnos: &[RefnoEnum],
    nouns: &[&str],
    include_spre: bool,
) -> Result<Vec<RefnoEnum>> {
    if include_spre {
        // 包含 SPRE，直接使用标准的多条件过滤
        kuzu_query_multi_filter_deep_children(refnos, nouns, 12).await
    } else {
        // 不包含 SPRE，使用 SPRE 过滤
        kuzu_query_multi_deep_children_filter_spre(refnos, nouns, None).await
    }
}

/// 按路径前缀过滤深层子孙
///
/// # 参数
/// * `refno` - 父节点refno
/// * `path_prefix` - 路径前缀
///
/// # 返回
/// * `Result<Vec<RefnoEnum>>` - 路径匹配的子孙refno列表
///
/// 注意：需要节点有 path 属性
pub async fn kuzu_query_filter_deep_children_by_path(
    refno: RefnoEnum,
    path_prefix: &str,
) -> Result<Vec<RefnoEnum>> {
    let query = format!(
        "MATCH (parent:PE {{refno: {}}})-[:OWNS*1..12]->(descendant:PE)
         WHERE descendant.deleted = false
               AND descendant.path STARTS WITH '{}'
         RETURN DISTINCT descendant.refno",
        refno.refno().0,
        path_prefix.replace('\'', "\\'")
    );

    log::debug!("Kuzu query: {}", query);

    let conn = create_kuzu_connection()
        .map_err(|e| KuzuQueryError::ConnectionError(e.to_string()))?;

    let mut result = conn.query(&query)
        .map_err(|e| KuzuQueryError::QueryExecutionError {
            query: query.clone(),
            error: e.to_string(),
        })?;

    let mut descendants = Vec::new();

    while let Some(row) = result.next() {
        if let Some(Value::Int64(refno_val)) = row.get(0) {
            descendants.push(RefnoEnum::from(RefU64(*refno_val as u64)));
        }
    }

    log::debug!("Found {} descendants with path prefix '{}' for refno {:?}",
        descendants.len(), path_prefix, refno);
    Ok(descendants)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::RefU64;

    #[tokio::test]
    #[ignore] // 需要数据库环境
    async fn test_query_multi_filter_deep_children() {
        let refnos = vec![
            RefnoEnum::from(RefU64(123)),
            RefnoEnum::from(RefU64(456)),
        ];
        let result = kuzu_query_multi_filter_deep_children(&refnos, &["PIPE", "EQUI"]).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore]
    async fn test_query_deep_children_filter_spre() {
        let refno = RefnoEnum::from(RefU64(123));
        let result = kuzu_query_deep_children_filter_spre(refno, Some(8)).await;
        assert!(result.is_ok());
    }
}
