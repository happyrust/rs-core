//! Kuzu 适配器创建函数

#[cfg(feature = "kuzu")]
use crate::db_adapter::kuzu_adapter::KuzuAdapter;
#[cfg(feature = "kuzu")]
use anyhow::Result;

/// 创建 Kuzu 适配器
#[cfg(feature = "kuzu")]
pub fn create_kuzu_adapter() -> Result<KuzuAdapter> {
    Ok(KuzuAdapter::new())
}