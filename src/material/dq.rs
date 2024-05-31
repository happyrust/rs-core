use std::collections::HashMap;
use calamine::{open_workbook, RangeDeserializerBuilder, Reader, Xls};
use surrealdb::engine::any::Any;
use surrealdb::Surreal;
use tokio::task::{self, JoinHandle};
use crate::aios_db_mgr::aios_mgr::AiosDBMgr;
use crate::material_query::{get_dq_bran_list, DqMaterial, MaterialDqMaterialList, MaterialDqMaterialListStru};
use crate::{insert_into_table_with_chunks, RefU64};
#[cfg(feature = "sql")]
use super::query::create_table_sql;
use super::query::save_material_value;

/// 电气专业 托盘及接地
pub async fn save_dq_material(refno:RefU64, db:Surreal<Any>,aios_mgr:&AiosDBMgr,mut handles:&mut Vec<JoinHandle<()>>) {
    match get_dq_bran_list(db.clone(), vec![refno]).await {
        Ok((mut r, mut str_r)) => {
            let material_data = read_dq_material_excel().unwrap_or_default();
            get_dq_value_from_material(&material_data, &mut r);
            get_dq_value_from_material_stru(&material_data, &mut str_r);

            let r_clone = r.clone();
            let str_r_clone = str_r.clone();
            let db_clone = db.clone();
            let task = task::spawn(async move {
                match insert_into_table_with_chunks(&db_clone, "material_elec_list", r_clone).await {
                    Ok(_) => {}
                    Err(e) => {
                        dbg!(&e.to_string());
                    }
                }
            });
            handles.push(task);
            let task = task::spawn(async move {
                match insert_into_table_with_chunks(&db, "material_elec_list", str_r_clone).await
                {
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
                    let table_name = "电气专业_托盘及接地".to_string();
                    let filed = vec!["参考号".to_string(), "机组号".to_string(), "元件等级名称".to_string(),
                                     "子项号".to_string(), "子项名称".to_string(), "专业".to_string(),
                                     "房间号".to_string(), "托盘段号".to_string(), "托盘标高".to_string(), "托盘类型".to_string(),
                                     "托盘支吊架名称".to_string(), "标准号".to_string(), "物项编号".to_string(), "材质".to_string(),
                                     "托盘宽度mm".to_string(), "托盘高度mm".to_string(), "规格型号".to_string(), "是否刷漆".to_string(), "刷漆颜色".to_string(),
                                     "有无盖板".to_string(), "有无隔板".to_string(), "隔板编号".to_string(), "共用隔板的托盘序号".to_string(),
                                     "水平/竖向".to_string(), "单位".to_string(), "数量".to_string()];
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
                            if !str_r.is_empty() {
                                let data = str_r
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

/// 读取电气专业材料表
fn read_dq_material_excel() -> anyhow::Result<HashMap<String, Vec<DqMaterial>>> {
    let mut map = HashMap::new();
    let mut workbook: Xls<_> = open_workbook("resource/电气专业大宗材料属性对应关系表.xls")?;
    let range = workbook.worksheet_range("电气专业大宗材料属性对应关系表")?;

    let mut iter = RangeDeserializerBuilder::new().from_range(&range)?;

    while let Some(result) = iter.next() {
        let value: DqMaterial = result?;
        if value.catr_name.is_some() {
            map.entry(value.catr_name.clone().unwrap()).or_insert_with(Vec::new).push(value);
        }
    }
    Ok(map)
}


// 将材料表的 标准号 和 单位 填入其中
fn get_dq_value_from_material(material_data: &HashMap<String, Vec<DqMaterial>>, mut value: &mut Vec<MaterialDqMaterialList>) {
    for mut d in value.iter_mut() {
        if d.spre.is_some() && material_data.contains_key(&d.spre.clone().unwrap()) {
            let material_value = material_data.get(&d.spre.clone().unwrap()).unwrap();
            if material_value.is_empty() { continue; };
            let value = material_value[0].clone();
            d.stander_num = value.stander_num;
            d.unit = value.unit;
            d.item_num = value.code;
        } else if d.catr.is_some() && material_data.contains_key(&d.catr.clone().unwrap()) {
            let material_value = material_data.get(&d.catr.clone().unwrap()).unwrap();
            if material_value.is_empty() { continue; };
            let value = material_value[0].clone();
            d.stander_num = value.stander_num;
            d.unit = value.unit;
            d.item_num = value.code;
        }
    }
}

// 将材料表的标准号 物项编号 和单位填入其中
fn get_dq_value_from_material_stru(material_data: &HashMap<String, Vec<DqMaterial>>, mut value: &mut Vec<MaterialDqMaterialListStru>) {
    for mut d in value.iter_mut() {
        if d.spre.is_some() && material_data.contains_key(&d.spre.clone().unwrap()) {
            let material_value = material_data.get(&d.spre.clone().unwrap()).unwrap();
            if material_value.is_empty() { continue; };
            let value = material_value[0].clone();
            d.stander_num = value.stander_num;
            d.unit = value.unit;
            d.item_num = value.code;
        } else if d.catr.is_some() && material_data.contains_key(&d.catr.clone().unwrap()) {
            let material_value = material_data.get(&d.catr.clone().unwrap()).unwrap();
            if material_value.is_empty() { continue; };
            let value = material_value[0].clone();
            d.stander_num = value.stander_num;
            d.unit = value.unit;
            d.item_num = value.code;
        }
    }
}
