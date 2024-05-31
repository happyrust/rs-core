#[cfg(feature = "sql")]
use super::query::create_table_sql;
use super::query::save_material_value;

use crate::aios_db_mgr::aios_mgr::AiosDBMgr;
use crate::{get_pe, insert_into_table_with_chunks, query_filter_deep_children, RefU64};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use surrealdb::engine::any::Any;
use surrealdb::sql::Thing;
use surrealdb::Surreal;
use tokio::task::{self, JoinHandle};

/// 给排水专业 大宗材料
pub async fn save_sb_material_dzcl(
    refno: RefU64,
    db: Surreal<Any>,
    aios_mgr: &AiosDBMgr,
    handles: &mut Vec<JoinHandle<()>>,
) {
    match get_sb_dzcl_list_material(db.clone(), vec![refno]).await {
        Ok(r) => {
            let r_clone = r.clone();
            let task = task::spawn(async move {
                match insert_into_table_with_chunks(&db, "material_sb_list", r_clone).await {
                    Ok(_) => {}
                    Err(e) => {
                        dbg!(&e.to_string());
                    }
                }
            });
            handles.push(task);
            #[cfg(feature = "sql")]
            {
                let Ok(pool) = aios_mgr.get_project_pool().await else {
                    return;
                };
                let task = task::spawn(async move {
                    let table_name = "设备专业_大宗材料".to_string();
                    let filed = vec![
                        "参考号".to_string(),
                        "设备位号".to_string(),
                        "所在房间".to_string(),
                        "轨道长度".to_string(),
                        "安装标高".to_string(),
                    ];
                    match create_table_sql(&pool, &table_name, &filed).await {
                        Ok(_) => {
                            if !r.is_empty() {
                                let data = r
                                    .into_iter()
                                    .map(|x| x.into_hashmap())
                                    .collect::<Vec<HashMap<String, String>>>();
                                match save_material_value(&pool, &table_name, &filed, data).await {
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

/// 设备专业 通信系统
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialTxTxsbData {
    pub id: RefU64,
    pub equi_name: String,
    pub ptre_desc: String,
    pub belong_factory: String,
    pub room_code: Option<String>,
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub z: Option<f32>,
    pub ptre_name: String,
}

impl MaterialTxTxsbData {
    pub fn into_hashmap(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.entry("参考号".to_string())
            .or_insert(self.id.to_pdms_str());
        map.entry("设备位号".to_string()).or_insert(self.equi_name);
        map.entry("设备名称".to_string()).or_insert(self.ptre_desc);
        map.entry("所属厂房的编号".to_string())
            .or_insert(self.belong_factory);
        map.entry("房间号".to_string())
            .or_insert(self.room_code.unwrap_or("".to_string()));
        map.entry("全局坐标X".to_string())
            .or_insert(self.x.unwrap_or(0.0).to_string());
        map.entry("全局坐标Y".to_string())
            .or_insert(self.y.unwrap_or(0.0).to_string());
        map.entry("全局坐标Z".to_string())
            .or_insert(self.z.unwrap_or(0.0).to_string());
        map.entry("设备型号".to_string()).or_insert(self.ptre_name);

        map
    }
}

/// 设备专业 大宗材料
pub async fn get_sb_dzcl_list_material(
    db: Surreal<Any>,
    refnos: Vec<RefU64>,
) -> anyhow::Result<Vec<MaterialSbListData>> {
    let mut data = Vec::new();
    for refno in refnos {
        let Some(pe) = get_pe(refno).await? else {
            continue;
        };
        // 如果是site，则需要过滤 site的 name
        if pe.noun == "SITE".to_string() {
            if !pe.name.contains("EQUI") {
                continue;
            };
        }
        // 查询 EQUI 的数据
        let refnos = query_filter_deep_children(refno, vec!["EQUI".to_string()]).await?;
        let refnos_str = serde_json::to_string(
            &refnos
                .into_iter()
                .map(|refno| refno.to_pe_key())
                .collect::<Vec<String>>(),
        )?;
        let sql = format!(
            r#"select
            id,
            fn::default_name($this.id) as name,
            fn::room_code($this.id)[0] as room_code,
            fn::find_group_sube_children($this.id) as boxs
            from {}"#,
            refnos_str
        );
        let mut response = db.query(sql).await?;
        let result: Vec<MaterialSbListData> = response.take(0)?;
        let mut equi_data = result
            .into_iter()
            .filter(|x| !x.name.contains("PR") || !x.name.contains("PD"))
            .map(|equi| (equi.id, equi))
            .collect::<HashMap<RefU64, MaterialSbListData>>();
        // 查询轨道长度
        let tray = equi_data
            .iter()
            .map(|x| x.1.boxs.clone())
            .collect::<Vec<_>>();
        let equi_children = filter_equi_children(tray);
        let sql = format!(
            r#"select
            (id.REFNO->pe_owner.out->pe_owner.out.refno)[0][0] as refno,
            array::max(array::max([XLEN,YLEN,ZLEN])) as length,
            array::max(id.REFNO->inst_relate.world_trans.d.translation[2])[0] as pos
            from {}"#,
            serde_json::to_string(&equi_children).unwrap_or("[]".to_string())
        );
        let mut response = db.query(sql).await?;
        let result: Vec<MaterialSubeData> = response.take(0)?;
        // 将轨道长度放到设备的数据中
        for r in result {
            let Some(value) = equi_data.get_mut(&r.refno) else {
                continue;
            };
            value.pos = r.pos;
            value.length = r.length;
        }
        data.append(&mut equi_data.into_iter().map(|x| x.1).collect::<Vec<_>>())
    }
    Ok(data)
}

fn filter_equi_children(datas: Vec<Vec<Vec<Thing>>>) -> Vec<Vec<String>> {
    let mut result = Vec::new();
    for data in datas {
        let filtered_data: Vec<Vec<String>> = data
            .into_iter()
            .filter(|inner_vec| inner_vec.iter().all(|s| s.tb == "BOX"))
            .filter(|inner_vec| {
                let count = inner_vec.iter().count();
                count == 3 || count == 4
            })
            .map(|vec| {
                vec.iter()
                    .map(|thing| format!("BOX:{}", thing.id.to_string()))
                    .collect::<Vec<String>>()
            })
            .collect();
        if !filtered_data.is_empty() {
            result.push(filtered_data[0].clone())
        }
    }
    result
}

/// 设备专业 大宗材料
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialSbListData {
    pub id: RefU64,
    pub name: String,
    pub pos: Option<f32>,
    pub length: Option<f32>,
    pub room_code: Option<String>,
    pub boxs: Vec<Vec<Thing>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MaterialSubeData {
    pub refno: RefU64,
    pub pos: Option<f32>,
    pub length: Option<f32>,
}

impl MaterialSbListData {
    pub fn into_hashmap(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.entry("参考号".to_string())
            .or_insert(self.id.to_pdms_str());
        map.entry("设备位号".to_string()).or_insert(self.name);
        map.entry("所在房间".to_string())
            .or_insert(self.room_code.unwrap_or("".to_string()));
        map.entry("轨道长度".to_string())
            .or_insert(self.length.unwrap_or(0.0).to_string());
        map.entry("安装标高".to_string())
            .or_insert(self.pos.unwrap_or(0.0).to_string());

        map
    }
}
