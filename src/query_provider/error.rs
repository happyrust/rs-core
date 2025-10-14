//! 查询错误类型定义

use std::fmt;

/// 查询错误类型
#[derive(Debug)]
pub enum QueryError {
    /// 数据库连接错误
    ConnectionError(String),

    /// 查询执行错误
    ExecutionError(String),

    /// 数据解析错误
    ParseError(String),

    /// 数据未找到
    NotFound(String),

    /// 无效的参数
    InvalidParameter(String),

    /// 超时错误
    Timeout(String),

    /// 其他错误
    Other(Box<dyn std::error::Error + Send + Sync>),
}

impl fmt::Display for QueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QueryError::ConnectionError(msg) => write!(f, "数据库连接错误: {}", msg),
            QueryError::ExecutionError(msg) => write!(f, "查询执行错误: {}", msg),
            QueryError::ParseError(msg) => write!(f, "数据解析错误: {}", msg),
            QueryError::NotFound(msg) => write!(f, "数据未找到: {}", msg),
            QueryError::InvalidParameter(msg) => write!(f, "无效的参数: {}", msg),
            QueryError::Timeout(msg) => write!(f, "查询超时: {}", msg),
            QueryError::Other(err) => write!(f, "其他错误: {}", err),
        }
    }
}

impl std::error::Error for QueryError {}

impl From<anyhow::Error> for QueryError {
    fn from(err: anyhow::Error) -> Self {
        QueryError::Other(err.into())
    }
}

/// 查询结果类型别名
pub type QueryResult<T> = Result<T, QueryError>;
