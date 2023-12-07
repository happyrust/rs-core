use crate::options::DbOption;
use config::{Config, File};
use surrealdb::engine::remote::ws::Ws;
use crate::SUL_DB;

pub mod test_mdb;
pub mod test_query_fuzzy;

pub mod test_query_regex;

pub mod test_basic_query;

pub mod test_query_group;

pub mod test_graph;

pub mod test_serde;

pub mod test_spatial;

pub mod test_geom;

pub async fn init_test_surreal() {
    let s = Config::builder()
        .add_source(File::with_name("DbOption"))
        .build()
        .unwrap();
    let db_option: DbOption = s.try_deserialize().unwrap();
    SUL_DB
        .connect::<Ws>(db_option.get_version_db_conn_str())
        .with_capacity(1000)
        .await
        .unwrap();
    SUL_DB
        .use_ns(&db_option.project_code)
        .use_db(&db_option.project_name)
        .await
        .unwrap();
}
