//! PE 节点查询
//!
//! 提供 PE (Plant Element) 的图查询功能

#[cfg(feature = "kuzu")]
use crate::types::*;
#[cfg(feature = "kuzu")]
use crate::rs_kuzu::create_kuzu_connection;
#[cfg(feature = "kuzu")]
use anyhow::Result;

#[cfg(feature = "kuzu")]
/// 查询单个 PE
pub async fn get_pe_from_kuzu(refno: RefnoEnum) -> Result<Option<SPdmsElement>> {
    let conn = create_kuzu_connection()?;

    let query = format!(
        "MATCH (pe:PE {{refno: {}}}) RETURN pe",
        refno.refno().0
    );

    let mut result = conn.query(&query)?;

    if let Some(record) = result.next() {
        let node = record.get(0)
            .ok_or_else(|| anyhow::anyhow!("无法获取PE节点"))?;

        // 从 Kuzu Value 转换为 SPdmsElement
        // TODO: 需要实现完整的转换逻辑
        log::debug!("获取到 PE 节点: {:?}", node);

        // 暂时返回空，需要实现转换
        Ok(None)
    } else {
        Ok(None)
    }
}

#[cfg(feature = "kuzu")]
/// 查询子元素
pub async fn query_children_refnos_kuzu(refno: RefnoEnum) -> Result<Vec<RefnoEnum>> {
    let conn = create_kuzu_connection()?;

    let query = format!(
        "MATCH (parent:PE {{refno: {}}})-[:OWNS]->(child:PE)
         RETURN child.refno",
        refno.refno().0
    );

    let mut result = conn.query(&query)?;
    let mut children = Vec::new();

    while let Some(record) = result.next() {
        if let Some(value) = record.get(0) {
            if let kuzu::Value::Int64(child_refno) = value {
                children.push(RefnoEnum::from(RefU64(*child_refno as u64)));
            }
        }
    }

    log::debug!("PE {} 有 {} 个子元素", refno.refno().0, children.len());

    Ok(children)
}