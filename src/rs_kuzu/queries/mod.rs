//! Kuzu 查询模块
//!
//! 提供各种图查询功能

#[cfg(feature = "kuzu")]
pub mod attr_query;
#[cfg(feature = "kuzu")]
pub mod graph_traverse;
#[cfg(feature = "kuzu")]
pub mod pe_query;
#[cfg(feature = "kuzu")]
pub mod relation_query;

// 新增的查询模块
#[cfg(feature = "kuzu")]
pub mod hierarchy;
#[cfg(feature = "kuzu")]
pub mod type_filter;
#[cfg(feature = "kuzu")]
pub mod batch;
#[cfg(feature = "kuzu")]
pub mod multi_filter;

#[cfg(feature = "kuzu")]
pub use attr_query::*;
#[cfg(feature = "kuzu")]
pub use graph_traverse::*;
#[cfg(feature = "kuzu")]
pub use pe_query::*;
#[cfg(feature = "kuzu")]
pub use relation_query::*;

// 导出新模块
#[cfg(feature = "kuzu")]
pub use hierarchy::*;
#[cfg(feature = "kuzu")]
pub use type_filter::*;
#[cfg(feature = "kuzu")]
pub use batch::*;
#[cfg(feature = "kuzu")]
pub use multi_filter::*;
