pub mod query;
pub mod graph;
pub mod spatial;
pub mod geom;
pub mod mdb;
pub mod uda;
pub mod resolve;
pub mod index;
pub mod datacenter_query;
mod table_const;

pub mod inst;

pub mod point;

pub mod function;

pub mod version;

pub mod e3d_db;



pub use query::*;
pub use graph::*;
pub use spatial::*;
pub use geom::*;
pub use mdb::*;
pub use uda::*;
pub use resolve::*;
pub use index::*;
pub use inst::*;
pub use point::*;
pub use e3d_db::*;
// pub use room::*;

use once_cell::sync::Lazy;
use surrealdb::engine::any::Any;
use surrealdb::Surreal;

pub type SurlValue = surrealdb::sql::Value;
pub type SurlStrand = surrealdb::sql::Strand;
pub static SUL_DB: Lazy<Surreal<Any>> = Lazy::new(Surreal::init);

//Error
///连接surreal
pub async fn connect_surdb(conn_str: &str, ns: &str, db: &str) -> Result<(), surrealdb::Error> {
    SUL_DB.connect(conn_str).with_capacity(1000).await?;
    SUL_DB.use_ns(ns).use_db(db).await?;
    Ok(())
}

pub fn convert_to_sql_str_array(nouns: &Vec<String>) -> String {
    let nouns_str = nouns
        .iter()
        .map(|s| format!("'{s}'"))
        .collect::<Vec<_>>()
        .join(",");
    nouns_str
}
