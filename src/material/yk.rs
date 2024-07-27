#[cfg(feature = "sql")]
use super::query::create_table_sql;
#[cfg(feature = "sql")]
use super::query::save_material_value;
use crate::aios_db_mgr::aios_mgr::{self, AiosDBMgr};
use crate::aios_db_mgr::PdmsDataInterface;
use crate::material::get_refnos_belong_major;
use crate::material::gy::MaterialGyData;
use crate::pe::SPdmsElement;
use crate::{
    get_pe, insert_into_table_with_chunks, query_filter_ancestors, query_filter_deep_children,
    RefU64,
};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use surrealdb::engine::any::Any;
use surrealdb::Surreal;
use tokio::task::{self, JoinHandle};

/// 仪控专业 大宗材料
pub async fn save_yk_material_dzcl(
    refno: RefU64,
    db: Surreal<Any>,
    aios_mgr: &AiosDBMgr,
    mut handles: &mut Vec<JoinHandle<()>>,
) {
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
                let Ok(pool) = aios_mgr.get_project_pool().await else {
                    dbg!("无法连接到数据库");
                    return;
                };
                let task = task::spawn(async move {
                    let filed = vec![
                        "参考号".to_string(),
                        "编码".to_string(),
                        "品名".to_string(),
                        "规格".to_string(),
                        "连接形式".to_string(),
                        "材料".to_string(),
                        "RCC-M".to_string(),
                        "质保等级".to_string(),
                        "抗震等级".to_string(),
                        "备注".to_string(),
                        "内部编码".to_string(),
                        "公称直径".to_string(),
                    ];
                    let table_name = "仪控专业_大宗材料".to_string();
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
            dbg!(&e.to_string());
        }
    }
}

/// 仪控专业 仪表管道
pub async fn save_yk_material_pipe(
    refno: RefU64,
    db: Surreal<Any>,
    aios_mgr: &AiosDBMgr,
    mut handles: &mut Vec<JoinHandle<()>>,
) {
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
                let Ok(pool) = aios_mgr.get_project_pool().await else {
                    dbg!("无法连接到数据库");
                    return;
                };
                let task = task::spawn(async move {
                    let table_name = "仪控专业_仪表管道".to_string();
                    let filed = vec![
                        "参考号".to_string(),
                        "传感器标识".to_string(),
                        "对应根阀编号".to_string(),
                        "房间号".to_string(),
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

/// 仪控专业 设备清单
pub async fn save_yk_material_equi(
    refno: RefU64,
    db: Surreal<Any>,
    aios_mgr: &AiosDBMgr,
    mut handles: &mut Vec<JoinHandle<()>>,
) {
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
                let Ok(pool) = aios_mgr.get_project_pool().await else {
                    dbg!("无法连接到数据库");
                    return;
                };
                let task = task::spawn(async move {
                    let table_name = "仪控专业_设备清单".to_string();
                    let filed = vec![
                        "参考号".to_string(),
                        "仪控设备位号".to_string(),
                        "所在房间号".to_string(),
                        "设备绝对标高".to_string(),
                        "设备相对楼板标高".to_string(),
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
            &[
                "VALV",
                "TEE",
                "COUP",
                "INST",
                "BEND",
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
        let refnos = query_filter_deep_children(refno, &["INST"]).await?;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
        let refnos = query_filter_deep_children(refno, &["EQUI"]).await?;
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

/// 查找仪控管段所属工艺管道的根阀
pub async fn query_yk_bran_belong_gy_valv_name(
    mut bran: RefU64,
    aios_mgr: &AiosDBMgr,
) -> anyhow::Result<Option<SPdmsElement>> {
    loop {
        // 获取href
        let Some(href) = aios_mgr.get_foreign_attr(bran, "HREF").await? else {
            break;
        };
        let Some(href_refno) = href.get_refno() else {
            break;
        };
        // 判断 href 是 bran还是 valv
        match href.get_type_str() {
            // 如果是bran就继续想上找href
            "BRAN" => {
                bran = href_refno;
            }
            // 如果是valv，判断该valv是否属于工艺专业，如果属于则返回valv，如果不属于则返回None
            "VALV" => {
                let major = get_refnos_belong_major(&vec![href_refno]).await?;
                let Some(major) = major.get(&href_refno) else {
                    break;
                };
                if major.major == "T".to_string() {
                    return get_pe(href_refno).await;
                } else {
                    return Ok(None);
                }
            }
            &_ => {
                break;
            }
        }
    }
    Ok(None)
}

/// 查找仪控管段所属工艺管道或设备
pub async fn query_yk_bran_belong_gy_pipe_name(
    mut bran: RefU64,
    aios_mgr: &AiosDBMgr,
) -> anyhow::Result<Option<SPdmsElement>> {
    loop {
        // 获取href
        let Some(href) = aios_mgr.get_foreign_attr(bran, "HREF").await? else {
            break;
        };
        let Some(href_refno) = href.get_refno() else {
            break;
        };
        // 判断 href 是 bran还是nozz
        match href.get_type_str() {
            // 如果是nozz，直接返回所属equi
            "NOZZ" => {
                let equi = query_filter_ancestors(href_refno, &["EQUI"]).await?;
                if equi.is_empty() {
                    break;
                };
                return get_pe(equi[0]).await;
            }
            // 如果是bran，判断该bran是否属于工艺专业，若是则返回对应的pipe，若不是，则继续向上找href
            "BRAN" => {
                let major = get_refnos_belong_major(&vec![href_refno]).await?;
                let Some(major) = major.get(&href_refno) else {
                    break;
                };
                if major.major == "T".to_string() {
                    return get_pe(href.get_owner()).await;
                } else {
                    bran = href_refno;
                }
            }
            &_ => {
                break;
            }
        }
    }
    Ok(None)
}

#[tokio::test]
async fn test_query_yk_bran_belong_gy_pipe_name() -> anyhow::Result<()> {
    let aios_mgr = AiosDBMgr::init_from_db_option().await?;
    let refno = RefU64::from_str("24381/177397").unwrap();
    let r = query_yk_bran_belong_gy_valv_name(refno, &aios_mgr).await?;
    dbg!(&r);
    let r = query_yk_bran_belong_gy_pipe_name(refno, &aios_mgr).await?;
    dbg!(&r);
    Ok(())
}
