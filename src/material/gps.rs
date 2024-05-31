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

/// 给排水专业 大宗材料
pub async fn save_gps_material_dzcl(
    refno: RefU64,
    db: Surreal<Any>,
    aios_mgr: &AiosDBMgr,
    mut handles: &mut Vec<JoinHandle<()>>,
) {
    match get_gps_dzcl_material(db.clone(), vec![refno]).await {
        Ok((r, tubi_r)) => {
            let r_clone = r.clone();
            let tubi_r_clone = tubi_r.clone();
            let task = task::spawn(async move {
                match insert_into_table_with_chunks(&db, "material_gps_list", r_clone).await {
                    Ok(_) => {}
                    Err(e) => {
                        dbg!(&e.to_string());
                    }
                }
                match insert_into_table_with_chunks(&db, "material_gps_list_tubi", tubi_r_clone)
                    .await
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
                let Ok(pool) = aios_mgr.get_project_pool().await else {
                    dbg!("无法连接到数据库");
                    return;
                };
                let task = task::spawn(async move {
                    let table_name = "给排水专业_大宗材料".to_string();
                    let filed = vec![
                        "参考号".to_string(),
                        "物项编码".to_string(),
                        "品名".to_string(),
                        "SCH/LB（主）".to_string(),
                        "SCH/LB（支）".to_string(),
                        "制造形式".to_string(),
                        "连接形式".to_string(),
                        "材料牌号".to_string(),
                        "材料标准".to_string(),
                        "规格标准".to_string(),
                        "RCC_M".to_string(),
                        "质保等级".to_string(),
                        "公称直径（主）".to_string(),
                        "公称直径（支）".to_string(),
                        "外径/Φ".to_string(),
                        "长度".to_string(),
                        "厚度".to_string(),
                        "数量".to_string(),
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
                            if !tubi_r.is_empty() {
                                let data = tubi_r
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

/// 给排水 大宗材料
#[derive(Debug, Clone, Serialize, Deserialize)]
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
) -> anyhow::Result<(Vec<MaterialGpsDzclData>, Vec<MaterialGpsDzclData>)> {
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
            tubi_data.append(&mut d);
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
    Ok((data, tubi_data))
}
