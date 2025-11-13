use sea_orm::DatabaseConnection;
use crate::options::DbOption;

pub mod test_table;
pub mod test_query_log;
pub mod test_insert;


pub async fn get_version_conn() -> DatabaseConnection{
    use config::{Config, File};
    let config_file_name = std::env::var("DB_OPTION_FILE").unwrap_or_else(|_| "DbOption".to_string());
    let s = Config::builder()
        .add_source(File::with_name(&config_file_name))
        .build()
        .unwrap();
    let db_option: DbOption = s.try_deserialize().unwrap();
    let conn_str = db_option.get_mysql_project_db_conn_str();
    let db = sea_orm::Database::connect(&conn_str).await.unwrap();
    db
}