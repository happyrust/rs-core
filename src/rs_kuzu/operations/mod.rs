//! Kuzu 操作模块
//!
//! 提供数据写入和更新操作

#[cfg(feature = "kuzu")]
pub mod pe_ops;
#[cfg(feature = "kuzu")]
pub mod attr_ops;
#[cfg(feature = "kuzu")]
pub mod relation_ops;

#[cfg(feature = "kuzu")]
pub use pe_ops::*;
#[cfg(feature = "kuzu")]
pub use attr_ops::*;
#[cfg(feature = "kuzu")]
pub use relation_ops::*;