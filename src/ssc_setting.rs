use std::collections::HashMap;
use anyhow::anyhow;
use crate::types::*;
use bevy_ecs::system::Resource;
use calamine::{open_workbook, RangeDeserializerBuilder, Reader, Xlsx};
use dashmap::DashMap;
use serde::{Serialize, Deserialize};
use crate::{DBType, get_mdb_world_site_pes};
use crate::aios_db_mgr::aios_mgr::AiosDBMgr;
use crate::aios_db_mgr::PdmsDataInterface;
use crate::options::DbOption;
use crate::test::test_surreal::init_test_surreal;

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

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SiteExcelData {
    pub code: Option<String>,
    pub name: Option<String>,
    pub att_type: Option<String>,
    pub site_pdms_name: Option<String>,
    pub zone_code: Option<String>,
    pub zone_name: Option<String>,
    pub zone_att_type: Option<String>,
    pub zone_pdms_name: Option<String>,
}

impl SiteExcelData {
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.code.is_some() && self.name.is_some() && self.att_type.is_some()
    }
}

/// 设置site和zone所属的专业
pub async fn set_pdms_major_code(aios_mgr: &AiosDBMgr) -> anyhow::Result<()> {
    // 找到所有的site和zone
    let mut site_children_map = HashMap::new();
    let sites = get_mdb_world_site_pes(format!("/{}", aios_mgr.db_option.mdb_name), DBType::DESI).await?;
    let mut site_name_map = HashMap::new();
    for site in sites {
        let Ok(children) = aios_mgr.get_children(site.refno).await else { continue; };
        for child in children {
            if child.noun != "ZONE".to_string() { continue; };
            site_children_map.entry(site.refno).or_insert_with(Vec::new).push(child);
        }
        site_name_map.entry(site.refno).or_insert(site.name);
    }
    // 读取专业配置表
    let major_codes = get_room_level_from_excel_refactor()?.pdms_name_code_map;
    // 给site和zone赋上对应的code
    let mut result = Vec::new();
    for codes in major_codes.into_iter().rev() {
        // 给site赋值
        for site in site_children_map.keys() {
            let Some(site_name) = site_name_map.get(site) else { continue; };
            if site_name.contains(&codes.site_name) {
                let mut zone_majors = HashMap::new();
                // 给zone赋值
                for (major_name, major_code) in &codes.zone_map {
                    for zone in site_children_map.get(&site).unwrap() {
                        if zone.name.contains(major_name) {
                            zone_majors.entry(zone.refno).or_insert(major_code.clone());
                        }
                    }
                }
                result.push(PdmsMajor {
                    site: *site,
                    major: codes.site_code.clone(),
                    zone: zone_majors,
                })
            }
        }
    }
    dbg!(&result);
    Ok(())
}

/// ssc专业配置excel表 返回的对应数据
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SscMajorCodeExcel {
    /// key : site 的 name (中文名) value : site 下对应的zone 的 name
    pub level: Vec<(String, Vec<String>)>,
    /// 英文 code 对应的中文名
    pub name_map: DashMap<String, String>,
    /// pdms中 site 和 zone name 对应的专业代码
    pub pdms_name_code_map: Vec<PdmsSscMajorCode>,
}

/// pdms site 和 zone name 对应的专业代码
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PdmsSscMajorCode {
    /// pdms site 的 name
    pub site_name: String,
    /// 专业代码
    pub site_code: String,
    /// site 下 zone name 对应的 专业代码
    pub zone_map: HashMap<String, String>,
}

/// 读取 专业分类 excel表 ，返回需要的值
pub fn get_room_level_from_excel_refactor() -> anyhow::Result<SscMajorCodeExcel> {
    let mut level: Vec<(String, Vec<String>)> = Vec::new();
    let mut name_map = DashMap::new();
    let mut pdms_zone_name_map = HashMap::new();
    let mut pdms_ssc_major_codes = Vec::new();

    let mut workbook: Xlsx<_> = open_workbook("resource/专业分类.xlsx")?;
    dbg!("加载专业分类.xlsx 成功");
    let range = workbook.worksheet_range("Sheet2")
        .ok_or(anyhow!("Cannot find 'Sheet1'"))??;
    dbg!("打开Sheet2成功");

    let mut iter = RangeDeserializerBuilder::new().from_range(&range)?;
    let mut b_first = true;
    let mut site_code = "".to_string();
    let mut site_chinese_name = "".to_string();
    let mut pdms_site_name = "".to_string();
    let mut zones = Vec::new();
    while let Some(result) = iter.next() {
        let v: SiteExcelData = result?;
        // site 的 name 、code 、att_type
        if v.code.is_some() && v.name.is_some() && v.att_type.is_some() && v.site_pdms_name.is_some() {
            let read_site_code = v.code.unwrap();
            let read_site_chinese_name = v.name.unwrap();
            let read_pdms_site_name = v.site_pdms_name.unwrap();
            // code != site_code 代表是下一个site的数据了 , b_first 防止第一个判断就是 != 会导致读取的数据错开，第一个site没值
            if read_site_code != site_code && !b_first {
                pdms_ssc_major_codes.push(PdmsSscMajorCode {
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
            if v.zone_name.is_some() && v.zone_code.is_some() {
                let read_zone_name = v.zone_name.unwrap();
                let read_zone_code = v.zone_code.unwrap();
                name_map.insert(read_zone_code.clone(), read_zone_name.clone());
                // 存放 pdms的site下 zone name 对应的 专业代码
                if v.zone_pdms_name.is_some() {
                    pdms_zone_name_map.entry(v.zone_pdms_name.unwrap()).or_insert(read_zone_code.clone());
                }
                zones.push(read_zone_code);
            }
        }
    }
    Ok(SscMajorCodeExcel {
        level,
        name_map,
        pdms_name_code_map: pdms_ssc_major_codes,
    })
}

#[tokio::test]
async fn test_set_pdms_major_code() -> anyhow::Result<()> {
    let aios_mgr = AiosDBMgr::init_from_db_option().await?;
    set_pdms_major_code(&aios_mgr).await?;
    Ok(())
}