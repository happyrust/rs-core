use crate::cache::mgr::BytesTrait;
use crate::consts::*;
use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::parsed_data::CateAxisParam;
use crate::prim_geo::cylinder::*;
use crate::prim_geo::sbox::SBox;
use crate::prim_geo::*;
use crate::tool::db_tool::{db1_dehash, db1_hash};
use crate::tool::hash_tool::*;
pub use crate::types::*;
use bevy_ecs::prelude::*;
use bevy_math::*;
use bevy_reflect::Reflect;
#[cfg(feature = "render")]
use bevy_render::mesh::Indices;
#[cfg(feature = "render")]
use bevy_render::render_resource::PrimitiveTopology::TriangleList;
use bevy_transform::prelude::*;
use bitflags::bitflags;
use dashmap::DashMap;
use derive_more::{Deref, DerefMut};
use glam::{Vec3, Vec4};
use id_tree::{NodeId, Tree};
use itertools::Itertools;
use nalgebra::Point3;
use parry3d::bounding_volume::Aabb;
use rkyv::with::Skip;
#[cfg(feature = "sea-orm")]
use sea_orm::entity::prelude::*;
#[cfg(feature = "sea-orm")]
use sea_query::*;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use serde_with::{DisplayFromStr, serde_as};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt::{Debug, Display, Pointer};
use std::fs::File;
use std::io::{Read, Write};
use std::ops::{Deref, DerefMut};
use std::path::Path;
use std::str::FromStr;
use std::string::ToString;
use std::sync::Arc;

///控制pdms显示的深度层级
pub const LEVEL_VISBLE: u32 = 6;

///非负实体基本体的种类
pub const PRIMITIVE_NOUN_NAMES: [&'static str; 8] = [
    "BOX", "CYLI", "SLCY", "CONE", "DISH", "CTOR", "RTOR", "PYRA",
];

///基本体的种类(包含负实体)
//"SPINE", "GENS",
pub const GNERAL_PRIM_NOUN_NAMES: [&'static str; 21] = [
    "BOX", "CYLI", "SLCY", "CONE", "DISH", "CTOR", "RTOR", "PYRA", "SNOU", "POHE",
     "NBOX", "NCYL", "NSBO", "NCON", "NSNO", "NPYR", "NDIS", "NCTO", "NRTO", "NSCY", "NREV"
];

///有loop的几何体
pub const GNERAL_LOOP_NOUN_NAMES: [&'static str; 2] = ["PLOO", "LOOP"];

///负实体基本体的种类
pub const GENRAL_NEG_NOUN_NAMES: [&'static str; 13] = [
    "NBOX", "NCYL", "NLCY", "NSBO", "NCON", "NSNO", "NPYR", "NDIS", "NXTR", "NCTO", "NRTO", "NREV",
    "NSCY",
];

///元件库的负实体类型
pub const CATE_NEG_NOUN_NAMES: [&'static str; 13] = [
    "NSBO", "NSCO", "NLSN", "NSSP", "NLCY", "NSCY", "NSCT", "NSRT", "NSDS", "NSSL", "NLPY", "NSEX",
    "NSRE",
];

pub const TOTAL_NEG_NOUN_NAMES: [&'static str; 26] = [
    "NBOX", "NCYL", "NLCY", "NSBO", "NCON", "NSNO", "NPYR", "NDIS", "NXTR", "NCTO", "NRTO", "NREV",
    "NSCY", "NSBO", "NSCO", "NLSN", "NSSP", "NLCY", "NSCY", "NSCT", "NSRT", "NSDS", "NSSL", "NLPY",
    "NSEX", "NSRE",
];

pub const GENRAL_POS_NOUN_NAMES: [&'static str; 25] = [
    "BOX", "CYLI", "SLCY", "CONE", "DISH", "CTOR", "RTOR", "PYRA", "SNOU", "FLOOR", "PANEL",
    "SBOX", "SCYL", "LCYL", "SSPH", "LCYL", "SCON", "LSNO", "LPYR", "SDSH", "SCTO", "SEXT", "SREV",
    "SRTO", "SSLC",
];

pub const TOTAL_GEO_NOUN_NAMES: [&'static str; 40] = [
    "BOX", "CYLI", "SLCY", "CONE", "DISH", "CTOR", "RTOR", "PYRA", "SNOU", "PLOO", "LOOP", "POHE",
    "SBOX", "SCYL", "SSPH", "LCYL", "SCON", "LSNO", "LPYR", "SDSH", "SCTO", "SEXT", "SREV", "SRTO",
    "SSLC", "SPRO", "SREC", "NBOX", "NCYL", "NLCY", "NSBO", "NCON", "NSNO", "NPYR", "NDIS", "NXTR",
    "NCTO", "NRTO", "NREV", "NSCY",
];

pub const TOTAL_CATA_GEO_NOUN_NAMES: [&'static str; 31] = [
    "SBOX", "SCYL", "SSPH", "LCYL", "SCON", "LSNO", "LPYR", "SDSH", "SCTO", "SEXT", "SREV", "SRTO",
    "SSLC", "SPRO", "SANN", "BOXI", "TUBE", "SREC", "NSBO", "NSCO", "NLSN", "NSSP", "NLCY", "NSCY", "NSCT",
    "NSRT", "NSDS", "NSSL", "NLPY", "NSEX", "NSRE",
];

///可能会与ngmr发生作用的类型
pub const TOTAL_CONTAIN_NGMR_GEO_NAEMS: [&'static str; 6] =
    ["WALL", "STWALL", "GWALL", "SCTN", "PANEL", "FLOOR"];

///POHE
pub const POHE_GEO_NAMES: [&'static str; 1] = ["POHE"];

///元件库的种类
pub const CATA_GEO_NAMES: [&'static str; 26] = [
    "BRAN", "HANG", "ELCONN", "CMPF", "WALL", "STWALL", "GWALL", "FIXING", "SJOI", "PJOI", "PFIT",
    "GENSEC", "RNODE", "PRTELE", "GPART", "SCREED", "NOZZ", "PALJ", "CABLE", "BATT", "CMFI",
    "SCOJ", "SEVE", "SBFI", "SCTN", "FITT",
];

///有tubi的类型
pub const CATA_HAS_TUBI_GEO_NAMES: [&'static str; 2] = ["BRAN", "HANG"];

///可以重用的类型
pub const CATA_SINGLE_REUSE_GEO_NAMES: [&'static str; 0] = [];

pub const CATA_WITHOUT_REUSE_GEO_NAMES: [&'static str; 24] = [
    "ELCONN", "CMPF", "WALL", "GWALL", "SJOI", "FITT", "PFIT", "FIXING", "PJOI", "GENSEC", "RNODE",
    "PRTELE", "GPART", "SCREED", "PALJ", "CABLE", "BATT", "CMFI", "SCOJ", "SEVE", "SBFI", "STWALL",
    "SCTN", "NOZZ",
];

pub const VISBILE_GEO_NOUNS: [&'static str; 38] = [
    "BOX", "CYLI", "SLCY", "CONE", "DISH", "CTOR", "RTOR", "PYRA", "SNOU", "POHE", "EXTR", "REVO",
    "FLOOR", "PANE", 
    "ELCONN", "CMPF", "WALL", "GWALL", "SJOI", "FITT", "PFIT", "FIXING", "PJOI", "GENSEC", "RNODE",
    "PRTELE", "GPART", "SCREED", "PALJ", "CABLE", "BATT", "CMFI", "SCOJ", "SEVE", "SBFI", "STWALL","SCTN", "NOZZ",
];


#[derive(Serialize, Deserialize, Clone, Debug, Default, Copy, Eq, PartialEq, Hash)]
pub enum SjusType {
    #[default]
    UNSET,
    UTOP,
    UBOT,
    UCEN,
}

#[derive(Serialize, Deserialize, Debug)]
struct Jsgf {
    #[serde(with = "string")]
    u: u64,
    #[serde(with = "string")]
    i: i64,
}

pub mod string {
    use std::fmt::Display;
    use std::str::FromStr;

    use serde::{de, Deserialize, Deserializer, Serializer};

    pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: Display,
        S: Serializer,
    {
        serializer.collect_str(value)
    }

    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where
        T: FromStr,
        T::Err: Display,
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(de::Error::custom)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifferenceValue {
    pub noun: String,
    pub old_value: Option<NamedAttrValue>,
    // 新增 old_value 为 none
    pub new_value: Option<NamedAttrValue>, // 删除 new_value 为 none
}

pub const DEFAULT_NOUNS: [NounHash; 4] = [TYPE_HASH, NAME_HASH, REFNO_HASH, OWNER_HASH];
pub const DEFAULT_NAMED_NOUNS: [&'static str; 4] = ["TYPE", "NAME", "REFNO", "OWNER"];

#[derive(Serialize, Deserialize, Clone, Debug, Default, Component)]
pub struct PdmsCachedAttrMap(pub HashMap<RefU64, AttrMap>);

impl PdmsCachedAttrMap {
    pub fn serialize_to_bin_file(&self, db_code: u32) -> bool {
        let mut file = File::create(format!("PdmsCachedAttrMap_{}.bin", db_code)).unwrap();
        let serialized = bincode::serialize(&self).unwrap();
        file.write_all(serialized.as_slice()).unwrap();
        true
    }

    pub fn deserialize_from_bin_file(db_code: u32) -> anyhow::Result<Self> {
        let mut file = File::open(format!("PdmsCachedAttrMap_{}.bin", db_code))?;
        let mut buf: Vec<u8> = Vec::new();
        file.read_to_end(&mut buf).ok();
        let r = bincode::deserialize(buf.as_slice())?;
        Ok(r)
    }
}

impl PdmsTree {
    pub fn serialize_to_bin_file(&self, db_code: u32) -> bool {
        let mut file = File::create(format!("PdmsTree_{}.bin", db_code)).unwrap();
        let serialized = bincode::serialize(&self).unwrap();
        file.write_all(serialized.as_slice()).unwrap();
        true
    }

    pub fn serialize_to_bin_file_with_name(&self, name: &str, db_code: u32) -> bool {
        let mut file = File::create(format!("{name}_{db_code}.bin")).unwrap();
        let serialized = bincode::serialize(&self).unwrap();
        file.write_all(serialized.as_slice()).unwrap();
        true
    }

    pub fn deserialize_from_bin_file(db_code: u32) -> anyhow::Result<Self> {
        let mut file = File::open(format!("PdmsTree_{}.bin", db_code))?;
        let mut buf: Vec<u8> = Vec::new();
        file.read_to_end(&mut buf).ok();
        let r = bincode::deserialize(buf.as_slice())?;
        Ok(r)
    }
    pub fn deserialize_from_bin_file_with_name(name: &str, db_code: u32) -> anyhow::Result<Self> {
        let mut file = File::open(format!("{name}_{db_code}.bin"))?;
        let mut buf: Vec<u8> = Vec::new();
        file.read_to_end(&mut buf).ok();
        let r = bincode::deserialize(buf.as_slice())?;
        Ok(r)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, Component)]
pub struct PdmsTree(pub Tree<EleTreeNode>);

impl Deref for PdmsTree {
    type Target = Tree<EleTreeNode>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for PdmsTree {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl PdmsTree{

    ///获得世界refno
    #[inline]
    pub fn get_world_refno(&self) -> Option<RefU64> {
        self.root_node_id().map(|x| self.get(x).map(|t| t.data().refno).ok()).flatten()
    }

}

/// 一个参考号是有可能重复的，project信息可以不用存储，获取信息时必须要带上 db_no
#[derive(
    Debug, Clone, Serialize, Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize,
)]
pub struct RefnoInfo {
    /// 参考号的ref0
    pub ref_0: u32,
    /// 对应db number
    pub db_no: u32,
}

///可以缩放的类型
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ScaledGeom {
    Box(Vec3),
    Cylinder(Vec3),
    Sphere(f32),
}

pub type PdmsMeshIdx = String;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[repr(C)]
pub enum GeoType {
    Box = 0,
    Cylinder,
    Dish,
    Sphere,
    Snout,
    CTorus,
    RTorus,
    Pyramid,
    Revo,
    Extru,
    Polyhedron,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AiosMaterial {
    pub color: Vec4,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum GeoData {
    Primitive(PdmsMeshIdx), //索引的哪个mesh,和对应的拉伸值， 先从dish开始判断相似性
                            // Raw(Mesh),          //原生的Mesh
}

#[derive(
    Serialize,
    Deserialize,
    Clone,
    Debug,
    Default,
    Deref,
    DerefMut, /*, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize,*/
)]
pub struct LevelShapeMgr {
    pub level_mgr: DashMap<RefU64, RefU64Vec>,
}

impl LevelShapeMgr {
    pub fn serialize_to_specify_file(&self, file_path: &str) -> bool {
        let mut file = File::create(file_path).unwrap();
        let serialized = bincode::serialize(&self).unwrap();
        file.write_all(serialized.as_slice()).unwrap();
        true
    }
}

bitflags! {
    struct PdmsGenericTypeFlag: u32 {
        const UNKOWN = 0x1 << 30;
        const GENRIC = 0x1 << 1;
        const PIPE = 0x1 << 2;
        const STRU = 0x1 << 3;
        const EQUI = 0x1 << 4;
        const ROOM = 0x1 << 5;
        const WALL = 0x1 << 6;
    }
}

#[repr(C)]
#[derive(
    Component,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
    Serialize,
    Deserialize,
    Default,
    Clone,
    // Display,
    strum_macros::Display,
    strum_macros::EnumString,
    Debug,
    Copy,
    Eq,
    PartialEq,
    Hash,
)]
pub enum PdmsGenericType {
    #[default]
    UNKOWN = 0,
    CE,
    PIPE,
    STRU,
    EQUI,
    ROOM,
    SCTN,
    WALL,
    GENSEC,
    HANG,
    HANDRA,
    PANE,
    CFLOOR,
    FLOOR,
    EXTR,
    CWBRAN,
    REVO,
    CTWALL,
    AREADEF,
    DEMOPA,
    INSURQ,
    STRLNG,
    HVAC,
    ATTA,
    GRIDPL,
    GRIDCYL,
    DIMGRO,
    AIDGRO,
    CPLATE,
    CSTIFF,
    CCURVE,
    CSEAM,
    HCOMPT,
    HIBRA,
    HIDOU,
    HICPLA,
    MPLATE,
    MPROF,
    HICSTI,
}

/// 几何体的基本类型
#[derive(
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
    Serialize,
    Deserialize,
    PartialEq,
    Debug,
    Clone,
    Default,
    Resource,
)]
pub enum GeoBasicType {
    #[default]
    UNKOWN,
    ///正实体
    Pos,
    ///普通负实体
    Neg,
    ///元件库的负实体
    CateNeg,
    ///元件库的需要和design运算的负实体
    CateCrossNeg,
    ///负实体运算过了
    Compound,
    ///属于隐含直段的类型
    Tubi,
}

#[derive(
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
    Serialize,
    Deserialize,
    Debug,
    Clone,
    Default,
    Resource,
)]
pub struct GeoEdge {
    //元件的参考号
    #[serde(serialize_with = "ser_inst_info_edge_as_key_str")]
    // #[serde(deserialize_with = "de_refno_from_key_str")]
    #[serde(rename = "_from")]
    pub refno: RefU64,
    #[serde(serialize_with = "ser_inst_geo_edge_as_key_str")]
    #[serde(rename = "_to")]
    pub geo_hash: u64,
    pub geo_type: GeoBasicType,
    pub cata_hash: Option<u64>,
}

#[inline]
fn ser_inst_info_edge_as_key_str<S>(refno: &RefU64, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str(format!("pdms_inst_infos/{}", refno.to_string()).as_str())
}

#[inline]
fn ser_inst_geo_edge_as_key_str<S>(k: &u64, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str(format!("pdms_inst_geos/{}", *k).as_str())
}

impl GeoEdge {
    #[inline]
    pub fn get_hash(&self) -> u64 {
        self.geo_hash
    }

    #[inline]
    pub fn is_pos(&self) -> bool {
        self.geo_type == GeoBasicType::Pos
    }

    #[inline]
    pub fn is_neg(&self) -> bool {
        self.geo_type == GeoBasicType::Neg
    }
}

/// 存储一个Element 包含的所有几何信息
#[derive(
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
    Serialize,
    Deserialize,
    Debug,
    Clone,
    Default,
    Resource,
)]
#[serde_as]
pub struct EleGeosInfo {
    #[serde(serialize_with = "ser_refno_as_key_str")]
    #[serde(deserialize_with = "de_refno_from_key_str")]
    #[serde(rename = "_key")]
    pub refno: RefU64,
    //有哪一些 geo insts 组成
    //也可以通过edge 来组合
    #[serde(default)]
    pub cata_hash: Option<String>,
    //记录对应的元件库参考号
    #[serde(default)]
    #[serde(skip)]
    #[with(rkyv::with::Skip)]
    pub cata_refno: Option<RefU64>,
    //是否可见
    pub visible: bool,
    //所属一般类型，ROOM、STRU、PIPE等, 用枚举处理
    pub generic_type: PdmsGenericType,
    pub aabb: Option<Aabb>,
    //相对世界坐标系下的变换矩阵 rot, translation, scale
    //现在保存在 relate 里了，不需要再存储到图数据库里
    pub world_transform: Transform,

    #[serde(default)]
    pub flow_pt_indexs: Vec<i32>,

    #[serde(default)]
    pub geo_type: GeoBasicType,

    #[serde(skip, default)]
    pub ptset_map: BTreeMap<i32, CateAxisParam>,
}

pub fn de_refno_from_key_str<'de, D>(deserializer: D) -> Result<RefU64, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(RefU64::from_str(&s).unwrap_or_default())
}

pub fn ser_refno_as_key_str<S>(refno: &RefU64, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str(refno.to_string().as_str())
}

// pub fn ser_refno_slash_str<S>(refno: &RefU64, s: S) -> Result<S::Ok, S::Error>
// where
//     S: Serializer,
// {
//     s.serialize_str(refno.to_string().as_str())
// }

impl EleGeosInfo {

    pub fn id(&self) -> u64 {
        self.cata_hash.as_ref().map(|x| x.parse().ok()).flatten().unwrap_or(*self.refno)
    }

    pub fn id_str(&self) -> String {
        self.cata_hash.clone().unwrap_or(self.refno.to_string())
    }

    ///生成surreal的json文件
    pub fn gen_sur_json(&self) -> String {
        let id = self.id();
        let mut json_string = serde_json::to_string_pretty(&serde_json::json!({
            // "id": self.,
            "visible": self.visible,
            "generic_type": self.generic_type,
            "flow_pt_indexs": self.flow_pt_indexs.clone(),
            "geo_type": self.geo_type.clone(),
        }))
        .unwrap();

        json_string.remove(json_string.len() - 1);
        json_string.push_str(",");
        json_string.push_str(&format!(r#""id": inst_info:⟨{}⟩ "#, id));
        json_string.push_str("}");


        json_string
    }

    /// Checks if the PDMS type is a compound.
    #[inline]
    pub fn is_compound(&self) -> bool {
        self.geo_type == GeoBasicType::Compound
    }

    #[inline]
    pub fn update_to_compound(&mut self, key: Option<&str>) {
        let inst_key = hash_two_str(
            &self.get_inst_key().to_string(),
            key.as_deref().unwrap_or("compound"),
        );
        self.cata_hash = Some(inst_key.to_string());
        self.geo_type = GeoBasicType::Compound;
    }

    #[inline]
    pub fn update_to_ngmr(&mut self, key: Option<&str>) {
        let inst_key = hash_two_str(
            &self.get_inst_key().to_string(),
            key.as_deref().unwrap_or("ngmr"),
        );
        self.cata_hash = Some(inst_key.to_string());
        self.geo_type = GeoBasicType::CateCrossNeg;
    }

    ///获取几何体数据的string key
    #[inline]
    pub fn get_inst_key(&self) -> String {
        if let Some(c) = &self.cata_hash {
            return c.clone();
        }
        self.refno.to_string()
    }

    ///获取几何体数据的u64 key
    #[inline]
    pub fn get_inst_key_u64(&self) -> u64 {
        if let Some(c) = &self.cata_hash {
            return c.parse::<u64>().unwrap_or(*self.refno);
        }
        *self.refno
    }

    #[inline]
    pub fn get_ele_world_transform(&self) -> Transform {
        self.world_transform
    }

    #[inline]
    pub fn get_geo_world_transform(&self, geo: &EleInstGeo) -> Transform {
        let ele_trans = self.get_ele_world_transform();
        if geo.is_tubi {
            geo.transform
        } else {
            ele_trans * geo.transform
        }
    }
}

/// instane数据集合管理
#[derive(
    Serialize,
    Deserialize,
    Debug,
    Default,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
    Resource,
)]
pub struct ShapeInstancesData {
    /// 保存instance信息数据
    pub inst_info_map: std::collections::HashMap<RefU64, EleGeosInfo>,
    ///保存所有用到的的tubi数据
    pub inst_tubi_map: std::collections::HashMap<RefU64, EleGeosInfo>,
    ///保存instance几何数据
    pub inst_geos_map: std::collections::HashMap<String, EleInstGeosData>,

    ///保存所有用到的的compound数据
    #[serde(skip)]
    #[with(Skip)]
    pub compound_inst_info_map: std::collections::HashMap<RefU64, EleGeosInfo>,

    ///保存所有用到的的ngmr数据
    #[serde(skip)]
    #[with(Skip)]
    pub ngmr_inst_info_map: std::collections::HashMap<RefU64, EleGeosInfo>,
}

/// shape instances 的管理方法
impl ShapeInstancesData {
    ///填充基本的形状
    pub fn fill_basic_shapes(&mut self) {
        let unit_cyli_aabb = Aabb::new(Point3::new(-0.5, -0.5, 0.0), Point3::new(0.5, 0.5, 1.0));
        let unit_box_aabb = Aabb::new(Point3::new(-0.5, -0.5, -0.5), Point3::new(0.5, 0.5, 0.5));
        self.insert_geos_data(
            TUBI_GEO_HASH.to_string(),
            EleInstGeosData {
                inst_key: TUBI_GEO_HASH.to_string(),
                refno: Default::default(),
                insts: vec![EleInstGeo {
                    geo_hash: TUBI_GEO_HASH,
                    refno: Default::default(),
                    owner_pos_refnos: Default::default(),
                    geo_param: PdmsGeoParam::PrimSCylinder(SCylinder::default()),
                    pts: vec![],
                    aabb: Some(unit_cyli_aabb),
                    transform: Default::default(),
                    visible: true,
                    is_tubi: true,
                    geo_type: GeoBasicType::Tubi,
                }],
                aabb: Some(unit_cyli_aabb),
                type_name: "TUBI".to_string(),
                ptset_map: Default::default(),
            },
        );
        self.insert_geos_data(
            BOXI_GEO_HASH.to_string(),
            EleInstGeosData {
                inst_key: BOXI_GEO_HASH.to_string(),
                refno: Default::default(),
                insts: vec![EleInstGeo {
                    geo_hash: BOXI_GEO_HASH,
                    refno: Default::default(),
                    owner_pos_refnos: Default::default(),
                    geo_param: PdmsGeoParam::PrimBox(SBox::default()),
                    pts: vec![],
                    aabb: Some(unit_box_aabb),
                    transform: Default::default(),
                    visible: true,
                    is_tubi: true,
                    geo_type: GeoBasicType::Tubi,
                }],
                aabb: Some(unit_box_aabb),
                type_name: "BOXI".to_string(),
                ptset_map: Default::default(),
            },
        );
    }

    #[inline]
    pub fn clear(&mut self) {
        self.inst_info_map.clear();
        self.inst_geos_map.clear();
        self.inst_tubi_map.clear();
        self.compound_inst_info_map.clear();
        self.ngmr_inst_info_map.clear();
    }

    #[inline]
    pub fn get_show_refnos(&self) -> HashSet<RefU64> {
        let mut ready_refnos: HashSet<RefU64> = self.inst_info_map.keys().cloned().collect();
        ready_refnos.extend(self.inst_tubi_map.keys().cloned());
        ready_refnos
    }

    pub fn merge_ref(&mut self, o: &Self) {
        for (k, v) in o.inst_info_map.clone() {
            self.insert_info(k, v);
        }
        for (k, v) in o.inst_geos_map.clone() {
            self.insert_geos_data(k, v);
        }
        for (k, v) in o.inst_tubi_map.clone() {
            self.insert_tubi(k, v);
        }
    }

    pub fn merge(&mut self, other: Self) {
        let Self {
            inst_info_map,
            inst_tubi_map,
            inst_geos_map,
            ..
        } = other;
        for (k, v) in inst_info_map {
            self.insert_info(k, v);
        }
        for (k, v) in inst_geos_map {
            self.insert_geos_data(k, v);
        }
        for (k, v) in inst_tubi_map {
            self.insert_tubi(k, v);
        }
    }

    ///获得所有的geo hash值
    #[inline]
    pub fn get_geo_hashs(&self) -> BTreeSet<u64> {
        let mut geo_hashes = BTreeSet::new();
        for g in self.inst_geos_map.values() {
            for inst in &g.insts {
                geo_hashes.insert(inst.geo_hash);
            }
        }
        geo_hashes
    }

    #[inline]
    pub fn get_inst_geos(&self, info: &EleGeosInfo) -> Option<&Vec<EleInstGeo>> {
        let k = info.get_inst_key();
        self.inst_geos_map.get(&k).map(|x| &x.insts)
    }

    #[inline]
    pub fn get_inst_geos_data(&self, info: &EleGeosInfo) -> Option<&EleInstGeosData> {
        let k = info.get_inst_key();
        self.inst_geos_map.get(&k)
    }

    #[inline]
    pub fn get_final_inst_info(&self, refno: RefU64) -> Option<&EleGeosInfo> {
        self.get_compound_info(refno).or(self.get_inst_info(refno))
    }

    #[inline]
    pub fn get_inst_geos_data_mut_by_refno(
        &mut self,
        refno: RefU64,
    ) -> Option<&mut EleInstGeosData> {
        let info = self.get_inst_info(refno)?;
        self.inst_geos_map.get_mut(&info.get_inst_key())
    }

    #[inline]
    pub fn get_inst_geos_data_mut(&mut self, info: &EleGeosInfo) -> Option<&mut EleInstGeosData> {
        let k = info.get_inst_key();
        self.inst_geos_map.get_mut(&k)
    }

    #[inline]
    pub fn get_inst_tubi(&self, refno: RefU64) -> Option<&EleGeosInfo> {
        self.inst_tubi_map.get(&refno)
    }

    #[inline]
    pub fn contains(&self, refno: &RefU64) -> bool {
        self.inst_info_map.contains_key(refno) || self.inst_tubi_map.contains_key(refno)
    }

    #[inline]
    pub fn get_inst_info(&self, refno: RefU64) -> Option<&EleGeosInfo> {
        self.inst_info_map.get(&refno)
    }

    #[inline]
    pub fn get_compound_info(&self, refno: RefU64) -> Option<&EleGeosInfo> {
        self.compound_inst_info_map.get(&refno)
    }

    #[inline]
    pub fn insert_info(&mut self, refno: RefU64, info: EleGeosInfo) {
        self.inst_info_map.insert(refno, info);
    }

    #[inline]
    pub fn insert_compound_info(&mut self, refno: RefU64, info: EleGeosInfo) {
        self.compound_inst_info_map.insert(refno, info);
    }

    #[inline]
    pub fn insert_ngmr_info(&mut self, refno: RefU64, info: EleGeosInfo) {
        self.ngmr_inst_info_map.insert(refno, info);
    }

    #[inline]
    pub fn insert_geos_data(&mut self, hash: String, geo: EleInstGeosData) {
        if self.inst_geos_map.contains_key(&hash) {
            self.inst_geos_map
                .get_mut(&hash)
                .unwrap()
                .insts
                .extend_from_slice(&geo.insts);
        } else {
            self.inst_geos_map.insert(hash, geo);
        }
    }

    #[inline]
    pub fn insert_tubi(&mut self, refno: RefU64, info: EleGeosInfo) {
        self.inst_tubi_map.insert(refno, info);
    }

    pub fn get_info(&self, refno: &RefU64) -> Option<&EleGeosInfo> {
        self.inst_info_map.get(refno)
    }

    pub fn get_ngmr_info(&self, refno: &RefU64) -> Option<&EleGeosInfo> {
        self.ngmr_inst_info_map.get(refno)
    }

    //serialize_to_bytes
    pub fn serialize_to_bytes(&self) -> Vec<u8> {
        let serialized = rkyv::to_bytes::<_, 512>(self).unwrap().to_vec();
        serialized
    }

    pub fn serialize_to_specify_file(&self, file_path: &str) -> bool {
        let mut file = File::create(file_path).unwrap();
        let serialized = rkyv::to_bytes::<_, 512>(self).unwrap().to_vec();
        file.write_all(serialized.as_slice()).unwrap();
        true
    }

    pub fn deserialize_from_bin_file(file_path: &dyn AsRef<Path>) -> anyhow::Result<Self> {
        let mut file = File::open(file_path)?;
        let mut buf: Vec<u8> = Vec::new();
        file.read_to_end(&mut buf).ok();
        use rkyv::Deserialize;
        let archived = unsafe { rkyv::archived_root::<Self>(buf.as_slice()) };
        let r: Self = archived.deserialize(&mut rkyv::Infallible)?;
        Ok(r)
    }

    ///保存compound的edge关系到arango图数据库
    pub async fn save_compound_edges_to_arango() {}
}

//todo mesh 增量传输
#[derive(
    Serialize, Deserialize, Debug, Default, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize,
)]
pub struct PdmsInstanceMeshData {
    pub shape_insts: ShapeInstancesData,
    pub meshes_data: PlantMeshesData,
}

impl PdmsInstanceMeshData {
    #[inline]
    pub fn serialize_to_bytes(&self) -> Vec<u8> {
        rkyv::to_bytes::<_, 1024>(self).unwrap().to_vec()
    }

    pub fn deserialize_from_bin_file(file_path: &dyn AsRef<Path>) -> anyhow::Result<Self> {
        let mut file = File::open(file_path)?;
        let mut buf: Vec<u8> = Vec::new();
        file.read_to_end(&mut buf).ok();
        use rkyv::Deserialize;
        let archived = unsafe { rkyv::archived_root::<Self>(buf.as_slice()) };
        let r: Self = archived.deserialize(&mut rkyv::Infallible)?;
        Ok(r)
    }

    pub fn deserialize_from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        use rkyv::Deserialize;
        let archived = unsafe { rkyv::archived_root::<Self>(bytes) };
        let r: Self = archived.deserialize(&mut rkyv::Infallible)?;
        Ok(r)
    }
}

pub type GeoHash = u64;

#[derive(
    Serialize,
    Deserialize,
    Debug,
    Default,
    Resource,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub struct PlantGeoData {
    #[serde(rename = "_key")]
    #[serde(deserialize_with = "de_from_str")]
    #[serde(serialize_with = "ser_u64_as_str")]
    pub geo_hash: u64,
    #[serde(default)]
    #[serde(serialize_with = "se_plant_mesh")]
    #[serde(deserialize_with = "de_plant_mesh")]
    pub mesh: Option<PlantMesh>,
    pub aabb: Option<Aabb>,
    //最好能反序列化，看看怎么实现
    #[cfg(feature = "opencascade")]
    #[serde(skip)]
    #[with(Skip)]
    pub occ_shape: Option<opencascade::OCCShape>,
}

impl Clone for PlantGeoData {
    fn clone(&self) -> Self {
        Self {
            geo_hash: self.geo_hash.clone(),
            mesh: self.mesh.clone(),
            aabb: self.aabb.clone(),
        }
    }
}

impl PlantGeoData{
    pub fn load_from_file_by_hash(hashes: u64, path: &str) -> Self{
        let file_path = format!("{path}/{}.mesh", hashes);
        if let Ok(d) = Self::deserialize_from_bin_file(&file_path) {
            return d;
        }
        Self::default()
    }
    pub fn load_from_file_by_hashes(hashes: Vec<u64>, path: &str) -> Vec<Self>{
        let mut r = vec![];
        for h in hashes {
            let file_path = format!("{path}/{}.mesh", h);
            if let Ok(d) = Self::deserialize_from_bin_file(&file_path) {
                r.push(d);
            }
        }
        r
    }
}

fn de_plant_mesh<'de, D>(deserializer: D) -> Result<Option<PlantMesh>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    if let Ok(r) = hex::decode(s.as_str()) {
        return Ok(PlantMesh::from_compress_bytes(&r).ok());
    }
    Ok(None)
}

fn se_plant_mesh<S>(mesh: &Option<PlantMesh>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mesh_string = mesh
        .as_ref()
        .and_then(|x| Some(hex::encode(x.into_compress_bytes())))
        .unwrap_or("".to_string());
    s.serialize_str(&mesh_string)
}

unsafe impl Sync for PlantGeoData {}

unsafe impl Send for PlantGeoData {}

#[cfg(feature = "sea-orm")]
use crate::orm::*;
use crate::ref64vec::RefU64Vec;
use crate::shape::pdms_shape::{BrepShapeTrait, PlantMesh};
use crate::types::attmap::AttrMap;
use crate::types::attval::{AttrVal, AttrValAql};
use crate::types::named_attvalue::NamedAttrValue;
#[cfg(feature = "render")]
use bevy_render::prelude::*;
use bevy_render::render_asset::RenderAssetUsages;
#[cfg(feature = "occ")]
use opencascade::primitives::*;
#[cfg(feature = "sea-orm")]
use sea_query::*;
use crate::prim_geo::basic::{BOXI_GEO_HASH, OccSharedShape, TUBI_GEO_HASH};

impl PlantGeoData {
    ///返回三角模型 （tri_mesh, AABB）
    #[cfg(feature = "render")]
    pub fn gen_bevy_mesh_with_aabb(&self) -> Option<(Mesh, Option<Aabb>)> {

        let mut mesh = bevy_render::prelude::Mesh::new(TriangleList, RenderAssetUsages::RENDER_WORLD);
        // let mut mesh = bevy_render::prelude::Mesh::new(TriangleList);
        let d = self.mesh.as_ref()?;
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, d.vertices.clone());
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, d.normals.clone());
        let n = d.vertices.len();
        let mut uvs = vec![];
        for i in 0..n {
            uvs.push([0.0f32, 0.0]);
        }
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        //todo 是否需要优化索引
        mesh.insert_indices(Indices::U32(d.indices.clone()));

        Some((mesh, self.aabb))
    }

    pub fn serialize_to_specify_file(&self, file_path: &dyn AsRef<Path>) -> bool {
        let mut file = File::create(file_path).unwrap();
        let serialized = rkyv::to_bytes::<_, 1024>(self).unwrap().to_vec();
        file.write_all(serialized.as_slice()).unwrap();
        true
    }

    pub fn deserialize_from_bin_file(file_path: &dyn AsRef<Path>) -> anyhow::Result<Self> {
        let mut file = File::open(file_path)?;
        let mut buf: Vec<u8> = Vec::new();
        file.read_to_end(&mut buf).ok();
        use rkyv::Deserialize;
        let archived = unsafe { rkyv::archived_root::<Self>(buf.as_slice()) };
        let r: Self = archived.deserialize(&mut rkyv::Infallible)?;
        Ok(r)
    }

    pub fn deserialize_from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        use rkyv::Deserialize;
        let archived = unsafe { rkyv::archived_root::<Self>(bytes) };
        let r: Self = archived.deserialize(&mut rkyv::Infallible)?;
        Ok(r)
    }
}

#[derive(
    Serialize,
    Deserialize,
    Debug,
    Default,
    Deref,
    DerefMut,
    Resource,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub struct PlantMeshesData {
    pub meshes: HashMap<GeoHash, PlantGeoData>, //世界坐标系的变换, 为了js兼容64位，暂时使用String
}

impl PlantMeshesData {
    #[inline]
    pub fn serialize_to_bytes(&self) -> Vec<u8> {
        rkyv::to_bytes::<_, 1024>(self).unwrap().to_vec()
    }

    /// 获得对应的bevy 三角模型和线框模型
    #[cfg(feature = "render")]
    pub fn get_bevy_mesh(
        &self,
        mesh_hash: &u64,
    ) -> Option<(bevy_render::prelude::Mesh, Option<Aabb>)> {
        if let Some(c) = self.get(mesh_hash) {
            let bevy_mesh = c.gen_bevy_mesh_with_aabb();
            return bevy_mesh;
        }
        None
    }

    pub fn get_mesh(&self, geo_hash: u64) -> Option<&PlantMesh> {
        self.meshes.get(&geo_hash).and_then(|x| x.mesh.as_ref())
    }

    pub fn get_aabb(&self, geo_hash: u64) -> Option<Aabb> {
        self.meshes.get(&geo_hash).and_then(|x| x.aabb)
    }

    #[cfg(feature = "opencascade")]
    pub fn get_occ_shape(&self, geo_hash: u64) -> Option<&OCCShape> {
        self.meshes
            .get(&geo_hash)
            .and_then(|x| x.occ_shape.as_ref())
    }

    ///生成mesh的hash值，并且保存mesh
    pub fn gen_plant_data(
        &mut self,
        m: Box<dyn BrepShapeTrait>,
        replace: bool,
        tol_ratio: Option<f32>,
    ) -> Option<(u64, Aabb)> {
        let hash = m.hash_unit_mesh_params();
        //如果是重新生成，会去覆盖模型
        if replace || !self.meshes.contains_key(&hash) {
            if let Ok(mut d) = m.gen_unit(tol_ratio) {
                d.geo_hash = hash;
                self.meshes.insert(hash, d);
            } else {
                return None;
            }
        }
        Some((hash, self.get_bbox(&hash).unwrap()))
    }

    pub fn get_bbox(&self, hash: &u64) -> Option<Aabb> {
        if self.meshes.contains_key(hash) {
            let mesh = self.meshes.get(hash).unwrap();
            return mesh.aabb.clone();
        }
        None
    }

    pub fn serialize_to_specify_file(&self, file_path: &str) -> bool {
        let mut file = File::create(file_path).unwrap();
        let serialized = rkyv::to_bytes::<_, 1024>(self).unwrap().to_vec();
        file.write_all(serialized.as_slice()).unwrap();
        true
    }

    pub fn deserialize_from_bin_file(file_path: &dyn AsRef<Path>) -> anyhow::Result<Self> {
        let mut file = File::open(file_path)?;
        let mut buf: Vec<u8> = Vec::new();
        file.read_to_end(&mut buf).ok();
        use rkyv::Deserialize;
        let archived = unsafe { rkyv::archived_root::<Self>(buf.as_slice()) };
        let r: Self = archived.deserialize(&mut rkyv::Infallible)?;
        Ok(r)
    }
}

#[serde_as]
#[derive(
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
    Serialize,
    Deserialize,
    Clone,
    Debug,
    Default,
    Resource,
)]
pub struct EleInstGeosData {
    //maybe some hash value, or refno
    #[serde(rename = "_key", alias = "id")]
    pub inst_key: String,
    //design refno
    #[serde(deserialize_with = "de_refno_from_str")]
    #[serde(serialize_with = "ser_refno_as_str")]
    pub refno: RefU64,
    //todo 需要单独保存，使用record link 去发访问？
    pub insts: Vec<EleInstGeo>,

    pub aabb: Option<Aabb>,
    pub type_name: String,

    #[serde(skip)]
    pub ptset_map: BTreeMap<i32, CateAxisParam>,
}

impl EleInstGeosData {

    pub fn id(&self) -> u64 {
        self.inst_key.parse().unwrap_or(*self.refno)
    }

    ///生成surreal的json文件
    pub fn gen_sur_json(&self) -> String {
        let mut json_string = serde_json::to_string_pretty(&serde_json::json!({
            "id": self.inst_key.clone(),
            "type_name": self.type_name,
            "aabb": self.aabb,
            "ptset_map": self.ptset_map,
            "insts": self.insts,
        }))
        .unwrap();

        json_string.remove(json_string.len() - 1);
        json_string.push_str(",");
        // json_string.push_str(&format!(r#""id": {},"#, id));
        json_string.push_str(&format!(r#""refno": pe:{}"#, self.refno.to_string()));
        json_string.push_str("}");
        json_string
    }

    #[inline]
    pub fn has_neg(&self) -> bool {
        self.insts.iter().any(|x| x.geo_type == GeoBasicType::Neg)
    }

    #[inline]
    pub fn has_cata_neg(&self) -> bool {
        self.insts
            .iter()
            .any(|x| x.geo_type == GeoBasicType::CateNeg)
    }

    #[inline]
    pub fn has_ngmr(&self) -> bool {
        self.insts
            .iter()
            .any(|x| x.geo_type == GeoBasicType::CateCrossNeg)
    }

    ///返回ngmr的组合shape和owner pos refno
    #[cfg(feature = "occ")]
    pub fn gen_ngmr_occ_shapes(&self, transform: &Transform) -> Vec<(Vec<RefU64>, OccSharedShape)> {
        let ngmr_shapes: Vec<_> = self
            .insts
            .iter()
            .filter(|x| x.geo_type == GeoBasicType::CateCrossNeg)
            .filter_map(|x| {
                if let Some(mut s) = x.gen_occ_shape() {
                    s.as_mut().transform_by_mat(&transform.compute_matrix().as_dmat4());
                    let own_pos_refnos = x.owner_pos_refnos.clone().into_iter().collect();
                    Some((own_pos_refnos, s))
                } else {
                    None
                }
            })
            .collect();
        ngmr_shapes
    }

    ///返回新的shape，如果只有负实体，需要返回对应正实体的参考号
    #[cfg(feature = "occ")]
    pub fn gen_occ_shape(&self, transform: &Transform) -> Option<(OccSharedShape, Vec<RefU64>)> {
        let mut neg_shapes: Vec<(OccSharedShape, Vec<RefU64>)> = self
            .insts
            .iter()
            .filter(|x| x.geo_type == GeoBasicType::Neg)
            .filter_map(|x| {
                if let Some(mut s) = x.gen_occ_shape() {
                    s.as_mut().transform_by_mat(&transform.compute_matrix().as_dmat4());
                    let own_pos_refnos = x.owner_pos_refnos.clone().into_iter().collect();
                    Some((s, own_pos_refnos))
                } else {
                    None
                }
            })
            .collect();
        //如果出现负实体，只会出现一个？暂时这么处理
        if neg_shapes.len() >= 1 {
            return neg_shapes.pop();
        }

        let mut pos_shapes: HashMap<RefU64, OccSharedShape> = self
            .insts
            .iter()
            .filter(|x| x.geo_type == GeoBasicType::Pos)
            .filter_map(|x| {
                if let Some(s) = x.gen_occ_shape() {
                    Some((x.refno, s))
                } else {
                    None
                }
            })
            .collect();
        //执行cut 运算
        for cate_neg_inst in self.insts.iter().filter(|x| x.is_cata_neg()) {
            cate_neg_inst.owner_pos_refnos.iter().for_each(|r| {
                if let Some(pos_shape) = pos_shapes.get_mut(r) {
                    if let Some(neg_shape) = cate_neg_inst.gen_occ_shape() {
                        *pos_shape = pos_shape.subtract(&neg_shape).into_shape().into();
                    }
                }
            });
        }
        let mut compound: Shape = opencascade::primitives::Compound::from_shapes(pos_shapes.values()).into();
        compound.transform_by_mat(&transform.compute_matrix().as_dmat4());
        Some((compound.into(), vec![]))
    }
}

///分拆的基本体信息, 应该是不需要复用的
#[derive(
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
    Serialize,
    Deserialize,
    Clone,
    Debug,
    Default,
    Resource,
)]
#[serde_as]
pub struct EleInstGeo {
    /// 几何hash参数
    #[serde(deserialize_with = "de_from_str")]
    #[serde(serialize_with = "ser_u64_as_str")]
    pub geo_hash: u64,
    ///对应几何体参考号
    #[serde(deserialize_with = "de_refno_from_str")]
    #[serde(serialize_with = "ser_refno_as_str")]
    pub refno: RefU64,
    ///如果是负实体, 指定它的附属正实体参考号
    // #[serde_as(as = "HashSet<String>")]
    #[serde(deserialize_with = "de_hashset_from_str")]
    #[serde(serialize_with = "ser_hashset_as_str")]
    #[serde(default)]
    pub owner_pos_refnos: HashSet<RefU64>,
    ///几何参数数据
    #[serde(default)]
    pub geo_param: PdmsGeoParam,
    pub pts: Vec<i32>,
    pub aabb: Option<Aabb>,
    //相对于自身的坐标系变换
    #[serde(default)]
    pub transform: Transform,
    pub visible: bool,
    pub is_tubi: bool,
    #[serde(default)]
    pub geo_type: GeoBasicType,
}

impl EleInstGeo {
    #[inline]
    pub fn is_cata_neg(&self) -> bool {
        self.geo_type == GeoBasicType::CateNeg
    }

    #[inline]
    pub fn is_neg(&self) -> bool {
        self.geo_type == GeoBasicType::Neg
    }

    #[inline]
    pub fn key_points(&self) -> Vec<Vec3> {
        self.geo_param
            .key_points()
            .into_iter()
            .map(|v| self.transform.transform_point(*v))
            .collect()
    }

    ///fix 生成surreal的geo json数据，其他数据放在边上
    pub fn gen_geo_sur_json(&self) -> String {
        let mut json_string = "".to_string();
        // let mut json_string = serde_json::to_string_pretty(&serde_json::json!({
        //     "id": self.geo_hash,
        //     //点集索引 放到边里
        //     // "ppts": self.pts,
        //     "aabb": self.aabb,
        // }))
        // .unwrap();

        // json_string.remove(json_string.len() - 1);
        // json_string.push_str(",");
        // json_string.push_str(&format!(r#""id": {},"#, self.geo_hash));
        //, 'verts': 0, 'faces': 0'
        json_string.push_str(&format!(
            "{{'id': inst_geo:⟨{}⟩, 'aabb': aabb:⟨{}⟩, 'verts': 0, 'faces': 0}}",
            self.geo_hash,
            gen_bytes_hash::<_, 64>(&self.aabb)
        ));
        // json_string.push_str(&format!(r#""refno": pe:{}"#, self.refno.to_string()));
        // json_string.push_str("}");
        json_string
    }

    #[cfg(feature = "occ")]
    pub fn gen_occ_shape(&self) -> Option<OccSharedShape> {
        let mut shape: OccSharedShape = self.geo_param.gen_occ_shape()?;
        //scale 不能要，已经包含在OCC的真实参数里
        let mut new_transform = self.transform;
        new_transform.scale = Vec3::ONE;
        shape.as_mut().transform_by_mat(&new_transform.compute_matrix().as_dmat4());
        Some(shape)
    }
}

fn de_from_str<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    s.parse::<u64>().map_err(de::Error::custom)
}

fn de_refno_from_str<'de, D>(deserializer: D) -> Result<RefU64, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    RefU64::from_str(&s).map_err(de::Error::custom)
}

fn de_hashset_from_str<'de, D>(deserializer: D) -> Result<HashSet<RefU64>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = String::deserialize(deserializer).unwrap_or_default();
    Ok(serde_json::from_str::<HashSet<String>>(s.as_str())
        .unwrap_or_default()
        .into_iter()
        .map(|x| RefU64::from_str(x.as_str()).unwrap_or_default())
        .collect())
}

pub fn ser_hashset_as_str<S>(refnos: &HashSet<RefU64>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let set = refnos
        .into_iter()
        .map(|x| x.to_string())
        .collect::<HashSet<_>>();
    s.serialize_str(serde_json::to_string(&set).unwrap_or_default().as_str())
    // s.ser(&set)
}

pub fn ser_u64_as_str<S>(id: &u64, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str((*id).to_string().as_str())
}

pub fn ser_refno_as_str<S>(refno: &RefU64, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str(refno.to_string().as_str())
}

#[test]
fn test_ele_geo_instance_serialize_deserialize() {
    let _data = EleInstGeo {
        geo_hash: 1,
        refno: RefU64(56882546920359),
        geo_param: Default::default(),
        // aabb: Some(Aabb::new(Point3::new(1.0, 0.0, 0.0), Point3::new(2.0, 2.0, 0.0))),
        pts: Vec::new(),
        aabb: None,
        transform: Transform::IDENTITY,
        visible: false,
        is_tubi: false,
        geo_type: Default::default(),
        owner_pos_refnos: Default::default(),
    };
    // let json = serde_json::to_string(&data).unwrap();
    // dbg!(&json);
    // let json = r#"
    // [{"_key":"24383_72810","data":[],"visible":true,"generic_type":"STRU","aabb":{"maxs":[-9247.12890625,-1.14835810546875e+4,4653],"mins":[-9814.478515625,-1.22652236328125e+4,4553]},"world_transform":[[0.212630033493042,-0.6743800640106201,0.6743800640106201,-0.21263009309768677],[-9787.6103515625,-1.14922998046875e+4,4603],[1,1,1]],"ptset_map":{},"flow_pt_indexs":[null,null]}]
    // "#;
    // let data: Vec<EleGeosInfo>  = serde_json::from_str(&json).unwrap();
    // dbg!(&data);

    let _json = r#"
    {
  "result": [
    {
      "_key": "24383_72809",
      "data": [
        {
          "aabb": {
            "maxs": [
              32.79999923706055,
              50,
              920.5880126953125
            ],
            "mins": [
              -15.200000762939453,
              -50,
              0
            ]
          },
          "geo_hash": "10994492164744429269",
          "geo_param": "Unknown",
          "is_tubi": false,
          "pts": [],
          "refno": "24383/72809",
          "transform": [
            [
              0,
              0,
              0,
              1
            ],
            [
              0,
              0,
              0
            ],
            [
              1,
              1,
              920.5880126953125
            ]
          ],
          "visible": true
        }
      ],
      "visible": true,
      "generic_type": "STRU",
      "aabb": {
        "maxs": [
          -9542.0185546875,
          -11690.072265625,
          4653
        ],
        "mins": [
          -10109.3681640625,
          -12471.703125,
          4553
        ]
      },
      "world_transform": [
        [
          0.21263228356838226,
          -0.6743793487548828,
          0.6743793487548828,
          -0.21263234317302704
        ],
        [
          -10082.5,
          -11698.7900390625,
          4603
        ],
        [
          1,
          1,
          1
        ]
      ],
      "ptset_map": {},
      "flow_pt_indexs": [
        null,
        null
      ]
    }
  ],
  "hasMore": false,
  "cached": false,
  "extra": {
    "warnings": [],
    "stats": {
      "writesExecuted": 0,
      "writesIgnored": 0,
      "scannedFull": 0,
      "scannedIndex": 1,
      "cursorsCreated": 1,
      "cursorsRearmed": 0,
      "cacheHits": 1,
      "cacheMisses": 0,
      "filtered": 0,
      "httpRequests": 0,
      "executionTime": 0.0026717909786384553,
      "peakMemoryUsage": 65536
    }
  },
  "error": false,
  "code": 201
}
    "#;
    // let data: Response<Cursor<EleGeosInfo>>  = serde_json::from_str(json).unwrap();
    // dbg!(&data);
}

pub trait PdmsNodeTrait {
    #[inline]
    fn get_refno(&self) -> RefU64 {
        RefU64::default()
    }

    #[inline]
    fn get_name(&self) -> &str {
        ""
    }

    #[inline]
    fn get_noun_hash(&self) -> u32 {
        0
    }

    #[inline]
    fn get_type_name(&self) -> &str {
        ""
    }

    #[inline]
    fn get_children_count(&self) -> usize {
        0
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct EleTreeNode {
    pub refno: RefU64,
    pub noun: String,
    pub name: String,
    pub owner: RefU64,
    pub children_count: usize,
}

impl EleTreeNode {
    pub fn new(
        refno: RefU64,
        noun: String,
        name: String,
        owner: RefU64,
        children_count: usize,
    ) -> Self {
        Self {
            refno,
            noun,
            name,
            owner,
            children_count,
        }
    }
}

impl Into<PdmsElement> for EleTreeNode {
    fn into(self) -> PdmsElement {
        PdmsElement {
            refno: self.refno,
            owner: self.owner,
            name: self.name,
            noun: self.noun,
            version: 0,
            children_count: self.children_count,
        }
    }
}

impl PdmsNodeTrait for EleTreeNode {
    #[inline]
    fn get_refno(&self) -> RefU64 {
        self.refno
    }

    #[inline]
    fn get_name(&self) -> &str {
        self.name.as_str()
    }

    #[inline]
    fn get_noun_hash(&self) -> u32 {
        db1_hash(&self.noun.to_uppercase())
    }

    #[inline]
    fn get_type_name(&self) -> &str {
        self.noun.as_str()
    }

    #[inline]
    fn get_children_count(&self) -> usize {
        self.children_count
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct EleNode {
    pub refno: RefU64,
    pub owner: RefU64,
    pub name_hash: AiosStrHash,
    // pub name: AiosStr,
    pub noun: u32,
    pub version: u32,
    // pub children_count: usize,
    pub children_count: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ChildrenNode {
    pub refno: RefU64,
    pub name: String,
    pub noun: String,
}

#[serde_as]
#[derive(Serialize, Deserialize, Clone, Debug, Default, Component)]
pub struct CataHashRefnoKV {
    #[serde(default)]
    pub cata_hash: String,
    #[serde(default)]
    pub exist_geo: Option<EleInstGeosData>,
    #[serde_as(as = "Vec<DisplayFromStr>")]
    #[serde(default)]
    pub group_refnos: Vec<RefU64>,
}

#[serde_as]
#[derive(Serialize, Deserialize, Clone, Debug, Default, Eq, PartialEq, Component)]
pub struct PdmsElement {
    #[serde_as(as = "DisplayFromStr")]
    #[serde(rename = "_key")]
    pub refno: RefU64,
    #[serde_as(as = "DisplayFromStr")]
    pub owner: RefU64,
    pub name: String,
    pub noun: String,
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub children_count: usize,
}

impl PdmsNodeTrait for PdmsElement {
    #[inline]
    fn get_refno(&self) -> RefU64 {
        self.refno
    }

    #[inline]
    fn get_name(&self) -> &str {
        self.name.as_str()
    }

    #[inline]
    fn get_noun_hash(&self) -> u32 {
        db1_hash(&self.noun.to_uppercase())
    }

    #[inline]
    fn get_type_name(&self) -> &str {
        self.noun.as_str()
    }

    #[inline]
    fn get_children_count(&self) -> usize {
        self.children_count
    }
}

impl PdmsElement {
    pub fn get_enso_headers() -> Vec<String> {
        vec![
            "refno".to_string(),
            "owner".to_string(),
            "name".to_string(),
            "noun".to_string(),
            "version".to_string(),
            "children_count".to_string(),
        ]
    }

    pub fn into_enso_value_json(self) -> Vec<NamedAttrValue> {
        vec![
            NamedAttrValue::StringType(self.refno.to_string()),
            NamedAttrValue::StringType(self.owner.to_string()),
            NamedAttrValue::StringType(self.name),
            NamedAttrValue::StringType(self.noun),
            NamedAttrValue::IntegerType(self.version as i32),
            NamedAttrValue::IntegerType(self.children_count as i32),
        ]
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, Deref, DerefMut)]
pub struct PdmsElementVec(pub Vec<PdmsElement>);

impl BytesTrait for PdmsElementVec {}

impl EleNode {
    pub fn set_default_name(name_hash: AiosStrHash) -> EleNode {
        EleNode {
            name_hash,
            ..Default::default()
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PdmsNodeId(pub NodeId);

/// 每个dbno对应的version
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct DbnoVersion {
    pub dbno: u32,
    pub version: u32,
}

#[test]
fn test_dashmap() {
    let dashmap_1 = DashMap::new();
    dashmap_1.insert("1", "hello");
    let dashmap_2 = DashMap::new();
    dashmap_2.insert("2", "world");
    let dashmap_3 = DashMap::new();
    dashmap_1.iter().for_each(|m| {
        dashmap_3.insert(m.key().clone(), m.value().clone());
    });
    dashmap_2.iter().for_each(|m| {
        dashmap_3.insert(m.key().clone(), m.value().clone());
    });
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum DbAttributeType {
    #[default]
    Unknown,
    INTEGER = 1,
    DOUBLE,
    BOOL,
    STRING,
    ELEMENT,
    WORD,
    DIRECTION,
    POSITION,
    ORIENTATION,
    DATETIME,

    //todo remove these
    DOUBLEVEC,
    INTVEC,
    FLOATVEC,
    TYPEX,
    Vec3Type,
    RefU64Vec,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AttrInfo {
    pub name: String,
    pub hash: i32,
    pub offset: u32,
    pub default_val: AttrVal,
    pub att_type: DbAttributeType,
}

impl AttrInfo {
    fn gen_value_schema(&self) -> Option<&str> {
        match self.default_val {
            AttrVal::IntegerType(_) => {
                if self.offset > 0 {
                    Some(
                        r#"
                         "xsd:integer"
                    "#,
                    )
                } else {
                    Some(
                        r#"{
                        "@class": "xsd:integer",
                        "@type": "Optional"
                    }"#,
                    )
                }
            }
            AttrVal::BoolType(_) => {
                if self.offset > 0 {
                    Some(
                        r#"
                        "xsd:boolean"
                    "#,
                    )
                } else {
                    Some(
                        r#"{
                        "@class": "xsd:boolean",
                        "@type": "Optional"
                    }"#,
                    )
                }
            }
            AttrVal::DoubleType(_) => {
                if self.offset > 0 {
                    Some(
                        r#"
                        "xsd:decimal"
                    "#,
                    )
                } else {
                    Some(
                        r#"{
                        "@class": "xsd:decimal",
                        "@type": "Optional"
                    }"#,
                    )
                }
            }
            //element 暂时为string，需要转换成type class
            AttrVal::StringType(_)
            | AttrVal::RefU64Type(_)
            | AttrVal::WordType(_)
            | AttrVal::ElementType(_) => {
                if self.offset > 0 {
                    Some(
                        r#"
                        "xsd:string"
                    "#,
                    )
                } else {
                    Some(
                        r#"{
                        "@class": "xsd:string",
                        "@type": "Optional"
                    }"#,
                    )
                }
            }
            AttrVal::DoubleArrayType(_) | AttrVal::Vec3Type(_) => Some(
                r#"{
                        "@class": "xsd:decimal",
                        "@type": "Array"
                    }"#,
            ),
            AttrVal::IntArrayType(_) => Some(
                r#"
                     {
                        "@class": "xsd:integer",
                        "@type": "Array"
                     }
                "#,
            ),
            AttrVal::BoolArrayType(_) => Some(
                r#"
                     {
                        "@class": "xsd:boolean",
                        "@type": "Array"
                     }
                "#,
            ),

            // DbAttributeType::ELEMENT => {
            //     Some(r#"{
            //             "@class": "xsd:string",
            //             "@type": "Optional"
            //         }"#)
            // }
            _ => None,
        }
    }

    pub fn gen_schema(&self) -> Option<String> {
        if let Some(s) = self.gen_value_schema() {
            let name = db1_dehash(self.hash as _);
            Some(format!(r#""{}": {}"#, &name, s))
        } else {
            None
        }
    }

    ///需要考虑
    pub fn gen_schema_old(&self) -> Option<String> {
        match self.att_type {
            DbAttributeType::INTEGER => Some(format!(r#""{}": "xsd:integer""#, &self.name)),
            DbAttributeType::DOUBLE => Some(format!(r#""{}": "xsd:decimal""#, &self.name)),
            DbAttributeType::BOOL => Some(format!(r#""{}": "xsd:bool""#, &self.name)),
            DbAttributeType::STRING | DbAttributeType::TYPEX | DbAttributeType::WORD => {
                Some(format!(r#""{}": "xsd:string""#, &self.name))
            }
            DbAttributeType::DIRECTION
            | DbAttributeType::POSITION
            | DbAttributeType::ORIENTATION
            | DbAttributeType::DOUBLEVEC
            | DbAttributeType::FLOATVEC
            | DbAttributeType::Vec3Type => Some(format!(
                r#""{}":
                     {{
                        "@class": "xsd:decimal",
                        "@type": "Array"
                    }}
                "#,
                &self.name
            )),
            DbAttributeType::INTVEC => Some(format!(
                r#""{}":
                     {{
                        "@class": "xsd:integer",
                        "@type": "Array"
                    }}
                "#,
                &self.name
            )),
            DbAttributeType::ELEMENT => Some(format!(r#""{}": "xsd:string""#, &self.name)),
            // DbAttributeType::RefU64Vec => {}
            _ => None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PDMSDBInfo {
    pub name: String,
    pub db_no: i32,
    pub db_type: String,
    pub version: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PdmsRefno {
    pub ref_no: String,
    pub db: String,
    pub type_name: String,
}

pub type AiosStrHash = u32;

#[derive(
    Debug,
    Clone,
    Default,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    Hash,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub struct AiosStr(pub String);

impl AiosStr {
    #[inline]
    pub fn get_u32_hash(&self) -> u32 {
        use hash32::{FnvHasher, Hasher};
        use std::hash::Hash;
        let mut fnv = FnvHasher::default();
        self.hash(&mut fnv);
        fnv.finish32()
    }
    pub fn take(self) -> String {
        self.0
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl Deref for AiosStr {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefnoNodeId {
    pub refno: u64,
    //  参考号对应的小版本
    pub version: u32,
    // 参考号在树中对应的nodeId
    pub node_id: NodeId,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, Component)]
pub struct ProjectDbno {
    pub mdb: u32,
    pub main_db: u32,
    // 每个模块（DESI,CATA .. ）对应得dbno
    pub dbs: HashMap<String, Vec<u32>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TxXt {
    pub map: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct YkGd {
    pub map: HashMap<String, String>,
}

/// 每种 type 对应的所有 uda name 和 default value
#[derive(Debug, Serialize, Deserialize)]
pub struct Uda {
    pub reference_type: String,
    pub data: Vec<(String, String)>,
}

/// 数据状态对应的数据结构
#[derive(Default, Clone, Debug, Serialize, Deserialize, Component)]
pub struct DataState {
    pub refno: RefU64,
    pub att_type: String,
    pub name: String,
    pub state: String,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize, Component)]
pub struct DataStateVec {
    pub data_states: Vec<DataState>,
}

/// 数据状态需要显示的pdms属性
#[derive(Default, Clone, Debug, Serialize, Deserialize, Component)]
pub struct DataScope {
    pub refno: RefU64,
    pub att_type: String,
    pub name: String,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize, Component)]
pub struct DataScopeVec {
    pub data_scopes: Vec<DataScope>,
}

unsafe impl Send for DataScopeVec {}

unsafe impl Sync for DataScopeVec {}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct IncrementDataSql {
    pub id: String,
    pub refno: RefU64,
    pub operate: EleOperation,
    pub version: u32,
    pub user: String,
    pub old_data: AttrMap,
    pub new_data: AttrMap,
    pub time: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum UdaMajorType {
    /// 工艺
    T,
    /// 通风
    V,
    /// 电气
    E,
    /// 仪控
    I,
    /// 核岛水工
    W,
    /// BOP-暖通
    N,
    /// BOP-水工
    Z,
    /// 通信
    K,
    /// 设备
    S,
    /// 照明
    L,
    /// 辐射安全
    F,
    /// 反应堆热工水力
    H,
    /// 辐射监测
    R,
    /// 建筑
    A,
    /// 结构
    J,
    /// NPIC管道
    P,
    /// NPIC设备
    B,
    /// NPIC电气
    C,
    /// NPIC仪表
    Y,
    /// 多专业
    X,

    NULL,
}

impl UdaMajorType {
    pub fn from_str(input: &str) -> Self {
        match input.to_uppercase().as_str() {
            "T" => Self::T,
            "V" => Self::V,
            "E" => Self::E,
            "I" => Self::I,
            "W" => Self::W,
            "N" => Self::N,
            "Z" => Self::Z,
            "K" => Self::K,
            "S" => Self::S,
            _ => Self::NULL,
        }
    }

    pub fn to_major_str(&self) -> String {
        match self {
            UdaMajorType::T => "T".to_string(),
            UdaMajorType::V => "V".to_string(),
            UdaMajorType::E => "E".to_string(),
            UdaMajorType::I => "I".to_string(),
            UdaMajorType::W => "W".to_string(),
            UdaMajorType::N => "N".to_string(),
            UdaMajorType::Z => "Z".to_string(),
            UdaMajorType::K => "K".to_string(),
            UdaMajorType::S => "S".to_string(),
            UdaMajorType::L => "L".to_string(),
            UdaMajorType::F => "F".to_string(),
            UdaMajorType::H => "H".to_string(),
            UdaMajorType::R => "R".to_string(),
            UdaMajorType::A => "A".to_string(),
            UdaMajorType::J => "J".to_string(),
            UdaMajorType::P => "P".to_string(),
            UdaMajorType::B => "B".to_string(),
            UdaMajorType::C => "C".to_string(),
            UdaMajorType::Y => "Y".to_string(),
            UdaMajorType::X => "X".to_string(),
            UdaMajorType::NULL => "NULL".to_string(),
        }
    }

    pub fn to_chinese_name(&self) -> String {
        match self {
            UdaMajorType::T => "工艺".to_string(),
            UdaMajorType::V => "通风".to_string(),
            UdaMajorType::E => "电气".to_string(),
            UdaMajorType::I => "仪控".to_string(),
            UdaMajorType::W => "给排水".to_string(),
            UdaMajorType::N => "BOP暖".to_string(),
            UdaMajorType::Z => "BOP水".to_string(),
            UdaMajorType::K => "通信".to_string(),
            UdaMajorType::S => "设备".to_string(),
            UdaMajorType::L => "照明".to_string(),
            UdaMajorType::F => "辐射安全".to_string(),
            UdaMajorType::H => "反应堆热工水力".to_string(),
            UdaMajorType::R => "辐射监测".to_string(),
            UdaMajorType::A => "建筑".to_string(),
            UdaMajorType::J => "结构".to_string(),
            UdaMajorType::P => "NPIC管道".to_string(),
            UdaMajorType::B => "NPIC设备".to_string(),
            UdaMajorType::C => "NPIC电气".to_string(),
            UdaMajorType::Y => "NPIC仪表".to_string(),
            UdaMajorType::X => "多专业".to_string(),
            UdaMajorType::NULL => "未知".to_string(),
        }
    }

    pub fn from_chinese_description(input: &str) -> Self {
        match input {
            "管道" | "工艺" => Self::T,
            "电气" => Self::E,
            "设备" => Self::S,
            "通风" => Self::V,
            "仪控" => Self::I,
            "照明" => Self::L,
            "通信" => Self::K,
            "给排水" => Self::W,
            "暖通" => Self::N,
            "辐射安全" => Self::F,
            "反应堆热工水力" => Self::H,
            "辐射监测" => Self::R,
            "建筑" => Self::A,
            "结构" => Self::J,
            "BOP水" => Self::Z,
            "BOP暖" => Self::N,
            "NPIC管道" => Self::P,
            "NPIC设备" => Self::B,
            "NPIC电气" => Self::C,
            "NPIC仪表" => Self::Y,
            "多专业" => Self::X,
            _ => Self::NULL,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PdmsAttrArangodb {
    pub _key: String,
    #[serde(flatten)]
    pub map: HashMap<String, AttrValAql>,
}

/// 参考号属于哪个房间
#[serde_as]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PdmsNodeBelongRoomName {
    #[serde_as(as = "DisplayFromStr")]
    pub refno: RefU64,
    pub room_name: String,
}

#[serde_as]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PdmsNameBelongRoomName {
    #[serde_as(as = "DisplayFromStr")]
    pub refno: RefU64,
    #[serde_as(as = "DisplayFromStr")]
    pub name: String,
    pub room_name: String,
}

/// 房间下的所有节点
#[serde_as]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RoomNodes {
    #[serde_as(as = "DisplayFromStr")]
    pub room_name: String,
    pub nodes: Vec<String>,
}

#[derive(PartialEq, Debug, Default, Clone, Copy, Serialize, Deserialize)]
pub enum EleOperation {
    #[default]
    None,
    Add,
    Modified,
    Deleted,
}

impl EleOperation {
    pub fn into_tidb_num(&self) -> u8 {
        match &self {
            EleOperation::None => 0,
            EleOperation::Add => 1,
            EleOperation::Modified => 2,
            EleOperation::Deleted => 3,
        }
    }
}

impl From<i32> for EleOperation {
    fn from(v: i32) -> Self {
        match v {
            1 => Self::Add,
            2 => Self::Modified,
            3 => Self::Deleted,
            _ => Self::None,
        }
    }
}

impl ToString for EleOperation {
    fn to_string(&self) -> String {
        match &self {
            Self::None => "Unknown".to_string(),
            EleOperation::Add => "增加".to_string(),
            EleOperation::Modified => "修改".to_string(),
            EleOperation::Deleted => "删除".to_string(),
        }
    }
}
