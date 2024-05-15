use crate::options::DbOption;
use crate::pe::SPdmsElement;
use crate::rs_surreal::table_const::GY_DZCL;
use crate::{
    connect_surdb, get_children_pes, get_pe, insert_into_table, insert_into_table_with_chunks,
    query_ele_filter_deep_children, query_filter_deep_children, NamedAttrValue, RefU64, SurlValue,
    SUL_DB,
};
use config::{Config, File};
use parry3d::partitioning::QbvhDataGenerator;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ptr::eq;
use std::str::FromStr;
use std::sync::mpsc;
use surrealdb::engine::any::Any;
use surrealdb::engine::remote::ws::{Client, Ws};
use surrealdb::key::thing::Thing;
use surrealdb::Surreal;
// use crate::test::test_surreal::init_test_surreal;
use crate::aios_db_mgr::aios_mgr::AiosDBMgr;
use crate::pdms_types::ser_refno_as_str;
use crate::ssc_setting::{execute_save_pbs, query_all_site_with_major, SaveDatabaseChannelMsg};
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use tokio::task;

/// 保存所有的材料表单数据
pub async fn save_all_material_data() -> anyhow::Result<()> {
    // 提前跑已经创建surreal的方法
    if let Err(e) = define_surreal_functions(SUL_DB.clone()).await {
        dbg!(e.to_string());
        return Ok(());
    }
    let mut handles = Vec::new();
    // 查找所有带专业的site
    let sites = query_all_site_with_major().await?;
    // 处理所有专业表单的数据
    for site in sites {
        if site.major != "V".to_string() {
            continue;
        };
        dbg!(&site);
        match site.major.as_str() {
            // 工艺
            "T" => {
                // 大宗材料
                println!("工艺布置专业-大宗材料");
                match get_gy_dzcl(SUL_DB.clone(), vec![site.id]).await {
                    Ok((r, tubi_r)) => {
                        let task = task::spawn(async move {
                            match insert_into_table_with_chunks(&SUL_DB, "material_gy_list", r)
                                .await
                            {
                                Ok(_) => {}
                                Err(e) => {
                                    dbg!(&e.to_string());
                                }
                            }
                            match insert_into_table_with_chunks(&SUL_DB, "material_gy_list", tubi_r)
                                .await
                            {
                                Ok(_) => {}
                                Err(e) => {
                                    dbg!(&e.to_string());
                                }
                            }
                        });
                        handles.push(task);
                    }
                    Err(e) => {
                        dbg!(&e.to_string());
                    }
                }
                // 设备清单
                println!("工艺布置专业-设备清单");
                match get_gy_equi_list(SUL_DB.clone(), vec![site.id]).await {
                    Ok(r) => {
                        let task = task::spawn(async move {
                            match insert_into_table_with_chunks(&SUL_DB, "material_gy_equi", r)
                                .await
                            {
                                Ok(_) => {}
                                Err(e) => {
                                    dbg!(&e.to_string());
                                }
                            }
                        });
                        handles.push(task);
                    }
                    Err(e) => {
                        dbg!(&e.to_string());
                    }
                }
                // 阀门清单
                println!("工艺布置专业-阀门清单");
                match get_gy_valv_list(SUL_DB.clone(), vec![site.id]).await {
                    Ok(r) => {
                        let task = task::spawn(async move {
                            match insert_into_table_with_chunks(&SUL_DB, "material_gy_valv", r)
                                .await
                            {
                                Ok(_) => {}
                                Err(e) => {
                                    dbg!(&e.to_string());
                                }
                            }
                        });
                        handles.push(task);
                    }
                    Err(e) => {
                        dbg!(&e.to_string());
                    }
                }
            }
            // 仪控
            "I" => {
                // 大宗材料
                println!("仪控专业-大宗材料");
                let r = get_yk_dzcl_list(SUL_DB.clone(), vec![site.id]).await?;
                let task = task::spawn(async move {
                    match insert_into_table_with_chunks(&SUL_DB, "material_inst_list", r).await {
                        Ok(_) => {}
                        Err(e) => {
                            dbg!(&e.to_string());
                        }
                    }
                });
                handles.push(task);
                // 仪表管道
                println!("仪控专业-仪表管道");
                let r = get_yk_inst_pipe(SUL_DB.clone(), vec![site.id]).await?;
                let task = task::spawn(async move {
                    match insert_into_table_with_chunks(&SUL_DB, "material_inst_pipe", r).await {
                        Ok(_) => {}
                        Err(e) => {
                            dbg!(&e.to_string());
                        }
                    }
                });
                handles.push(task);
                // 设备清单
                println!("仪控专业-设备清单");
                let r = get_yk_equi_list_material(SUL_DB.clone(), vec![site.id]).await?;
                let task = task::spawn(async move {
                    match insert_into_table_with_chunks(&SUL_DB, "material_inst_equi", r).await {
                        Ok(_) => {}
                        Err(e) => {
                            dbg!(&e.to_string());
                        }
                    }
                });
                handles.push(task);
            }
            // 通风
            "V" => {
                // 风管管段
                let r = get_tf_hvac_material(&SUL_DB, vec![site.id]).await?;
                let task = task::spawn(async move {
                    match insert_into_table_with_chunks(&SUL_DB, "material_hvac_pipe", r).await {
                        Ok(_) => {}
                        Err(e) => {
                            dbg!(&e.to_string());
                        }
                    }
                });
                handles.push(task);
            }
            // 电气
            "E" => {
                // 托盘及接地
                println!("电气专业-托盘及接地");
                let (r, str_r) = get_dq_bran_list(SUL_DB.clone(), vec![site.id]).await?;
                let task = task::spawn(async move {
                    match insert_into_table_with_chunks(&SUL_DB, "material_elec_list", r).await {
                        Ok(_) => {}
                        Err(e) => {
                            dbg!(&e.to_string());
                        }
                    }
                });
                handles.push(task);
                let task = task::spawn(async move {
                    match insert_into_table_with_chunks(&SUL_DB, "material_elec_list", str_r).await
                    {
                        Ok(_) => {}
                        Err(e) => {
                            dbg!(&e.to_string());
                        }
                    }
                });
                handles.push(task);
            }
            // 通信
            "TX" => {
                // 通信系统
                println!("通信专业-通信系统");
                let r = get_tx_txsb_list_material(SUL_DB.clone(), vec![site.id]).await?;
                let task = task::spawn(async move {
                    match insert_into_table_with_chunks(&SUL_DB, "material_tx_list", r).await {
                        Ok(_) => {}
                        Err(e) => {
                            dbg!(&e.to_string());
                        }
                    }
                });
                handles.push(task);
            }
            // 给排水
            "W" => {
                // 大宗材料
                println!("给排水专业-大宗材料");
                let r = get_gps_dzcl_material(SUL_DB.clone(), vec![site.id]).await?;
                let task = task::spawn(async move {
                    match insert_into_table_with_chunks(&SUL_DB, "material_gps_list", r).await {
                        Ok(_) => {}
                        Err(e) => {
                            dbg!(&e.to_string());
                        }
                    }
                });
                handles.push(task);
            }
            // 设备
            "SB" => {
                // 大宗材料
                println!("设备专业-大宗材料");
                let r = get_sb_dzcl_list_material(SUL_DB.clone(), vec![site.id]).await?;
                let task = task::spawn(async move {
                    match insert_into_table_with_chunks(&SUL_DB, "material_sb_list", r).await {
                        Ok(_) => {}
                        Err(e) => {
                            dbg!(&e.to_string());
                        }
                    }
                });
                handles.push(task);
            }
            // 暖通
            "N" => {
                // 阀门清单
                println!("暖通专业-阀门清单");
                let r = get_nt_valv_list_material(SUL_DB.clone(), vec![site.id]).await?;
                let task = task::spawn(async move {
                    match insert_into_table_with_chunks(&SUL_DB, "material_nt_valv", r).await {
                        Ok(_) => {}
                        Err(e) => {
                            dbg!(&e.to_string());
                        }
                    }
                });
                handles.push(task);
            }

            _ => {}
        }
    }
    // 等待保存线程完成
    dbg!("查询完毕，等待数据库保存完成");
    futures::prelude::future::join_all(handles).await;
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MaterialGyDataBend {
    pub id: RefU64,
    pub code: String,
    pub noun: String,
    pub count: f32,
}

impl MaterialGyDataBend {
    //// 将结构体转为HashMap
    pub fn into_hashmap(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.entry("参考号".to_string())
            .or_insert(self.id.to_pdms_str());
        map.entry("编码".to_string()).or_insert(self.code);
        map.entry("部件".to_string()).or_insert(self.noun);
        map.entry("数量".to_string())
            .or_insert(self.count.to_string());
        map
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MaterialGyData {
    pub id: RefU64,
    pub code: String,
    pub noun: String,
}

impl MaterialGyData {
    //// 将结构体转为HashMap
    pub fn into_hashmap(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.entry("参考号".to_string())
            .or_insert(self.id.to_pdms_str());
        map.entry("编码".to_string()).or_insert(self.code);
        map.entry("类型".to_string()).or_insert(self.noun);
        map
    }

    pub fn into_yk_hashmap(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.entry("参考号".to_string())
            .or_insert(self.id.to_pdms_str());
        map.entry("编码".to_string()).or_insert(self.code);
        map.entry("品名".to_string()).or_insert(self.noun);
        map
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MaterialGyValvList {
    pub id: RefU64,
    pub valv_name: String,
    pub room_code: Option<String>,
    pub valv_belong: String,
    pub valv_length: Option<f32>,
    pub valv_weight: Option<f32>,
    pub valv_x: Option<f32>,
    pub valv_y: Option<f32>,
    pub valv_z: Option<f32>,
    pub valv_supp: String,
}

impl MaterialGyValvList {
    //// 将结构体转为HashMap
    pub fn into_hashmap(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.entry("参考号".to_string())
            .or_insert(self.id.to_pdms_str());
        map.entry("阀门位号".to_string())
            .or_insert(self.valv_name.to_string());
        map.entry("所在房间号".to_string())
            .or_insert(self.room_code.unwrap_or("".to_string()));
        map.entry("阀门归属".to_string())
            .or_insert(self.valv_belong.to_string());
        // 没有的给个默认值
        let valv_length = self.valv_length.map_or("0".to_string(), |x| x.to_string());
        map.entry("阀门长度".to_string())
            .or_insert(valv_length.to_string());
        let valv_weight = self.valv_weight.map_or("0".to_string(), |x| x.to_string());
        map.entry("阀门重量".to_string())
            .or_insert(valv_weight.to_string());
        let valv_x = self.valv_x.map_or("0".to_string(), |x| x.to_string());
        map.entry("阀门重心X".to_string())
            .or_insert(valv_x.to_string());
        let valv_y = self.valv_y.map_or("0".to_string(), |x| x.to_string());
        map.entry("阀门重心Y".to_string())
            .or_insert(valv_y.to_string());
        let valv_z = self.valv_z.map_or("0".to_string(), |x| x.to_string());
        map.entry("阀门重心Z".to_string())
            .or_insert(valv_z.to_string());

        map.entry("是否阀门支架".to_string())
            .or_insert(self.valv_supp.to_string());
        map
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MaterialGyEquiList {
    pub id: RefU64,
    pub name: String,
    pub room_code: Option<String>,
    pub nozz_name: Vec<String>,
    pub nozz_pos: Vec<Vec<f32>>,
    pub nozz_cref: Vec<String>,
}

impl MaterialGyEquiList {
    //// 将结构体转为HashMap
    pub fn into_hashmap(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.entry("参考号".to_string())
            .or_insert(self.id.to_pdms_str());
        map.entry("设备位号".to_string())
            .or_insert(self.name.to_string());
        map.entry("所在房间号".to_string())
            .or_insert(self.room_code.unwrap_or("".to_string()));
        map.entry("管口号".to_string())
            .or_insert(serde_json::to_string(&self.nozz_name).unwrap_or("[]".to_string()));
        map.entry("管口坐标".to_string())
            .or_insert(serde_json::to_string(&self.nozz_pos).unwrap_or("[]".to_string()));
        map.entry("相连管道编号".to_string())
            .or_insert(serde_json::to_string(&self.nozz_cref).unwrap_or("[]".to_string()));

        map
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

/// 电气 托盘及接地
#[derive(Debug, Serialize, Deserialize)]
pub struct MaterialDqMaterialList {
    pub id: RefU64,
    pub num: Option<String>,
    pub project_num: Option<String>,
    pub project_name: Option<String>,
    pub major: String,
    pub room_code: Option<String>,
    pub name: String,
    pub pos: Option<f32>,
    pub bran_type: Option<String>,
    pub material: Option<String>,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub size_num: Option<f32>,
    pub b_painting: Option<String>,
    pub painting_color: Option<String>,
    pub b_cover: Option<String>,
    pub b_partition: Option<String>,
    pub partition_num: Option<String>,
    pub spre: Option<String>,
    pub catr: Option<String>,
    pub horizontal_or_vertical: Option<String>,
    #[serde(default)]
    pub stander_num: Option<String>,
    #[serde(default)]
    pub item_num: Option<String>,
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub count: Option<f32>,
}

impl MaterialDqMaterialList {
    //// 将结构体转为HashMap
    pub fn into_hashmap(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.entry("参考号".to_string())
            .or_insert(self.id.to_pdms_str());
        map.entry("机组号".to_string())
            .or_insert(self.num.unwrap_or("".to_string()));
        map.entry("子项号".to_string())
            .or_insert(self.project_num.unwrap_or("".to_string()));
        map.entry("元件等级名称".to_string())
            .or_insert(self.spre.clone().unwrap_or("".to_string()));
        map.entry("子项名称".to_string())
            .or_insert(self.project_name.unwrap_or("".to_string()));
        map.entry("专业".to_string()).or_insert(self.major);
        map.entry("房间号".to_string())
            .or_insert(self.room_code.unwrap_or("".to_string()));
        map.entry("托盘段号".to_string()).or_insert(self.name);
        map.entry("托盘标高".to_string())
            .or_insert(self.pos.unwrap_or(0.0).to_string());
        map.entry("托盘类型".to_string())
            .or_insert(self.bran_type.unwrap_or("".to_string()));
        map.entry("材质".to_string())
            .or_insert(self.material.unwrap_or("".to_string()));
        map.entry("托盘宽度mm".to_string())
            .or_insert(self.width.unwrap_or(0.0).to_string());
        map.entry("托盘高度mm".to_string())
            .or_insert(self.height.unwrap_or(0.0).to_string());
        map.entry("规格型号".to_string())
            .or_insert(self.size_num.unwrap_or(0.0).to_string());
        map.entry("是否刷漆".to_string())
            .or_insert(self.b_painting.unwrap_or("".to_string()));
        map.entry("刷漆颜色".to_string())
            .or_insert(self.painting_color.unwrap_or("".to_string()));
        map.entry("有无隔板".to_string())
            .or_insert(self.b_partition.unwrap_or("".to_string()));
        map.entry("隔板编号".to_string())
            .or_insert(self.partition_num.unwrap_or("".to_string()));
        map.entry("spre".to_string())
            .or_insert(self.spre.unwrap_or("".to_string()));
        map.entry("catr".to_string())
            .or_insert(self.catr.unwrap_or("".to_string()));
        map.entry("水平/竖向".to_string())
            .or_insert(self.horizontal_or_vertical.unwrap_or("".to_string()));
        map.entry("标准号".to_string())
            .or_insert(self.stander_num.unwrap_or("".to_string()));
        map.entry("物项编号".to_string())
            .or_insert(self.item_num.unwrap_or("".to_string()));
        map.entry("单位".to_string())
            .or_insert(self.unit.unwrap_or("".to_string()));
        map.entry("数量".to_string())
            .or_insert(self.count.unwrap_or(0.0).to_string());
        map
    }
}

/// 电气 托盘及接地
#[derive(Debug, Serialize, Deserialize)]
pub struct MaterialDqMaterialListStru {
    pub id: RefU64,
    pub num: Option<String>,
    pub project_num: Option<String>,
    pub project_name: Option<String>,
    pub major: String,
    pub room_code: Option<String>,
    pub pos: Option<f32>,
    pub material: Option<String>,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub supp_name: Option<String>,
    pub size_num: Option<String>,
    pub b_painting: Option<String>,
    pub painting_color: Option<String>,
    pub b_cover: Option<String>,
    pub b_partition: Option<String>,
    pub partition_num: Option<String>,
    pub spre: Option<String>,
    pub catr: Option<String>,
    pub horizontal_or_vertical: Option<String>,
    #[serde(default)]
    pub stander_num: Option<String>,
    #[serde(default)]
    pub item_num: Option<String>,
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub count: Option<String>,
}

impl MaterialDqMaterialListStru {
    //// 将结构体转为HashMap
    pub fn into_hashmap(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.entry("参考号".to_string())
            .or_insert(self.id.to_pdms_str());
        map.entry("机组号".to_string())
            .or_insert(self.num.unwrap_or("".to_string()));
        map.entry("子项号".to_string())
            .or_insert(self.project_num.unwrap_or("".to_string()));
        map.entry("元件等级名称".to_string())
            .or_insert(self.spre.clone().unwrap_or("".to_string()));
        map.entry("子项名称".to_string())
            .or_insert(self.project_name.unwrap_or("".to_string()));
        map.entry("专业".to_string()).or_insert(self.major);
        map.entry("房间号".to_string())
            .or_insert(self.room_code.unwrap_or("".to_string()));
        map.entry("托盘标高".to_string())
            .or_insert(self.pos.unwrap_or(0.0).to_string());
        map.entry("材质".to_string())
            .or_insert(self.material.unwrap_or("".to_string()));
        map.entry("托盘支吊架名称".to_string())
            .or_insert(self.supp_name.unwrap_or("".to_string()));
        map.entry("托盘宽度mm".to_string())
            .or_insert(self.width.unwrap_or(0.0).to_string());
        map.entry("托盘高度mm".to_string())
            .or_insert(self.height.unwrap_or(0.0).to_string());
        map.entry("规格型号".to_string())
            .or_insert(self.size_num.unwrap_or("".to_string()));
        map.entry("是否刷漆".to_string())
            .or_insert(self.b_painting.unwrap_or("".to_string()));
        map.entry("刷漆颜色".to_string())
            .or_insert(self.painting_color.unwrap_or("".to_string()));
        map.entry("有无隔板".to_string())
            .or_insert(self.b_partition.unwrap_or("".to_string()));
        map.entry("隔板编号".to_string())
            .or_insert(self.partition_num.unwrap_or("".to_string()));
        map.entry("spre".to_string())
            .or_insert(self.spre.unwrap_or("".to_string()));
        map.entry("catr".to_string())
            .or_insert(self.catr.unwrap_or("".to_string()));
        map.entry("水平/竖向".to_string())
            .or_insert(self.horizontal_or_vertical.unwrap_or("".to_string()));
        map.entry("标准号".to_string())
            .or_insert(self.stander_num.unwrap_or("".to_string()));
        map.entry("物项编号".to_string())
            .or_insert(self.item_num.unwrap_or("".to_string()));
        map.entry("单位".to_string())
            .or_insert(self.unit.unwrap_or("".to_string()));
        map
    }
}

///查询工艺大宗材料数据
pub async fn get_gy_dzcl(
    db: Surreal<Any>,
    refnos: Vec<RefU64>,
) -> anyhow::Result<(Vec<MaterialGyData>, Vec<MaterialGyDataBend>)> {
    let mut data = Vec::new();
    let mut tubi_data = Vec::new();
    for refno in refnos {
        let Some(pe) = get_pe(refno).await? else {
            continue;
        };
        // 如果是site，则需要过滤 site的 name
        if pe.noun == "SITE".to_string() {
            if !pe.name.contains("PIPE") {
                continue;
            };
        }
        // 查询bend的数据
        let refnos = query_filter_deep_children(refno, vec!["BEND".to_string()]).await?;
        let refnos_str = serde_json::to_string(
            &refnos
                .into_iter()
                .map(|refno| refno.to_pe_key())
                .collect::<Vec<String>>(),
        )?;
        let sql = format!(
            r#"select
    id as id,
    string::split(string::split(if refno.SPRE.name == NONE {{ "//:" }} else {{ refno.SPRE.name }},'/')[2],':')[0] as code, // 编码
    refno.TYPE as noun, // 部件
    math::fixed((refno.ANGL / 360) * 2 * 3.1415 * refno.SPRE.refno.CATR.refno.PARA[1],2) as count // 长度
    from {}"#,
            refnos_str
        );
        let mut response = db.query(sql).await?;
        let mut result: Vec<MaterialGyDataBend> = response.take(0)?;
        tubi_data.append(&mut result);
        // 查询tubi数据
        let refnos = query_filter_deep_children(refno, vec!["BRAN".to_string()]).await?;
        let refnos_str = serde_json::to_string(
            &refnos
                .into_iter()
                .map(|refno| refno.to_pe_key())
                .collect::<Vec<String>>(),
        )?;
        let sql = format!(
            r#"
    select value (select leave as id,
    (select value ( if leave.refno.LSTU.refno.NAME != NONE {{ string::split(array::at(string::split(leave.refno.LSTU.name, '/'), 2), ':')[0] }} else if leave.refno.HSTU.refno.NAME != NONE {{
    string::split(array::at(string::split(leave.refno.HSTU.name, '/'), 2), ':')[0]
    }} else {{ '' }}  ) from $self)[0]  as code,
    'TUBI' as noun,
    world_trans.d.scale[2] as count from ->tubi_relate) from {}"#,
            refnos_str
        );
        let mut response = db.query(sql).await?;
        let mut result: Vec<Vec<MaterialGyDataBend>> = response.take(0)?;
        if !result.is_empty() {
            result.iter_mut().for_each(|x| tubi_data.append(x));
        }
        // 查询 elbo,tee,flan,gask,olet,redu,cap,couplig
        let refnos = query_filter_deep_children(
            refno,
            vec![
                "ELBO".to_string(),
                "TEE".to_string(),
                "FLAN".to_string(),
                "GASK".to_string(),
                "OLET".to_string(),
                "REDU".to_string(),
                "CAP".to_string(),
                "COUP".to_string(),
            ],
        )
        .await?;
        let refnos_str = serde_json::to_string(
            &refnos
                .into_iter()
                .map(|refno| refno.to_pe_key())
                .collect::<Vec<String>>(),
        )?;
        let sql = format!(
            r#"select
    id as id,
    string::split(string::split(if refno.SPRE.name == NONE {{ "//:" }} else {{ refno.SPRE.name }},'/')[2],':')[0] as code, // 编码
    refno.TYPE as noun // 部件
    from {}"#,
            refnos_str
        );
        let mut response = db.query(sql).await?;
        // let mut result: Vec<MaterialGyData> = response.take(0)?;
        let mut result: Vec<MaterialGyData> = response.take(0)?;
        data.append(&mut result);
        // tubi_data.append(&mut result);
    }
    Ok((data, tubi_data))
}

/// 查询工艺阀门清单数据
pub async fn get_gy_valv_list(
    db: Surreal<Any>,
    refnos: Vec<RefU64>,
) -> anyhow::Result<Vec<MaterialGyValvList>> {
    let mut data = Vec::new();
    for refno in refnos {
        let Some(pe) = get_pe(refno).await? else {
            continue;
        };
        // 如果是site，则需要过滤 site的 name
        if pe.noun == "SITE".to_string() {
            if !pe.name.contains("PIPE") {
                continue;
            };
        }
        // 查询阀门的数据
        let refnos =
            query_filter_deep_children(refno, vec!["VALV".to_string(), "INST".to_string()]).await?;
        let refnos_str = serde_json::to_string(
            &refnos
                .into_iter()
                .map(|refno| refno.to_pe_key())
                .collect::<Vec<String>>(),
        )?;
        let sql = format!(
            r#"select
        id,
        fn::default_name(id) as valv_name, // 阀门位号
        fn::room_code($this.id)[0] as room_code, // 房间号
        string::split(string::slice(array::at(->pe_owner.out.name,0),1),'-')[0] as valv_belong, // 阀门归属
        if refno.SPRE.refno.CATR.refno.PARA[1] == NONE {{ 0 }} else {{ refno.SPRE.refno.CATR.refno.PARA[1] }} * 2 as valv_length, // 阀门长度
        if refno.SPRE.refno.CATR.refno.NAME != NONE && string::slice(refno.SPRE.refno.CATR.refno.NAME,4,1) != "R" {{ refno.SPRE.refno.CATR.refno.PARA[10] }} else if refno.SPRE.refno.CATR.refno.NAME != NONE && string::slice(refno.SPRE.refno.CATR.refno.NAME,4,1) == "R" {{ refno.SPRE.refno.CATR.refno.PARA[14] }} else {{ 0 }} as valv_weight, // 阀门重量
        if refno.SPRE.refno.CATR.refno.NAME != NONE && string::slice(refno.SPRE.refno.CATR.refno.NAME,4,1) != "R" {{ refno.SPRE.refno.CATR.refno.PARA[7] }} else if refno.SPRE.refno.CATR.refno.NAME != NONE && string::slice(refno.SPRE.refno.CATR.refno.NAME,4,1) == "R" {{ refno.SPRE.refno.CATR.refno.PARA[11] }} else {{ 0 }} as valv_x, // 阀门重心X
        if refno.SPRE.refno.CATR.refno.NAME != NONE && string::slice(refno.SPRE.refno.CATR.refno.NAME,4,1) != "R" {{ refno.SPRE.refno.CATR.refno.PARA[8] }} else if refno.SPRE.refno.CATR.refno.NAME != NONE && string::slice(refno.SPRE.refno.CATR.refno.NAME,4,1) == "R" {{ refno.SPRE.refno.CATR.refno.PARA[12] }} else {{ 0 }} as valv_y, // 阀门重心Y
        if refno.SPRE.refno.CATR.refno.NAME != NONE && string::slice(refno.SPRE.refno.CATR.refno.NAME,4,1) != "R" {{ refno.SPRE.refno.CATR.refno.PARA[9] }} else if refno.SPRE.refno.CATR.refno.NAME != NONE && string::slice(refno.SPRE.refno.CATR.refno.NAME,4,1) == "R" {{ refno.SPRE.refno.CATR.refno.PARA[13] }} else {{ 0 }} as valv_z, // 阀门重心Z
        fn::valv_b_supp(id) as valv_supp // 阀门支架
        from {}"#,
            refnos_str
        );
        let mut response = db.query(sql).await?;
        let mut result: Vec<MaterialGyValvList> = response.take(0)?;
        data.append(&mut result);
    }
    Ok(data)
}

/// 查询工艺设备清单数据
pub async fn get_gy_equi_list(
    db: Surreal<Any>,
    refnos: Vec<RefU64>,
) -> anyhow::Result<Vec<MaterialGyEquiList>> {
    let mut data = Vec::new();
    for refno in refnos {
        let Some(pe) = get_pe(refno).await? else {
            continue;
        };
        // 如果是site，则需要过滤 site的 name
        if pe.noun == "SITE".to_string() {
            if !pe.name.contains("PIPE") {
                continue;
            };
        }
        // 查询设备的数据
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
        string::slice(refno.NAME,1) as name, // 设备位号
        fn::room_code($this.id)[0] as room_code, // 房间号
        fn::default_names(array::flatten([<-pe_owner[where in.noun='NOZZ']<-pe,  <-pe_owner.in<-pe_owner[where in.noun='NOZZ'].in])) as nozz_name, // 管口号
        array::clump(array::flatten([<-pe_owner[where in.noun='NOZZ']<-pe.refno.POS,  <-pe_owner.in<-pe_owner[where in.noun='NOZZ'].in.refno.POS]),3) as nozz_pos, // 管口坐标

        (select value if (name == NONE) {{ '' }} else {{ string::slice(name, 1) }} from array::flatten([<-pe_owner[where in.noun='NOZZ']<-pe,  <-pe_owner.in<-pe_owner[where in.noun='NOZZ'].in])) as nozz_cref // 相连管道编号
        from {}"#,
            refnos_str
        );
        let mut response = db.query(sql).await?;
        let mut result: Vec<MaterialGyEquiList> = response.take(0)?;
        data.append(&mut result);
    }
    Ok(data)
}

/// 查询电气 托盘及接地 托盘
pub async fn get_dq_bran_list(
    db: Surreal<Any>,
    refnos: Vec<RefU64>,
) -> anyhow::Result<(Vec<MaterialDqMaterialList>, Vec<MaterialDqMaterialListStru>)> {
    let mut data = Vec::new();
    let mut stru_data = Vec::new();
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
        // 查询电气托盘的数据
        let refnos = query_filter_deep_children(refno, vec!["BRAN".to_string()]).await?;
        let refnos_str = serde_json::to_string(
            &refnos
                .into_iter()
                .map(|refno| refno.to_pe_key())
                .collect::<Vec<String>>(),
        )?;
        let sql = format!(
            r#"select
        id,
        string::slice(fn::find_ancestor_type($this.id,"SITE")[0].refno.NAME,1,1) as num, // 机组号
        string::slice(string::split(fn::find_ancestor_type($this.id,"SITE")[0].refno.NAME,'-')[0],2) as project_num, //子项号
        if string::contains(fn::find_ancestor_type($this.id,"SITE")[0].refno.NAME,'MCT') || string::contains(fn::default_name(fn::find_ancestor_type($this.id,"ZONE")[0]),'MSUP')  {{ '主托盘' }} else {{ '次托盘' }} as major,//专业
        fn::find_ancestor_type($this.id,"SITE")[0].refno.DESC as project_name, //子项名称
        fn::room_code($this.id)[0] as room_code,
        fn::default_name($this.id) as name,// 托盘段号
        refno.HPOS[2] as pos, // 托盘标高
        //<-pe_owner[where in.noun!='ATTA'] order by order_num,

        fn::dq_bran_type($this.id) as bran_type, // 托盘类型
        '碳钢Q235' as material, // 材质
        (select value (select value v from only udas.* where u.NAME =='/BranWidth' limit 1) from type::thing("ATT_UDA", meta::id(id)))[0][0] as width, // 托盘宽度
        (select value (select value v from only udas.* where u.NAME =='/BranHigh' limit 1) from type::thing("ATT_UDA", meta::id(id)))[0][0] as height, // 托盘高度
        if (select value (select value v from only udas.* where u.NAME =='/BranWidth' limit 1) from type::thing("ATT_UDA", meta::id(id)))[0][0] == NONE {{ 0 }} else {{ (select value (select value v from only udas.* where u.NAME=='/BranWidth' limit 1) from type::thing("ATT_UDA", meta::id(id)))[0][0] }} * if (select value (select value v from only udas.* where u.NAME=='/BranHigh' limit 1) from type::thing("ATT_UDA", meta::id(id)))[0][0] == NONE {{ 0 }} else {{ (select value (select value v from only udas.* where u.NAME=='/BranHigh' limit 1) from type::thing("ATT_UDA", meta::id(id)))[0][0] }} as size_num, // 规格型号
        'Y' as b_painting, // 是否刷漆
        string::split(fn::default_name($this.id),'-')[1] as painting_color,
        if (select value in from only (select * from <-pe_owner[where in.noun != 'ATTA'] order by order_num) limit 1).refno.DESP[2] == 1 {{ '是' }} else {{ '否' }} as b_cover, //有无盖板
        if refno.DESC == NONE {{ '无' }} else {{ '是' }} as b_partition, // 有无隔板
        refno.DESC as partition_num, // 隔板编号
        (select value in from only (select * from <-pe_owner[where in.noun != 'ATTA'] order by order_num) limit 1).refno.SPRE.refno.NAME as spre,
        (select value in from only (select * from <-pe_owner[where in.noun != 'ATTA'] order by order_num) limit 1).refno.SPRE.refno.CATR.refno.NAME as catr,
        fn::dq_horizontal_or_vertical($this.id) as horizontal_or_vertical
        from {}"#,
            refnos_str
        );
        let mut response = db.query(sql).await?;
        let mut result: Vec<MaterialDqMaterialList> = response.take(0)?;
        data.append(&mut result);
        // 查询电气支吊架的数据
        let zones = get_children_pes(pe.refno).await?;
        for zone in zones {
            if zone.name.contains("MTGD") {
                continue;
            };
            let refnos = query_filter_deep_children(refno, vec!["STRU".to_string()]).await?;
            let refnos_str = serde_json::to_string(
                &refnos
                    .into_iter()
                    .map(|refno| refno.to_pe_key())
                    .collect::<Vec<String>>(),
            )?;
            let sql = format!(
                r#"select id,
        string::slice(fn::find_ancestor_type($this.id,"SITE")[0].refno.NAME,1,1) as num, // 机组号
        string::slice(string::split(fn::find_ancestor_type($this.id,"SITE")[0].refno.NAME,'-')[0],2) as project_num, //子项号
        fn::find_ancestor_type($this.id,"SITE")[0].refno.DESC as project_name, //子项名称
        '支吊架' as major,//专业
        fn::room_code($this.id)[0] as room_code, // 房间号
        fn::default_name($this.id) as supp_name, // 托盘支吊架名称
        '碳钢Q355' as material,  //材质
        if (<-pe_owner.in<-pe_owner[where in.noun='SCTN'|| in.noun = 'GENSEC'].in.refno.SPRE.name)[0] == NONE {{ '' }}
        else {{ array::last(string::split((<-pe_owner.in<-pe_owner[where in.noun='SCTN'|| in.noun = 'GENSEC'].in.refno.SPRE.name)[0],'-')) }} as size_num, // 规格型号
        (<-pe_owner.in<-pe_owner[where in.noun='SCTN'|| in.noun = 'GENSEC'].in.refno.SPRE.name)[0] as spre,
        (<-pe_owner.in<-pe_owner[where in.noun='SCTN'|| in.noun = 'GENSEC'].in.refno.SPRE.refno.CATR.refno.NAME)[0] as catr,
        '2' as count // 数量
        from {}"#,
                refnos_str
            );
            let mut response = db.query(sql).await?;
            let mut result: Vec<MaterialDqMaterialListStru> = response.take(0)?;
            stru_data.append(&mut result);
        }
        // 电缆及接地
        let zones = get_children_pes(pe.refno).await?;
        for zone in zones {
            if !zone.name.contains("MTGD") {
                continue;
            };
            let refnos = query_filter_deep_children(refno, vec!["GENSEC".to_string()]).await?;
            let refnos_str = serde_json::to_string(
                &refnos
                    .into_iter()
                    .map(|refno| refno.to_pe_key())
                    .collect::<Vec<String>>(),
            )?;
            let sql = format!(
                r#"select id,
            string::slice(fn::find_ancestor_type($this.id,"SITE")[0].NAME,1,1) as num, // 机组号
            string::slice(string::split(fn::find_ancestor_type($this.id,"SITE")[0].NAME,'-')[0],2) as project_num, //子项号
            fn::find_ancestor_type($this.id,"SITE")[0].NAME.DESC as project_name, //子项名称
            '主托盘接地' as major,//专业
            fn::room_code($this.id)[0] as room_code, // 房间号
            fn::default_name($this.id) as name, // 托盘段号
            math::fixed(refno.POS[2],3) as pos,
            '裸铜缆' as material,  //材质
            refno.SPRE.refno.NAME as spre,
            refno.SPRE.refno.CATR.refno.NAME as catr,
            math::fixed(fn::vec3_distance(array::clump(<-pe_owner.in<-pe_owner[where in.noun='POINSP'].in.refno.POS,3)[0],array::clump(<-pe_owner.in<-pe_owner[where in.noun='POINSP'].in.refno.POS,3)[1]),2) as count
            from {}"#,
                refnos_str
            );
            let mut response = db.query(sql).await?;
            let mut result: Vec<MaterialDqMaterialList> = response.take(0)?;
            data.append(&mut result);
        }
    }
    Ok((data, stru_data))
}

/// 查询仪控 大宗材料
pub async fn get_yk_dzcl_list(
    db: Surreal<Any>,
    refnos: Vec<RefU64>,
) -> anyhow::Result<Vec<MaterialGyData>> {
    let mut data = Vec::new();
    for refno in refnos {
        let Some(pe) = get_pe(refno).await? else {
            continue;
        };
        // 如果是site，则需要过滤 site的 name
        if pe.noun == "SITE".to_string() {
            if !pe.name.contains("INST") {
                continue;
            };
        }
        // 查询bend的数据
        let refnos = query_filter_deep_children(
            refno,
            vec![
                "VALV".to_string(),
                "TEE".to_string(),
                "COUP".to_string(),
                "INST".to_string(),
                "BEND".to_string(),
            ],
        )
        .await?;
        let refnos_str = serde_json::to_string(
            &refnos
                .into_iter()
                .map(|refno| refno.to_pe_key())
                .collect::<Vec<String>>(),
        )?;
        let sql = format!(
            r#"select
        id,
        if refno.SPRE.name != NONE {{ string::split(string::split(refno.SPRE.name,'/')[2],':')[0] }} else {{ ' ' }} as code ,// 编码
        refno.TYPE as noun
        from {}"#,
            refnos_str
        );
        let mut response = db.query(sql).await?;
        let mut result: Vec<MaterialGyData> = response.take(0)?;
        data.append(&mut result);
    }
    Ok(data)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MaterialYkInstData {
    pub id: RefU64,
    pub name: String,
    pub pipe_name: Option<String>,
    pub room_code: Option<String>,
}

impl MaterialYkInstData {
    pub fn into_hashmap(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.entry("参考号".to_string())
            .or_insert(self.id.to_pdms_str());
        map.entry("传感器标识".to_string()).or_insert(self.name);
        map.entry("对应根阀编号".to_string())
            .or_insert(self.pipe_name.unwrap_or("".to_string()));
        map.entry("房间号".to_string())
            .or_insert(self.room_code.unwrap_or("".to_string()));
        map
    }
}

/// 仪控 仪表管道
pub async fn get_yk_inst_pipe(
    db: Surreal<Any>,
    refnos: Vec<RefU64>,
) -> anyhow::Result<Vec<MaterialYkInstData>> {
    let mut data = Vec::new();
    for refno in refnos {
        let Some(pe) = get_pe(refno).await? else {
            continue;
        };
        // 如果是site，则需要过滤 site的 name
        if pe.noun == "SITE".to_string() {
            if !pe.name.contains("INST") {
                continue;
            };
        }
        // 查询 inst 的数据
        let refnos = query_filter_deep_children(refno, vec!["INST".to_string()]).await?;
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
        fn::find_gy_bran($this.id)[0][0] as pipe_name,
        fn::room_code($this.id)[0] as room_code
        from {}"#,
            refnos_str
        );
        let mut response = db.query(sql).await?;
        let mut result: Vec<MaterialYkInstData> = response.take(0)?;
        data.append(&mut result);
    }
    Ok(data)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MaterialYkEquiListData {
    pub id: RefU64,
    pub equi_name: String,
    pub room_code: Option<String>,
    pub pos: Option<f32>,
    pub floor_height: Option<f32>,
}

impl MaterialYkEquiListData {
    pub fn into_hashmap(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.entry("参考号".to_string())
            .or_insert(self.id.to_pdms_str());
        map.entry("仪控设备位号".to_string())
            .or_insert(self.equi_name);
        map.entry("所在房间号".to_string())
            .or_insert(self.room_code.unwrap_or("".to_string()));
        map.entry("设备绝对标高".to_string())
            .or_insert(self.pos.unwrap_or(0.0).to_string());
        map.entry("设备相对楼板标高".to_string())
            .or_insert(self.floor_height.unwrap_or(0.0).to_string());
        map
    }
}

/// 仪控 设备清单
pub async fn get_yk_equi_list_material(
    db: Surreal<Any>,
    refnos: Vec<RefU64>,
) -> anyhow::Result<Vec<MaterialYkEquiListData>> {
    let mut data = Vec::new();
    for refno in refnos {
        let Some(pe) = get_pe(refno).await? else {
            continue;
        };
        // 如果是site，则需要过滤 site的 name
        if pe.noun == "SITE".to_string() {
            if !pe.name.contains("INST") {
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
        fn::default_name($this.id) as equi_name,
        fn::room_code($this.id)[0] as room_code,
        fn::get_world_pos($this.id)[0][2] as pos, // 坐标 z
        (->nearest_relate.dist)[0] as floor_height
        from {}"#,
            refnos_str
        );
        let mut response = db.query(sql).await?;
        let mut result: Vec<MaterialYkEquiListData> = response.take(0)?;
        data.append(&mut result);
    }
    Ok(data)
}

/// 给排水 大宗材料
#[derive(Debug, Serialize, Deserialize)]
pub struct MaterialGpsDzclData {
    pub id: RefU64,
    pub code: String,
    pub noun: String,
    pub radius: Option<String>,
    pub length: Option<f32>,
    pub thick: Option<f32>,
    pub count: Option<f32>,
}

impl MaterialGpsDzclData {
    pub fn into_hashmap(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.entry("参考号".to_string())
            .or_insert(self.id.to_pdms_str());
        map.entry("物项编码".to_string()).or_insert(self.code);
        map.entry("品名".to_string()).or_insert(self.noun);
        map.entry("外径/Φ".to_string())
            .or_insert(self.radius.unwrap_or("0.0".to_string()));
        map.entry("长度".to_string())
            .or_insert(self.count.unwrap_or(0.0).to_string());
        map.entry("厚度".to_string())
            .or_insert(self.thick.unwrap_or(0.0).to_string());
        map.entry("数量".to_string())
            .or_insert(self.count.unwrap_or(0.0).to_string());

        map
    }
}

/// 给排水 大宗材料
pub async fn get_gps_dzcl_material(
    db: Surreal<Any>,
    refnos: Vec<RefU64>,
) -> anyhow::Result<Vec<MaterialGpsDzclData>> {
    let mut data = Vec::new();
    for refno in refnos {
        let Some(pe) = get_pe(refno).await? else {
            continue;
        };
        // 如果是site，则需要过滤 site的 name
        if pe.noun == "SITE".to_string() {
            if !pe.name.contains("PIPE") {
                continue;
            };
        }
        // 查询 BEND 的数据
        let refnos = query_filter_deep_children(refno, vec!["BEND".to_string()]).await?;
        let refnos_str = serde_json::to_string(
            &refnos
                .into_iter()
                .map(|refno| refno.to_pe_key())
                .collect::<Vec<String>>(),
        )?;
        let sql = format!(
            r#"select
        id,
        string::split(string::split(if refno.SPRE.name == NONE {{ "//:" }} else {{ refno.SPRE.name }},'/')[2],':')[0] as code, // 编码
        refno.TYPE as noun ,// 部件
        string::replace(<string>math::fixed(if refno.SPRE.refno.CATR.refno.PARA[3] == NONE {{ 0 }} else {{ refno.SPRE.refno.CATR.refno.PARA[3] }},3),'f','') as radius, // 外径
        math::fixed(if refno.SPRE.refno.CATR.refno.PARA == NONE && refno.ANGL == NONE {{ 0 }}
        else {{ (refno.ANGL / 360) * 2 * 3.1415 * refno.SPRE.refno.CATR.refno.PARA[1] }},2) as count // 数量
        from {}"#,
            refnos_str
        );
        let mut response = db.query(sql).await?;
        let mut result: Vec<MaterialGpsDzclData> = response.take(0)?;
        data.append(&mut result);
        // 查询tubi的数据
        let refnos = query_filter_deep_children(refno, vec!["BRAN".to_string()]).await?;
        let refnos_str = serde_json::to_string(
            &refnos
                .into_iter()
                .map(|refno| refno.to_pe_key())
                .collect::<Vec<String>>(),
        )?;
        let sql = format!(
            r#"select value (select leave as id,
        (select value ( if leave.refno.LSTU.refno.NAME != NONE {{ string::split(array::at(string::split(leave.refno.LSTU.name, '/'), 2), ':')[0] }} else if leave.refno.HSTU.refno.NAME != NONE {{
          string::split(array::at(string::split(leave.refno.HSTU.name, '/'), 2), ':')[0]
        }} else {{ '' }}  ) from $self)[0]  as code,
        'TUBI' as noun,
        string::replace(<string>math::fixed(if leave.refno.LSTU.refno.NAME != NONE {{ leave.refno.LSTU.refno.CATR.refno.PARA[1] }} else if leave.refno.HSTU.refno.NAME != NONE {{ leave.refno.HSTU.refno.CATR.refno.PARA[1] }} else {{ 0 }},3 ),'f','') as radius, // 外径
        world_trans.d.scale[2] as count from ->tubi_relate) from {};"#,
            refnos_str
        );
        let mut response = db.query(sql).await?;
        let mut result: Vec<Vec<MaterialGpsDzclData>> = response.take(0)?;
        for mut d in result {
            data.append(&mut d);
        }
        // 查询elbo的数据
        let refnos = query_filter_deep_children(refno, vec!["ELBO".to_string()]).await?;
        let refnos_str = serde_json::to_string(
            &refnos
                .into_iter()
                .map(|refno| refno.to_pe_key())
                .collect::<Vec<String>>(),
        )?;
        let sql = format!(
            r#"select id,
        string::split(string::split(if refno.SPRE.name == NONE {{ "//:" }} else {{ refno.SPRE.name }},'/')[2],':')[0] as code, // 编码
        refno.TYPE as noun ,// 部件
        string::replace(<string>math::fixed(if refno.SPRE.refno.CATR.refno.PARA[3] == NONE {{ 0 }} else {{ refno.SPRE.refno.CATR.refno.PARA[3] }},3),'f','') as radius // 外径
        from {};"#,
            refnos_str
        );
        let mut response = db.query(sql).await?;
        let mut result: Vec<MaterialGpsDzclData> = response.take(0)?;
        data.append(&mut result);
        // 查询flan的数据
        let refnos = query_filter_deep_children(refno, vec!["FLAN".to_string()]).await?;
        let refnos_str = serde_json::to_string(
            &refnos
                .into_iter()
                .map(|refno| refno.to_pe_key())
                .collect::<Vec<String>>(),
        )?;
        let sql = format!(
            r#"select id,
        string::split(string::split(if refno.SPRE.name == NONE {{ "//:" }} else {{ refno.SPRE.name }},'/')[2],':')[0] as code, // 编码
        refno.TYPE as noun ,// 部件
        string::replace(<string>math::fixed(if refno.SPRE.refno.CATR.refno.PARA[6] == NONE {{ 0 }} else {{ refno.SPRE.refno.CATR.refno.PARA[6] }},3),'f','') as radius, // 外径
        math::fixed(if refno.SPRE.refno.CATR.refno.PARA[4] == NONE {{ 0 }} else {{ refno.SPRE.refno.CATR.refno.PARA[4] }},3) as thick // 厚度
        from {};"#,
            refnos_str
        );
        let mut response = db.query(sql).await?;
        let mut result: Vec<MaterialGpsDzclData> = response.take(0)?;
        data.append(&mut result);
        // 查询redu的数据
        let refnos = query_filter_deep_children(refno, vec!["REDU".to_string()]).await?;
        let refnos_str = serde_json::to_string(
            &refnos
                .into_iter()
                .map(|refno| refno.to_pe_key())
                .collect::<Vec<String>>(),
        )?;
        let sql = format!(
            r#"select id,
        string::split(string::split(if refno.SPRE.name == NONE {{ "//:" }} else {{ refno.SPRE.name }},'/')[2],':')[0] as code, // 编码
        refno.TYPE as noun ,// 部件
        string::replace(<string>array::join([math::fixed(if refno.SPRE.refno.CATR.refno.PARA[5] == NONE {{ 0 }} else {{ refno.SPRE.refno.CATR.refno.PARA[5] }},3),math::fixed(if refno.SPRE.refno.CATR.refno.PARA[6] == NONE {{ 0 }} else {{ refno.SPRE.refno.CATR.refno.PARA[6] }},3)],';'),'f','') as radius, // 外径
        math::fixed(if refno.SPRE.refno.CATR.refno.PARA[3] == NONE {{ 0 }} else {{ refno.SPRE.refno.CATR.refno.PARA[3] }},3) as length // 长度
        from {};"#,
            refnos_str
        );
        let mut response = db.query(sql).await?;
        let mut result: Vec<MaterialGpsDzclData> = response.take(0)?;
        data.append(&mut result);
        // 查询tee的数据
        let refnos = query_filter_deep_children(refno, vec!["TEE".to_string()]).await?;
        let refnos_str = serde_json::to_string(
            &refnos
                .into_iter()
                .map(|refno| refno.to_pe_key())
                .collect::<Vec<String>>(),
        )?;
        let sql = format!(
            r#"select id,
        string::split(string::split(if refno.SPRE.name == NONE {{ "//:" }} else {{ refno.SPRE.name }},'/')[2],':')[0] as code, // 编码
        refno.TYPE as noun, // 部件
        string::replace(<string>array::join([<string>math::fixed(if refno.SPRE.refno.CATR.refno.PARA[6] != NONE {{ refno.SPRE.refno.CATR.refno.PARA[6] }} else {{ 0 }},3),<string>math::fixed(if refno.SPRE.refno.CATR.refno.PARA[7] != NONE {{ refno.SPRE.refno.CATR.refno.PARA[7] }} else {{ 0 }},3)],';'),'f','') as radius // 外径
        from {};"#,
            refnos_str
        );
        let mut response = db.query(sql).await?;
        let mut result: Vec<MaterialGpsDzclData> = response.take(0)?;
        data.append(&mut result);
    }
    Ok(data)
}

/// 设备专业 通信系统
#[derive(Debug, Serialize, Deserialize)]
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
        let zones = get_children_pes(pe.refno).await?;
        for zone in zones {
            if !zone.name.contains("FAD") {
                continue;
            };
            // 查询 EQUI 的数据
            let refnos = query_filter_deep_children(refno, vec!["ELCONN".to_string()]).await?;
            let refnos_str = serde_json::to_string(
                &refnos
                    .into_iter()
                    .map(|refno| refno.to_pe_key())
                    .collect::<Vec<String>>(),
            )?;
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
            from {}"#,
                refnos_str
            );
            let mut response = db.query(sql).await?;
            let mut result: Vec<MaterialTxTxsbData> = response.take(0)?;
            data.append(&mut result);
        }
    }
    Ok(data)
}

/// 设备专业 大宗材料
#[derive(Debug, Serialize, Deserialize)]
pub struct MaterialSbListData {
    pub id: RefU64,
    pub name: String,
    pub pos: Option<f32>,
    pub length: Option<f32>,
    pub room_code: Option<String>,
    pub boxs: Vec<Vec<String>>,
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
            array::max(POS[2]) as pos
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

/// 暖通 阀门清单
#[derive(Debug, Serialize, Deserialize)]
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
                vec![
                    "BEND".to_string(),
                    "BRCO".to_string(),
                    "CAP".to_string(),
                    "FLEX".to_string(),
                    "OFST".to_string(),
                    "STIF".to_string(),
                    "STRT".to_string(),
                    "TAPE".to_string(),
                    "THRE".to_string(),
                    "TRNS".to_string(),
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
    dbg!(&refnos);
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

    fn::hvac_nut_qty_BEND((id) as nut_qty,  //螺母数量
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

/// 提前运行定义好的方法
pub async fn define_surreal_functions(db: Surreal<Any>) -> anyhow::Result<()> {
    let response = db
        .query(include_str!("material_list/default_name.surql"))
        .await?;
    let response = db
        .query(include_str!("material_list/dq/fn_dq_bran_type.surql"))
        .await?;
    let response = db
        .query(include_str!("material_list/dq/fn_vec3_distance.surql"))
        .await?;
    let response = db
        .query(include_str!("material_list/yk/fn_find_gy_bran.surql"))
        .await?;
    let response = db
        .query(include_str!("material_list/gy/fn_b_valv_supp.surql"))
        .await?;
    let response = db
        .query(include_str!(
            "material_list/dq/fn_dq_horizontal_or_vertical.surql"
        ))
        .await?;
    let response = db
        .query(include_str!("material_list/fn_get_ancestor.surql"))
        .await?;
    let response = db
        .query(include_str!(
            "material_list/sb/fn_find_group_sube_children.surql"
        ))
        .await?;
    let response = db
        .query(include_str!("material_list/nt/fn_get_valv_material.surql"))
        .await?;
    let response = db
        .query(include_str!("material_list/fn_get_world_pos.surql"))
        .await?;
    let response = db
        .query(include_str!("schemas/fn_query_room_code.surql"))
        .await?;
    db.query(include_str!("tools/bolt.surql")).await?;
    db.query(include_str!("tools/common.surql")).await?;
    db.query(include_str!("tools/fln.surql")).await?;
    db.query(include_str!("tools/formula.surql")).await?;
    db.query(include_str!("tools/hvac.surql")).await?;
    db.query(include_str!("tools/len.surql")).await?;
    db.query(include_str!("tools/stif.surql")).await?;
    db.query(include_str!("tools/washer.surql")).await?;
    Ok(())
}

fn filter_equi_children(datas: Vec<Vec<Vec<String>>>) -> Vec<Vec<String>> {
    let mut result = Vec::new();
    for data in datas {
        let filtered_data: Vec<Vec<String>> = data
            .into_iter()
            .filter(|inner_vec| inner_vec.iter().all(|s| s.starts_with("BOX:")))
            .filter(|inner_vec| {
                let count = inner_vec.iter().count();
                count == 3 || count == 4
            })
            .collect();
        if !filtered_data.is_empty() {
            result.push(filtered_data[0].clone())
        }
    }
    result
}

#[tokio::test]
async fn test_save_all_material_data() -> anyhow::Result<()> {
    let aios_mgr = AiosDBMgr::init_from_db_option().await?;
    save_all_material_data().await?;
    Ok(())
}
