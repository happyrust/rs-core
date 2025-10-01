//! 图遍历查询
//!
//! 提供高级图遍历功能

#[cfg(feature = "kuzu")]
use super::pe_query::node_refno;
#[cfg(feature = "kuzu")]
use crate::rs_kuzu::create_kuzu_connection;
#[cfg(feature = "kuzu")]
use crate::types::*;
#[cfg(feature = "kuzu")]
use anyhow::{Context, Result, anyhow};

#[cfg(feature = "kuzu")]
/// 最短路径查询
pub async fn shortest_path_kuzu(from: RefnoEnum, to: RefnoEnum) -> Result<Vec<RefnoEnum>> {
    let conn = create_kuzu_connection()?;

    let query = format!(
        "MATCH (start:PE {{refno: {}}}),
               (end:PE {{refno: {}}}),
               p = SHORTEST_PATH((start)-[:OWNS|REFERS_TO*..10]-(end))
         RETURN nodes(p) AS path",
        from.refno().0,
        to.refno().0
    );

    let mut result = conn.query(&query)?;

    if let Some(record) = result.next() {
        let value = record
            .get(0)
            .with_context(|| "最短路径查询结果缺少路径列".to_string())?;

        match value {
            kuzu::Value::List(_, nodes) => {
                let mut path = Vec::new();
                for node_value in nodes {
                    match node_value {
                        kuzu::Value::Node(node) => {
                            let refno = node_refno(node)?;
                            path.push(refno);
                        }
                        other => {
                            log::warn!("最短路径结果包含非节点值: {:?}", other);
                        }
                    }
                }

                return Ok(path);
            }
            other => {
                return Err(anyhow!("最短路径查询返回的列类型不是节点列表: {:?}", other));
            }
        }
    }

    log::debug!("未找到从 {} 到 {} 的路径", from.refno().0, to.refno().0);
    Ok(vec![])
}

#[cfg(feature = "kuzu")]
/// 查询子树（深度优先遍历）
pub async fn query_subtree_kuzu(root: RefnoEnum, max_depth: Option<u32>) -> Result<Vec<RefnoEnum>> {
    let conn = create_kuzu_connection()?;

    let depth = max_depth.unwrap_or(5);
    let query = format!(
        "MATCH (root:PE {{refno: {}}})-[:OWNS*1..{}]->(descendant:PE)
         RETURN DISTINCT descendant.refno",
        root.refno().0,
        depth
    );

    let mut result = conn.query(&query)?;
    let mut subtree = vec![root]; // 包含根节点

    while let Some(record) = result.next() {
        if let Some(value) = record.get(0) {
            if let kuzu::Value::Int64(refno) = value {
                subtree.push(RefnoEnum::from(RefU64(*refno as u64)));
            }
        }
    }

    log::debug!(
        "子树查询: 根节点 {}, 深度 {}, 找到 {} 个节点",
        root.refno().0,
        depth,
        subtree.len()
    );

    Ok(subtree)
}

#[cfg(feature = "kuzu")]
/// 查询连通分量
pub async fn find_connected_component_kuzu(refno: RefnoEnum) -> Result<Vec<RefnoEnum>> {
    let conn = create_kuzu_connection()?;

    // 使用无向查询找到所有连通的节点
    let query = format!(
        "MATCH (start:PE {{refno: {}}})-[:OWNS|REFERS_TO*]-(connected:PE)
         RETURN DISTINCT connected.refno",
        refno.refno().0
    );

    let mut result = conn.query(&query)?;
    let mut component = vec![refno]; // 包含起始节点

    while let Some(record) = result.next() {
        if let Some(value) = record.get(0) {
            if let kuzu::Value::Int64(connected_refno) = value {
                component.push(RefnoEnum::from(RefU64(*connected_refno as u64)));
            }
        }
    }

    log::debug!(
        "连通分量: 节点 {} 连通到 {} 个节点",
        refno.refno().0,
        component.len()
    );

    Ok(component)
}
