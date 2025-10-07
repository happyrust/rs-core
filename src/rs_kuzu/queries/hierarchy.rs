//! Kuzu 层级查询模块
//!
//! 提供层级遍历相关的查询功能，包括祖先查询、子节点查询、深层递归查询等。

use crate::rs_kuzu::{
    create_kuzu_connection,
    error::KuzuQueryError,
    query_builder::HierarchyQueryBuilder,
};
use crate::types::{RefnoEnum, RefU64};
use anyhow::Result;
use kuzu::Value;

/// 获取直接子节点的refno列表
///
/// # 参数
/// * `refno` - 父节点的refno
///
/// # 返回
/// * `Result<Vec<RefnoEnum>>` - 子节点refno列表
///
/// # 示例
/// ```no_run
/// # use aios_core::rs_kuzu::queries::hierarchy::kuzu_get_children_refnos;
/// # use aios_core::types::*;
/// # tokio_test::block_on(async {
/// let refno = RefnoEnum::from(RefU64(123));
/// let children = kuzu_get_children_refnos(refno).await.unwrap();
/// println!("子节点数量: {}", children.len());
/// # });
/// ```
pub async fn kuzu_get_children_refnos(refno: RefnoEnum) -> Result<Vec<RefnoEnum>> {
    let query = HierarchyQueryBuilder::children(refno)
        .single_depth(1)
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

    log::debug!("Found {} children for refno {:?}", refnos.len(), refno);
    Ok(refnos)
}

/// 查询所有祖先的refno列表
///
/// # 参数
/// * `refno` - 子节点的refno
///
/// # 返回
/// * `Result<Vec<RefnoEnum>>` - 祖先refno列表（从近到远）
pub async fn kuzu_query_ancestor_refnos(refno: RefnoEnum) -> Result<Vec<RefnoEnum>> {
    let query = HierarchyQueryBuilder::ancestors(refno)
        .unlimited_depth()
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

    log::debug!("Found {} ancestors for refno {:?}", refnos.len(), refno);
    Ok(refnos)
}

/// 查询特定类型的祖先
///
/// # 参数
/// * `refno` - 子节点的refno
/// * `ancestor_type` - 祖先的noun类型
///
/// # 返回
/// * `Result<Option<RefnoEnum>>` - 找到的第一个匹配的祖先
pub async fn kuzu_query_ancestor_of_type(
    refno: RefnoEnum,
    ancestor_type: &str,
) -> Result<Option<RefnoEnum>> {
    let query = HierarchyQueryBuilder::ancestors(refno)
        .unlimited_depth()
        .filter_nouns(&[ancestor_type])
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

/// 获取所有祖先的noun类型列表
///
/// # 参数
/// * `refno` - 子节点的refno
///
/// # 返回
/// * `Result<Vec<String>>` - 祖先的noun类型列表（去重）
pub async fn kuzu_get_ancestor_types(refno: RefnoEnum) -> Result<Vec<String>> {
    let query = HierarchyQueryBuilder::ancestors(refno)
        .unlimited_depth()
        .return_fields(&["noun"])
        .build();

    log::debug!("Kuzu query: {}", query);

    let conn = create_kuzu_connection()
        .map_err(|e| KuzuQueryError::ConnectionError(e.to_string()))?;

    let mut result = conn.query(&query)
        .map_err(|e| KuzuQueryError::QueryExecutionError {
            query: query.clone(),
            error: e.to_string(),
        })?;

    let mut types = Vec::new();

    while let Some(row) = result.next() {
        if let Some(Value::String(noun)) = row.get(0) {
            if !types.contains(noun) {
                types.push(noun.clone());
            }
        }
    }

    log::debug!("Found {} ancestor types for refno {:?}", types.len(), refno);
    Ok(types)
}

/// 查询所有深层子孙的refno列表（递归深度最大12层）
///
/// # 参数
/// * `refno` - 父节点的refno
///
/// # 返回
/// * `Result<Vec<RefnoEnum>>` - 所有子孙的refno列表
pub async fn kuzu_query_deep_children_refnos(refno: RefnoEnum) -> Result<Vec<RefnoEnum>> {
    let query = HierarchyQueryBuilder::children(refno)
        .depth(1, Some(12))
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

    log::debug!("Found {} deep children for refno {:?}", refnos.len(), refno);
    Ok(refnos)
}

/// 按类型过滤深层子孙
///
/// # 参数
/// * `refno` - 父节点的refno
/// * `nouns` - 要过滤的noun类型列表
///
/// # 返回
/// * `Result<Vec<RefnoEnum>>` - 匹配的子孙refno列表
pub async fn kuzu_query_filter_deep_children(
    refno: RefnoEnum,
    nouns: &[&str],
) -> Result<Vec<RefnoEnum>> {
    let query = HierarchyQueryBuilder::children(refno)
        .depth(1, Some(12))
        .filter_nouns(nouns)
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

    log::debug!("Found {} filtered deep children for refno {:?}", refnos.len(), refno);
    Ok(refnos)
}

/// 按类型过滤直接子节点
///
/// # 参数
/// * `refno` - 父节点的refno
/// * `types` - 要过滤的noun类型列表（空数组表示不过滤）
///
/// # 返回
/// * `Result<Vec<RefnoEnum>>` - 匹配的子节点refno列表
pub async fn kuzu_query_filter_children(
    refno: RefnoEnum,
    types: &[&str],
) -> Result<Vec<RefnoEnum>> {
    let builder = HierarchyQueryBuilder::children(refno)
        .single_depth(1);

    let query = if types.is_empty() {
        builder.build()
    } else {
        builder.filter_nouns(types).build()
    };

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

    log::debug!("Found {} filtered children for refno {:?}", refnos.len(), refno);
    Ok(refnos)
}

/// 按类型过滤祖先节点
///
/// # 参数
/// * `refno` - 子节点的refno
/// * `nouns` - 要过滤的noun类型列表
///
/// # 返回
/// * `Result<Vec<RefnoEnum>>` - 匹配的祖先refno列表
pub async fn kuzu_query_filter_ancestors(
    refno: RefnoEnum,
    nouns: &[&str],
) -> Result<Vec<RefnoEnum>> {
    let query = HierarchyQueryBuilder::ancestors(refno)
        .unlimited_depth()
        .filter_nouns(nouns)
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

    log::debug!("Found {} filtered ancestors for refno {:?}", refnos.len(), refno);
    Ok(refnos)
}

/// 查询所有 BRAN 和 HANG 类型的深层子孙
///
/// # 参数
/// * `refno` - 父节点的refno
///
/// # 返回
/// * `Result<Vec<RefnoEnum>>` - BRAN和HANG类型的子孙refno列表
pub async fn kuzu_query_filter_all_bran_hangs(refno: RefnoEnum) -> Result<Vec<RefnoEnum>> {
    kuzu_query_filter_deep_children(refno, &["BRAN", "HANG"]).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::RefU64;

    // 注意：这些测试需要先初始化 Kuzu 数据库和导入测试数据

    #[tokio::test]
    #[ignore] // 需要数据库环境
    async fn test_get_children_refnos() {
        let refno = RefnoEnum::from(RefU64(123));
        let result = kuzu_get_children_refnos(refno).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore]
    async fn test_query_ancestor_refnos() {
        let refno = RefnoEnum::from(RefU64(456));
        let result = kuzu_query_ancestor_refnos(refno).await;
        assert!(result.is_ok());
    }
}
