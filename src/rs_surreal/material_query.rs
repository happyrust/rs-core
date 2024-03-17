use std::collections::HashMap;
use std::str::FromStr;
use config::{Config, File};
use parry3d::partitioning::QbvhDataGenerator;
use crate::{connect_surdb, get_children_pes, get_pe, query_filter_deep_children, RefU64, SUL_DB, SurlValue};
use serde::{Serialize, Deserialize};
use surrealdb::engine::remote::ws::{Client, Ws};
use surrealdb::key::thing::Thing;
use surrealdb::Surreal;
use crate::options::DbOption;
use crate::pe::SPdmsElement;
use crate::rs_surreal::table_const::GY_DZCL;
use surrealdb::engine::any::Any;
use crate::test::test_surreal::init_test_surreal;
use crate::pdms_types::{ser_refno_as_str, de_refno_from_key_str};

#[derive(Debug, Serialize, Deserialize)]
pub struct MaterialGyDataBend {
    #[serde(deserialize_with = "de_refno_from_key_str")]
    #[serde(serialize_with = "ser_refno_as_str")]
    pub id: RefU64,
    pub code: String,
    pub noun: String,
    pub count: f32,
}

impl MaterialGyDataBend {
    //// 将结构体转为HashMap
    pub fn into_hashmap(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.entry("参考号".to_string()).or_insert(self.id.to_pdms_str());
        map.entry("编码".to_string()).or_insert(self.code);
        map.entry("部件".to_string()).or_insert(self.noun);
        map.entry("数量".to_string()).or_insert(self.count.to_string());
        map
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MaterialGyData {
    #[serde(deserialize_with = "de_refno_from_key_str")]
    #[serde(serialize_with = "ser_refno_as_str")]
    pub id: RefU64,
    pub code: String,
    pub noun: String,
}

impl MaterialGyData {
    //// 将结构体转为HashMap
    pub fn into_hashmap(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.entry("参考号".to_string()).or_insert(self.id.to_pdms_str());
        map.entry("编码".to_string()).or_insert(self.code);
        map.entry("部件".to_string()).or_insert(self.noun);
        map
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MaterialGyValvList {
    #[serde(deserialize_with = "de_refno_from_key_str")]
    #[serde(serialize_with = "ser_refno_as_str")]
    pub id: RefU64,
    pub valv_name: String,
    pub room_code: String,
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
        map.entry("参考号".to_string()).or_insert(self.id.to_pdms_str());
        map.entry("阀门位号".to_string()).or_insert(self.valv_name.to_string());
        map.entry("所在房间号".to_string()).or_insert(self.room_code.to_string());
        map.entry("阀门归属".to_string()).or_insert(self.valv_belong.to_string());
        // 没有的给个默认值
        let valv_length = self.valv_length.map_or("0".to_string(), |x| x.to_string());
        map.entry("阀门长度".to_string()).or_insert(valv_length.to_string());
        let valv_weight = self.valv_weight.map_or("0".to_string(), |x| x.to_string());
        map.entry("阀门重量".to_string()).or_insert(valv_weight.to_string());
        let valv_x = self.valv_x.map_or("0".to_string(), |x| x.to_string());
        map.entry("阀门重心X".to_string()).or_insert(valv_x.to_string());
        let valv_y = self.valv_y.map_or("0".to_string(), |x| x.to_string());
        map.entry("阀门重心Y".to_string()).or_insert(valv_y.to_string());
        let valv_z = self.valv_z.map_or("0".to_string(), |x| x.to_string());
        map.entry("阀门重心Z".to_string()).or_insert(valv_z.to_string());

        map.entry("是否阀门支架".to_string()).or_insert(self.valv_supp.to_string());
        map
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MaterialGyEquiList {
    #[serde(deserialize_with = "de_refno_from_key_str")]
    #[serde(serialize_with = "ser_refno_as_str")]
    pub id: RefU64,
    pub name: String,
    pub room_code: String,
    pub nozz_name: Vec<String>,
    pub nozz_pos: Vec<Vec<f32>>,
    pub nozz_cref: Vec<String>,
}

impl MaterialGyEquiList {
    //// 将结构体转为HashMap
    pub fn into_hashmap(self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.entry("参考号".to_string()).or_insert(self.id.to_pdms_str());
        map.entry("设备位号".to_string()).or_insert(self.name.to_string());
        map.entry("所在房间号".to_string()).or_insert(self.room_code.to_string());
        map.entry("管口号".to_string()).or_insert(serde_json::to_string(&self.nozz_name).unwrap_or("[]".to_string()));
        map.entry("管口坐标".to_string()).or_insert(serde_json::to_string(&self.nozz_pos).unwrap_or("[]".to_string()));
        map.entry("相连管道编号".to_string()).or_insert(serde_json::to_string(&self.nozz_cref).unwrap_or("[]".to_string()));

        map
    }
}

///查询工艺大宗材料数据
pub async fn get_gy_dzcl(db: Surreal<Any>, refnos: Vec<RefU64>) -> anyhow::Result<(Vec<MaterialGyData>, Vec<MaterialGyDataBend>)> {
    let mut data = Vec::new();
    let mut tubi_data = Vec::new();
    for refno in refnos {
        let Some(pe) = get_pe(refno).await? else { continue; };
        // 如果是site，则需要过滤 site的 name
        if pe.noun == "SITE".to_string() {
            if !pe.name.contains("PIPE") { continue; };
        }
        // 查询bend的数据
        let refnos = query_filter_deep_children(refno, vec!["BEND".to_string()]).await?;
        let refnos_str = serde_json::to_string(&refnos.into_iter()
            .map(|refno| refno.to_pe_key()).collect::<Vec<String>>())?;
        let sql = format!(r#"select
    id as id,
    string::split(string::split(if refno.SPRE.name == NONE {{ "//:" }} else {{ refno.SPRE.name }},'/')[2],':')[0] as code, // 编码
    refno.TYPE as noun, // 部件
    math::fixed((refno.ANGL / 360) * 2 * 3.1415 * refno.SPRE.refno.CATR.refno.PARA[1],2) as count // 长度
    from {}"#, refnos_str);
        let mut response = db
            .query(sql)
            .await?;
        let mut result: Vec<MaterialGyDataBend> = response.take(0)?;
        tubi_data.append(&mut result);
        // 查询tubi数据
        let refnos = query_filter_deep_children(refno, vec!["BRAN".to_string()]).await?;
        let refnos_str = serde_json::to_string(&refnos.into_iter()
            .map(|refno| refno.to_pe_key()).collect::<Vec<String>>())?;
        let sql = format!(r#"
    select value (select leave as id,
    (select value ( if leave.refno.LSTU.refno.NAME != NONE {{ string::split(array::at(string::split(leave.refno.LSTU.name, '/'), 2), ':')[0] }} else if leave.refno.HSTU.refno.NAME != NONE {{
    string::split(array::at(string::split(leave.refno.HSTU.name, '/'), 2), ':')[0]
    }} else {{ '' }}  ) from $self)[0]  as code,
    'TUBI' as noun,
    world_trans.d.scale[2] as count from ->tubi_relate) from {}"#, refnos_str);
        let mut response = db
            .query(sql)
            .await?;
        let mut result: Vec<Vec<MaterialGyDataBend>> = response.take(0)?;
        if !result.is_empty() {
            result.iter_mut().for_each(|x| tubi_data.append( x));
        }
        // 查询 elbo,tee,flan,gask,olet,redu,cap,couplig
        let refnos = query_filter_deep_children(refno, vec!["ELBO".to_string(),
                                                            "TEE".to_string(), "FLAN".to_string(), "GASK".to_string(),
                                                            "OLET".to_string(), "REDU".to_string(), "CAP".to_string(),
                                                            "COUP".to_string()]).await?;
        let refnos_str = serde_json::to_string(&refnos.into_iter()
            .map(|refno| refno.to_pe_key()).collect::<Vec<String>>())?;
        let sql = format!(r#"select
    id as id,
    string::split(string::split(if refno.SPRE.name == NONE {{ "//:" }} else {{ refno.SPRE.name }},'/')[2],':')[0] as code, // 编码
    refno.TYPE as noun // 部件
    from {}"#, refnos_str);
        let mut response = db
            .query(sql)
            .await?;
        // let mut result: Vec<MaterialGyData> = response.take(0)?;
        let mut result: Vec<MaterialGyData> = response.take(0)?;
        data.append(&mut result);
        // tubi_data.append(&mut result);
    }
    Ok((data, tubi_data))
}

/// 查询工艺阀门清单数据
pub async fn get_gy_valv_list(db: Surreal<Any>, refnos: Vec<RefU64>) -> anyhow::Result<Vec<MaterialGyValvList>> {
    let mut data = Vec::new();
    for refno in refnos {
        let Some(pe) = get_pe(refno).await? else { continue; };
        // 如果是site，则需要过滤 site的 name
        if pe.noun == "SITE".to_string() {
            if !pe.name.contains("PIPE") { continue; };
        }
        // 查询阀门的数据
        let refnos = query_filter_deep_children(refno, vec!["VALV".to_string(), "INST".to_string()]).await?;
        let refnos_str = serde_json::to_string(&refnos.into_iter()
            .map(|refno| refno.to_pe_key()).collect::<Vec<String>>())?;
        let sql = format!(r#"select
        id,
        fn::default_name(id) as valv_name, // 阀门位号
        '' as room_code, // 房间号 todo
        string::split(string::slice(array::at(->pe_owner.out.name,0),1),'-')[0] as valv_belong, // 阀门归属
        if refno.SPRE.refno.CATR.refno.PARA[1] == NONE {{ 0 }} else {{ refno.SPRE.refno.CATR.refno.PARA[1] }} * 2 as valv_length, // 阀门长度
        if refno.SPRE.refno.CATR.refno.NAME != NONE && string::slice(refno.SPRE.refno.CATR.refno.NAME,4,1) != "R" {{ refno.SPRE.refno.CATR.refno.PARA[10] }} else if refno.SPRE.refno.CATR.refno.NAME != NONE && string::slice(refno.SPRE.refno.CATR.refno.NAME,4,1) == "R" {{ refno.SPRE.refno.CATR.refno.PARA[14] }} else {{ 0 }} as valv_weight, // 阀门重量
        if refno.SPRE.refno.CATR.refno.NAME != NONE && string::slice(refno.SPRE.refno.CATR.refno.NAME,4,1) != "R" {{ refno.SPRE.refno.CATR.refno.PARA[7] }} else if refno.SPRE.refno.CATR.refno.NAME != NONE && string::slice(refno.SPRE.refno.CATR.refno.NAME,4,1) == "R" {{ refno.SPRE.refno.CATR.refno.PARA[11] }} else {{ 0 }} as valv_x, // 阀门重心X
        if refno.SPRE.refno.CATR.refno.NAME != NONE && string::slice(refno.SPRE.refno.CATR.refno.NAME,4,1) != "R" {{ refno.SPRE.refno.CATR.refno.PARA[8] }} else if refno.SPRE.refno.CATR.refno.NAME != NONE && string::slice(refno.SPRE.refno.CATR.refno.NAME,4,1) == "R" {{ refno.SPRE.refno.CATR.refno.PARA[12] }} else {{ 0 }} as valv_y, // 阀门重心Y
        if refno.SPRE.refno.CATR.refno.NAME != NONE && string::slice(refno.SPRE.refno.CATR.refno.NAME,4,1) != "R" {{ refno.SPRE.refno.CATR.refno.PARA[9] }} else if refno.SPRE.refno.CATR.refno.NAME != NONE && string::slice(refno.SPRE.refno.CATR.refno.NAME,4,1) == "R" {{ refno.SPRE.refno.CATR.refno.PARA[13] }} else {{ 0 }} as valv_z, // 阀门重心Z
        fn::valv_b_supp(id) as valv_supp // 阀门支架
        from {}"#, refnos_str);
        let mut response = db
            .query(sql)
            .await?;
        let mut result: Vec<MaterialGyValvList> = response.take(0)?;
        data.append(&mut result);
    }
    Ok(data)
}

/// 查询工艺设备清单数据
pub async fn get_gy_equi_list(db: Surreal<Any>, refnos: Vec<RefU64>) -> anyhow::Result<Vec<MaterialGyEquiList>> {
    let mut data = Vec::new();
    for refno in refnos {
        let Some(pe) = get_pe(refno).await? else { continue; };
        // 如果是site，则需要过滤 site的 name
        if pe.noun == "SITE".to_string() {
            if !pe.name.contains("PIPE") { continue; };
        }
        // 查询阀门的数据
        let refnos = query_filter_deep_children(refno, vec!["EQUI".to_string()]).await?;
        let refnos_str = serde_json::to_string(&refnos.into_iter()
            .map(|refno| refno.to_pe_key()).collect::<Vec<String>>())?;
        let sql = format!(r#"select
        id,
        string::slice(refno.NAME,1) as name, // 设备位号
        '' as room_code, // 房间号 todo
        fn::default_names(array::flatten([<-pe_owner[where in.noun='NOZZ']<-pe,  <-pe_owner.in<-pe_owner[where in.noun='NOZZ'].in])) as nozz_name, // 管口号
        array::clump(array::flatten([<-pe_owner[where in.noun='NOZZ']<-pe.refno.POS,  <-pe_owner.in<-pe_owner[where in.noun='NOZZ'].in.refno.POS]),3) as nozz_pos, // 管口坐标

        (select value if (name == NONE) {{ '' }} else {{ string::slice(name, 1) }} from array::flatten([<-pe_owner[where in.noun='NOZZ']<-pe,  <-pe_owner.in<-pe_owner[where in.noun='NOZZ'].in])) as nozz_cref // 相连管道编号
        from {}"#, refnos_str);
        let mut response = db
            .query(sql)
            .await?;
        let mut result: Vec<MaterialGyEquiList> = response.take(0)?;
        data.append(&mut result);
    }
    Ok(data)
}

#[tokio::test]
async fn test_get_gy_dzcl() -> anyhow::Result<()> {
    let s = Config::builder()
        .add_source(File::with_name("DbOption"))
        .build()?;
    let db_option: DbOption = s.try_deserialize().unwrap();
    let url = format!("{}:{}", db_option.v_ip, db_option.v_port);
    let db = Surreal::new::<Ws>(url).await?;
    // db.signin(Root {
    //     username: "root",
    //     password: "root",
    // }).await?;
    // db.use_ns(db_option.project_code).use_db(db_option.project_name).await?;
    db
        .use_ns(&db_option.project_code)
        .use_db(&db_option.project_name)
        .await
        .unwrap();
    let refnos = vec![RefU64::from_str("24383/66456").unwrap()];
    get_gy_dzcl(db, refnos).await.unwrap();
    Ok(())
}