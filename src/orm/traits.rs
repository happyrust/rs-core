#[cfg(feature = "reflect")]
use bevy_reflect::{DynamicStruct, reflect_trait};
use sea_orm::{DatabaseBackend, DatabaseConnection};

/// ORM 数据库操作 trait
///
/// 当启用 "reflect" feature 时,此 trait 支持反射系统访问
#[cfg_attr(feature = "reflect", reflect_trait)]
pub trait DbOpTrait {
    #[cfg(feature = "reflect")]
    fn gen_insert_many(&self, models: Vec<DynamicStruct>, backend: DatabaseBackend) -> String;

    #[cfg(not(feature = "reflect"))]
    fn gen_insert_many(&self, models: Vec<serde_json::Value>, backend: DatabaseBackend) -> String;

    fn gen_create_table(&self, backend: DatabaseBackend) -> String;
}
