use crate::options::DbOption;
use config::{Config, File};
use surrealdb::opt::auth::Root;
use crate::function::define_common_functions;
use crate::SUL_DB;

pub mod test_mdb;
pub mod test_query_fuzzy;

pub mod test_query_regex;

pub mod test_basic_query;

pub mod test_query_group;

pub mod test_graph;

pub mod test_serde;

pub mod test_spatial;

pub mod test_room;

pub mod test_geom;

pub mod test_uda;



pub async fn init_test_surreal() -> DbOption {
    let s = Config::builder()
        .add_source(File::with_name("DbOption"))
        .build()
        .unwrap();
    let db_option: DbOption = s.try_deserialize().unwrap();
    SUL_DB
        .connect(db_option.get_version_db_conn_str())
        .with_capacity(1000)
        .await
        .unwrap();
    SUL_DB
        .use_ns(&db_option.project_code)
        .use_db(&db_option.project_name)
        .await
        .unwrap();
    SUL_DB
        .signin(Root {
            username: &db_option.v_user,
            password: &db_option.v_password,
        }).await.unwrap();
    define_common_functions().await.unwrap();
    db_option
}

