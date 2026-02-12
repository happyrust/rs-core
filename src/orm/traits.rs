use sea_orm::{DatabaseBackend, DatabaseConnection};

/// ORM 数据库操作 trait
///
pub trait DbOpTrait {
    fn gen_insert_many(&self, models: Vec<serde_json::Value>, backend: DatabaseBackend) -> String;

    fn gen_create_table(&self, backend: DatabaseBackend) -> String;
}
