use std::collections::HashMap;
use surrealdb::engine::any::Any;
use surrealdb::Surreal;
use tokio::task::{self, JoinHandle};
use crate::aios_db_mgr::aios_mgr::AiosDBMgr;
use crate::material_query::{get_nt_valv_list_material};
use crate::{insert_into_table_with_chunks, RefU64};
#[cfg(feature = "sql")]
use super::query::create_table_sql;
use super::query::save_material_value;

/// 暖通专业 大宗材料
pub async fn save_nt_material_dzcl(refno:RefU64, db:Surreal<Any>,aios_mgr:&AiosDBMgr,handles:&mut Vec<JoinHandle<()>>) {
    match get_nt_valv_list_material(db.clone(), vec![refno]).await {
        Ok(r) => {
            let r_clone = r.clone();
            let task = task::spawn(async move {
                match insert_into_table_with_chunks(&db, "material_nt_valv", r_clone).await {
                    Ok(_) => {}
                    Err(e) => {
                        dbg!(&e.to_string());
                    }
                }
            });
            handles.push(task);
            #[cfg(feature = "sql")]
            {
                let Ok(pool) = aios_mgr.get_project_pool().await else { return;};
                let task = task::spawn(async move {
                    let table_name = "暖通专业_阀门清单".to_string();
                    let filed = vec!["参考号".to_string(), "阀门位号".to_string(), "所在房间号".to_string(), "阀门归属".to_string(), "阀门尺寸".to_string(), "阀门材质".to_string(), "阀门功能".to_string()];
                    match create_table_sql(&pool, &table_name, &filed).await {
                        Ok(_) => {
                            if !r.is_empty() {
                                let data = r
                                    .into_iter()
                                    .map(|x| x.into_hashmap())
                                    .collect::<Vec<HashMap<String, String>>>();
                                match save_material_value(
                                    &pool,
                                    &table_name,
                                    &filed,
                                    data,
                                )
                                    .await
                                {
                                    Ok(_) => {}
                                    Err(e) => {
                                        dbg!(&e.to_string());
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            dbg!(&e.to_string());
                        }
                    }
                });
                handles.push(task);
            }
        }
        Err(e) => {
            dbg!(e.to_string());
        }
    }
}