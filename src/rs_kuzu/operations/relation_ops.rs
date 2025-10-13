//! Owner 关系批量操作
//!
//! 提供基于 HashMap 的批量 owner 关系保存功能

#[cfg(feature = "kuzu")]
use crate::rs_kuzu::create_kuzu_connection;
#[cfg(feature = "kuzu")]
use crate::types::*;
#[cfg(feature = "kuzu")]
use anyhow::Result;
#[cfg(feature = "kuzu")]
use std::collections::HashMap;

#[cfg(feature = "kuzu")]
/// 批量保存 owner 关系
///
/// # 参数
/// * `owner_map` - HashMap<refno, owner_refno>，key 是子节点的 refno，value 是父节点（owner）的 refno
///
/// # 返回
/// 成功保存的关系数量和失败的关系数量
pub async fn batch_save_owner_relations(
    owner_map: &HashMap<u64, u64>,
) -> Result<(usize, usize)> {
    let conn = create_kuzu_connection()?;

    let mut success_count = 0;
    let mut fail_count = 0;

    conn.query("BEGIN TRANSACTION")?;

    let result = (|| {
        for (refno, owner_refno) in owner_map {
            // 跳过无效的 refno
            if *refno == 0 || *owner_refno == 0 {
                fail_count += 1;
                continue;
            }

            // 直接创建关系
            let query = format!(
                "CREATE (PE {{refno: {}}})-[:OWNS]->(PE {{refno: {}}})",
                owner_refno, refno
            );

            match conn.query(&query) {
                Ok(_) => {
                    success_count += 1;
                    log::debug!("创建 OWNS 关系: {} -> {}", owner_refno, refno);
                }
                Err(e) => {
                    fail_count += 1;
                    log::warn!(
                        "创建 OWNS 关系失败 ({}->{}): {}",
                        owner_refno,
                        refno,
                        e
                    );
                }
            }
        }
        Ok::<(), anyhow::Error>(())
    })();

    match result {
        Ok(_) => {
            conn.query("COMMIT")?;
            log::info!(
                "批量保存 owner 关系完成: 成功 {}, 失败 {}",
                success_count,
                fail_count
            );
            Ok((success_count, fail_count))
        }
        Err(e) => {
            conn.query("ROLLBACK")?;
            log::error!("批量保存 owner 关系事务失败: {}", e);
            Err(e)
        }
    }
}

#[cfg(feature = "kuzu")]
/// 创建单个关系
pub async fn create_relation_kuzu(from: RefnoEnum, to: RefnoEnum, rel_type: &str) -> Result<()> {
    use crate::types::refno::RefnoEnum;
    let conn = create_kuzu_connection()?;

    let from_refno = from.refno().0;
    let to_refno = to.refno().0;

    let query = format!(
        "MATCH (a:PE {{refno: {}}}), (b:PE {{refno: {}}}) CREATE (a)-[:{}]->(b)",
        from_refno,
        to_refno,
        rel_type
    );

    conn.query(&query)?;
    log::debug!("创建关系: {} -[{}]-> {}", from_refno, rel_type, to_refno);
    Ok(())
}

#[cfg(feature = "kuzu")]
/// 创建 PE 的所有关系（完整模式）
pub async fn create_all_relations(_pe: &SPdmsElement, _attmap: &NamedAttrMap) -> Result<()> {
    // 完整模式下创建所有关系
    // 暂时返回成功，因为在 PE + Owner 模式下不需要
    Ok(())
}

#[cfg(feature = "kuzu")]
/// 批量创建关系（完整模式）
pub async fn create_relations_batch(_models: &[(SPdmsElement, NamedAttrMap)]) -> Result<()> {
    // 完整模式下批量创建关系
    // 暂时返回成功，因为在 PE + Owner 模式下不需要
    Ok(())
}
