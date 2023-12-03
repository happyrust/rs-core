pub mod query;

use anyhow::Ok;
pub use query::*;

use once_cell::sync::Lazy;
use surrealdb::engine::remote::ws::{Client, Ws};
use surrealdb::Surreal;

pub type SurlValue = surrealdb::sql::Value;
pub static SUL_DB: Lazy<Surreal<Client>> = Lazy::new(Surreal::init);

///连接surreal
pub async fn connect_surdb(conn_str: &str, ns: &str, db: &str) -> anyhow::Result<()> {
    SUL_DB.connect::<Ws>(conn_str).with_capacity(1000).await?;
    SUL_DB.use_ns(ns).use_db(db).await?;
    Ok(())
}
