use anyhow::{Context, Result};
use surrealdb::IndexedResults as Response;
use surrealdb::opt::QueryResult as SurrealQueryResult;
use surrealdb::Surreal;
use surrealdb::engine::any::Any;
use surrealdb::types::SurrealValue;

use crate::error::init_query_error;

/// 为 `Surreal<Any>` 提供更友好的查询接口。
pub trait SurrealQueryExt {
    /// 执行查询并返回完整的 `Response`。
    #[track_caller]
    async fn query_response(&self, sql: &str) -> Result<Response>;

    /// 执行查询并将第 `index` 个结果反序列化为目标类型。
    #[track_caller]
    async fn query_take<T>(&self, sql: &str, index: usize) -> Result<T>
    where
        T: SurrealValue,
        usize: SurrealQueryResult<T>;
}

#[track_caller]
pub async fn query_response(db: &Surreal<Any>, sql: &str) -> Result<Response> {
    query_response_with_location(db, sql, std::panic::Location::caller()).await
}

async fn query_response_with_location(
    db: &Surreal<Any>,
    sql: &str,
    location: &'static std::panic::Location<'static>,
) -> Result<Response> {
    let location = location.to_string();
    db.query(sql).await.map_err(|e| {
        init_query_error(sql, &e, &location);
        anyhow::anyhow!("执行查询失败：{e}")
    })
}

impl SurrealQueryExt for Surreal<Any> {
    #[track_caller]
    async fn query_response(&self, sql: &str) -> Result<Response> {
        let location = std::panic::Location::caller();
        query_response_with_location(self, sql, location).await
    }

    #[track_caller]
    async fn query_take<T>(&self, sql: &str, index: usize) -> Result<T>
    where
        T: SurrealValue,
        usize: SurrealQueryResult<T>,
    {
        let location = std::panic::Location::caller();
        let mut response: Response =
            query_response_with_location(self, sql, location).await?;
        response
            .take::<T>(index)
            .map_err(anyhow::Error::from)
            .with_context(|| format!("SQL: {sql} @ {}", location))
    }
}
