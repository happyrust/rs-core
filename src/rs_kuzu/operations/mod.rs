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
pub async fn save_model_to_kuzu(
    pe: &SPdmsElement,
    attmap: &NamedAttrMap
) -> Result<()> {
    let conn = create_kuzu_connection()?;

    // 开启事务
    conn.query("BEGIN TRANSACTION")?;

    let result = async {
        // 1. 保存 PE 节点
        pe_ops::save_pe_node(pe).await?;

        // 2. 保存属性节点
        attr_ops::save_attr_node(pe, attmap).await?;

        // 3. 创建关系
        relation_ops::create_all_relations(pe, attmap).await?;

        Ok::<(), anyhow::Error>(())
    }.await;

    match result {
        Ok(_) => {
            conn.query("COMMIT")?;
            log::info!("成功保存模型: {} ({}) refno={}", pe.name, pe.noun, pe.refno.refno());
            Ok(())
        }
        Err(e) => {
            conn.query("ROLLBACK")?;
            log::error!("保存模型失败 {}: {}", pe.refno.refno(), e);
            Err(e)
        }
    }
}

#[cfg(feature = "kuzu")]
/// 批量保存模型
pub async fn save_models_batch(
    models: Vec<(SPdmsElement, NamedAttrMap)>
) -> Result<()> {
    let conn = create_kuzu_connection()?;

    conn.query("BEGIN TRANSACTION")?;

    let result = async {
        // 批量保存 PE 节点
        let pes: Vec<_> = models.iter().map(|(pe, _)| pe.clone()).collect();
        pe_ops::save_pe_batch(&pes).await?;

        // 批量保存属性节点
        attr_ops::save_attr_batch(&models).await?;

        // 批量创建关系
        relation_ops::create_relations_batch(&models).await?;

        Ok::<(), anyhow::Error>(())
    }.await;

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
pub async fn save_attmaps_to_kuzu(
    attmaps: Vec<NamedAttrMap>,
    dbnum: i32
) -> Result<()> {
    let models: Vec<(SPdmsElement, NamedAttrMap)> = attmaps
        .into_iter()
        .map(|attmap| {
            let pe = attmap.pe(dbnum);
            (pe, attmap)
        })
        .collect();

    save_models_batch(models).await
}
