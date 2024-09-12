#[cfg(feature = "sql")]
use super::query::create_table_sql;
#[cfg(feature = "sql")]
use super::query::save_material_value;

use crate::aios_db_mgr::aios_mgr::AiosDBMgr;
use crate::material::sb::MaterialTxTxsbData;
use crate::{
    get_children_pes, get_pe, insert_into_table_with_chunks, query_filter_deep_children, RefU64,
};
use std::collections::HashMap;
use surrealdb::engine::any::Any;
use surrealdb::Surreal;
use tokio::task::{self, JoinHandle};

const FIELDS: [&str; 9] = [
    "参考号",
    "设备位号",
    "设备名称",
    "所属厂房的编号",
    "房间号",
    "全局坐标X",
    "全局坐标Y",
    "全局坐标Z",
    "设备型号",
];

/// 通信专业 通信设备
pub async fn save_tx_material_equi(
    refno: RefU64,
    db: Surreal<Any>,
    mut handles: &mut Vec<JoinHandle<()>>,
) {
    match get_tx_txsb_list_material(db.clone(), vec![refno]).await {
        Ok(r) => {
            if r.is_empty() { return; }
            let r_clone = r.clone();
            let task = task::spawn(async move {
                match insert_into_table_with_chunks(&db, "material_tx_list", r_clone).await {
                    Ok(_) => {}
                    Err(e) => {
                        dbg!(&e.to_string());
                    }
                }
            });
            handles.push(task);
            #[cfg(feature = "sql")]
            {
                let Ok(pool) = AiosDBMgr::get_project_pool().await else {
                    dbg!("无法连接到数据库");
                    return;
                };
                let task = task::spawn(async move {
                    let table_name = "通信专业_通信系统".to_string();
                    match create_table_sql(&pool, &table_name, &FIELDS).await {
                        Ok(_) => {
                            if !r.is_empty() {
                                let data = r
                                    .into_iter()
                                    .map(|x| x.into_hashmap())
                                    .collect::<Vec<HashMap<String, String>>>();
                                match save_material_value(&pool, &table_name, &FIELDS, data).await {
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

/// 通信专业 通信设备
pub async fn get_tx_txsb_list_material(
    db: Surreal<Any>,
    refnos: Vec<RefU64>,
) -> anyhow::Result<Vec<MaterialTxTxsbData>> {
    let mut data = Vec::new();
    for refno in refnos {
        let Some(pe) = get_pe(refno).await? else {
            continue;
        };
        // 如果是site，则需要过滤 site的 name
        if pe.noun == "SITE".to_string() {
            if !pe.name.contains("ELEC") {
                continue;
            };
        }
        // 过滤zone
        let zones = get_children_pes(pe.refno()).await?;
        for zone in zones {
            if !zone.name.contains("FAD") {
                continue;
            };
            // 查询 EQUI 的数据
            let refnos = query_filter_deep_children(refno, &["ELCONN"]).await?;
            let refnos_str =
                &refnos
                    .into_iter()
                    .map(|refno| refno.to_pe_key())
                    .collect::<Vec<String>>().join(",");
            let sql = format!(
                r#"select
            id,
            fn::default_name(owner) as equi_name,
            string::slice(if refno.CATR.refno.PRTREF.desc == NONE {{ '/' }} else {{ refno.CATR.refno.PRTREF.desc }},1) as ptre_desc, // 设备名称
            string::slice(string::split(array::at(refno.REFNO->pe_owner.out->pe_owner.out.name,0),'-')[0],1,3) as belong_factory, // 所属厂房编号
            fn::room_code($this.id)[0] as room_code,
            fn::get_world_pos($this.id)[0][0] as x, // 坐标 x
            fn::get_world_pos($this.id)[0][1] as y, // 坐标 y
            fn::get_world_pos($this.id)[0][2] as z, // 坐标 z
            string::slice(refno.CATR.refno.PRTREF.refno.NAME,1) as ptre_name
            from [{}]"#,
                refnos_str
            );
            let mut response = db.query(sql).await?;
            let mut result: Vec<MaterialTxTxsbData> = response.take(0)?;
            data.append(&mut result);
        }
    }
    Ok(data)
}
