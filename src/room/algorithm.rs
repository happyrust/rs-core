use crate::pdms_types::RoomNodes;
use crate::{RefU64, SUL_DB};
use once_cell::sync::Lazy;
use regex::Regex;
use serde_derive::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::str::FromStr;
use tokio::sync::RwLock;

#[derive(Serialize, Deserialize, Default, Clone, Hash)]
pub struct RoomInfo {
    pub refno: RefU64,
    pub name: String,
}

/// 查询项目的所有房间
///
/// 返回值: k 厂房 v 房间编号
pub async fn query_all_room_name() -> anyhow::Result<HashMap<String, BTreeSet<RoomNodes>>> {
    let mut map = HashMap::new();
    let mut response = SUL_DB
        .query(r#"
            select value (array::concat(REFNO<-pe_owner.in<-pe_owner.in<-pe_owner[where in.refno.NAME != NONE && in.noun == 'FRMW'].in.refno.NAME)) from (
            select REFNO from SITE where NAME != NONE && string::contains(NAME,'ARCH'));
        "#)
        .await?;
    let results: Vec<Vec<String>> = response.take(0)?;
    for r in results.clone() {
        for room in r {
            let split = room.split("-").collect::<Vec<_>>();
            let Some(first) = split.first() else {
                continue;
            };
            let Some(last) = split.last() else {
                continue;
            };
            if !match_room_name(last) {
                continue;
            };
            map.entry(first[1..].to_string())
                .or_insert_with(BTreeSet::new)
                .insert(last.to_string());
        }
    }
    Ok(Default::default())
    // Ok(map)
}

/// 查询多个refno所属的房间号，bran和equi也适用
pub async fn query_room_name_from_refnos(
    owner: Vec<RefU64>,
) -> anyhow::Result<HashMap<RefU64, String>> {
    #[serde_as]
    #[derive(Debug, Serialize, Deserialize)]
    struct RoomNameQueryRequest {
        #[serde_as(as = "DisplayFromStr")]
        pub id: RefU64,
        pub room: Option<String>,
    }

    let owners = owner
        .into_iter()
        .map(|o| o.to_pe_key())
        .collect::<Vec<String>>();
    let sql = format!(
        "select id,fn::room_code(id)[0] as room from {}",
        serde_json::to_string(&owners).unwrap_or("[]".to_string())
    );
    let mut response = SUL_DB.query(sql).await?;
    let result: Vec<RoomNameQueryRequest> = response.take(0)?;
    let r = result
        .into_iter()
        .map(|x| (x.id, x.room.unwrap_or("".to_string())))
        .collect::<HashMap<RefU64, String>>();
    Ok(r)
}

/// 查找设备和阀门所属的楼板
pub async fn query_equi_or_valv_belong_floors(
    refnos: Vec<RefU64>,
) -> anyhow::Result<HashMap<RefU64, (String, f32)>> {
    #[serde_as]
    #[derive(Serialize, Deserialize, Debug)]
    struct BelongFloorResponse {
        #[serde_as(as = "DisplayFromStr")]
        pub id: RefU64,
        pub floor: Option<String>,
        pub height: Option<f32>,
    }

    let refnos = refnos
        .into_iter()
        .map(|refno| refno.to_pe_key())
        .collect::<Vec<_>>();
    let request = serde_json::to_string(&refnos)?;
    let sql = format!("select id,(->nearest_relate.out.REFNO)[0] as floor,(->nearest_relate.dist)[0] as height from {}", request);
    let mut response = SUL_DB.query(sql).await?;
    let result: Vec<BelongFloorResponse> = response.take(0)?;
    let r = result
        .into_iter()
        .map(|x| {
            (
                x.id,
                (
                    x.floor.map_or("".to_string(), |x| {
                        RefU64::from_str(&x).unwrap().to_pdms_str()
                    }),
                    x.height.unwrap_or(0.0),
                ),
            )
        })
        .collect::<HashMap<RefU64, (String, f32)>>();
    Ok(r)
}

/// 正则匹配是否满足房间命名规则
fn match_room_name(room_name: &str) -> bool {
    let regex = Regex::new(r"^[A-Z]\d{3}$").unwrap();
    regex.is_match(room_name)
}

#[tokio::test]
async fn test_query_all_room_name() {
    // init_test_surreal().await;
    // let r = query_all_room_name().await.unwrap();
    // dbg!(&r);
}
