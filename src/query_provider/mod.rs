//! 统一查询接口架构
//!
//! 提供跨数据库的统一查询接口
//!
//! # 设计理念
//!
//! - **统一接口**: 所有数据库实现相同的 `QueryProvider` trait
//! - **类型安全**: 使用 Rust 类型系统确保查询正确性
//! - **异步优先**: 全面支持 async/await
//! - **可扩展**: 易于添加新的数据库实现
//!
//! # 使用示例
//!
//! ```rust,ignore
//! use aios_core::query_provider::{QueryProvider, SurrealQueryProvider};
//!
//! async fn example() -> Result<()> {
//!     // 创建查询提供者
//!     let provider = SurrealQueryProvider::new()?;
//!
//!     // 使用统一接口查询
//!     let children = provider.get_children(refno).await?;
//!     let pipes = provider.query_by_type(&["PIPE"], 1112).await?;
//!
//!     Ok(())
//! }
//! ```

pub mod error;
pub mod router;
pub mod surreal_provider;
pub mod traits;
pub mod tree_index_provider;

pub use error::{QueryError, QueryResult};
pub use router::{QueryEngine, QueryRouter, QueryStrategy};
pub use surreal_provider::SurrealQueryProvider;
pub use traits::{BatchQuery, GraphQuery, HierarchyQuery, QueryProvider, TypeQuery};
pub use tree_index_provider::TreeIndexQueryProvider;
