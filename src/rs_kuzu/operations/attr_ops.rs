//! 属性操作
//!
//! 提供属性的写入和更新操作

#[cfg(feature = "kuzu")]
use crate::rs_kuzu::create_kuzu_connection;
#[cfg(feature = "kuzu")]
use crate::types::*;
#[cfg(feature = "kuzu")]
use anyhow::Result;

#[cfg(feature = "kuzu")]
/// 保存属性（占位实现）
pub async fn save_attmap_kuzu(refno: RefnoEnum, _attmap: &NamedAttrMap) -> Result<()> {
    let _conn = create_kuzu_connection()?;

    log::debug!("保存属性: {:?}", refno);

    Ok(())
}
