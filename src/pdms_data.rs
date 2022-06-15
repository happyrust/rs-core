use std::collections::BTreeSet;
use std::fmt::{Debug, Formatter};
use std::ops::Deref;
use dashmap::{DashMap, DashSet};
use dashmap::mapref::one::Ref;
use smol_str::SmolStr;
use crate::pdms_types::{AttrInfo, AttrMap, DbAttributeType, RefU64};
use crate::tool::db_tool::db1_dehash;
use serde::{Serialize, Deserialize};
use itertools::Itertools;


#[derive(Clone, Debug)]
pub struct ScomInfo {
    pub attr_map: AttrMap,
    pub gtype: SmolStr,
    pub dtse_params: Vec<DatasetParamStr>,
    pub gm_params: Vec<GmParam>,
    pub axis_params: Vec<AxisParam>,
    pub params: SmolStr,
    pub axis_param_numbers: Vec<i32>,
}

#[derive(Clone, Debug, Default)]
pub struct DatasetParamStr {
    pub refno: SmolStr,
    pub name: SmolStr,
    pub self_type: SmolStr,
    pub lock: bool,
    pub owner: SmolStr,
    pub description: SmolStr,
    pub dkey: SmolStr,
    pub ptype: SmolStr,
    pub pproperty: SmolStr,
    pub dproperty: SmolStr,
    pub purpose: SmolStr,
    pub number: i32,
    pub dtitle: SmolStr,
    pub punits: SmolStr,
    pub ruse: SmolStr,
    pub lhide: bool,
}

//还是要用枚举，来列举各个情况
//GMSE GMSS
#[derive(Clone, Debug, Default)]
pub struct GmParam {
    pub refno: RefU64,
    /// SCYL  LSNO  SCTO  SDSH  SBOX
    pub gm_type: SmolStr,  //SCYL  LSNO  SCTO  SDSH  SBOX  SANN  SPRO

    pub prad: SmolStr,
    pub pang: SmolStr,
    pub pwid: SmolStr,
    /// 顺序 pdiameter pbdiameter ptdiameter, 先bottom, 后top
    pub diameters: Vec<SmolStr>,
    /// 顺序 pdistance pbdistance ptdistance, 先bottom, 后top
    pub distances: Vec<SmolStr>,
    pub shears: Vec<SmolStr>,
    pub phei: SmolStr,
    pub offset: SmolStr,
    /// 顺序 x y z
    pub box_lengths: Vec<SmolStr>,
    pub xyz: Vec<SmolStr>,

    // pub profile:
    //profile  SPVE   SANN(PX, PY)
    pub verts: Vec<[SmolStr; 2]>,
    //SANN: dx dy dradius dwidth
    pub dxy: Vec<[SmolStr; 2]>,
    pub drad: SmolStr,
    pub dwid: SmolStr,
    /// 顺序 paxis pa_axis pb_axis pc_axis
    pub paxises: Vec<SmolStr>,
    pub centre_line_flag: bool,
    pub visible_flag: bool,
    pub prads: Vec<SmolStr>,
}

#[derive(Clone, Debug, Default)]
pub struct AxisParam {
    pub attr_map: AttrMap,
    pub x: SmolStr,
    pub y: SmolStr,
    pub z: SmolStr,
    pub distance: SmolStr,
    pub direction: SmolStr,
    pub pconnect: SmolStr,
    pub pbore: SmolStr,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum NewDataState {
    Modify = 0,
    Increase = 1,
    Delete = 2,
    Invalid,
}

impl From<i32> for NewDataState {
    fn from(v: i32) -> Self {
        match v {
            0 => { Self::Modify }
            1 => { Self::Increase }
            2 => { Self::Delete }
            _ => { Self::Invalid }
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct IncrementData {
    pub refno: RefU64,
    pub attr_data_map: AttrMap,
    pub state: NewDataState,
    pub version: u32,
}


impl Debug for IncrementData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IncrementData")
            .field("refno", &self.refno.to_refno_str())
            .field("map", &self.attr_data_map.to_string_hashmap())
            .field("state", &self.state)
            .field("version", &self.version)
            .finish()
    }
}

lazy_static! {
    static ref ATTR_INFO_MAP: AttInfoMap = {
        let db_info: PdmsDatabaseInfo = serde_json::from_str(include_str!("all_attr_info.json")).unwrap();
        //调用方法
        let mut att_info_map = AttInfoMap{
            map: db_info.noun_attr_info_map,
            type_att_names_map: Default::default(),
            type_explicit_att_names_map: Default::default(),
            att_name_type_map: Default::default(),
            has_cat_ref_types_set: Default::default(),
        };
        att_info_map.init_type_att_names_map();
        att_info_map
    };
}


#[derive(Default, Debug, Clone)]
pub struct AttInfoMap{
    pub map: DashMap<i32, DashMap<i32, AttrInfo>>,
    pub type_att_names_map: DashMap<String, BTreeSet<String>>,
    pub type_explicit_att_names_map: DashMap<String, BTreeSet<String>>,
    pub att_name_type_map: DashMap<String, DbAttributeType>,
    pub has_cat_ref_types_set: DashSet<String>,
}

impl Deref for AttInfoMap {
    type Target = DashMap<i32, DashMap<i32, AttrInfo>>;

    fn deref(&self) -> &Self::Target {
        &self.map
    }
}

impl AttInfoMap {
    #[inline]
    pub fn init_type_att_names_map(&mut self){
        for k in &self.map {
            let type_name = db1_dehash(*k.key() as u32);
            for v in k.value() {
                self.type_att_names_map.entry(type_name.clone())
                    .or_insert(BTreeSet::new()).insert(v.name.to_string());
                if v.offset > 0 {
                    self.type_explicit_att_names_map.entry(type_name.clone())
                        .or_insert(BTreeSet::new()).insert(v.name.to_string());
                }
                self.att_name_type_map.insert(v.name.to_string(), v.att_type);
                if v.name.as_str() == "CATR" || v.name.as_str() == "SPRE" {
                    self.has_cat_ref_types_set.insert(type_name.clone());
                }
            }
        }
    }

    /// 有元件库的类型
    #[inline]
    pub fn get_has_cat_ref_types_set(&self) -> &DashSet<String> {
        &self.has_cat_ref_types_set
    }

    /// 获取有catref的类型
    #[inline]
    pub fn get_has_cat_ref_type_names(&self) -> Vec<String> {
        self.get_has_cat_ref_types_set().iter().map(|x| x.clone()).collect::<Vec<_>>()
    }

    /// 获取有catref的类型
    #[inline]
    pub fn get_type_implicit_att_names(&self, type_name: &str) -> Vec<String> {
        self.type_explicit_att_names_map.get(type_name).map(|v|{
            v.value().iter().cloned().collect_vec()
        }).unwrap_or_default()
        // self.type_explicit_att_names_map.iter().map(|x| x.clone()).collect::<Vec<_>>()
    }

    #[inline]
    pub fn get_names_map(&self) -> &DashMap<String, BTreeSet<String>> {
        &self.type_att_names_map
    }

    #[inline]
    pub fn get_names_of_type(&self, type_name: &str) -> Option<Ref<String, BTreeSet<String>>> {
        self.type_att_names_map.get(type_name)
    }

    #[inline]
    pub fn exist_att_by_name(&self, type_name: &str, att_name: &str) -> bool {
        self.type_att_names_map.get(type_name).map(|x| x.contains(att_name)).unwrap_or(false)
    }

    /// 至少有一个 name 存在
    #[inline]
    pub fn exist_least_one_att_by_names(&self, type_name: &str, att_names: &Vec<&str>) -> bool {
        self.type_att_names_map.get(type_name).map(|x|
            att_names.iter().any(|v| x.value().contains(*v))).unwrap_or(false)
    }

    #[inline]
    pub fn get_val_type_of_att(&self, att_name: &str) -> Option<Ref<String, DbAttributeType>> {
        self.att_name_type_map.get(att_name)
    }
}
