#[cfg(feature = "sql")]
use super::query::create_table_sql;
#[cfg(feature = "sql")]
use super::query::save_material_value;
use crate::aios_db_mgr::aios_mgr::AiosDBMgr;
use crate::{
    get_pe, insert_into_table_with_chunks, query_ele_filter_deep_children, NamedAttrValue, RefU64,
};
use serde_derive::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use std::collections::HashMap;
use surrealdb::engine::any::Any;
use surrealdb::Surreal;
use tokio::task::{self, JoinHandle};

/// 通风专业 风管管段
pub async fn save_tf_material_hvac(
    refno: RefU64,
    db: Surreal<Any>,
    aios_mgr: &AiosDBMgr,
    mut handles: &mut Vec<JoinHandle<()>>,
) {
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
                let Ok(pool) = aios_mgr.get_project_pool().await else {
                    dbg!("无法连接到数据库");
                    return;
                };
                let task = task::spawn(async move {
                    let table_name = "通风专业_风管管段清单".to_string();
                    let filed = vec![
                        "参考号".to_string(),
                        "描述".to_string(),
                        "管段编号".to_string(),
                        "子项号".to_string(),
                        "材质".to_string(),
                        "压力等级".to_string(),
                        "风管长度".to_string(),
                        "风管宽度".to_string(),
                        "风管高度".to_string(),
                        "风管壁厚".to_string(),
                        "风管面积".to_string(),
                        "风管重量".to_string(),
                        "加强筋型材".to_string(),
                        "加强筋长度".to_string(),
                        "加强筋重量".to_string(),
                        "法兰规格".to_string(),
                        "法兰长度".to_string(),
                        "法兰重量".to_string(),
                        "垫圈类型".to_string(),
                        "垫圈长度".to_string(),
                        "螺栓数量".to_string(),
                        "其它材料类型".to_string(),
                        "其它材料数量".to_string(),
                        "螺杆".to_string(),
                        "螺母数量".to_string(),
                        "螺母数量_2".to_string(),
                        "所在房间号".to_string(),
                        "系统".to_string(),
                    ];
                    match create_table_sql(&pool, &table_name, &filed).await {
                        Ok(_) => {
                            if !r.is_empty() {
                                match save_material_value(&pool, &table_name, &filed, r).await {
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

/// 通风 风管管段
#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct MaterialTfHavcList {
    #[serde_as(as = "DisplayFromStr")]
    pub id: RefU64,
    #[serde(default)]
    pub bolt_qty: i32,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub duct_area: Vec<f32>,
    #[serde(default)]
    pub duct_weight: f32,
    #[serde(default)]
    pub fl_len: f32,
    #[serde(default)]
    pub fl_type: String,
    #[serde(default)]
    pub fl_wei: f32,
    #[serde(default)]
    pub height: String,
    #[serde(default)]
    pub length: f32,
    #[serde(default)]
    pub material: Vec<String>,
    #[serde(default)]
    pub nut_qty: i32,
    #[serde(default)]
    pub other_qty: String,
    #[serde(default)]
    pub other_type: String,
    #[serde(default)]
    pub pressure: String,
    #[serde(default)]
    pub room_no: Option<String>,
    #[serde(default)]
    pub seg_code: String,
    #[serde(default)]
    pub stif_len: f32,
    #[serde(default)]
    pub stif_sctn: String,
    #[serde(default)]
    pub stif_wei: f32,
    #[serde(default)]
    pub stud: String,
    #[serde(default)]
    pub sub_code: String,
    #[serde(default)]
    pub system: String,
    #[serde(default)]
    pub wall_thk: Vec<f32>,
    #[serde(default)]
    pub washer_len: f32,
    #[serde(default)]
    pub washer_type: Vec<String>,
    #[serde(default)]
    pub washer_qty: i32,
    #[serde(default)]
    pub width: String,
}

impl MaterialTfHavcList {
    //// 将结构体转为HashMap
    pub fn into_hashmap(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.entry("参考号".to_string())
            .or_insert(self.id.to_pdms_str());
        map.entry("描述".to_string()).or_insert(self.description);
        map.entry("管段编号".to_string()).or_insert(self.seg_code);
        map.entry("子项号".to_string()).or_insert(self.sub_code);
        map.entry("材质".to_string())
            .or_insert(serde_json::to_string(&self.material).unwrap_or("[]".to_string()));
        map.entry("压力等级".to_string()).or_insert(self.pressure);
        map.entry("风管长度".to_string())
            .or_insert(self.length.to_string());
        map.entry("风管宽度".to_string()).or_insert(self.width);
        map.entry("风管高度".to_string()).or_insert(self.height);
        map.entry("风管壁厚".to_string())
            .or_insert(serde_json::to_string(&self.wall_thk).unwrap_or("[]".to_string()));
        map.entry("风管面积".to_string())
            .or_insert(serde_json::to_string(&self.duct_area).unwrap_or("[]".to_string()));
        map.entry("风管重量".to_string())
            .or_insert(self.duct_weight.to_string());
        map.entry("加强筋型材".to_string())
            .or_insert(self.stif_sctn);
        map.entry("加强筋长度".to_string())
            .or_insert(self.stif_len.to_string());
        map.entry("加强筋重量".to_string())
            .or_insert(self.stif_wei.to_string());
        map.entry("法兰规格".to_string()).or_insert(self.fl_type);
        map.entry("法兰长度".to_string())
            .or_insert(self.fl_len.to_string());
        map.entry("法兰重量".to_string())
            .or_insert(self.fl_wei.to_string());
        map.entry("垫圈类型".to_string())
            .or_insert(serde_json::to_string(&self.washer_type).unwrap_or("[]".to_string()));
        map.entry("垫圈长度".to_string())
            .or_insert(self.washer_len.to_string());
        map.entry("螺栓数量".to_string())
            .or_insert(self.bolt_qty.to_string());
        map.entry("其它材料类型".to_string())
            .or_insert(self.other_type);
        map.entry("其它材料数量".to_string())
            .or_insert(self.other_qty);
        map.entry("螺杆".to_string()).or_insert(self.stud);
        map.entry("螺母数量".to_string())
            .or_insert(self.nut_qty.to_string());
        map.entry("螺母数量_2".to_string())
            .or_insert(self.washer_qty.to_string());
        map.entry("所在房间号".to_string())
            .or_insert(self.room_no.unwrap_or("".to_string()));
        map.entry("系统".to_string()).or_insert(self.system);
        map
    }
}

/// 通风 风管管段
#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct MaterialTfHavcTapeList {
    #[serde_as(as = "DisplayFromStr")]
    pub id: RefU64,
    #[serde(default)]
    pub bolt_qty: Vec<f32>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub duct_area: Vec<f32>,
    #[serde(default)]
    pub duct_weight: f32,
    #[serde(default)]
    pub fl_len: f32,
    #[serde(default)]
    pub fl_type: String,
    #[serde(default)]
    pub fl_wei: f32,
    #[serde(default)]
    pub height: Vec<String>,
    #[serde(default)]
    pub length: f32,
    #[serde(default)]
    pub material: Vec<String>,
    #[serde(default)]
    pub nut_qty: Vec<i32>,
    #[serde(default)]
    pub other_qty: String,
    #[serde(default)]
    pub other_type: String,
    #[serde(default)]
    pub pressure: String,
    #[serde(default)]
    pub room_no: Option<String>,
    #[serde(default)]
    pub seg_code: String,
    #[serde(default)]
    pub stif_len: f32,
    #[serde(default)]
    pub stif_sctn: String,
    #[serde(default)]
    pub stif_wei: f32,
    #[serde(default)]
    pub stud: String,
    #[serde(default)]
    pub sub_code: String,
    #[serde(default)]
    pub system: String,
    #[serde(default)]
    pub wall_thk: Vec<f32>,
    #[serde(default)]
    pub washer_len: f32,
    #[serde(default)]
    pub washer_type: Vec<String>,
    #[serde(default)]
    pub washer_qty: Vec<i32>,
    #[serde(default)]
    pub width: Vec<String>,
}

impl MaterialTfHavcTapeList {
    //// 将结构体转为HashMap
    pub fn into_hashmap(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.entry("参考号".to_string())
            .or_insert(self.id.to_pdms_str());
        map.entry("描述".to_string()).or_insert(self.description);
        map.entry("管段编号".to_string()).or_insert(self.seg_code);
        map.entry("子项号".to_string()).or_insert(self.sub_code);
        map.entry("材质".to_string())
            .or_insert(serde_json::to_string(&self.material).unwrap_or("[]".to_string()));
        map.entry("压力等级".to_string()).or_insert(self.pressure);
        map.entry("风管长度".to_string())
            .or_insert(self.length.to_string());
        map.entry("风管宽度".to_string())
            .or_insert(serde_json::to_string(&self.width).unwrap_or("[]".to_string()));
        map.entry("风管高度".to_string())
            .or_insert(serde_json::to_string(&self.height).unwrap_or("[]".to_string()));
        map.entry("风管壁厚".to_string())
            .or_insert(serde_json::to_string(&self.wall_thk).unwrap_or("[]".to_string()));
        map.entry("风管面积".to_string())
            .or_insert(serde_json::to_string(&self.duct_area).unwrap_or("[]".to_string()));
        map.entry("风管重量".to_string())
            .or_insert(self.duct_weight.to_string());
        map.entry("加强筋型材".to_string())
            .or_insert(self.stif_sctn);
        map.entry("加强筋长度".to_string())
            .or_insert(self.stif_len.to_string());
        map.entry("加强筋重量".to_string())
            .or_insert(self.stif_wei.to_string());
        map.entry("法兰规格".to_string()).or_insert(self.fl_type);
        map.entry("法兰长度".to_string())
            .or_insert(self.fl_len.to_string());
        map.entry("法兰重量".to_string())
            .or_insert(self.fl_wei.to_string());
        map.entry("垫圈类型".to_string())
            .or_insert(serde_json::to_string(&self.washer_type).unwrap_or("[]".to_string()));
        map.entry("垫圈长度".to_string())
            .or_insert(self.washer_len.to_string());
        map.entry("螺栓数量".to_string())
            .or_insert(serde_json::to_string(&self.bolt_qty).unwrap_or("[]".to_string()));
        map.entry("其它材料类型".to_string())
            .or_insert(self.other_type);
        map.entry("其它材料数量".to_string())
            .or_insert(self.other_qty);
        map.entry("螺杆".to_string()).or_insert(self.stud);
        map.entry("螺母数量".to_string())
            .or_insert(serde_json::to_string(&self.nut_qty).unwrap_or("[]".to_string()));
        map.entry("螺母数量_2".to_string())
            .or_insert(serde_json::to_string(&self.washer_qty).unwrap_or("[]".to_string()));
        map.entry("所在房间号".to_string())
            .or_insert(self.room_no.unwrap_or("".to_string()));
        map.entry("系统".to_string()).or_insert(self.system);
        map
    }
}

/// 通风 风管管段
#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct MaterialTfHavcFlexList {
    #[serde_as(as = "DisplayFromStr")]
    pub id: RefU64,
    #[serde(default)]
    pub bolt_qty: Vec<f32>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub duct_area: Vec<f32>,
    #[serde(default)]
    pub duct_weight: f32,
    #[serde(default)]
    pub fl_len: f32,
    #[serde(default)]
    pub fl_type: String,
    #[serde(default)]
    pub fl_wei: f32,
    #[serde(default)]
    pub height: String,
    #[serde(default)]
    pub length: f32,
    #[serde(default)]
    pub material: Vec<String>,
    #[serde(default)]
    pub nut_qty: Vec<i32>,
    #[serde(default)]
    pub other_qty: String,
    #[serde(default)]
    pub other_type: String,
    #[serde(default)]
    pub pressure: String,
    #[serde(default)]
    pub room_no: Option<String>,
    #[serde(default)]
    pub seg_code: String,
    #[serde(default)]
    pub stif_len: f32,
    #[serde(default)]
    pub stif_sctn: String,
    #[serde(default)]
    pub stif_wei: f32,
    #[serde(default)]
    pub stud: String,
    #[serde(default)]
    pub sub_code: String,
    #[serde(default)]
    pub system: String,
    #[serde(default)]
    pub wall_thk: Vec<f32>,
    #[serde(default)]
    pub washer_len: f32,
    #[serde(default)]
    pub washer_type: Vec<String>,
    #[serde(default)]
    pub washer_qty: Vec<i32>,
    #[serde(default)]
    pub width: String,
}

impl MaterialTfHavcFlexList {
    //// 将结构体转为HashMap
    pub fn into_hashmap(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.entry("参考号".to_string())
            .or_insert(self.id.to_pdms_str());
        map.entry("描述".to_string()).or_insert(self.description);
        map.entry("管段编号".to_string()).or_insert(self.seg_code);
        map.entry("子项号".to_string()).or_insert(self.sub_code);
        map.entry("材质".to_string())
            .or_insert(serde_json::to_string(&self.material).unwrap_or("[]".to_string()));
        map.entry("压力等级".to_string()).or_insert(self.pressure);
        map.entry("风管长度".to_string())
            .or_insert(self.length.to_string());
        map.entry("风管宽度".to_string()).or_insert(self.width);
        map.entry("风管高度".to_string()).or_insert(self.height);
        map.entry("风管壁厚".to_string())
            .or_insert(serde_json::to_string(&self.wall_thk).unwrap_or("[]".to_string()));
        map.entry("风管面积".to_string())
            .or_insert(serde_json::to_string(&self.duct_area).unwrap_or("[]".to_string()));
        map.entry("风管重量".to_string())
            .or_insert(self.duct_weight.to_string());
        map.entry("加强筋型材".to_string())
            .or_insert(self.stif_sctn);
        map.entry("加强筋长度".to_string())
            .or_insert(self.stif_len.to_string());
        map.entry("加强筋重量".to_string())
            .or_insert(self.stif_wei.to_string());
        map.entry("法兰规格".to_string()).or_insert(self.fl_type);
        map.entry("法兰长度".to_string())
            .or_insert(self.fl_len.to_string());
        map.entry("法兰重量".to_string())
            .or_insert(self.fl_wei.to_string());
        map.entry("垫圈类型".to_string())
            .or_insert(serde_json::to_string(&self.washer_type).unwrap_or("[]".to_string()));
        map.entry("垫圈长度".to_string())
            .or_insert(self.washer_len.to_string());
        map.entry("螺栓数量".to_string())
            .or_insert(serde_json::to_string(&self.bolt_qty).unwrap_or("[]".to_string()));
        map.entry("其它材料类型".to_string())
            .or_insert(self.other_type);
        map.entry("其它材料数量".to_string())
            .or_insert(self.other_qty);
        map.entry("螺杆".to_string()).or_insert(self.stud);
        map.entry("螺母数量".to_string())
            .or_insert(serde_json::to_string(&self.nut_qty).unwrap_or("[]".to_string()));
        map.entry("螺母数量_2".to_string())
            .or_insert(serde_json::to_string(&self.washer_qty).unwrap_or("[]".to_string()));
        map.entry("所在房间号".to_string())
            .or_insert(self.room_no.unwrap_or("".to_string()));
        map.entry("系统".to_string()).or_insert(self.system);
        map
    }
}

/// 通风 风管管段
pub async fn get_tf_hvac_material(
    db: &Surreal<Any>,
    refnos: Vec<RefU64>,
) -> anyhow::Result<Vec<HashMap<String, String>>> {
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
            let refnos = query_ele_filter_deep_children(
                refno,
                &[
                    "BEND",
                    "BRCO",
                    "CAP",
                    "FLEX",
                    "OFST",
                    "STIF",
                    "STRT",
                    "TAPE",
                    "THRE",
                    "TRNS",
                ],
            )
            .await?;
            // STRT
            let strts = refnos
                .iter()
                .filter(|x| x.noun == "STRT".to_string())
                .map(|x| x.refno)
                .collect::<Vec<_>>();
            let mut result = get_tf_hvac_strt_data(db, strts).await?;
            data.append(&mut result);
            // TAPE
            let tapes = refnos
                .iter()
                .filter(|x| x.noun == "TAPE".to_string())
                .map(|x| x.refno)
                .collect::<Vec<_>>();
            let mut result = get_tf_hvac_tape_data(db, tapes).await?;
            data.append(&mut result);
            // FLEX
            let tapes = refnos
                .iter()
                .filter(|x| x.noun == "FLEX".to_string())
                .map(|x| x.refno)
                .collect::<Vec<_>>();
            let mut result = get_tf_hvac_flex_data(db, tapes).await?;
            data.append(&mut result);
            // BEND
            let bends = refnos
                .iter()
                .filter(|x| x.noun == "BEND".to_string())
                .map(|x| x.refno)
                .collect::<Vec<_>>();
            let mut result = get_tf_hvac_bend_data(db, bends).await?;
            data.append(&mut result);
            // OFST
            let bends = refnos
                .iter()
                .filter(|x| x.noun == "OFST".to_string())
                .map(|x| x.refno)
                .collect::<Vec<_>>();
            let mut result = get_tf_hvac_ofst_data(db, bends).await?;
            data.append(&mut result);
            // TRNS
            let trnses = refnos
                .iter()
                .filter(|x| x.noun == "TRNS".to_string())
                .map(|x| x.refno)
                .collect::<Vec<_>>();
            let mut result = get_tf_hvac_trns_data(db, trnses).await?;
            data.append(&mut result);
            // BRCO
            let brcos = refnos
                .iter()
                .filter(|x| x.noun == "BRCO".to_string())
                .map(|x| x.refno)
                .collect::<Vec<_>>();
            let mut result = get_tf_hvac_brco_data(db, brcos).await?;
            data.append(&mut result);
            // THRE
            let brcos = refnos
                .iter()
                .filter(|x| x.noun == "THRE".to_string())
                .map(|x| x.refno)
                .collect::<Vec<_>>();
            let mut result = get_tf_hvac_thre_data(db, brcos).await?;
            data.append(&mut result);
            // CAP
            let caps = refnos
                .iter()
                .filter(|x| x.noun == "CAP".to_string())
                .map(|x| x.refno)
                .collect::<Vec<_>>();
            let mut result = get_tf_hvac_cap_data(db, caps).await?;
            data.append(&mut result);
            // STIF
            let caps = refnos
                .iter()
                .filter(|x| x.noun == "STIF".to_string())
                .map(|x| x.refno)
                .collect::<Vec<_>>();
            let mut result = get_tf_hvac_stif_data(db, caps).await?;
            data.append(&mut result);
        }
    }
    Ok(data)
}

/// 获取 通风 风管管段 strt 的数据
async fn get_tf_hvac_strt_data(
    db: &Surreal<Any>,
    refnos: Vec<RefU64>,
) -> anyhow::Result<Vec<HashMap<String, String>>> {
    if refnos.is_empty() {
        return Ok(vec![]);
    };
    let mut data = Vec::new();
    let refnos = refnos
        .into_iter()
        .map(|refno| refno.to_pe_key())
        .collect::<Vec<_>>();
    let sql = format!(
        "select
    fn::refno(id) as id,
    noun,
    string::concat(fn::shape_name(id), '直管') as description,
    fn::hvac_seg_code(id) as seg_code,
    fn::hvac_sub_code(id) as sub_code,
    fn::hvac_mats(id) as material,
    fn::hvac_pressure(id) as pressure,
    fn::hvac_len(id) as length,
    fn::hvac_width_format(id) as width,
    fn::hvac_height_format(id) as height,
    fn::hvac_thks(id, 'DAMP') as wall_thk,
    fn::hvac_duct_areas(id, 'DAMP') as duct_area,
    fn::hvac_duct_weight_format(id, 'DAMP') as duct_weight,
    fn::cal_stif_sctn_str(id) as stif_sctn,
    fn::cal_stif_len_str(id) as stif_len,
    fn::cal_stif_wei_str(id) as stif_wei,
    fn::hvac_fl_name(id) as fl_type,
    fn::hvac_fl_len(id) as fl_len,
    fn::hvac_fl_wei(id) as fl_wei,

    fn::common_washer_types(id) as washer_type,
    fn::common_washer_len(id) as washer_len,
    fn::hvac_bolt_qty(id) as bolt_qty,

    '-' as other_qty,
    '-' as stud,

    fn::hvac_bolt_qty(id) as nut_qty,  //螺母数量
    fn::hvac_bolt_qty(id) * 2 as washer_qty,

    fn::get_room_number(id) as room_no,
    fn::hvac_system(id) as system
	from {};",
        serde_json::to_string(&refnos).unwrap_or("[]".to_string())
    );
    let mut response = db.query(sql).await?;
    let result: Vec<HashMap<String, NamedAttrValue>> = response.take(0).unwrap();
    let r = change_hvac_result_to_map(result);
    Ok(data)
}

/// 获取 通风 风管管段 tape 的数据
async fn get_tf_hvac_tape_data(
    db: &Surreal<Any>,
    refnos: Vec<RefU64>,
) -> anyhow::Result<Vec<HashMap<String, String>>> {
    if refnos.is_empty() {
        return Ok(vec![]);
    };
    let mut data = Vec::new();
    let refnos = refnos
        .into_iter()
        .map(|refno| refno.to_pe_key())
        .collect::<Vec<_>>();
    let sql = format!(
        "select
    fn::refno(id) as id,
    noun,
    string::concat(fn::shape_name(id), '变截面管') as description,
    fn::hvac_seg_code(id) as seg_code,
    fn::hvac_sub_code(id) as sub_code,
    fn::hvac_mats(id) as material,
    fn::hvac_pressure(id) as pressure,
    fn::hvac_len(id) as length,
    [fn::hvac_width_format(id), string::replace(<string>fn::hvac_width2(id),'f','')] as width,
    [fn::hvac_height_format(id), string::replace(<string>fn::hvac_height2(id),'f','')] as height,
    fn::hvac_thks(id) as wall_thk,
    fn::hvac_duct_areas(id, 'DAMP') as duct_area,
    fn::hvac_duct_weight_format(id, 'DAMP') as duct_weight,
    fn::cal_stif_len_str(id) as stif_len,
    fn::cal_stif_wei_str(id) as stif_wei,
    fn::hvac_fl_name(id) as fl_type,
    fn::hvac_fl_len(id) as fl_len,
    fn::hvac_fl_wei(id) as fl_wei,
    fn::common_washer_types(id) as washer_type,

    fn::hvac_bolt_qtys_1(id) as bolt_qty,

    '-' as other_type,
    '-' as other_qty,
    '-' as stud,

    fn::hvac_bolt_qtys_1(id) as nut_qty,  //螺母数量
    fn::hvac_washer_bolt_qtys_1(id) as washer_qty,

    fn::hvac_system(id) as system,
    fn::get_room_number(id) as room_no
	from {};",
        serde_json::to_string(&refnos).unwrap_or("[]".to_string())
    );
    let mut response = db.query(sql).await?;
    let result: Vec<HashMap<String, NamedAttrValue>> = response.take(0).unwrap();
    let r = change_hvac_result_to_map(result);
    Ok(data)
}

/// 获取 通风 风管管段 flex 的数据
async fn get_tf_hvac_flex_data(
    db: &Surreal<Any>,
    refnos: Vec<RefU64>,
) -> anyhow::Result<Vec<HashMap<String, String>>> {
    if refnos.is_empty() {
        return Ok(vec![]);
    };
    let mut data = Vec::new();
    let refnos = refnos
        .into_iter()
        .map(|refno| refno.to_pe_key())
        .collect::<Vec<_>>();

    let sql = format!(
        "select
    fn::refno(id) as id,
    string::concat(fn::shape_name(id), '软连接') as description,
    fn::hvac_seg_code(id) as seg_code,
    fn::hvac_sub_code(id) as sub_code,
    fn::hvac_mats(id) as material,
    fn::hvac_pressure(id) as pressure,
    fn::hvac_len(id) as length,
    fn::hvac_width_format(id) as width,
    fn::hvac_height_format(id) as height,
    fn::hvac_thks(id, 'DAMP') as wall_thk,
    fn::hvac_duct_areas(id, 'DAMP') as duct_area,
    fn::hvac_duct_weight_format(id, 'DAMP') as duct_weight,
    fn::cal_stif_sctn_str(id) as stif_sctn,
    fn::cal_stif_len_str(id) as stif_len,
    fn::cal_stif_wei_str(id) as stif_wei,
    fn::hvac_fl_name(id) as fl_type,
    fn::hvac_fl_len(id) as fl_len,
    fn::hvac_fl_wei(id) as fl_wei,

     fn::common_washer_types(id) as washer_type,
     fn::common_washer_len(id) as washer_len,

     fn::hvac_bolt_qtys_THRE(id) as bolt_qty,
    '-' as other_type,
    '-' as other_qty,
    '-' as stud,

     fn::hvac_bolt_qtys_THRE(id) as nut_qty,  //螺母数量
     fn::hvac_washer_bolt_qtys_THRE(id) as washer_qty,  //垫片数量


     fn::hvac_system(id) as system,
     fn::get_room_number(id) as room_no
	from {};",
        serde_json::to_string(&refnos).unwrap_or("[]".to_string())
    );

    let mut response = db.query(sql).await?;
    let result: Vec<HashMap<String, NamedAttrValue>> = response.take(0).unwrap();
    let r = change_hvac_result_to_map(result);
    Ok(data)
}

/// 获取 通风 风管管段 bend 的数据
async fn get_tf_hvac_bend_data(
    db: &Surreal<Any>,
    refnos: Vec<RefU64>,
) -> anyhow::Result<Vec<HashMap<String, String>>> {
    if refnos.is_empty() {
        return Ok(vec![]);
    };
    let mut data = Vec::new();
    let refnos = refnos
        .into_iter()
        .map(|refno| refno.to_pe_key())
        .collect::<Vec<_>>();
    let sql = format!("select
    fn::refno(id) as id,
    string::concat(fn::shape_name(id), '变截面管') as description,
    fn::hvac_seg_code(id) as seg_code,
    fn::hvac_sub_code(id) as sub_code,
    fn::hvac_mats(id) as material,
    fn::hvac_pressure(id) as pressure,
    fn::hvac_len(id) as length,
    [fn::hvac_width_format(id), string::replace(<string>fn::hvac_width2(id),'f','')] as width,
    [fn::hvac_height_format(id), string::replace(<string>fn::hvac_height2(id),'f','')] as height,
    fn::hvac_thks(id) as wall_thk,
    fn::hvac_duct_areas(id, 'DAMP') as duct_area,
    fn::hvac_duct_weight_format(id, 'DAMP') as duct_weight,
    fn::cal_stif_len_str(id) as stif_len,
    fn::cal_stif_wei_str(id) as stif_wei,
    fn::hvac_fl_name(id) as fl_type,
    fn::hvac_fl_len(id) as fl_len,
    fn::hvac_fl_wei(id) as fl_wei,
    fn::common_washer_types(id) as washer_type,
    fn::washer_len_BEND(id) as washer_len,

    fn::hvac_nut_qty_BEND(id) as nut_qty,  //螺母数量
    if fn::hvac_bolt_qtys_1(id) != NONE {{
        [fn::hvac_bolt_qtys_1(id)[0] * 2,fn::hvac_bolt_qtys_1(id)[1] * 2] }} else {{ [0] }} as washer_qty,

    fn::hvac_system(id) as system,
    fn::get_room_number(id) as room_no
	from {};", serde_json::to_string(&refnos).unwrap_or("[]".to_string()));
    let mut response = db.query(sql).await?;
    let result: Vec<HashMap<String, NamedAttrValue>> = response.take(0).unwrap();
    let r = change_hvac_result_to_map(result);
    Ok(data)
}

/// 将通风材料表单的数据转化为hashmap，并将key改为对应的中文
fn change_hvac_result_to_map(
    input: Vec<HashMap<String, NamedAttrValue>>,
) -> Vec<HashMap<String, String>> {
    let chinese_name_map = get_hvac_chinese_name_map();
    let mut result = Vec::new();
    for i in input {
        let mut map = HashMap::new();
        for (k, v) in i {
            let Some(chinese) = chinese_name_map.get(&k) else {
                continue;
            };
            map.entry(chinese.to_string())
                .or_insert(v.get_val_as_string());
        }
        result.push(map);
    }
    result
}

/// 获取通风材料表单字段对应的中文名
fn get_hvac_chinese_name_map() -> HashMap<String, String> {
    let mut map = HashMap::new();
    map.entry("id".to_string()).or_insert("参考号".to_string());
    map.entry("description".to_string())
        .or_insert("描述".to_string());
    map.entry("seg_code".to_string())
        .or_insert("管段编号".to_string());
    map.entry("sub_code".to_string())
        .or_insert("子项号".to_string());
    map.entry("material".to_string())
        .or_insert("材质".to_string());
    map.entry("pressure".to_string())
        .or_insert("压力等级".to_string());
    map.entry("length".to_string())
        .or_insert("风管长度".to_string());
    map.entry("width".to_string())
        .or_insert("风管宽度".to_string());
    map.entry("height".to_string())
        .or_insert("风管高度".to_string());
    map.entry("wall_thk".to_string())
        .or_insert("风管壁厚".to_string());
    map.entry("duct_area".to_string())
        .or_insert("风管面积".to_string());
    map.entry("duct_weight".to_string())
        .or_insert("法兰重量".to_string());
    map.entry("stif_sctn".to_string())
        .or_insert("加强筋型材".to_string());
    map.entry("stif_len".to_string())
        .or_insert("加强筋长度".to_string());
    map.entry("stif_wei".to_string())
        .or_insert("加强筋重量".to_string());
    map.entry("fl_type".to_string())
        .or_insert("法兰规格".to_string());
    map.entry("fl_len".to_string())
        .or_insert("法兰长度".to_string());
    map.entry("fl_wei".to_string())
        .or_insert("法兰重量".to_string());
    map.entry("washer_type".to_string())
        .or_insert("垫圈类型".to_string());
    map.entry("washer_len".to_string())
        .or_insert("垫圈长度".to_string());
    map.entry("bolt_qty".to_string())
        .or_insert("螺栓数量".to_string());
    map.entry("other_type".to_string())
        .or_insert("其它材料类型".to_string());
    map.entry("other_qty".to_string())
        .or_insert("其它材料数量".to_string());
    map.entry("stud".to_string()).or_insert("螺杆".to_string());
    map.entry("nut_qty".to_string())
        .or_insert("螺母数量".to_string());
    map.entry("washer_qty".to_string())
        .or_insert("螺母数量_2".to_string());
    map.entry("room_no".to_string())
        .or_insert("所在房间号".to_string());
    map.entry("system".to_string())
        .or_insert("系统".to_string());

    map
}

/// 获取 通风 风管管段 ofst 的数据
async fn get_tf_hvac_ofst_data(
    db: &Surreal<Any>,
    refnos: Vec<RefU64>,
) -> anyhow::Result<Vec<HashMap<String, String>>> {
    if refnos.is_empty() {
        return Ok(vec![]);
    };
    let refnos = refnos
        .into_iter()
        .map(|refno| refno.to_pe_key())
        .collect::<Vec<_>>();
    let sql = format!(
        "select
    fn::refno(id) as id,
    string::concat(fn::shape_name(id), '软连接') as description,
    fn::hvac_seg_code(id) as seg_code,
    fn::hvac_sub_code(id) as sub_code,
    fn::hvac_mats(id) as material,
    fn::hvac_pressure(id) as pressure,
    fn::hvac_len(id) as length,
    fn::hvac_width_format(id) as width,
    fn::hvac_height_format(id) as height,
    fn::hvac_thks(id, 'DAMP') as wall_thk,
    fn::hvac_duct_areas(id, 'DAMP') as duct_area,
    fn::hvac_duct_weight_format(id, 'DAMP') as duct_weight,
    fn::cal_stif_sctn_str(id) as stif_sctn,
    fn::cal_stif_len_str(id) as stif_len,
    fn::cal_stif_wei_str(id) as stif_wei,
    fn::hvac_fl_name(id) as fl_type,
    fn::hvac_fl_len(id) as fl_len,
    fn::hvac_fl_wei(id) as fl_wei,

    fn::common_washer_types(id) as washer_type,
    fn::common_washer_len(id) as washer_len,

    fn::hvac_bolt_qtys_OFST(id) as bolt_qty,

    '-' as other_type,
    NONE as other_qty,
    '-' as stud,

    fn::hvac_bolt_qtys_OFST(id) as nut_qty,  //螺母数量
    [fn::hvac_bolt_qtys_OFST(id)*2, fn::hvac_bolt_qtys_OFST(id)*2] as washer_qty,  //垫片数量

    fn::hvac_system(id) as system,
    fn::get_room_number(id) as room_no
	from {};",
        serde_json::to_string(&refnos).unwrap_or("[]".to_string())
    );
    let mut response = db.query(sql).await?;
    let result: Vec<HashMap<String, NamedAttrValue>> = response.take(0).unwrap();
    let r = change_hvac_result_to_map(result);
    Ok(r)
}

/// 获取 通风 风管管段 trns 的数据
async fn get_tf_hvac_trns_data(
    db: &Surreal<Any>,
    refnos: Vec<RefU64>,
) -> anyhow::Result<Vec<HashMap<String, String>>> {
    if refnos.is_empty() {
        return Ok(vec![]);
    };
    let refnos = refnos
        .into_iter()
        .map(|refno| refno.to_pe_key())
        .collect::<Vec<_>>();
    let sql = format!(
        "select
        fn::refno(id) as id,
        '天圆地方' as description,
        fn::hvac_seg_code(id) as seg_code,
        fn::hvac_sub_code(id) as sub_code,
        fn::hvac_mats(id) as material,
        fn::hvac_pressure(id) as pressure,
        fn::hvac_len(id) as length,
        fn::hvac_width_format(id) as width,
        fn::hvac_height_format(id) as height,
        fn::hvac_thks(id, 'DAMP') as wall_thk,
        fn::hvac_duct_areas(id, 'DAMP') as duct_area,
        fn::hvac_duct_weight_format(id, 'DAMP') as duct_weight,
        fn::cal_stif_sctn_str(id) as stif_sctn,
        fn::cal_stif_len_str(id) as stif_len,
        fn::cal_stif_wei_str(id) as stif_wei,
        fn::hvac_fl_name(id) as fl_type,
        fn::hvac_fl_len_TRNS(id) as fl_len,
        fn::hvac_fl_wei(id) as fl_wei,

        fn::washer_type_TRNS(id) as washer_type,
        fn::washer_len_TRNS(id) as washer_len,

        fn::hvac_bolt_qtys_TRNS(id) as bolt_qty,  //螺栓数量

        '-' as other_type,
        '-' as other_qty,
        '-' as stud,

        fn::hvac_bolt_qtys_TRNS(id) as nut_qty,  //螺母数量
        fn::hvac_washer_bolt_qtys_TRNS(id) as washer_qty,  //垫片数量

        fn::hvac_system(id) as system,
        fn::get_room_number(id) as room_no
	from {};",
        serde_json::to_string(&refnos).unwrap_or("[]".to_string())
    );
    let mut response = db.query(sql).await?;
    let result: Vec<HashMap<String, NamedAttrValue>> = response.take(0).unwrap();
    let r = change_hvac_result_to_map(result);
    Ok(r)
}

/// 获取 通风 风管管段 brco 的数据
async fn get_tf_hvac_brco_data(
    db: &Surreal<Any>,
    refnos: Vec<RefU64>,
) -> anyhow::Result<Vec<HashMap<String, String>>> {
    if refnos.is_empty() {
        return Ok(vec![]);
    };
    let refnos = refnos
        .into_iter()
        .map(|refno| refno.to_pe_key())
        .collect::<Vec<_>>();
    let sql = format!(
        "select
      fn::refno(id) as id,
      string::concat(fn::shape_name(id), '连接管') as description,
      fn::hvac_seg_code(id) as seg_code,
      fn::hvac_sub_code(id) as sub_code,
      fn::hvac_mats(id) as material,
      fn::hvac_pressure(id) as pressure,
      fn::hvac_len(id) as length,
      fn::hvac_width_format(id, 3) as width,
      fn::hvac_height_format(id, 3) as height,
      fn::hvac_thks(id, 'DAMP') as wall_thk,
      fn::hvac_duct_areas(id, 'DAMP') as duct_area,
      fn::hvac_duct_weight_format(id, 'DAMP') as duct_weight,
      fn::cal_stif_sctn_str(id) as stif_sctn,
      fn::cal_stif_len_str(id) as stif_len,
      fn::cal_stif_wei_str(id) as stif_wei,
      fn::hvac_fl_name(id) as fl_type,
      fn::hvac_fl_len(id) as fl_len,
      fn::hvac_fl_wei(id) as fl_wei,

       fn::common_washer_types(id) as washer_type,
       fn::common_washer_len(id) as washer_len,

      // fn::hvac_bolt_qtys_BRCO(id) as bolt_qty,
      '-' as other_type,
      '-' as other_qty,
      '-' as stud,

       // fn::hvac_bolt_qtys_BRCO(id) as nut_qty,  //螺母数量
       fn::hvac_washer_bolt_qtys_THRE(id) as washer_qty,  //垫片数量

       fn::hvac_system(id) as system,
       fn::get_room_number(id) as room_no
	from {};",
        serde_json::to_string(&refnos).unwrap_or("[]".to_string())
    );
    let mut response = db.query(sql).await?;
    let result: Vec<HashMap<String, NamedAttrValue>> = response.take(0).unwrap();
    let r = change_hvac_result_to_map(result);
    Ok(r)
}

/// 获取 通风 风管管段 thre 的数据
async fn get_tf_hvac_thre_data(
    db: &Surreal<Any>,
    refnos: Vec<RefU64>,
) -> anyhow::Result<Vec<HashMap<String, String>>> {
    if refnos.is_empty() {
        return Ok(vec![]);
    };
    let refnos = refnos
        .into_iter()
        .map(|refno| refno.to_pe_key())
        .collect::<Vec<_>>();
    let sql = format!(
        "select
    fn::refno(id) as id,
    string::concat(fn::shape_name(id), '三通管') as description,
    fn::hvac_seg_code(id) as seg_code,
    fn::hvac_sub_code(id) as sub_code,
    fn::hvac_mats(id) as material,
    fn::hvac_pressure(id) as pressure,
    fn::hvac_len(id) as length,
    fn::hvac_width_three(id) as width,
    fn::hvac_height_three(id) as height,
    fn::hvac_thks(id) as wall_thk,
    fn::hvac_duct_areas(id, 'DAMP') as duct_area,
    fn::hvac_duct_weight_format(id, 'DAMP') as duct_weight,
    '-' as stif_sctn,
    0 as stif_len,
    0 as stif_wei,
    fn::hvac_fl_name_THRE(id) as fl_type,
    fn::hvac_fl_len_THRE(id) as fl_len,
    fn::hvac_fl_wei(id) as fl_wei,
    fn::washer_type_THRE(id) as washer_type,
    fn::washer_len_THRE(id) as washer_len,

    '-' as other_type,
    '-' as other_qty,
    '-' as stud,

    fn::hvac_bolt_qtys_THRE(id) as nut_qty,  //螺母数量
    fn::hvac_washer_bolt_qtys_THRE(id) as washer_qty,  //垫片数量

    fn::hvac_system(id) as system,
    fn::get_room_number(id) as room_no
	from {};",
        serde_json::to_string(&refnos).unwrap_or("[]".to_string())
    );
    let mut response = db.query(sql).await?;
    let result: Vec<HashMap<String, NamedAttrValue>> = response.take(0).unwrap();
    let r = change_hvac_result_to_map(result);
    Ok(r)
}

/// 获取 通风 风管管段 cap 的数据
async fn get_tf_hvac_cap_data(
    db: &Surreal<Any>,
    refnos: Vec<RefU64>,
) -> anyhow::Result<Vec<HashMap<String, String>>> {
    if refnos.is_empty() {
        return Ok(vec![]);
    };
    let refnos = refnos
        .into_iter()
        .map(|refno| refno.to_pe_key())
        .collect::<Vec<_>>();
    let sql = format!(
        "select
    fn::refno(id) as id,
    string::concat(fn::shape_name(id), '封头') as description,
    fn::hvac_seg_code(id) as seg_code,
    fn::hvac_sub_code(id) as sub_code,
    fn::hvac_mats(id) as material,
    fn::hvac_pressure(id) as pressure,
    fn::hvac_len(id) as length,
    fn::hvac_width_format(id) as width,
    fn::hvac_height_format(id) as height,
    fn::hvac_thks(id) as wall_thk,
    fn::hvac_duct_areas(id, 'DAMP') as duct_area,
    fn::hvac_duct_weight_format(id, 'DAMP') as duct_weight,
    fn::cal_stif_len_str(id) as stif_len,
    fn::cal_stif_wei_str(id) as stif_wei,
    fn::hvac_fl_name(id) as fl_type,
    fn::hvac_fl_len(id) as fl_len,
    fn::hvac_fl_wei(id) as fl_wei,
    fn::washer_type_CAP(id) as washer_type,
    fn::washer_len_CAP(id) as washer_len,

    fn::hvac_bolt_qtys_1(id) as nut_qty,  //螺母数量
    fn::hvac_washer_bolt_qtys_1(id) as washer_qty,

    fn::hvac_system(id) as system,
    fn::get_room_number(id) as room_no
	from {};",
        serde_json::to_string(&refnos).unwrap_or("[]".to_string())
    );
    let mut response = db.query(sql).await?;
    let result: Vec<HashMap<String, NamedAttrValue>> = response.take(0).unwrap();
    let r = change_hvac_result_to_map(result);
    Ok(r)
}

/// 获取 通风 风管管段 stif 的数据
async fn get_tf_hvac_stif_data(
    db: &Surreal<Any>,
    refnos: Vec<RefU64>,
) -> anyhow::Result<Vec<HashMap<String, String>>> {
    if refnos.is_empty() {
        return Ok(vec![]);
    };
    let refnos = refnos
        .into_iter()
        .map(|refno| refno.to_pe_key())
        .collect::<Vec<_>>();
    let sql = format!(
        "select
    fn::refno(id) as id,
    string::concat(fn::shape_name(id), '加强筋') as description,
    fn::hvac_seg_code(id) as seg_code,
    fn::hvac_sub_code(id) as sub_code,
    fn::hvac_mats(id) as material,
    fn::hvac_pressure(id) as pressure,
    fn::hvac_len(id) as length,
    fn::hvac_width_format(id) as width,
    fn::hvac_height_format(id) as height,
    fn::hvac_thks(id, 'DAMP') as wall_thk,
    fn::hvac_duct_areas(id, 'DAMP') as duct_area,
    fn::hvac_duct_weight_format(id, 'DAMP') as duct_weight,
    fn::cal_stif_sctn_str(id) as stif_sctn,
    fn::cal_stif_len_str(id) as stif_len,
    fn::cal_stif_wei_str(id) as stif_wei,
    '-' as fl_type,
    '-' as fl_len,
    '-' as fl_wei,

    '-' as washer_type,
    '-' as washer_len,
    '-' as bolt_qty,
    '-' as other_type,
    '-' as other_qty,
    '-' as stud,

    fn::hvac_bolt_qty(id) as nut_qty,  //螺母数量
    fn::hvac_bolt_qty(id) * 2 as washer_qty,

    fn::get_room_number(id) as room_no,
    fn::hvac_system(id) as system
	from {};",
        serde_json::to_string(&refnos).unwrap_or("[]".to_string())
    );
    let mut response = db.query(sql).await?;
    let result: Vec<HashMap<String, NamedAttrValue>> = response.take(0).unwrap();
    let r = change_hvac_result_to_map(result);
    Ok(r)
}
