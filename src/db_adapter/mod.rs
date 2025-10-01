//! 数据库适配器模块
//!
//! 提供统一的数据库接口抽象，支持多种数据库后端

pub mod config;
pub mod traits;

#[cfg(not(target_arch = "wasm32"))]
pub mod surreal_adapter;

#[cfg(feature = "kuzu")]
pub mod kuzu_adapter;

pub mod hybrid_manager;

pub use config::*;
pub use traits::*;

#[cfg(not(target_arch = "wasm32"))]
pub use surreal_adapter::SurrealAdapter;

#[cfg(feature = "kuzu")]
pub use kuzu_adapter::KuzuAdapter;

pub use hybrid_manager::*;
