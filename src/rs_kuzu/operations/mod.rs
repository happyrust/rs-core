//! Kuzu 操作模块
//!
//! 提供数据写入和更新操作

#[cfg(feature = "kuzu")]
pub mod attr_ops;
#[cfg(feature = "kuzu")]
pub mod pe_ops;
#[cfg(feature = "kuzu")]
pub mod relation_ops;

#[cfg(feature = "kuzu")]
pub use attr_ops::*;
#[cfg(feature = "kuzu")]
pub use pe_ops::*;
#[cfg(feature = "kuzu")]
pub use relation_ops::*;

#[cfg(feature = "kuzu")]
use crate::rs_kuzu::create_kuzu_connection;
#[cfg(feature = "kuzu")]
use crate::types::*;
#[cfg(feature = "kuzu")]
use anyhow::Result;

#[cfg(feature = "kuzu")]
/// 保存完整模型到 Kuzu (PE + 属性 + 关系)
pub async fn save_model_to_kuzu(pe: &SPdmsElement, attmap: &NamedAttrMap) -> Result<()> {
    let conn = create_kuzu_connection()?;

    // 提取 TYPEX 值
    let mut pe_with_typex = pe.clone();
    pe_with_typex.extract_typex(attmap);

    // 开启事务
    conn.query("BEGIN TRANSACTION")?;

    let result = async {
        // 1. 保存 PE 节点（包含 typex）
        pe_ops::save_pe_node(&pe_with_typex).await?;

        // 2. 保存属性节点
        attr_ops::save_attr_node(&pe_with_typex, attmap).await?;

        // 3. 创建关系
        relation_ops::create_all_relations(&pe_with_typex, attmap).await?;

        Ok::<(), anyhow::Error>(())
    }
    .await;

    match result {
        Ok(_) => {
            conn.query("COMMIT")?;
            log::info!(
                "成功保存模型: {} ({}) refno={} typex={:?}",
                pe_with_typex.name,
                pe_with_typex.noun,
                pe_with_typex.refno.refno(),
                pe_with_typex.typex
            );
            Ok(())
        }
        Err(e) => {
            conn.query("ROLLBACK")?;
            log::error!("保存模型失败 {}: {}", pe_with_typex.refno.refno(), e);
            Err(e)
        }
    }
}

#[cfg(feature = "kuzu")]
/// 保存完整模型到 Kuzu (无事务版本 - 用于避免事务冲突)
pub async fn save_model_to_kuzu_no_transaction(
    pe: &SPdmsElement,
    attmap: &NamedAttrMap,
) -> Result<()> {
    // 提取 TYPEX 值
    let mut pe_with_typex = pe.clone();
    pe_with_typex.extract_typex(attmap);

    // 直接保存，不使用事务
    // 1. 保存 PE 节点（包含 typex）
    pe_ops::save_pe_node(&pe_with_typex).await?;

    // 2. 保存属性节点
    attr_ops::save_attr_node(&pe_with_typex, attmap).await?;

    // 3. 创建关系
    relation_ops::create_all_relations(&pe_with_typex, attmap).await?;

    log::debug!(
        "成功保存模型 (无事务): {} ({}) refno={} typex={:?}",
        pe_with_typex.name,
        pe_with_typex.noun,
        pe_with_typex.refno.refno(),
        pe_with_typex.typex
    );

    Ok(())
}

#[cfg(feature = "kuzu")]
/// 批量保存模型
pub async fn save_models_batch(models: Vec<(SPdmsElement, NamedAttrMap)>) -> Result<()> {
    let conn = create_kuzu_connection()?;

    // 先尝试清理任何可能存在的事务
    let _ = conn.query("ROLLBACK");

    conn.query("BEGIN TRANSACTION")?;

    let result = async {
        // 提取所有 PE 的 TYPEX 并保存
        let mut pes_with_typex: Vec<SPdmsElement> = models
            .iter()
            .map(|(pe, attmap)| {
                let mut pe_clone = pe.clone();
                pe_clone.extract_typex(attmap);
                pe_clone
            })
            .collect();

        pe_ops::save_pe_batch(&pes_with_typex).await?;

        // 批量保存属性节点（使用带 typex 的 PE）
        let models_with_typex: Vec<_> = pes_with_typex
            .iter()
            .zip(models.iter())
            .map(|(pe, (_, attmap))| (pe.clone(), attmap.clone()))
            .collect();

        attr_ops::save_attr_batch(&models_with_typex).await?;

        // 批量创建关系
        relation_ops::create_relations_batch(&models_with_typex).await?;

        Ok::<(), anyhow::Error>(())
    }
    .await;

    match result {
        Ok(_) => {
            conn.query("COMMIT")?;
            log::info!("批量保存模型成功: {} 个", models.len());
            Ok(())
        }
        Err(e) => {
            conn.query("ROLLBACK")?;
            log::error!("批量保存模型失败: {}", e);
            Err(e)
        }
    }
}

#[cfg(feature = "kuzu")]
/// 保存模型列表 (从 NamedAttrMap 列表)
pub async fn save_attmaps_to_kuzu(attmaps: Vec<NamedAttrMap>, dbnum: i32) -> Result<()> {
    let models: Vec<(SPdmsElement, NamedAttrMap)> = attmaps
        .into_iter()
        .map(|attmap| {
            let mut pe = attmap.pe(dbnum);
            // TYPEX 会在 save_models_batch 中提取
            (pe, attmap)
        })
        .collect();

    save_models_batch(models).await
}

// 导出 PE + Owner 模式函数
#[cfg(feature = "kuzu")]
pub use pe_ops::{save_pe_nodes_batch, save_pe_owns_batch, save_pe_owns_from_map};
