use anyhow::{Context, Result};
use surrealdb::Connection;
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
pub async fn query_response<C>(db: &Surreal<C>, sql: impl AsRef<str>) -> Result<Response>
where
    C: Connection,
{
    query_response_with_location(db, sql, std::panic::Location::caller()).await
}

async fn query_response_with_location<C>(
    db: &Surreal<C>,
    sql: impl AsRef<str>,
    location: &'static std::panic::Location<'static>,
) -> Result<Response>
where
    C: Connection,
{
    let sql_str = sql.as_ref();
    let location = location.to_string();
    db.query(sql_str).await.map_err(|e| {
        init_query_error(sql_str, &e, &location);
        anyhow::anyhow!("执行查询失败：{e}")
    })
}

fn is_namespace_empty_error(error: &impl std::fmt::Display) -> bool {
    let msg = error.to_string();
    msg.contains("Specify a namespace to use") || msg.contains("NamespaceEmpty")
}

fn namespace_prefixed_sql(sql: &str) -> Option<String> {
    let trimmed = sql.trim_start();
    if trimmed.to_ascii_uppercase().starts_with("USE NS ") {
        return None;
    }

    let db_option = crate::get_db_option();
    let ns = db_option.surreal_ns.replace('`', "\\`");
    let db = db_option.project_name.replace('`', "\\`");
    if ns.trim().is_empty() || db.trim().is_empty() {
        return None;
    }

    Some(format!("USE NS `{ns}` DB `{db}`;\n{sql}"))
}

impl<C> SurrealQueryExt for Surreal<C>
where
    C: Connection,
{
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
        let mut response: Response =
            match query_response_with_location(self, sql_str, location).await {
                Ok(response) => response,
                Err(e) if is_namespace_empty_error(&e) => {
                    if let Some(prefixed_sql) = namespace_prefixed_sql(sql_str) {
                        query_response_with_location(self, &prefixed_sql, location).await?
                    } else {
                        return Err(e);
                    }
                }
                Err(e) => return Err(e),
            };

        match response.take::<T>(index) {
            Ok(v) => Ok(v),
            Err(e) if is_namespace_empty_error(&e) => {
                if let Some(prefixed_sql) = namespace_prefixed_sql(sql_str) {
                    let mut response =
                        query_response_with_location(self, &prefixed_sql, location).await?;
                    response.take::<T>(index + 1).map_err(|e| {
                        error!("query_take error at {}: {}", location, e);
                        anyhow::Error::from(e)
                    })
                } else {
                    error!("query_take error at {}: {}", location, e);
                    Err(anyhow::Error::from(e))
                }
            }
            Err(e) => {
                error!("query_take error at {}: {}", location, e);
                Err(anyhow::Error::from(e))
            }
        }
        .with_context(|| format!("SQL: {sql_str} @ {}", location))
    }
}
