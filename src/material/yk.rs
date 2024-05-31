use std::collections::HashMap;
use surrealdb::engine::any::Any;
use surrealdb::Surreal;
use tokio::task::{self, JoinHandle};
use crate::aios_db_mgr::aios_mgr::{self, AiosDBMgr};
use crate::material_query::{get_yk_dzcl_list, get_yk_equi_list_material, get_yk_inst_pipe};
use crate::{insert_into_table_with_chunks, RefU64};
#[cfg(feature = "sql")]
use super::query::create_table_sql;
use super::query::{save_material_value};

/// 仪控专业 大宗材料
pub async fn save_yk_material_dzcl(refno:RefU64, db:Surreal<Any>,aios_mgr:&AiosDBMgr,mut handles:&mut Vec<JoinHandle<()>>) {
    match get_yk_dzcl_list(db.clone(), vec![refno]).await {
        Ok(r) => {
            let r_clone = r.clone();
                let task = task::spawn(async move {
                    match insert_into_table_with_chunks(&db, "material_inst_list", r_clone).await {
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
                        let filed = vec!["参考号".to_string(), "编码".to_string(), "品名".to_string(), "规格".to_string(),
                                         "连接形式".to_string(), "材料".to_string(), "RCC-M".to_string(), "质保等级".to_string(), "抗震等级".to_string(),
                                         "备注".to_string(), "内部编码".to_string(), "公称直径".to_string()];
                        let table_name = "仪控专业_大宗材料".to_string();
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
            dbg!(&e.to_string());
        }
    }
}

/// 仪控专业 仪表管道
pub async fn save_yk_material_pipe(refno:RefU64, db:Surreal<Any>,aios_mgr:&AiosDBMgr,mut handles:&mut Vec<JoinHandle<()>>) {
    match get_yk_inst_pipe(db.clone(), vec![refno]).await {
        Ok(r) => {
            let r_clone = r.clone();
            let task = task::spawn(async move {
                match insert_into_table_with_chunks(&db, "material_inst_pipe", r_clone).await {
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
                    let table_name = "仪控专业_仪表管道".to_string();
                    let filed = vec!["参考号".to_string(), "传感器标识".to_string(), "对应根阀编号".to_string(), "房间号".to_string()];
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

/// 仪控专业 设备清单
pub async fn save_yk_material_equi(refno:RefU64, db:Surreal<Any>,aios_mgr:&AiosDBMgr,mut handles:&mut Vec<JoinHandle<()>>) {
    match get_yk_equi_list_material(db.clone(), vec![refno]).await {
        Ok(r) => {
            let r_clone = r.clone();
                let task = task::spawn(async move {
                    match insert_into_table_with_chunks(&db, "material_inst_equi", r_clone).await {
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
                        let table_name = "仪控专业_设备清单".to_string();
                        let filed = vec!["参考号".to_string(), "仪控设备位号".to_string(), "所在房间号".to_string(), "设备绝对标高".to_string(), "设备相对楼板标高".to_string()];
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