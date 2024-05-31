#[cfg(feature = "sql")]
use super::query::create_table_sql;
#[cfg(feature = "sql")]
use super::query::{save_material_data_to_mysql, save_two_material_data_to_mysql};
use crate::aios_db_mgr::aios_mgr::{self, AiosDBMgr};
use crate::{get_pe, insert_into_table_with_chunks, query_filter_deep_children, RefU64};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use surrealdb::engine::any::Any;
use surrealdb::Surreal;
use tokio::task::{self, JoinHandle};

/// 工艺专业 大宗材料
pub async fn save_gy_material_dzcl(
    refno: RefU64,
    db: Surreal<Any>,
    aios_mgr: &AiosDBMgr,
    mut handles: &mut Vec<JoinHandle<()>>,
) {
    match get_gy_dzcl(db.clone(), vec![refno]).await {
        Ok((r, tubi_r)) => {
            let r_clone = r.clone();
            let tubi_r_clone = tubi_r.clone();
            let task = task::spawn(async move {
                match insert_into_table_with_chunks(&db, "material_gy_list", r_clone).await {
                    Ok(_) => {}
                    Err(e) => {
                        dbg!(&e.to_string());
                    }
                }
                match insert_into_table_with_chunks(&db, "material_gy_list_tubi", tubi_r_clone)
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
                    return;
                };
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

                    match create_table_sql(&pool, &table_name, &table_field).await {
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
                                match save_two_material_data_to_mysql(
                                    &table_field,
                                    &table_name,
                                    &data_field_1,
                                    data_1,
                                    &data_field_2,
                                    data_2,
                                    &pool,
                                )
                                .await
                                {
                                    Ok(_) => {}
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
pub async fn save_gy_material_equi(
    refno: RefU64,
    db: Surreal<Any>,
    aios_mgr: &AiosDBMgr,
    mut handles: &mut Vec<JoinHandle<()>>,
) {
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
                let Ok(pool) = aios_mgr.get_project_pool().await else {
                    return;
                };
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
                                match save_material_data_to_mysql(
                                    &filed,
                                    &table_name,
                                    &filed,
                                    data,
                                    pool,
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

/// 工艺专业 阀门清单
pub async fn save_gy_material_valv(
    refno: RefU64,
    db: Surreal<Any>,
    aios_mgr: &AiosDBMgr,
    mut handles: &mut Vec<JoinHandle<()>>,
) {
    match get_gy_valv_list(db.clone(), vec![refno]).await {
        Ok(r) => {
            let r_clone = r.clone();
            let task = task::spawn(async move {
                match insert_into_table_with_chunks(&db, "material_gy_valv", r_clone).await {
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
                                match save_material_data_to_mysql(
                                    &filed,
                                    &table_name,
                                    &filed,
                                    data,
                                    pool,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
