//! 数据库适配器模块
//!
//! 提供统一的数据库接口抽象，目前仅支持 SurrealDB 后端

pub mod traits;

#[cfg(not(target_arch = "wasm32"))]
pub mod surreal_adapter;

pub use traits::*;

#[cfg(not(target_arch = "wasm32"))]
pub use surreal_adapter::SurrealAdapter;
