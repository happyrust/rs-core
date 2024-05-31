use std::collections::HashMap;
use surrealdb::engine::any::Any;
use surrealdb::Surreal;
use tokio::task::{self, JoinHandle};
use crate::aios_db_mgr::aios_mgr::{self, AiosDBMgr};
use crate::material_query::{get_gy_dzcl, get_gy_equi_list, get_gy_valv_list};
use crate::{insert_into_table_with_chunks, RefU64};
#[cfg(feature = "sql")]
use super::query::create_table_sql;
use super::query::{save_material_data_to_mysql, save_two_material_data_to_mysql};


/// 工艺专业 大宗材料
pub async fn save_gy_material_dzcl(refno:RefU64, db:Surreal<Any>,aios_mgr:&AiosDBMgr,mut handles:&mut Vec<JoinHandle<()>>) {
    match get_gy_dzcl(db.clone(), vec![refno]).await {
        Ok((r, tubi_r)) => {
            let r_clone = r.clone();
            let tubi_r_clone = tubi_r.clone();
            let task = task::spawn(async move {
                match insert_into_table_with_chunks(&db, "material_gy_list", r_clone)
                    .await{
                    Ok(_) => {}
                    Err(e) => {
                        dbg!(&e.to_string());
                    }
                }
                match insert_into_table_with_chunks(
                    &db,
                    "material_gy_list_tubi",
                    tubi_r_clone,
                )
                .await{
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
                    let table_name = "工艺布置专业_阀门清单".to_string();
                    let table_field = vec![
                        "参考号".to_string(),
                        "编码".to_string(),
                        "类型".to_string(),
                        "部件".to_string(),
                        "公称直径（主）".to_string(),
                        "公称直径（支）".to_string(),
                        "材料".to_string(),
                        "RCC_M".to_string(),
                        "SCH/LB（主）".to_string(),
                        "SCH/LB（支）".to_string(),
                        "制造形式".to_string(),
                        "连接形式".to_string(),
                        "标准".to_string(),
                        "单重（kg）".to_string(),
                        "总重（kg）".to_string(),
                        "数量".to_string(),
                        "单位".to_string(),
                    ];
                    
                    match create_table_sql(&pool,&table_name,&table_field).await{
                        Ok(_) => {
                            // 保存到数据库
                            if !r.is_empty() {
                                let data_1 = r
                                    .into_iter()
                                    .map(|x| x.into_hashmap())
                                    .collect::<Vec<HashMap<String, String>>>();
                                let data_field_1 = vec![
                                    "参考号".to_string(),
                                    "编码".to_string(),
                                    "类型".to_string(),
                                ];
                                let data_field_2 = vec![
                                    "参考号".to_string(),
                                    "编码".to_string(),
                                    "类型".to_string(),
                                    "数量".to_string(),
                                ];
                                let data_2 = tubi_r
                                    .into_iter()
                                    .map(|x| x.into_hashmap())
                                    .collect::<Vec<HashMap<String, String>>>();
                                match save_two_material_data_to_mysql(&table_field,&table_name,&data_field_1,data_1,&data_field_2,data_2,&pool).await{
                                    Ok(_) => {},
                                    Err(e) => {
                                        dbg!(e.to_string());
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

/// 工艺专业 设备清单
pub async fn save_gy_material_equi(refno:RefU64, db:Surreal<Any>,aios_mgr:&AiosDBMgr,mut handles:&mut Vec<JoinHandle<()>>) {
    match get_gy_equi_list(db.clone(), vec![refno]).await {
        Ok(r) => {
            let r_clone = r.clone();
            let task = task::spawn(async move {
                match insert_into_table_with_chunks(&db, "material_gy_equi", r_clone).await {
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
                    let filed = vec![
                        "参考号".to_string(),
                        "设备位号".to_string(),
                        "所在房间号".to_string(),
                        "管口号".to_string(),
                        "管口坐标".to_string(),
                        "相连管道编号".to_string(),
                    ];
                    let table_name = "工艺布置专业_设备清单".to_string();
                    
                    match create_table_sql(&pool, &table_name, &filed).await {
                        Ok(_) => {
                            if !r.is_empty() {
                                let data = r
                                    .into_iter()
                                    .map(|x| x.into_hashmap())
                                    .collect::<Vec<HashMap<String, String>>>();
                                match save_material_data_to_mysql(&filed,&table_name,&filed,data,pool).await {
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

/// 工艺专业 阀门清单
pub async fn save_gy_material_valv(refno:RefU64, db:Surreal<Any>,aios_mgr:&AiosDBMgr,mut handles:&mut Vec<JoinHandle<()>>) {
    match get_gy_valv_list(db.clone(), vec![refno]).await {
        Ok(r) => {
            let r_clone = r.clone();
            let task = task::spawn(async move {
                match insert_into_table_with_chunks(&db, "material_gy_valv", r_clone).await{
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
                    let filed = vec![
                        "参考号".to_string(),
                        "阀门位号".to_string(),
                        "所在房间号".to_string(),
                        "阀门归属".to_string(),
                        "阀门长度".to_string(),
                        "阀门重量".to_string(),
                        "阀门重心X".to_string(),
                        "阀门重心Y".to_string(),
                        "阀门重心Z".to_string(),
                        "是否阀门支架".to_string(),
                    ];
                    let table_name = "工艺布置专业_阀门清单".to_string();
                    match create_table_sql(&pool, &table_name, &filed).await {
                        Ok(_) => {
                            if !r.is_empty() {
                                let data = r
                                    .into_iter()
                                    .map(|x| x.into_hashmap())
                                    .collect::<Vec<HashMap<String, String>>>();
                                match save_material_data_to_mysql(&filed,&table_name,&filed,data,pool).await{
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