//! Kuzu 批量查询模块
//!
//! 提供批量查询功能，包括批量获取子节点、全名查询等

use crate::rs_kuzu::{create_kuzu_connection, error::KuzuQueryError};
use crate::types::{RefnoEnum, RefU64, SPdmsElement};
use anyhow::Result;
use indexmap::IndexMap;
use itertools::Itertools;
use kuzu::Value;

/// 批量获取子节点的refno列表
///
/// # 参数
/// * `refnos` - 父节点refno列表
///
/// # 返回
/// * `Result<Vec<RefnoEnum>>` - 所有父节点的子节点refno列表（去重）
pub async fn kuzu_get_all_children_refnos(refnos: &[RefnoEnum]) -> Result<Vec<RefnoEnum>> {
    if refnos.is_empty() {
        return Ok(Vec::new());
    }

    let refno_list = refnos.iter().map(|r| r.refno().0).join(", ");

    let query = format!(
        "MATCH (parent:PE)-[:OWNS]->(child:PE)
         WHERE parent.refno IN [{}] AND child.deleted = false
         RETURN DISTINCT child.refno",
        refno_list
    );

    log::debug!("Kuzu query: {}", query);

    let conn = create_kuzu_connection()
        .map_err(|e| KuzuQueryError::ConnectionError(e.to_string()))?;

    let mut result = conn.query(&query)
        .map_err(|e| KuzuQueryError::QueryExecutionError {
            query: query.clone(),
            error: e.to_string(),
        })?;

    let mut children = Vec::new();

    while let Some(row) = result.next() {
        if let Some(Value::Int64(refno_val)) = row.get(0) {
            children.push(RefnoEnum::from(RefU64(*refno_val as u64)));
        }
    }

    log::debug!("Found {} children for {} parents", children.len(), refnos.len());
    Ok(children)
}

/// 查询全名列表
///
/// # 参数
/// * `refnos` - refno列表
///
/// # 返回
/// * `Result<Vec<String>>` - 对应的全名列表
///
/// 注意：全名通过递归拼接所有祖先的name字段生成
pub async fn kuzu_query_full_names(refnos: &[RefnoEnum]) -> Result<Vec<String>> {
    if refnos.is_empty() {
        return Ok(Vec::new());
    }

    let refno_list = refnos.iter().map(|r| r.refno().0).join(", ");

    // 查询每个节点及其所有祖先的name
    let query = format!(
        "MATCH path = (child:PE)<-[:OWNS*0..]-(ancestor:PE)
         WHERE child.refno IN [{}]
         WITH child.refno as refno,
              [node IN nodes(path) | node.name] AS names,
              length(path) as depth
         ORDER BY refno, depth DESC
         RETURN refno, names",
        refno_list
    );

    log::debug!("Kuzu query: {}", query);

    let conn = create_kuzu_connection()
        .map_err(|e| KuzuQueryError::ConnectionError(e.to_string()))?;

    let mut result = conn.query(&query)
        .map_err(|e| KuzuQueryError::QueryExecutionError {
            query: query.clone(),
            error: e.to_string(),
        })?;

    let mut full_names_map: IndexMap<i64, String> = IndexMap::new();

    while let Some(row) = result.next() {
        if let (Some(Value::Int64(refno_val)), Some(Value::List(_, names_list))) = (row.get(0), row.get(1)) {
            // 只取最长路径（第一个结果，因为已经按 depth DESC 排序）
            if full_names_map.contains_key(refno_val) {
                continue;
            }

            // 拼接 name
            let names: Vec<String> = names_list.iter()
                .filter_map(|v| {
                    if let Value::String(s) = v {
                        Some(s.clone())
                    } else {
                        None
                    }
                })
                .collect();

            // 反转后用 / 连接
            let mut names_rev = names.clone();
            names_rev.reverse();
            let full_name = names_rev.join("/");

            full_names_map.insert(*refno_val, full_name);
        }
    }

    // 按原始 refnos 顺序返回结果
    let full_names: Vec<String> = refnos.iter()
        .filter_map(|r| {
            let refno_i64 = r.refno().0 as i64;
            full_names_map.get(&refno_i64).cloned()
        })
        .collect();

    log::debug!("Generated {} full names for {} refnos", full_names.len(), refnos.len());
    Ok(full_names)
}

/// 查询全名映射
///
/// # 参数
/// * `refnos` - refno列表
///
/// # 返回
/// * `Result<Vec<(RefnoEnum, String)>>` - refno到全名的元组列表
pub async fn kuzu_query_full_names_map(refnos: &[RefnoEnum]) -> Result<Vec<(RefnoEnum, String)>> {
    let full_names = kuzu_query_full_names(refnos).await?;

    let result: Vec<(RefnoEnum, String)> = refnos.iter()
        .zip(full_names.iter())
        .map(|(refno, name)| (*refno, name.clone()))
        .collect();

    Ok(result)
}

/// 查询子节点的全名映射
///
/// # 参数
/// * `refno` - 父节点refno
///
/// # 返回
/// * `Result<Vec<(RefnoEnum, String)>>` - 子节点refno到全名的元组列表
pub async fn kuzu_query_children_full_names_map(
    refno: RefnoEnum,
) -> Result<Vec<(RefnoEnum, String)>> {
    use crate::rs_kuzu::queries::hierarchy::kuzu_get_children_refnos;

    // 先获取所有子节点
    let children = kuzu_get_children_refnos(refno).await?;

    // 再查询他们的全名
    kuzu_query_full_names_map(&children).await
}

/// 批量查询 PE 元素
///
/// # 参数
/// * `refnos` - refno列表
///
/// # 返回
/// * `Result<Vec<SPdmsElement>>` - PE 元素列表（保持顺序，跳过不存在的）
pub async fn kuzu_get_pes_batch(refnos: &[RefnoEnum]) -> Result<Vec<SPdmsElement>> {
    use crate::rs_kuzu::queries::pe_query;
    pe_query::kuzu_get_pes_batch(refnos).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::RefU64;

    #[tokio::test]
    #[ignore] // 需要数据库环境
    async fn test_get_all_children_refnos() {
        let refnos = vec![
            RefnoEnum::from(RefU64(123)),
            RefnoEnum::from(RefU64(456)),
        ];
        let result = kuzu_get_all_children_refnos(&refnos).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore]
    async fn test_query_full_names() {
        let refnos = vec![
            RefnoEnum::from(RefU64(123)),
        ];
        let result = kuzu_query_full_names(&refnos).await;
        assert!(result.is_ok());
    }
}
