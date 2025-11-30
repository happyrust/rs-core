use crate::orm::traits::*;
use crate::tool::db_tool::db1_dehash;
use crate::{get_default_pdms_db_info, orm};
use anyhow::anyhow;
#[cfg(feature = "reflect")]
use bevy_reflect::{DynamicStruct, ReflectFromReflect};
use sea_orm::DatabaseBackend;

#[cfg(feature = "reflect")]
pub fn get_all_create_table_sqls() -> anyhow::Result<Vec<String>> {
    let db_info = get_default_pdms_db_info(); // 获取默认的数据库信息

    let mut sqls = vec![gen_create_table_sql_reflect("pdms_element")?];
    let type_sqls = db_info.gen_all_create_table_sql();
    sqls.extend_from_slice(&type_sqls);
    Ok(sqls)
}

#[cfg(not(feature = "reflect"))]
pub fn get_all_create_table_sqls() -> anyhow::Result<Vec<String>> {
    Err(anyhow!(
        "get_all_create_table_sqls requires 'reflect' feature"
    ))
}

#[cfg(feature = "reflect")]
pub fn gen_create_table_sql_reflect(type_name: &str) -> anyhow::Result<String> {
    // dbg!(&type_name);
    let type_id = orm::get_type_name_cache()
        .id_for_name(type_name)
        .ok_or_else(|| anyhow!("Type '{}' not found in cache", type_name))?;

    let registry = orm::get_type_registry()
        .ok_or_else(|| anyhow!("TypeRegistry not available (reflect feature disabled)"))?;

    let rfr = registry
        .get_type_data::<ReflectFromReflect>(type_id)
        .ok_or_else(|| anyhow!("FromReflect trait not registered for type '{}'", type_name))?;

    //Call from_reflect
    let dynamic_struct = DynamicStruct::default();
    let reflected = rfr
        .from_reflect(&dynamic_struct)
        .ok_or_else(|| anyhow!("Failed to reflect type '{}'", type_name))?;

    let reflect_do_op = registry
        .get_type_data::<ReflectDbOpTrait>(type_id)
        .ok_or_else(|| anyhow!("DbOpTrait not registered for type '{}'", type_name))?;

    let op_trait: &dyn DbOpTrait = reflect_do_op
        .get(&*reflected)
        .ok_or_else(|| anyhow!("Failed to get DbOpTrait for type '{}'", type_name))?;

    // // Which means we can now call do_thing(). Magic!
    // println!("{}", op_trait.gen_insert_many(vec![dynamic_struct], DatabaseBackend::MySql));
    let create_sql = op_trait.gen_create_table(DatabaseBackend::MySql);
    // dbg!(&create_sql);
    Ok(create_sql)
}

#[cfg(not(feature = "reflect"))]
pub fn gen_create_table_sql_reflect(type_name: &str) -> anyhow::Result<String> {
    Err(anyhow!(
        "gen_create_table_sql_reflect requires 'reflect' feature"
    ))
}

#[cfg(feature = "reflect")]
pub fn gen_insert_many_sql(
    type_name: &str,
    data_vec: Vec<DynamicStruct>,
) -> anyhow::Result<String> {
    let type_id = orm::get_type_name_cache()
        .id_for_name(type_name)
        .ok_or_else(|| anyhow!("Type '{}' not found in cache", type_name))?;

    let registry = orm::get_type_registry()
        .ok_or_else(|| anyhow!("TypeRegistry not available (reflect feature disabled)"))?;

    let rfr = registry
        .get_type_data::<ReflectFromReflect>(type_id)
        .ok_or_else(|| anyhow!("FromReflect trait not registered for type '{}'", type_name))?;

    let dynamic_struct = DynamicStruct::default();
    let reflected = rfr
        .from_reflect(&dynamic_struct)
        .ok_or_else(|| anyhow!("Failed to reflect type '{}'", type_name))?;

    let reflect_do_op = registry
        .get_type_data::<ReflectDbOpTrait>(type_id)
        .ok_or_else(|| anyhow!("DbOpTrait not registered for type '{}'", type_name))?;

    let op_trait: &dyn DbOpTrait = reflect_do_op
        .get(&*reflected)
        .ok_or_else(|| anyhow!("Failed to get DbOpTrait for type '{}'", type_name))?;

    let sql = op_trait.gen_insert_many(data_vec, DatabaseBackend::MySql);
    Ok(sql)
}

#[cfg(not(feature = "reflect"))]
pub fn gen_insert_many_sql(
    type_name: &str,
    data_vec: Vec<serde_json::Value>,
) -> anyhow::Result<String> {
    Err(anyhow!("gen_insert_many_sql requires 'reflect' feature"))
}

#[test]
#[cfg(feature = "reflect")]
fn test_do_op_reflect_sql() {
    let sqls = get_all_create_table_sqls().unwrap_or_default();
    let merged_sql = sqls.join(";");
    dbg!(merged_sql);

    let mut dynamic_struct = DynamicStruct::default();
    dynamic_struct.insert("NAME", "So hot".to_string());
    let insert_sql = gen_insert_many_sql("BOX", vec![dynamic_struct]);
    dbg!(insert_sql);
}
