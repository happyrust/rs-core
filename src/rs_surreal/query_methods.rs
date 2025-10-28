//! Database query methods for geometry operations
//!
//! This module contains all the database query methods and utilities
//! for geometry generation and boolean operations.

use crate::SurrealQueryExt;
use crate::rs_surreal::query_structs::*;
use anyhow::{Result, anyhow};
use serde_json::Value as JsonValue;
use std::path::PathBuf;

/// Database query error handling macro
///
/// Automatically captures and prints detailed error information, including:
/// - Error location (file name and line number)
/// - Currently processed refno (if provided)
/// - Error details
/// - SQL query statement
///
/// # Usage Examples
///
/// ```rust
/// // Without refno
/// let response = query_db!(sql)?;
///
/// // With refno
/// let response = query_db!(sql, Some(refno))?;
/// ```
#[macro_export]
macro_rules! query_db {
    ($sql:expr, $refno:expr) => {{
        let location = format!("{}:{}", file!(), line!());
        log::trace!("📝 执行查询: {}", $sql);

        match crate::rs_surreal::SUL_DB.query_response($sql).await {
            Ok(response) => {
                log::trace!("✅ 查询成功");
                Ok(response)
            }
            Err(e) => {
                eprintln!("\n❌ 数据库查询失败");
                eprintln!("  📍 位置: {}", location);
                if let Some(r) = $refno {
                    eprintln!("  🔖 Refno: {}", r);
                }
                eprintln!("  ⚠️  错误: {}", e);
                eprintln!("  📄 SQL: {}", $sql);
                eprintln!();
                Err(anyhow::anyhow!("数据库查询失败: {}", e))
            }
        }
    }};
    ($sql:expr) => {
        query_db!($sql, None::<crate::types::RefnoEnum>)
    };
}
