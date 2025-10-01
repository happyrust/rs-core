//! SurrealDB 适配器创建函数

use crate::db_adapter::surreal_adapter::SurrealAdapter;
use anyhow::Result;

/// 创建 SurrealDB 适配器
pub fn create_surreal_adapter() -> Result<SurrealAdapter> {
    Ok(SurrealAdapter::new())
}
