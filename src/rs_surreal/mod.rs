pub mod query;
pub mod graph;
pub mod spatial;
pub mod geom;
pub mod mdb;
pub mod uda;
pub mod resolve;
pub mod index;

use anyhow::Ok;
pub use query::*;
pub use graph::*;
pub use spatial::*;
pub use geom::*;
pub use mdb::*;
pub use uda::*;
pub use resolve::*;
pub use index::*;

use once_cell::sync::Lazy;
use surrealdb::engine::any::Any;
use surrealdb::Surreal;

pub type SurlValue = surrealdb::sql::Value;
pub static SUL_DB: Lazy<Surreal<Any>> = Lazy::new(Surreal::init);

///连接surreal
pub async fn connect_surdb(conn_str: &str, ns: &str, db: &str) -> anyhow::Result<()> {
    SUL_DB.connect(conn_str).with_capacity(1000).await?;
    SUL_DB.use_ns(ns).use_db(db).await?;
    Ok(())
}
