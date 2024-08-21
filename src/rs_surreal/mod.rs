pub mod datacenter_query;
pub mod geom;
pub mod graph;
pub mod index;
pub mod mdb;
pub mod query;

pub mod cate;
pub mod resolve;
pub mod spatial;
mod table_const;
pub mod uda;

pub mod pbs;

pub mod inst;

pub mod point;

pub mod function;

pub mod version;

pub mod e3d_db;

pub use e3d_db::*;
pub use geom::*;
pub use graph::*;
pub use index::*;
pub use inst::*;
pub use mdb::*;
pub use point::*;
pub use query::*;
pub use cate::*;
pub use resolve::*;
pub use spatial::*;
pub use uda::*;
pub use pbs::*;

use once_cell::sync::Lazy;
use surrealdb::engine::any::Any;
use surrealdb::opt::auth::Root;
use surrealdb::Surreal;

// pub type SurlValue = surrealdb::Value;
pub type SurlValue = surrealdb::sql::Value;
pub type SurlStrand = surrealdb::sql::Strand;
pub static SUL_DB: Lazy<Surreal<Any>> = Lazy::new(Surreal::init);

///连接surreal
pub async fn connect_surdb(
    conn_str: &str,
    ns: &str,
    db: &str,
    username: &str,
    password: &str,
) -> Result<(), surrealdb::Error> {
    SUL_DB.connect(conn_str).with_capacity(1000).await?;
    SUL_DB.use_ns(ns).use_db(db).await?;
    SUL_DB.signin(Root { username, password }).await?;
    Ok(())
}

pub fn convert_to_sql_str_array(nouns: &[&str]) -> String {
    let nouns_str = nouns
        .iter()
        .map(|s| format!("'{s}'"))
        .collect::<Vec<_>>()
        .join(",");
    nouns_str
}
