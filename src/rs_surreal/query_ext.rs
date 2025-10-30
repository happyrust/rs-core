use anyhow::{Context, Result};
use surrealdb::IndexedResults as Response;
use surrealdb::Surreal;
use surrealdb::engine::any::Any;
use surrealdb::opt::QueryResult as SurrealQueryResult;
use surrealdb::types::SurrealValue;

use crate::error::init_query_error;
use log::error;

/// 为 `Surreal<Any>` 提供更友好的查询接口。
pub trait SurrealQueryExt {
    /// 执行查询并返回完整的 `Response`。
    /// 
    /// # Arguments
    /// 
    /// * `sql` - SQL 查询语句，可以是 `&str`, `String`, 或任何实现了 `AsRef<str>` 的类型
    #[track_caller]
    async fn query_response(&self, sql: impl AsRef<str>) -> Result<Response>;

    /// 执行查询并将第 `index` 个结果反序列化为目标类型。
    /// 
    /// # Arguments
    /// 
    /// * `sql` - SQL 查询语句，可以是 `&str`, `String`, 或任何实现了 `AsRef<str>` 的类型
    /// * `index` - 要提取的结果索引
    #[track_caller]
    async fn query_take<T>(&self, sql: impl AsRef<str>, index: usize) -> Result<T>
    where
        T: SurrealValue,
        usize: SurrealQueryResult<T>;
}

#[track_caller]
pub async fn query_response(db: &Surreal<Any>, sql: impl AsRef<str>) -> Result<Response> {
    query_response_with_location(db, sql, std::panic::Location::caller()).await
}

async fn query_response_with_location(
    db: &Surreal<Any>,
    sql: impl AsRef<str>,
    location: &'static std::panic::Location<'static>,
) -> Result<Response> {
    let sql_str = sql.as_ref();
    let location = location.to_string();
    db.query(sql_str).await.map_err(|e| {
        init_query_error(sql_str, &e, &location);
        anyhow::anyhow!("执行查询失败：{e}")
    })
}

impl SurrealQueryExt for Surreal<Any> {
    #[track_caller]
    async fn query_response(&self, sql: impl AsRef<str>) -> Result<Response> {
        let location = std::panic::Location::caller();
        query_response_with_location(self, sql, location).await
    }

    #[track_caller]
    async fn query_take<T>(&self, sql: impl AsRef<str>, index: usize) -> Result<T>
    where
        T: SurrealValue,
        usize: SurrealQueryResult<T>,
    {
        let location = std::panic::Location::caller();
        let sql_str = sql.as_ref();
        let mut response: Response = query_response_with_location(self, sql_str, location).await?;
        response
            .take::<T>(index)
            .map_err(|e| {
                error!("query_take error at {}: {}", location, e);
                anyhow::Error::from(e)
            })
            .with_context(|| format!("SQL: {sql_str} @ {}", location))
    }
}
