use surrealdb::engine::any::Any;
use surrealdb::Surreal;
use tokio::task::{self, JoinHandle};
use crate::aios_db_mgr::aios_mgr::{AiosDBMgr};
use crate::material_query::{get_tf_hvac_material};
use crate::{insert_into_table_with_chunks, RefU64};
#[cfg(feature = "sql")]
use super::query::create_table_sql;
use super::query::{save_material_value};

/// 通风专业 风管管段
pub async fn save_tf_material_hvac(refno:RefU64, db:Surreal<Any>,aios_mgr:&AiosDBMgr,mut handles:&mut Vec<JoinHandle<()>>) {
    match get_tf_hvac_material(&db, vec![refno]).await {
        Ok(r) => {
            let r_clone = r.clone();
                let task = task::spawn(async move {
                    match insert_into_table_with_chunks(&db, "material_hvac_pipe", r_clone).await {
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
                        let table_name = "通风专业_风管管段清单".to_string();
                        let filed = vec!["参考号".to_string(), "描述".to_string(), "管段编号".to_string(),
                                         "子项号".to_string(), "材质".to_string(), "压力等级".to_string(), "风管长度".to_string(),
                                         "风管宽度".to_string(), "风管高度".to_string(), "风管壁厚".to_string(), "风管面积".to_string(),
                                         "风管重量".to_string(), "加强筋型材".to_string(), "加强筋长度".to_string(), "加强筋重量".to_string(),
                                         "法兰规格".to_string(), "法兰长度".to_string(), "法兰重量".to_string(), "垫圈类型".to_string(),
                                         "垫圈长度".to_string(), "螺栓数量".to_string(), "其它材料类型".to_string(), "其它材料数量".to_string(),
                                         "螺杆".to_string(), "螺母数量".to_string(), "螺母数量_2".to_string(), "所在房间号".to_string(),
                                         "系统".to_string()];
                        match create_table_sql(&pool, &table_name, &filed).await {
                            Ok(_) => {
                                if !r.is_empty() {
                                    match save_material_value(
                                        &pool,
                                        &table_name,
                                        &filed,
                                        r,
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