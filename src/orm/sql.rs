use crate::orm::traits::*;
use crate::tool::db_tool::db1_dehash;
use crate::{get_default_pdms_db_info, orm};
use anyhow::anyhow;
use sea_orm::DatabaseBackend;

pub fn get_all_create_table_sqls() -> anyhow::Result<Vec<String>> {
    let db_info = get_default_pdms_db_info(); // 获取默认的数据库信息

    let mut sqls = vec![gen_create_table_sql_reflect("pdms_element")?];
    let type_sqls = db_info.gen_all_create_table_sql();
    sqls.extend_from_slice(&type_sqls);
    Ok(sqls)
}

pub fn gen_create_table_sql_reflect(type_name: &str) -> anyhow::Result<String> {
    match type_name.trim().to_ascii_lowercase().as_str() {
        "pdms_element" | "pdmselement" => {
            Ok(orm::pdms_element::Model::default().gen_create_table(DatabaseBackend::MySql))
        }
        "box" => Ok(orm::BOX::Model::default().gen_create_table(DatabaseBackend::MySql)),
        "cyli" => Ok(orm::CYLI::Model::default().gen_create_table(DatabaseBackend::MySql)),
        _ => Err(anyhow!("Type '{}' not supported", type_name)),
    }
}

pub fn gen_insert_many_sql(
    type_name: &str,
    data_vec: Vec<serde_json::Value>,
) -> anyhow::Result<String> {
    match type_name.trim().to_ascii_lowercase().as_str() {
        "pdms_element" | "pdmselement" => {
            Ok(orm::pdms_element::Model::default()
                .gen_insert_many(data_vec, DatabaseBackend::MySql))
        }
        "box" => Ok(orm::BOX::Model::default().gen_insert_many(data_vec, DatabaseBackend::MySql)),
        "cyli" => Ok(orm::CYLI::Model::default().gen_insert_many(data_vec, DatabaseBackend::MySql)),
        _ => Err(anyhow!("Type '{}' not supported", type_name)),
    }
}
