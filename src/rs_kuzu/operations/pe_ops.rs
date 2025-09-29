//! PE 操作
//!
//! 提供 PE 节点的写入和更新操作

#[cfg(feature = "kuzu")]
use crate::types::*;
#[cfg(feature = "kuzu")]
use crate::rs_kuzu::create_kuzu_connection;
#[cfg(feature = "kuzu")]
use anyhow::Result;

#[cfg(feature = "kuzu")]
/// 保存 PE（占位实现）
pub async fn save_pe_kuzu(pe: &SPdmsElement) -> Result<()> {
    let _conn = create_kuzu_connection()?;

    log::debug!("保存 PE: {:?}", pe.refno);

    Ok(())
}

#[cfg(feature = "kuzu")]
/// 批量保存 PE（占位实现）
pub async fn save_pe_batch_kuzu(pes: Vec<SPdmsElement>) -> Result<()> {
    let _conn = create_kuzu_connection()?;

    log::debug!("批量保存 PE: {} 个", pes.len());

    Ok(())
}