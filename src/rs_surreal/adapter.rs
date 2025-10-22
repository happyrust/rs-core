//! SurrealDB 适配器创建函数

#[cfg(not(target_arch = "wasm32"))]
use crate::db_adapter::surreal_adapter::SurrealAdapter;
use anyhow::Result;

/// 创建 SurrealDB 适配器
#[cfg(not(target_arch = "wasm32"))]
pub fn create_surreal_adapter() -> Result<SurrealAdapter> {
    Ok(SurrealAdapter::new())
}

/// WASM 环境下的占位符实现
#[cfg(target_arch = "wasm32")]
pub fn create_surreal_adapter() -> Result<()> {
    // WASM 环境下不使用 SurrealAdapter，返回成功以避免编译错误
    Ok(())
}
