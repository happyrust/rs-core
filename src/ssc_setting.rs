use crate::aios_db_mgr::aios_mgr::AiosDBMgr;
use crate::aios_db_mgr::PdmsDataInterface;
use crate::options::DbOption;
use crate::pdms_types::{PdmsElement, PdmsNodeTrait};
use crate::pdms_types::UdaMajorType::S;
use crate::pe::SPdmsElement;
use crate::room::algorithm::query_all_room_name;
use crate::table_const::{PBS_OWNER, PBS_TABLE, PDMS_MAJOR};
use crate::tool::hash_tool::{hash_str, hash_two_str};
use crate::types::*;
use crate::{
    get_mdb_world_site_pes, insert_into_table, insert_relate_to_table,
    query_ele_filter_deep_children, query_filter_deep_children, DBType, SUL_DB,
};
use anyhow::anyhow;
use bevy_ecs::system::Resource;
use calamine::{open_workbook, RangeDeserializerBuilder, Reader, Xlsx};
use dashmap::DashMap;
use itertools::Itertools;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::future::Future;
use std::str::FromStr;
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use std::thread;
use surrealdb::engine::any::Any;
use surrealdb::sql::Thing;
use surrealdb::Surreal;
use tokio::task;

#[derive(Resource, Serialize, Deserialize, PartialEq, Debug, Default, Clone)]
pub struct SiteData {
    pub refno: RefU64,
    pub name: String,
    pub is_selected: bool,
}

#[derive(Resource, Serialize, Deserialize, Debug, Default, Clone)]
pub struct SiteVec {
    pub data: Vec<SiteData>,
}

#[derive(Resource, Serialize, Deserialize, Debug, Default, Clone)]
pub struct SelectedSiteVec {
    pub data: Vec<SiteData>,
}

#[derive(Resource, Serialize, Deserialize, Debug, Default, Clone)]
pub struct FlagSiteVec {
    pub data: Vec<SiteData>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct PdmsMajor {
    pub site: RefU64,
    pub major: String,
    // site下zone对应的专业代码
    pub zone: HashMap<RefU64, String>,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct PbsMajorValue {
    #[serde_as(as = "DisplayFromStr")]
    pub id: RefU64,
    pub noun: String,
    pub major: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PbsConfig {
    pub code: Option<String>,
    pub name: Option<String>,
    pub att_type: Option<String>,
    pub site_pdms_name: Option<String>,
    pub zone_code: Option<String>,
    pub zone_name: Option<String>,
    pub zone_att_type: Option<String>,
    pub zone_pdms_name: Option<String>,
}

impl PbsConfig {
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.code.is_some() && self.name.is_some() && self.att_type.is_some()
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub(crate) struct SscLevel {
    pub name: Option<String>,
    pub att_type: Option<String>,
    pub owner: Option<String>,
}

impl SscLevel {
    pub fn is_valid(&self) -> bool {
        if self.name.is_none() || self.att_type.is_none() {
            return false;
        }
        true
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub enum SaveDatabaseChannelMsg {
    // 插入数据  表名和数据
    InsertSql(String, String),
    // 插入数据  表名和数据
    InsertPbsElements(String, Vec<PbsElement>),
    // 插入relate数据
    InsertRelateSql(Vec<String>),
    // 结束
    Quit,
}

/// 设置site和zone所属的专业
pub async fn set_pdms_major_code(aios_mgr: &AiosDBMgr) -> anyhow::Result<()> {
    // 读取专业配置表
    let major_codes = get_room_level_from_excel_refactor().await?.name_code_map;
    // 找到所有的site和zone
    let mut site_children_map = HashMap::new();
    let sites =
        get_mdb_world_site_pes(format!("/{}", aios_mgr.db_option.mdb_name), DBType::DESI).await?;
    let mut site_name_map = HashMap::new();
    for site in sites {
        let Ok(children) = aios_mgr.get_children(site.refno).await else {
            continue;
        };
        for child in children {
            if child.noun != "ZONE".to_string() {
                continue;
            };
            site_children_map
                .entry(site.refno)
                .or_insert_with(Vec::new)
                .push(child);
        }
        site_name_map.entry(site.refno).or_insert(site.name);
    }
    // 给site和zone赋上对应的code
    let mut result = Vec::new();
    for codes in major_codes.into_iter().rev() {
        // 给site赋值
        for site in site_children_map.keys() {
            let Some(site_name) = site_name_map.get(site) else {
                continue;
            };
            if site_name.contains(&codes.site_name) {
                // let mut zone_majors = HashMap::new();
                result.push(PbsMajorValue {
                    id: *site,
                    noun: "SITE".to_string(),
                    major: codes.site_code.clone(),
                });
                // 给zone赋值
                for (major_name, major_code) in &codes.zone_map {
                    for zone in site_children_map.get(&site).unwrap() {
                        if zone.name.contains(major_name) {
                            // zone_majors.entry(zone.refno).or_insert(major_code.clone());
                            result.push(PbsMajorValue {
                                id: zone.refno,
                                noun: zone.noun.clone(),
                                major: major_code.clone(),
                            })
                        }
                    }
                }
                // 方便测试查看使用
                // result.push(PdmsMajor {
                //     site: *site,
                //     major: codes.site_code.clone(),
                //     zone: zone_majors,
                // })
            }
        }
    }
    // 将分配好的专业代码保存到数据库中
    let json = serde_json::to_string(&result)?;
    insert_into_table(&SUL_DB, PDMS_MAJOR, &json).await?;
    Ok(())
}

/// ssc专业配置excel表 返回的对应数据
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SscMajorCodeConfig {
    /// key : site 的 name (中文名) value : site 下对应的zone 的 name
    pub level: Vec<(String, Vec<String>)>,
    /// 英文 code 对应的中文名
    pub name_map: DashMap<String, String>,
    /// pdms中 site 和 zone name 对应的专业代码
    pub name_code_map: Vec<PbsMajorCode>,
}

/// pdms site 和 zone name 对应的专业代码
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PbsMajorCode {
    /// pdms site 的 name
    pub site_name: String,
    /// 专业代码
    pub site_code: String,
    /// site 下 zone name 对应的 专业代码
    pub zone_map: HashMap<String, String>,
}

/// 读取 专业分类 excel表 ，返回需要的值
pub async fn get_room_level_from_excel_refactor() -> anyhow::Result<SscMajorCodeConfig> {
    let mut level: Vec<(String, Vec<String>)> = Vec::new();
    let mut name_map = DashMap::new();
    let mut pdms_zone_name_map = HashMap::new();
    let mut name_code_map = Vec::new();

    let mut workbook: Xlsx<_> = open_workbook("resource/专业分类.xlsx")?;
    dbg!("加载专业分类.xlsx 成功");
    let range = workbook
        .worksheet_range("Sheet2")
        .ok_or(anyhow!("Cannot find 'Sheet1'"))??;
    dbg!("打开Sheet2成功");

    let mut iter = RangeDeserializerBuilder::new().from_range(&range)?;
    let mut b_first = true;
    let mut site_code = "".to_string();
    let mut site_chinese_name = "".to_string();
    let mut pdms_site_name = "".to_string();
    let mut zones = Vec::new();
    // let mut configs = Vec::new();
    let mut configs_sql = String::new();
    let mut index = 0;
    while let Some(result) = iter.next() {
        let config: PbsConfig = result?;
        configs_sql.push_str(&format!(
            "create pbs_config:{index} content {};",
            serde_json::to_string(&config)?
        ));
        // configs.push(config.clone());
        // site 的 name 、code 、att_type
        if config.code.is_some()
            && config.name.is_some()
            && config.att_type.is_some()
            && config.site_pdms_name.is_some()
        {
            let read_site_code = config.code.unwrap();
            let read_site_chinese_name = config.name.unwrap();
            let read_pdms_site_name = config.site_pdms_name.unwrap();
            // code != site_code 代表是下一个site的数据了 , b_first 防止第一个判断就是 != 会导致读取的数据错开，第一个site没值
            if read_site_code != site_code && !b_first {
                name_code_map.push(PbsMajorCode {
                    site_name: pdms_site_name.clone(),
                    site_code: site_code.clone(),
                    zone_map: pdms_zone_name_map.clone(),
                });
                pdms_zone_name_map.clear();

                level.push((site_code, zones.clone()));
                zones.clear();
            }
            b_first = false;
            site_code = read_site_code.clone();
            site_chinese_name = read_site_chinese_name.clone();
            pdms_site_name = read_pdms_site_name.clone();
            // 存储专业编码对应的中文名称
            name_map.insert(read_site_code, site_chinese_name.clone());

            // 存放 site 下 zone 的专业代码
            if config.zone_name.is_some() && config.zone_code.is_some() {
                let read_zone_name = config.zone_name.unwrap();
                let read_zone_code = config.zone_code.unwrap();
                name_map.insert(read_zone_code.clone(), read_zone_name.clone());
                // 存放 pdms的site下 zone name 对应的 专业代码
                if config.zone_pdms_name.is_some() {
                    pdms_zone_name_map
                        .entry(config.zone_pdms_name.unwrap())
                        .or_insert(read_zone_code.clone());
                }
                zones.push(read_zone_code);
            }
        }

        index += 1;
    }

    SUL_DB.query(configs_sql).await?;

    Ok(SscMajorCodeConfig {
        level,
        name_map,
        name_code_map,
    })
}

#[derive(Debug)]
struct PBSRelate {
    pub in_id: Thing,
    pub out_id: Thing,
    pub order_num: u32,
}

impl PBSRelate {
    pub fn to_surreal_relate(self, relate_name: &str) -> String {
        format!(
            "relate {}->{}:{}->{}",
            self.in_id.to_raw(),
            relate_name,
            self.order_num,
            self.out_id.to_raw()
        )
    }
}

pub static PBS_ROOT_ID: Lazy<Thing> = Lazy::new(|| Thing::from_str("pbs:0").unwrap());
pub const PBS_STR: &'static str = "PBS";

/// 生成pbs固定节点
pub async fn set_pbs_fixed_node(tx: &Sender<SaveDatabaseChannelMsg>) -> anyhow::Result<()> {
    let mut eless = Vec::new();
    let mut edge_results = Vec::new();

    let mut workbook: Xlsx<_> = open_workbook("resource/ssc_level.xlsx")?;
    let range = workbook
        .worksheet_range("Sheet1")
        .ok_or(anyhow::anyhow!("Cannot find 'Sheet1'"))??;

    let mut iter = RangeDeserializerBuilder::new().from_range(&range)?;
    let mut idx = 0;
    eless.push(PbsElement {
        id: PBS_ROOT_ID.clone(),
        owner: None,
        noun: None,
        name: "/*".to_string(),
        refno: None,
        children_cnt: 1,
    });
    while let Some(result) = iter.next() {
        let v: SscLevel = result?;
        if v.is_valid() {
            let name = v.name.unwrap();
            let name_hash = hash_str(&name);
            let owner = if v.owner.is_some() {
                hash_str(&v.owner.unwrap())
            } else {
                0
            };
            let cur: Thing = ("pbs".to_string(), name_hash.to_string()).into();
            let owner: Thing = ("pbs".to_string(), owner.to_string()).into();
            eless.push(PbsElement {
                id: cur.clone(),
                noun: v.att_type.clone(),
                name,
                owner: Some(owner.clone()),
                refno: None,
                children_cnt: 0,
            });

            edge_results.push(
                PBSRelate {
                    in_id: cur.clone(),
                    out_id: owner.clone(),
                    order_num: idx,
                }
                .to_surreal_relate(&PBS_OWNER),
            );
            idx += 1;
        }
    }
    // 保存树节点
    // let ele_json = serde_json::to_string(&eles_results)?;
    tx.send(SaveDatabaseChannelMsg::InsertPbsElements(
        PBS_TABLE.to_string(),
        eless,
    ))?;
    tx.send(SaveDatabaseChannelMsg::InsertRelateSql(edge_results))?;
    Ok(())
}

struct PbsRoomNodeResult {
    pub rooms: HashMap<String, BTreeSet<String>>,
    pub sql: String,
    pub relate_sql: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PbsElement {
    pub id: Thing,
    pub owner: Option<Thing>,
    pub refno: Option<RefU64>,
    pub name: String,
    pub noun: Option<String>,
    pub children_cnt: usize,
}

impl Default for PbsElement{
    fn default() -> Self {
        Self {
            id: Thing::from_str("unset:0").unwrap(),
            owner: None,
            refno: None,
            name: String::default(),
            noun: None,
            children_cnt: 0,
        }
    }
}

impl PbsElement {
    pub fn gen_sur_json(&self) -> String {
        let mut json_string = serde_json::to_string_pretty(&serde_json::json!({
            "id": self.id.to_string(),
            "owner": self.owner.as_ref().map(|x| x.to_string()),
            "refno": self.refno,
            "name": self.name,
            "noun": self.noun,
            "children_cnt": self.children_cnt,
        }))
        .unwrap();
        json_string
    }

    pub fn root_ele() -> Self {
        Self {
            id: PBS_ROOT_ID.clone(),
            name: "/*".to_string(),
            children_cnt: 1,
            ..Default::default()
        }
    }
}

impl PdmsNodeTrait for PbsElement {

    #[inline]
    fn get_children_count(&self) -> usize {
        self.children_cnt
    }

    #[inline]
    fn get_name(&self) -> &str {
        self.name.as_str()
    }

    #[inline]
    fn get_id(&self) -> Option<&Thing> {
        Some(&self.id)
    }
}


/// 生成房间节点
pub async fn set_pbs_room_node(
    tx: &Sender<SaveDatabaseChannelMsg>,
) -> anyhow::Result<HashMap<String, BTreeSet<String>>> {
    let mut result = Vec::new();
    let mut relate_result = Vec::new();
    let rooms = query_all_room_name().await?;
    let mut name_set = HashSet::new();
    name_set.insert("一号机组".to_string());
    let first_jizhu: Thing = ("pbs".to_string(), hash_str("一号机组").to_string()).into();
    // 将项目中所有的房间，通过厂房 、 层位 、 房间号进行排列和存储
    for (factory_idx, (factory, room)) in rooms.clone().into_iter().enumerate() {
        let factory_hash = hash_str(&factory).to_string();
        let factory_id: Thing = ("pbs".to_string(), factory_hash).into();
        // 存放厂房
        result.push(PbsElement {
            id: factory_id.clone(),
            owner: Some(first_jizhu.clone()),
            name: factory.clone(),
            ..Default::default()
        });
        relate_result.push(
            PBSRelate {
                in_id: factory_id.clone(),
                out_id: first_jizhu.clone(),
                order_num: factory_idx as u32,
            }
            .to_surreal_relate(&PBS_OWNER),
        );
        // 存放厂房下 安装层位 和 安装分区 两个固定节点
        let install_level = hash_str(&format!("{}安装层位", factory)).to_string(); //将厂房放在一起hash，否则不同厂房的这两个节点会重复
        let install_area = hash_str(&format!("{}安装分区", factory)).to_string();
        let install_level_id: Thing = ("pbs".to_string(), install_level).into();
        let install_area_id: Thing = ("pbs".to_string(), install_area).into();
        result.push(PbsElement {
            id: install_level_id.clone(),
            owner: Some(factory_id.clone()),
            name: "安装层位".to_string(),
            ..Default::default()
        });
        relate_result.push(
            PBSRelate {
                in_id: install_level_id.clone(),
                out_id: factory_id.clone(),
                order_num: 0,
            }
            .to_surreal_relate(&PBS_OWNER),
        );
        result.push(PbsElement {
            id: install_area_id.clone(),
            owner: Some(factory_id.clone()),
            name: "安装分区".to_string(),
            ..Default::default()
        });
        relate_result.push(
            PBSRelate {
                in_id: install_area_id.clone(),
                out_id: factory_id.clone(),
                order_num: 1,
            }
            .to_surreal_relate(&PBS_OWNER),
        );
        // 存放层位以及房间信息
        let mut level_map = HashSet::new();
        for (idx, r) in room.into_iter().enumerate() {
            let level = r[1..2].to_string(); // 房间号第二位就是层位,之前已经做过长度的判断，所以直接切片
            let Ok(level_num) = level.parse::<u32>() else {
                continue;
            };
            // 将厂房和层位放在一起hash，单独的层位hash id会重复
            let level_hash = hash_str(&format!("{}{}", factory, level)).to_string();
            let level_id: Thing = ("pbs".to_string(), level_hash.clone()).into();
            // 层位
            if !level_map.contains(&level) {
                result.push(PbsElement {
                    id: level_id.clone(),
                    owner: Some(install_level_id.clone()),
                    name: format!("{}层", level),
                    ..Default::default()
                });
                relate_result.push(
                    PBSRelate {
                        in_id: level_id.clone(),
                        out_id: install_level_id.clone(),
                        order_num: level_num,
                    }
                    .to_surreal_relate(&PBS_OWNER),
                );
                level_map.insert(level);
            }
            // 房间
            let room_hash = hash_str(&r).to_string();
            let room_id: Thing = ("pbs".to_string(), room_hash.clone()).into();
            result.push(PbsElement {
                id: room_id.clone(),
                owner: Some(level_id.clone()),
                name: r,
                ..Default::default()
            });
            relate_result.push(
                PBSRelate {
                    in_id: room_id.clone(),
                    out_id: level_id.clone(),
                    order_num: idx as u32,
                }
                .to_surreal_relate(&PBS_OWNER),
            );
        }
    }
    // 保存树节点
    // let ele_json = serde_json::to_string(&result)?;
    tx.send(SaveDatabaseChannelMsg::InsertPbsElements(
        PBS_TABLE.to_string(),
        result,
    ))?;
    tx.send(SaveDatabaseChannelMsg::InsertRelateSql(relate_result))?;
    Ok(rooms)
}

/// 保存房间下的专业
pub async fn set_pbs_room_major_node(
    rooms: &HashMap<String, BTreeSet<String>>,
    tx: &Sender<SaveDatabaseChannelMsg>,
) -> anyhow::Result<()> {
    let mut result = Vec::new();
    let mut relate_result = Vec::new();
    // 获取 pdms site 和 zone 对应的专业代码
    let pdms_level = get_room_level_from_excel_refactor().await?;
    let major_map = pdms_level.name_map;
    for (_, room) in rooms {
        for r in room {
            // site 下的专业
            for (site_idx, (site_name, zones)) in pdms_level.level.iter().enumerate() {
                let site_hash = hash_str(&format!("{}{}", r, site_name)).to_string();
                let site_id: Thing = ("pbs".to_string(), site_hash).into();
                let room_hash = hash_str(r).to_string();
                let room_id: Thing = ("pbs".to_string(), room_hash).into();
                let Some(site_major) = major_map.get(site_name) else {
                    continue;
                };
                result.push(PbsElement {
                    id: site_id.clone(),
                    owner: Some(room_id.clone()),
                    name: site_major.value().to_string(),
                    ..Default::default()
                });
                relate_result.push(
                    PBSRelate {
                        in_id: site_id.clone(),
                        out_id: room_id.clone(),
                        order_num: site_idx as u32,
                    }
                    .to_surreal_relate(&PBS_OWNER),
                );
                // 专业下的子专业
                for (zone_idx, zone) in zones.iter().enumerate() {
                    let zone_hash = hash_str(&format!("{}{}", r, zone)).to_string(); // 避免不同专业下的子专业重复
                    let zone_id: Thing = ("pbs".to_string(), zone_hash).into();
                    let Some(zone_major) = major_map.get(zone) else {
                        continue;
                    };
                    result.push(PbsElement {
                        id: zone_id.clone(),
                        owner: Some(site_id.clone()),
                        name: zone_major.value().to_string(),
                        ..Default::default()
                    });
                    relate_result.push(
                        PBSRelate {
                            in_id: zone_id.clone(),
                            out_id: site_id.clone(),
                            order_num: zone_idx as u32,
                        }
                        .to_surreal_relate(&PBS_OWNER),
                    );
                }
            }
        }
    }
    // 保存树节点
    // let ele_json = serde_json::to_string(&result)?;
    // tx.send(SaveDatabaseChannelMsg::InsertSql(PBS_TABLE.to_string(), ele_json))?;
    tx.send(SaveDatabaseChannelMsg::InsertPbsElements(
        PBS_TABLE.to_string(),
        result,
    ))?;
    tx.send(SaveDatabaseChannelMsg::InsertRelateSql(relate_result))?;
    Ok(())
}

/// 获取所有赋过专业值的zone
async fn query_all_zone_with_major() -> anyhow::Result<Vec<PbsMajorValue>> {
    let mut response = SUL_DB
        .query("select * from pdms_major where noun == 'ZONE';")
        .await?;
    let result: Vec<PbsMajorValue> = response.take(0)?;
    Ok(result)
}

/// 保存房间下节点所属的专业
pub async fn set_pbs_node(tx: &Sender<SaveDatabaseChannelMsg>) -> anyhow::Result<()> {
    let zones = query_all_zone_with_major().await?;
    let len = zones.len();
    // 查找zone下所有需要进行pbs计算的节点
    for (idx, zone) in zones.iter().enumerate() {
        println!(
            "正在处理 zone: {} ,目前第 {} 个,总共 {} 个",
            zone.id, idx, len
        );
        // 找到所有需要处理的节点
        // let nodes = query_ele_filter_deep_children(zone.id, vec!["BRAN".to_string(),
        //                                                          "EQUI".to_string(), "STRU".to_string(), "REST".to_string()]).await?;
        let bran_refnos = query_filter_deep_children(zone.id, vec!["BRAN".to_string()]).await?;
        // 处理bran
        set_pbs_bran_node(&bran_refnos, &zone, tx).await?;
        // 处理equi
        // set_pbs_equi_node(&nodes, &zone, tx).await?;
        // 处理支吊架
        // set_pbs_supp_node(&nodes, &zone, tx).await?;
    }
    Ok(())
}

/// 保存房间下bran相关的节点
async fn set_pbs_bran_node(
    refnos: &[RefU64],
    zone: &PbsMajorValue,
    tx: &Sender<SaveDatabaseChannelMsg>,
) -> anyhow::Result<()> {
    let mut result = Vec::new();
    let mut relate_result = Vec::new();
    // 查找bran相关的pdms树的数据
    let pdms_nodes = query_pbs_pdms_node(refnos).await?;
    for (idx, node) in pdms_nodes.into_iter().enumerate() {
        // 没有房间号的就跳过
        if node.room_code.is_none() {
            continue;
        };
        let room_code = node.room_code.clone().unwrap();
        let owner = hash_str(&format!("{}{}", room_code, zone.major)).to_string();
        let owner_id: Thing = ("pbs".to_string(), owner).into();
        result.push(PbsElement {
            id: node.id.to_pbs_thing(),
            refno: Some(node.id.clone()),
            owner: Some(owner_id.clone()),
            name: node.name.clone(),
            noun: Some(node.noun.clone()),
            ..Default::default()
        });
        relate_result.push(
            PBSRelate {
                in_id: node.id.to_pbs_thing(),
                out_id: owner_id.clone(),
                order_num: idx as u32,
            }
            .to_surreal_relate(&PBS_OWNER),
        );
        // 存放children
        for (child_idx, child) in node.children.into_iter().enumerate() {
            relate_result.push(
                PBSRelate {
                    in_id: child.id.clone(),
                    out_id: child.owner.clone().unwrap(),
                    order_num: child_idx as u32,
                }
                .to_surreal_relate(&PBS_OWNER),
            );
            result.push(child);
        }
    }
    // 保存树节点
    // let pbs_json = serde_json::to_string(&result)?;
    tx.send(SaveDatabaseChannelMsg::InsertPbsElements(
        PBS_TABLE.to_string(),
        result,
    ))?;
    tx.send(SaveDatabaseChannelMsg::InsertRelateSql(relate_result))?;
    Ok(())
}

// /// 保存房间下equi相关的节点
// async fn set_pbs_equi_node(nodes: &Vec<SPdmsElement>, zone: &PdmsMajorValue, tx: &Sender<SaveDatabaseChannelMsg>) -> anyhow::Result<()> {
//     let mut result = Vec::new();
//     let mut relate_result = Vec::new();
//     // 查找equi相关的pdms树的数据
//     let equis = nodes.iter().filter(|node| node.noun == "EQUI").collect::<Vec<_>>();
//     let equi_refnos = equis.iter().map(|bran| bran.refno).collect::<Vec<_>>();
//     let pdms_nodes = query_pbs_pdms_node(equi_refnos).await?;
//     // 收集sube
//     let mut subes = Vec::new();
//     for node in &pdms_nodes {
//         for child in &node.children {
//             if child.noun != "SUBE".to_string() { continue; };
//             subes.push(child.refno);
//         }
//     }
//     // 查询sube的children
//     let sube_children = query_pbs_children_by_refnos(subes).await?;
//     // 将equi节点放到pbs中
//     for (idx, node) in pdms_nodes.iter().enumerate() {
//         // 没有房间号的就跳过
//         if node.room_code.is_none() { continue; };
//         let room_code = node.room_code.clone().unwrap();
//         let owner = hash_str(&format!("{}{}", room_code, zone.major));
//         result.push(SPdmsElement {
//             id: node.id.to_pbs_key(),
//             refno: node.id,
//             owner: RefU64(owner),
//             name: node.name.clone(),
//             noun: node.noun.clone(),
//             dbnum: 0,
//             e3d_version: 0,
//             version_tag: None,
//             status_tag: None,
//             cata_hash: "".to_string(),
//             lock: false,
//             deleted: false,
//         });
//         relate_result.push(PBSRelate {
//             in_id: node.id.to_pbs_key(),
//             out_id: RefU64(owner).to_pbs_key(),
//             order_num: idx as u32,
//         }.to_surreal_relate(&PBS_OWNER));
//         // 存放children
//         for (child_idx, child) in node.children.iter().enumerate() {
//             result.push(child.clone());
//             relate_result.push(PBSRelate {
//                 in_id: child.id.clone(),
//                 out_id: child.owner.to_pbs_key(),
//                 order_num: child_idx as u32,
//             }.to_surreal_relate(&PBS_OWNER));
//             // 将sube的children放到pbs中的sube下
//             if child.noun == "SUBE".to_string() {
//                 let Some(sube_children) = sube_children.get(&child.refno) else { continue; };
//                 for (sube_idx, sube) in sube_children.iter().enumerate() {
//                     result.push(sube.clone());
//                     relate_result.push(PBSRelate {
//                         in_id: sube.id.clone(),
//                         out_id: sube.owner.to_pbs_key(),
//                         order_num: sube_idx as u32,
//                     }.to_surreal_relate(&PBS_OWNER));
//                 }
//             }
//         }
//     }
//     // 保存树节点
//     let ele_json = serde_json::to_string(&result)?;
//     tx.send(SaveDatabaseChannelMsg::InsertSql(PBS_TABLE.to_string(), ele_json))?;
//     tx.send(SaveDatabaseChannelMsg::InsertRelateSql(relate_result))?;
//     Ok(())
// }
//
// /// 保存房间下supp相关的节点
// async fn set_pbs_supp_node(nodes: &Vec<SPdmsElement>, zone: &PdmsMajorValue, tx: &Sender<SaveDatabaseChannelMsg>) -> anyhow::Result<()> {
//     let mut result = Vec::new();
//     let mut relate_result = Vec::new();
//     // 这几个支架下面只有STRU，不需要找REST
//     if zone.major == "HVACSU".to_string() || zone.major == "ELEMSU".to_string() || zone.major == "ELELSU".to_string() {
//         let supps = nodes.iter().filter(|node| node.noun == "STRU").collect::<Vec<_>>();
//         let supp_refnos = supps.iter().map(|bran| bran.refno).collect::<Vec<_>>();
//         let pdms_nodes = query_pbs_pdms_node(supp_refnos).await?;
//         // 收集 FRMW
//         let mut frmws = Vec::new();
//         for node in &pdms_nodes {
//             for child in &node.children {
//                 if child.noun != "FRMW".to_string() { continue; };
//                 frmws.push(child.refno);
//             }
//         }
//         // 查询 FRMW 和 HANG的children
//         let frmw_children = query_pbs_children_by_refnos(frmws).await?;
//         for (idx, supp) in pdms_nodes.iter().enumerate() {
//             if supp.room_code.is_none() { continue; };
//             let room_code = supp.room_code.clone().unwrap();
//             let owner = hash_str(&format!("{}{}", room_code, zone.major));
//             // 存放 STRU
//             result.push(PbsElement {
//                 id: supp.id.to_pbs_key(),
//                 refno: supp.id,
//                 owner: RefU64(owner),
//                 name: supp.noun.clone(),
//                 noun: supp.noun.clone(),
//                 dbnum: 0,
//                 e3d_version: 0,
//                 version_tag: None,
//                 status_tag: None,
//                 cata_hash: "".to_string(),
//                 lock: false,
//                 deleted: false,
//             });
//             relate_result.push(PBSRelate {
//                 in_id: supp.id.to_pbs_key(),
//                 out_id: RefU64(owner).to_pbs_key(),
//                 order_num: 0,
//             }.to_surreal_relate(&PBS_OWNER));
//             // 存放children
//             for (child_idx, child) in supp.children.iter().enumerate() {
//                 result.push(PbsElement {
//                     id: child.refno.to_pbs_key(),
//                     ..child.clone()
//                 });
//                 relate_result.push(PBSRelate {
//                     in_id: child.refno.to_pe_key(),
//                     out_id: child.owner.to_pbs_key(),
//                     order_num: child_idx as u32,
//                 }.to_surreal_relate(&PBS_OWNER));
//                 // 将FRMW的children放到pbs中的sube下
//                 if child.noun.as_str() == "FRMW" {
//                     let Some(children) = frmw_children.get(&child.refno) else { continue; };
//                     for (supp_idx, supp) in children.iter().enumerate() {
//                         result.push(PbsElement {
//                             id: supp.refno.to_pbs_key(),
//                             refno: supp.refno.clone(),
//                             owner: supp.owner.clone(),
//                             name: supp.name.clone(),
//                             noun: supp.noun.clone(),
//                         });
//                         relate_result.push(PBSRelate {
//                             in_id: supp.refno.to_pe_key(),
//                             out_id: supp.owner.to_pbs_key(),
//                             order_num: supp_idx as u32,
//                         }.to_surreal_relate(&PBS_OWNER));
//                     }
//                 }
//             }
//         }
//     } else {
//         let supps = nodes.iter().filter(|node| node.noun == "STRU" || node.noun == "REST").collect::<Vec<_>>();
//         let supp_refnos = supps.iter().map(|bran| bran.refno).collect::<Vec<_>>();
//         let pdms_nodes = query_pbs_pdms_node(supp_refnos).await?;
//         // 支吊架 pbs结构需要在REST和STRU上面加一层房间号 + 流水号的固定节点
//         let mut supp_owner_map = HashSet::new();
//         // 收集 FRMW 和 HANG
//         let mut hangs = Vec::new();
//         for node in &pdms_nodes {
//             for child in &node.children {
//                 if child.noun != "FRMW".to_string() && child.noun != "HANG".to_string() { continue; };
//                 hangs.push(child.refno);
//             }
//         }
//         // 查询 FRMW 和 HANG的children
//         let hang_children = query_pbs_children_by_refnos(hangs).await?;
//         for (idx, supp) in pdms_nodes.iter().enumerate() {
//             if supp.room_code.is_none() { continue; };
//             let room_code = supp.room_code.clone().unwrap();
//             let supp_fixed_name_split = supp.name[1..].to_string().split("/")
//                 .map(|x| x.to_string()).collect::<Vec<_>>();
//             let Some(supp_fixed_name) = supp_fixed_name_split.last() else { continue; };
//             let fixed_hash = hash_str(supp_fixed_name);
//             let owner = hash_str(&format!("{}{}", room_code, zone.major));
//             // 存放固定节点
//             if !supp_owner_map.contains(&fixed_hash) {
//                 result.push(SPdmsElement {
//                     id: RefU64(fixed_hash).to_pbs_key(),
//                     refno: RefU64(fixed_hash),
//                     owner: RefU64(owner),
//                     name: supp_fixed_name.to_string(),
//                    noun: None,
//                     dbnum: 0,
//                     e3d_version: 0,
//                     version_tag: None,
//                     status_tag: None,
//                     cata_hash: "".to_string(),
//                     lock: false,
//                     deleted: false,
//                 });
//                 relate_result.push(PBSRelate {
//                     in_id: RefU64(fixed_hash).to_pbs_key(),
//                     out_id: RefU64(owner).to_pbs_key(),
//                     order_num: idx as u32,
//                 }.to_surreal_relate(&PBS_OWNER));
//                 supp_owner_map.insert(fixed_hash);
//             }
//             // 存放 STRU/REST
//             result.push(SPdmsElement {
//                 id: supp.id.to_pbs_key(),
//                 refno: supp.id,
//                 owner: RefU64(fixed_hash),
//                 name: supp.noun.clone(),
//                 noun: supp.noun.clone(),
//                 dbnum: 0,
//                 e3d_version: 0,
//                 version_tag: None,
//                 status_tag: None,
//                 cata_hash: "".to_string(),
//                 lock: false,
//                 deleted: false,
//             });
//             relate_result.push(PBSRelate {
//                 in_id: supp.id.to_pbs_key(),
//                 out_id: RefU64(fixed_hash).to_pbs_key(),
//                 order_num: if supp.noun == "STRU".to_string() { 0 } else { 1 },
//             }.to_surreal_relate(&PBS_OWNER));
//             // 存放children
//             for (child_idx, child) in supp.children.iter().enumerate() {
//                 result.push(SPdmsElement {
//                     id: child.refno.to_pe_key(),
//                     refno: child.refno,
//                     owner: child.owner,
//                     name: child.name.clone(),
//                     noun: child.noun.clone(),
//                     dbnum: 0,
//                     e3d_version: 0,
//                     version_tag: None,
//                     status_tag: None,
//                     cata_hash: child.cata_hash.clone(),
//                     lock: false,
//                     deleted: false,
//                 });
//                 relate_result.push(PBSRelate {
//                     in_id: child.refno.to_pe_key(),
//                     out_id: child.owner.to_pbs_key(),
//                     order_num: child_idx as u32,
//                 }.to_surreal_relate(&PBS_OWNER));
//                 // 将FRMW/HANG的children放到pbs中的sube下
//                 if child.noun == "FRMW".to_string() || child.noun == "HANG".to_string() {
//                     let Some(children) = hang_children.get(&child.refno) else { continue; };
//                     for (supp_idx, supp) in children.iter().enumerate() {
//                         result.push(SPdmsElement {
//                             id: supp.refno.to_pe_key(),
//                             refno: supp.refno,
//                             owner: supp.owner,
//                             name: supp.name.clone(),
//                             noun: supp.noun.clone(),
//                             dbnum: 0,
//                             e3d_version: 0,
//                             version_tag: None,
//                             status_tag: None,
//                             cata_hash: supp.cata_hash.clone(),
//                             lock: false,
//                             deleted: false,
//                         });
//                         relate_result.push(PBSRelate {
//                             in_id: supp.refno.to_pe_key(),
//                             out_id: supp.owner.to_pbs_key(),
//                             order_num: supp_idx as u32,
//                         }.to_surreal_relate(&PBS_OWNER));
//                     }
//                 }
//             }
//         }
//     }
//     // 保存树节点
//     let ele_json = serde_json::to_string(&result)?;
//     tx.send(SaveDatabaseChannelMsg::InsertSql(PBS_TABLE.to_string(), ele_json))?;
//     tx.send(SaveDatabaseChannelMsg::InsertRelateSql(relate_result))?;
//     Ok(())
// }

/// pbs下重新划分的pdms树节点，bran equi等
#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
struct PBSRoomNode {
    pub id: RefU64,
    pub name: String,
    pub noun: String,
    pub room_code: Option<String>,
    pub children: Vec<PbsElement>,
}

/// 查找pbs需要的pdms的节点以及房间号
async fn query_pbs_pdms_node(refnos: &[RefU64]) -> anyhow::Result<Vec<PBSRoomNode>> {
    if refnos.is_empty() {
        return Ok(vec![]);
    };
    let refnos = refnos.into_iter().map(|refno| refno.to_pe_key()).join(",");
    let sql = format!(
        r#"select id,name,noun,fn::room_code($this.id)[0] as room_code,
            (select value in.* from $this.id<-pe_owner) as children from [{}]"#,
        refnos
    );
    let mut response = SUL_DB.query(sql).await?;
    let result: Vec<PBSRoomNode> = response.take(0)?;
    Ok(result)
}

/// 查询多个参考号的children
async fn query_pbs_children_by_refnos(
    refnos: Vec<RefU64>,
) -> anyhow::Result<HashMap<RefU64, Vec<PbsElement>>> {
    if refnos.is_empty() {
        return Ok(HashMap::default());
    };
    let mut map = HashMap::new();
    let pes = refnos.into_iter().map(|refno| refno.to_pe_key()).join(",");
    let sql = format!(
        r#"select value in.* from $this.id<-pe_owner from [{}]"#,
        pes
    );
    let mut response = SUL_DB.query(sql).await?;
    let result: Vec<Vec<PbsElement>> = response.take(0)?;
    for r in result {
        if r.is_empty() {
            continue;
        };
        if let Some(o) = &r[0].owner {
            map.entry(o.clone().into()).or_insert(r);
        }
    }
    Ok(map)
}

/// 接受保存数据库请求并执行操作
pub async fn execute_save_pbs(rx: mpsc::Receiver<SaveDatabaseChannelMsg>) -> anyhow::Result<()> {
    for msg in rx {
        match msg {
            // 保存table数据
            SaveDatabaseChannelMsg::InsertSql(table_name, sql) => {
                insert_into_table(&SUL_DB, &table_name, &sql).await?;
            }
            SaveDatabaseChannelMsg::InsertPbsElements(table, eles) => {
                // SUL_DB.create()
                let json = eles.iter().map(|x| x.gen_sur_json()).join(",");
                // println!("json is {}", &json);
                SUL_DB.query(format!("insert into {} [{}];", table, json)).await.unwrap();
            }
            // 保存 relate
            SaveDatabaseChannelMsg::InsertRelateSql(relate_sql) => {
                insert_relate_to_table(&SUL_DB, relate_sql).await?;
            }
            SaveDatabaseChannelMsg::Quit => {
                break;
            }
        }
    }
    Ok(())
}

#[tokio::test]
async fn test_set_pbs_fixed_node() -> anyhow::Result<()> {
    let aios_mgr = AiosDBMgr::init_from_db_option().await?;
    // 创建通道
    let (tx, rx) = mpsc::channel();

    set_pbs_fixed_node(&tx).await?;
    let rooms = set_pbs_room_node(&tx).await?;
    set_pbs_room_major_node(&rooms, &tx).await?;
    set_pbs_node(&tx).await?;
    // 创建数据库处理线程
    let db_thread = task::spawn(async move {
        execute_save_pbs(rx).await.unwrap();
    });

    // 发送退出消息
    tx.send(SaveDatabaseChannelMsg::Quit)?;

    // 等待数据库处理任务结束
    db_thread.await?;
    Ok(())
}

#[tokio::test]
async fn test_set_pdms_major_code() -> anyhow::Result<()> {
    let aios_mgr = AiosDBMgr::init_from_db_option().await?;
    set_pdms_major_code(&aios_mgr).await?;
    Ok(())
}

#[tokio::test]
async fn test_set_pbs_room_node() -> anyhow::Result<()> {
    let aios_mgr = AiosDBMgr::init_from_db_option().await?;
    // set_pbs_node().await?;
    Ok(())
}
