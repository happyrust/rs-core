//! Kuzu 数据库集成测试

#[cfg(feature = "kuzu")]
pub mod test_connection;
#[cfg(feature = "kuzu")]
pub mod test_schema;
#[cfg(feature = "kuzu")]
pub mod test_types;
#[cfg(feature = "kuzu")]
pub mod test_save_model;
#[cfg(all(feature = "kuzu", feature = "surreal"))]
pub mod test_db1112_comparison;
