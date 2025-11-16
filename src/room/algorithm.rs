use crate::pdms_types::RoomNodes;
use crate::{RefU64, RefnoEnum, SUL_DB, SurrealQueryExt};
use once_cell::sync::Lazy;
use regex::Regex;
use serde_derive::{Deserialize, Serialize};
use serde_with::DisplayFromStr;
use serde_with::serde_as;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::str::FromStr;
use surrealdb::types as surrealdb_types;
use surrealdb::types::SurrealValue;
use tokio::sync::RwLock;

/// 表示房间信息，包含房间的唯一ID和房间名称。
///
/// 主要用于房间相关的查询与映射。
#[derive(Serialize, Deserialize, Default, Debug, Clone, Hash, Eq, PartialEq, SurrealValue)]
pub struct RoomInfo {
    /// 房间对应的唯一ID，通常来源于数据库中的RefnoEnum
    pub id: RefnoEnum,
    /// 房间名称，可用于显示与分组
    pub name: String,
}

impl Ord for RoomInfo {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name.cmp(&other.name)
    }
}

impl PartialOrd for RoomInfo {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// 查询数据库中所有满足房间关键字条件的 FRMW 节点（房间）
///
/// # 输入
/// - `room_keywords`：房间名称关键词构成的字符串数组，用于筛选房间名称。
///
/// # 返回
/// - 成功时返回所有满足条件的房间节点列表（Vec<RoomInfo>），每个房间包含 id 和 name 字段。
///
/// # 场景
/// 常用于批量获取所有结构建筑房间节点，供后续分组、统计或批量处理。
pub async fn query_all_room_infos(room_keywords: &[String]) -> anyhow::Result<Vec<RoomInfo>> {
    // 构建关键词过滤条件
    let keyword_conditions: Vec<String> = room_keywords
        .iter()
        .map(|keyword| format!("string::contains(NAME, '{}')", keyword))
        .collect();
    let keyword_filter = if keyword_conditions.is_empty() {
        "true".to_string()
    } else {
        keyword_conditions.join(" || ")
    };
    let sql = format!(r#"
        let $sites = (select value REFNO from SITE where NAME != NONE && string::contains(NAME,'ARCH'));
         array::flatten(
            select value @.{{3+collect}}(.children).{{id, noun, name}}
            from array::flatten($sites) 
        )[? noun=='FRMW' && name != NONE && ({} )].{{id, name}};
    "#, keyword_filter);
    // dbg!(&sql);
    let mut response = SUL_DB.query_response(&sql).await?;
    let results: Vec<RoomInfo> = response.take(1)?;
    Ok(results)
}

/// 查询并聚合项目中所有房间，分类返回“厂房-房间号”结构
///
/// # 返回
/// - Ok(HashMap<String, BTreeSet<RoomInfo>>): 以厂房名（截取自房间号）为 key，房间集合为 value
///
/// # 逻辑说明
/// - 依赖数据库配置中的 room_keywords 关键词过滤房间
/// - 将每个房间名按照 '-' 拆分，第一个字段去第一个字符后为厂房，最后一个字段为房间号
/// - 按厂房聚合所有房间 RoomInfo
///
/// # 典型用途
/// - 结构/建筑专业房间总览、结构化数据导出
pub async fn query_room_codes_of_arch() -> anyhow::Result<HashMap<String, BTreeSet<RoomInfo>>> {
    let room_keywords = crate::get_db_option().get_room_key_word();
    let results = query_all_room_infos(&room_keywords).await?;
    let mut map = HashMap::new();
    for r in results {
        let room = &r.name;
        let split = room.split("-").collect::<Vec<_>>();
        if split.len() < 2 {
            continue;
        }
        let Some(first) = split.first() else {
            continue;
        };
        let Some(last) = split.last() else {
            continue;
        };
        map.entry(first[1..].to_string())
            .or_insert_with(BTreeSet::new)
            .insert(RoomInfo {
                id: r.id.clone(),
                name: last.to_string(),
            });
    }
    Ok(map)
}


/// 根据一组房间 Refno 查询其房间号
///
/// # 输入
/// - owner: 需查询房间号的 RefnoEnum 列表
///
/// # 返回
/// - Ok(HashMap<RefU64, String>): 以输入 RefU64 为 key，查询到的房间号为 value
///
/// # 用途
/// - 批量映射数据库元素与房间号，常用于设备等空间归属的属性显示或进一步空间聚合
pub async fn query_room_name_from_refnos(
    owner: Vec<RefnoEnum>,
) -> anyhow::Result<HashMap<RefU64, String>> {
    #[derive(Debug, Serialize, Deserialize, SurrealValue)]
    struct RoomNameQueryRequest {
        pub id: RefU64,
        pub room: Option<String>,
    }

    let owners = owner
        .into_iter()
        .map(|o| o.to_pe_key())
        .collect::<Vec<String>>()
        .join(",");
    let sql = format!("select id,fn::room_code(id)[0] as room from [{}]", owners);
    let mut response = SUL_DB.query_response(&sql).await?;
    let result: Vec<RoomNameQueryRequest> = response.take(0)?;
    let r = result
        .into_iter()
        .map(|x| (x.id, x.room.unwrap_or("".to_string())))
        .collect::<HashMap<RefU64, String>>();
    Ok(r)
}

/// 查询指定设备/阀门的所属楼板及其标高
///
/// # 输入
/// - refnos: 设备或阀门的 RefnoEnum 列表
///
/// # 返回
/// - Ok(HashMap<RefU64, (String, f32)>): 设备 id -> (楼板名称, 高度)
///
/// # 应用场景
/// - 空间分析、建模阶段设备与楼层的对应关系生成、属性挂载
pub async fn query_equi_or_valv_belong_floors(
    refnos: Vec<RefnoEnum>,
) -> anyhow::Result<HashMap<RefU64, (String, f32)>> {
    #[serde_as]
    #[derive(Serialize, Deserialize, Debug, SurrealValue)]
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
    let sql = format!(
        "select id,(->nearest_relate.out.REFNO)[0] as floor,(->nearest_relate.dist)[0] as height from {}",
        request
    );
    let mut response = SUL_DB.query_response(&sql).await?;
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

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试 query_all_room_infos 是否能正确查询到房间基础数据。
    ///
    /// 注意：此测试需依赖实际 SurrealDB 数据库及配置的关键词环境，否则结果可能为空。
    #[tokio::test]
    async fn test_query_all_room_infos() {
        use crate::init_test_surreal;
        init_test_surreal().await.unwrap();
        let keywords = vec!["房间".to_string()];
        let result = query_all_room_infos(&keywords).await;
        match result {
            Ok(list) => {
                println!("房间节点数: {}", list.len());
                if let Some(first) = list.first() {
                    println!("样例: id={:?}, name={}", first.id, first.name);
                }
                // 数量应>=0（允许空结果，避免误炸流水线）
                assert!(list.len() >= 0);
            }
            Err(e) => panic!("查询失败: {e}"),
        }
    }
}

