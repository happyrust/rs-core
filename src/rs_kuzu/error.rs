//! Kuzu 错误处理模块

use std::fmt;

/// Kuzu 查询错误类型
#[derive(Debug)]
pub enum KuzuQueryError {
    /// 数据库未初始化
    DatabaseNotInitialized,

    /// 连接错误
    ConnectionError(String),

    /// 查询执行错误
    QueryExecutionError { query: String, error: String },

    /// 结果解析错误
    ResultParseError {
        expected_type: String,
        error: String,
    },

    /// 参数错误
    InvalidParameter { param: String, reason: String },

    /// 其他错误
    Other(String),
}

impl fmt::Display for KuzuQueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DatabaseNotInitialized => {
                write!(f, "Kuzu 数据库未初始化")
            }
            Self::ConnectionError(msg) => {
                write!(f, "Kuzu 连接错误: {}", msg)
            }
            Self::QueryExecutionError { query, error } => {
                write!(f, "Kuzu 查询执行失败\n查询: {}\n错误: {}", query, error)
            }
            Self::ResultParseError {
                expected_type,
                error,
            } => {
                write!(
                    f,
                    "Kuzu 结果解析失败\n期望类型: {}\n错误: {}",
                    expected_type, error
                )
            }
            Self::InvalidParameter { param, reason } => {
                write!(f, "参数错误 '{}': {}", param, reason)
            }
            Self::Other(msg) => {
                write!(f, "Kuzu 错误: {}", msg)
            }
        }
    }
}

impl std::error::Error for KuzuQueryError {}

impl From<kuzu::Error> for KuzuQueryError {
    fn from(err: kuzu::Error) -> Self {
        Self::Other(err.to_string())
    }
}

impl From<anyhow::Error> for KuzuQueryError {
    fn from(err: anyhow::Error) -> Self {
        Self::Other(err.to_string())
    }
}

// 移除与 anyhow 冲突的 From 实现
// anyhow 已经为所有实现 std::error::Error 的类型提供了 From 实现
