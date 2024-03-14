use std::str::FromStr;
use crate::{connect_surdb, RefU64, SUL_DB, SurlValue};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
struct MaterialGyData {
    pub id: RefU64,
    pub code: String,
    pub noun: String,
    pub count: f32,
}

///查询工艺大宗材料数据
pub async fn get_gy_dzcl(refno: RefU64) -> anyhow::Result<Vec<MaterialGyData>> {
    let mut response = SUL_DB
        .query(include_str!(
            "material_list/gy/gy_dzcl_bend.surql"
        ))
        .bind(("refno", refno.to_pe_key()))
        .await?;
    dbg!(&refno.to_pe_key());
    let o: SurlValue = response.take(0)?;
    // let os: Vec<SurlValue> = o.try_into().unwrap();
    // let data: Vec<MaterialGyData> = os.into_iter().map(|x| x.into()).collect();
    dbg!(&o);
    Ok(vec![])
}

#[tokio::test]
async fn test_get_gy_dzcl() {
    // SUL_DB.use_ns("1516").use_db("AvevaMarineSample").await.unwrap();
    connect_surdb("ws://127.0.0.1:9001","1516","AvevaMarineSample").await.unwrap();
    let refno = RefU64::from_str("24383_84092").unwrap();
    let data = get_gy_dzcl(refno).await.unwrap();
    println!("{:?}", data);
}