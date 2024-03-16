use std::collections::HashMap;
use std::str::FromStr;
use config::{Config, File};
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
    pub fn into_hashmap(self) -> HashMap<String,String> {
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
    pub fn into_hashmap(self) -> HashMap<String,String> {
        let mut map = HashMap::new();
        map.entry("参考号".to_string()).or_insert(self.id.to_pdms_str());
        map.entry("编码".to_string()).or_insert(self.code);
        map.entry("部件".to_string()).or_insert(self.noun);
        map
    }
}

pub async fn get_pe_with_db(db: Surreal<Any>, refno: RefU64) -> anyhow::Result<Option<SPdmsElement>> {
    let mut response = db
        .query(include_str!("schemas/query_pe_by_refno.surql"))
        .bind(("refno", refno.to_string()))
        .await?;
    let pe: Option<SPdmsElement> = response.take(0)?;
    Ok(pe)
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

pub async fn get_gy_dzcl_test(refnos: Vec<RefU64>) -> anyhow::Result<(Vec<MaterialGyData>, Vec<MaterialGyDataBend>)> {
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
        let mut response = SUL_DB
            .query(sql)
            .await?;
        let mut result: Vec<MaterialGyDataBend> = response.take(0)?;
        tubi_data.append(&mut result);
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
        let mut response = SUL_DB
            .query(sql)
            .await?;
        let mut result: Vec<MaterialGyData> = response.take(0)?;
        data.append(&mut result);
    }
    Ok((data, tubi_data))
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