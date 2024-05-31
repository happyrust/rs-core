#[cfg(feature = "sql")]
use super::query::create_table_sql;
#[cfg(feature = "sql")]
use super::query::save_material_value;

use crate::aios_db_mgr::aios_mgr::AiosDBMgr;
use crate::{get_pe, insert_into_table_with_chunks, query_filter_deep_children, RefU64};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use surrealdb::engine::any::Any;
use surrealdb::Surreal;
use tokio::task::{self, JoinHandle};

/// 暖通专业 大宗材料
pub async fn save_nt_material_dzcl(
    refno: RefU64,
    db: Surreal<Any>,
    aios_mgr: &AiosDBMgr,
    handles: &mut Vec<JoinHandle<()>>,
) {
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
                let Ok(pool) = aios_mgr.get_project_pool().await else {
                    return;
                };
                let task = task::spawn(async move {
                    let table_name = "暖通专业_阀门清单".to_string();
                    let filed = vec![
                        "参考号".to_string(),
                        "阀门位号".to_string(),
                        "所在房间号".to_string(),
                        "阀门归属".to_string(),
                        "阀门尺寸".to_string(),
                        "阀门材质".to_string(),
                        "阀门功能".to_string(),
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

/// 暖通 阀门清单
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialNtValvData {
    pub id: RefU64,
    pub name: String,
    pub room_code: Option<String>,
    pub bran_name: String,
    pub valv_size: Vec<f32>,
    pub material: String,
    pub valv_use: String,
}

impl MaterialNtValvData {
    pub fn into_hashmap(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.entry("参考号".to_string())
            .or_insert(self.id.to_pdms_str());
        map.entry("阀门位号".to_string()).or_insert(self.name);
        map.entry("所在房间号".to_string())
            .or_insert(self.room_code.unwrap_or("".to_string()));
        map.entry("阀门归属".to_string()).or_insert(self.bran_name);
        map.entry("阀门尺寸".to_string())
            .or_insert(serde_json::to_string(&self.valv_size).unwrap_or("[]".to_string()));
        map.entry("阀门材质".to_string()).or_insert(self.material);
        map.entry("阀门功能".to_string()).or_insert(self.valv_use);
        map
    }
}

/// 暖通 阀门清单
pub async fn get_nt_valv_list_material(
    db: Surreal<Any>,
    refnos: Vec<RefU64>,
) -> anyhow::Result<Vec<MaterialNtValvData>> {
    let mut data = Vec::new();
    for refno in refnos {
        let Some(pe) = get_pe(refno).await? else {
            continue;
        };
        // 如果是site，则需要过滤 site的 name
        if pe.noun == "SITE".to_string() {
            if !pe.name.contains("HVAC") {
                continue;
            };
        }
        // 查询 DAMP 的数据
        let refnos = query_filter_deep_children(refno, vec!["DAMP".to_string()]).await?;
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
            (->pe_owner.out->pe_owner.in.refno.NAME)[0] as bran_name,
            [if refno.DESP[1] == NONE {{ 0 }} else {{ refno.DESP[1] }},if refno.DESP[2] == NONE {{ 0 }} else {{ refno.DESP[2] }},
            if refno.DESP[5] == NONE {{ 0 }} else {{ refno.DESP[5] }}] as valv_size,
            fn::get_valv_material($this.id) as material,
            if name == NONE {{ '' }} else {{ string::slice(name,-3) }} as valv_use
            from {}"#,
            refnos_str
        );
        let mut response = db.query(sql).await?;
        let mut result: Vec<MaterialNtValvData> = response.take(0)?;
        data.append(&mut result);
    }
    Ok(data)
}
