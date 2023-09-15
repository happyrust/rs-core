use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::tool::hash_tool::*;
use anyhow::anyhow;
use bevy_ecs::prelude::*;
use bitflags::bitflags;

use dashmap::DashMap;
use derive_more::{Deref, DerefMut};
use glam::{Affine3A, DVec3, Mat4, Quat, Vec3, Vec4};
use id_tree::{NodeId, Tree};
use indexmap::IndexMap;
use itertools::Itertools;

use parry3d::bounding_volume::Aabb;

use parry3d::shape::SharedShape;
use rkyv::with::Skip;

use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use serde_with::{serde_as, DisplayFromStr};

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use std::fmt::{Debug, Display, Formatter, Pointer};
use std::fs::File;
use std::io::{Read, Write};
use std::ops::{Deref, DerefMut};
use std::path::Path;

use std::str::FromStr;
use std::vec::IntoIter;
use std::{fmt, hash};

use crate::cache::mgr::BytesTrait;

use crate::consts::*;
use crate::consts::{ATT_CURD, UNSET_STR};
use crate::parsed_data::CateAxisParam;
use crate::pdms_data::NewDataOperate;
use crate::pdms_types::AttrVal::*;
use crate::prim_geo::ctorus::CTorus;
use crate::prim_geo::cylinder::SCylinder;
use crate::prim_geo::dish::Dish;
use crate::prim_geo::pyramid::Pyramid;
use crate::prim_geo::rtorus::RTorus;
use crate::prim_geo::sbox::SBox;
use crate::prim_geo::snout::LSnout;
use crate::prim_geo::sphere::Sphere;

use crate::tool::db_tool::{db1_dehash, db1_hash, db1_hash_i32};
use crate::tool::float_tool::{hash_f32, hash_f64_slice};
use crate::tool::math_tool::*;
use crate::BHashMap;

use bevy_math::*;
use bevy_reflect::Reflect;
#[cfg(feature = "render")]
use bevy_render::mesh::Indices;
#[cfg(feature = "render")]
use bevy_render::render_resource::PrimitiveTopology::TriangleList;
use bevy_transform::prelude::*;
use clap::builder::Str;

///控制pdms显示的深度层级
pub const LEVEL_VISBLE: u32 = 6;

///非负实体基本体的种类
pub const PRIMITIVE_NOUN_NAMES: [&'static str; 9] = [
    "BOX", "CYLI", "SLCY", "SPHE", "CONE", "DISH", "CTOR", "RTOR", "PYRA",
];

///基本体的种类(包含负实体)
//"SPINE", "GENS",
pub const GNERAL_PRIM_NOUN_NAMES: [&'static str; 22] = [
    "BOX", "CYLI", "SLCY", "SPHE", "CONE", "DISH", "CTOR", "RTOR", "PYRA", "SNOU", "POHE", "NBOX",
    "NCYL", "NSBO", "NCON", "NSNO", "NPYR", "NDIS", "NCTO", "NRTO", "NSLC", "NSCY",
];

///有loop的几何体
pub const GNERAL_LOOP_NOUN_NAMES: [&'static str; 2] = ["PLOO", "LOOP"];

///负实体基本体的种类
pub const GENRAL_NEG_NOUN_NAMES: [&'static str; 14] = [
    "NBOX", "NCYL", "NLCY", "NSBO", "NCON", "NSNO", "NPYR", "NDIS", "NXTR", "NCTO", "NRTO", "NSLC",
    "NREV", "NSCY",
];

///元件库的负实体类型
pub const CATE_NEG_NOUN_NAMES: [&'static str; 13] = [
    "NSBO", "NSCO", "NLSN", "NSSP", "NLCY", "NSCY", "NSCT", "NSRT", "NSDS", "NSSL", "NLPY", "NSEX",
    "NSRE",
];

pub const TOTAL_NEG_NOUN_NAMES: [&'static str; 27] = [
    "NBOX", "NCYL", "NLCY", "NSBO", "NCON", "NSNO", "NPYR", "NDIS", "NXTR", "NCTO", "NRTO", "NSLC",
    "NREV", "NSCY", "NSBO", "NSCO", "NLSN", "NSSP", "NLCY", "NSCY", "NSCT", "NSRT", "NSDS", "NSSL",
    "NLPY", "NSEX", "NSRE",
];

pub const GENRAL_POS_NOUN_NAMES: [&'static str; 26] = [
    "BOX", "CYLI", "SLCY", "SPHE", "CONE", "DISH", "CTOR", "RTOR", "PYRA", "SNOU", "FLOOR",
    "PANEL", "SBOX", "SCYL", "LCYL", "SSPH", "LCYL", "SCON", "LSNO", "LPYR", "SDSH", "SCTO",
    "SEXT", "SREV", "SRTO", "SSLC",
];

pub const TOTAL_GEO_NOUN_NAMES: [&'static str; 42] = [
    "BOX", "CYLI", "SLCY", "SPHE", "CONE", "DISH", "CTOR", "RTOR", "PYRA", "SNOU", "PLOO", "LOOP",
    "POHE", "SBOX", "SCYL", "SSPH", "LCYL", "SCON", "LSNO", "LPYR", "SDSH", "SCTO", "SEXT", "SREV",
    "SRTO", "SSLC", "SPRO", "SREC", "NBOX", "NCYL", "NLCY", "NSBO", "NCON", "NSNO", "NPYR", "NDIS",
    "NXTR", "NCTO", "NRTO", "NSLC", "NREV", "NSCY",
];

pub const TOTAL_CATA_GEO_NOUN_NAMES: [&'static str; 30] = [
    "SBOX", "SCYL", "SSPH", "LCYL", "SCON", "LSNO", "LPYR", "SDSH", "SCTO", "SEXT", "SREV", "SRTO",
    "SSLC", "SPRO", "SANN", "BOXI", "SREC", "NSBO", "NSCO", "NLSN", "NSSP", "NLCY", "NSCY", "NSCT",
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
pub const CATA_SINGLE_REUSE_GEO_NAMES: [&'static str; 3] = ["STWALL", "SCTN", "NOZZ"];

pub const CATA_WITHOUT_REUSE_GEO_NAMES: [&'static str; 21] = [
    "ELCONN", "CMPF", "WALL", "GWALL", "SJOI", "FITT", "PFIT", "FIXING", "PJOI", "GENSEC", "RNODE",
    "PRTELE", "GPART", "SCREED", "PALJ", "CABLE", "BATT", "CMFI", "SCOJ", "SEVE", "SBFI",
];

#[derive(Serialize, Deserialize, Clone, Debug, Default, Copy, Eq, PartialEq, Hash)]
pub enum SjusType {
    #[default]
    UNSET,
    UTOP,
    UBOT,
    UCEN,
}

///pdms的参考号
#[derive(Serialize, Deserialize, Clone, Debug, Default, Copy, Eq, PartialEq, Hash)]
pub struct RefI32Tuple(pub (i32, i32));
use std::string::String;
impl Into<String> for RefI32Tuple {
    fn into(self) -> String {
        String::from(format!("{}/{}", self.get_0(), self.get_1()))
    }
}

impl From<&[u8]> for RefI32Tuple {
    fn from(input: &[u8]) -> Self {
        Self::new(
            i32::from_be_bytes(input[0..4].try_into().unwrap()),
            i32::from_be_bytes(input[4..8].try_into().unwrap()),
        )
    }
}

impl From<&str> for RefI32Tuple {
    fn from(s: &str) -> Self {
        let x: Vec<i32> = s
            .split('/')
            .map(|x| x.parse::<i32>().unwrap_or_default())
            .collect();
        Self::new(x[0], x[1])
    }
}

impl From<&RefU64> for RefI32Tuple {
    fn from(n: &RefU64) -> Self {
        let n = n.0.to_be_bytes();
        Self((
            i32::from_be_bytes(n[..4].try_into().unwrap()),
            i32::from_be_bytes(n[4..].try_into().unwrap()),
        ))
    }
}

impl RefI32Tuple {
    #[inline]
    pub fn new(ref_0: i32, ref_1: i32) -> Self {
        Self { 0: (ref_0, ref_1) }
    }

    #[inline]
    pub fn get_0(&self) -> i32 {
        self.0 .0
    }

    #[inline]
    pub fn get_1(&self) -> i32 {
        self.0 .1
    }
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

#[derive(Debug, PartialEq, Eq, derive_more::Display)]
pub struct ParseRefU64Error;

//把Refno当作u64
#[derive(
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
    Hash,
    Serialize,
    Deserialize,
    Clone,
    Copy,
    Default,
    Component,
    Eq,
    PartialEq,
    PartialOrd,
    Ord,
)]
pub struct RefU64(pub u64);

impl FromStr for RefU64 {
    type Err = ParseRefU64Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = if s.starts_with("=") {
            s[1..].to_string()
        } else {
            s.to_string()
        };
        if s.contains('_') {
            Self::from_url_refno(&s).ok_or(ParseRefU64Error)
        } else if s.contains('/') {
            Self::from_refno_str(&s).map_err(|_| ParseRefU64Error)
        } else {
            let d = s.parse::<u64>().map_err(|_| ParseRefU64Error)?;
            Ok(Self(d))
        }
    }
}

impl From<&str> for RefU64 {
    fn from(s: &str) -> Self {
        Self::from_str(s).unwrap_or_default()
    }
}

impl hash::Hash for ArchivedRefU64 {
    #[inline]
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl PartialEq for ArchivedRefU64 {
    fn eq(&self, other: &ArchivedRefU64) -> bool {
        self.0 == other.0
    }
}

impl Eq for ArchivedRefU64 {}

impl Deref for RefU64 {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Debug for RefU64 {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.to_refno_str().as_str())
    }
}

impl From<u64> for RefU64 {
    fn from(d: u64) -> Self {
        Self(d)
    }
}

impl From<&RefI32Tuple> for RefU64 {
    fn from(n: &RefI32Tuple) -> Self {
        let bytes: Vec<u8> = [n.get_0().to_be_bytes(), n.get_1().to_be_bytes()].concat();
        let v = u64::from_be_bytes(bytes[..8].try_into().unwrap());
        Self(v)
    }
}

impl From<RefI32Tuple> for RefU64 {
    fn from(n: RefI32Tuple) -> Self {
        let bytes: Vec<u8> = [n.get_0().to_be_bytes(), n.get_1().to_be_bytes()].concat();
        let v = u64::from_be_bytes(bytes[..8].try_into().unwrap());
        Self(v)
    }
}

impl From<&[u8]> for RefU64 {
    fn from(input: &[u8]) -> Self {
        Self(u64::from_be_bytes(input[..8].try_into().unwrap()))
    }
}

impl Into<Vec<u8>> for RefU64 {
    fn into(self) -> Vec<u8> {
        self.0.to_be_bytes().to_vec().into()
    }
}

impl BytesTrait for RefU64 {
    fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        Ok(self.0.to_be_bytes().to_vec().into())
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        Ok(Self(u64::from_be_bytes(bytes[..8].try_into()?)))
    }
}

impl Display for RefU64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let refno: RefI32Tuple = self.into();
        write!(f, "{}_{}", refno.get_0(), refno.get_1())
    }
}

impl RefU64 {
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.get_0() > 0 && self.get_1() > 0
    }

    #[inline]
    pub fn get_sled_key(&self) -> [u8; 8] {
        self.0.to_be_bytes()
    }

    #[inline]
    pub fn get_0(&self) -> u32 {
        let bytes = self.0.to_be_bytes();
        u32::from_be_bytes(bytes[0..4].try_into().unwrap())
    }

    #[inline]
    pub fn get_1(&self) -> u32 {
        let bytes = self.0.to_be_bytes();
        u32::from_be_bytes(bytes[4..8].try_into().unwrap())
    }

    #[inline]
    pub fn get_u32_hash(&self) -> u32 {
        use hash32::{FnvHasher, Hasher};
        use std::hash::Hash;
        let mut fnv = FnvHasher::default();
        self.hash(&mut fnv);
        fnv.finish32()
    }

    #[inline]
    pub fn to_refno_str(&self) -> String {
        let refno: RefI32Tuple = self.into();
        refno.into()
    }

    #[inline]
    pub fn to_refno_string(&self) -> String {
        let refno: RefI32Tuple = self.into();
        let refno_str: String = refno.into();
        refno_str.to_string()
    }

    #[inline]
    pub fn format_url_name(&self, col: &str) -> String {
        format!("{}/{}", col, self.to_url_refno())
    }

    ///转换成数据库允许的字符串
    #[inline]
    pub fn to_refno_normal_string(&self) -> String {
        self.to_refno_string().replace("/", "_")
    }

    #[inline]
    pub fn from_two_nums(i: u32, j: u32) -> Self {
        let bytes: Vec<u8> = [i.to_be_bytes(), j.to_be_bytes()].concat();
        let v = u64::from_be_bytes(bytes[..8].try_into().unwrap());
        Self(v)
    }

    #[inline]
    pub fn from_refno_string(refno: String) -> anyhow::Result<RefU64> {
        Self::from_refno_str(refno.as_str())
    }

    // abcd/2333
    #[inline]
    pub fn from_refno_str(refno: &str) -> anyhow::Result<RefU64> {
        let refno = if refno.starts_with("=") {
            refno[1..].to_string()
        } else {
            refno.to_string()
        };
        if refno.contains("/") {
            let split_refno = refno.split('/').collect::<Vec<_>>();
            if split_refno.len() != 2 {
                return Err(anyhow!("参考号错误, 没有斜线!".to_string()));
            }
            let refno0: i32 = split_refno[0].parse::<i32>()?;
            let refno1: i32 = split_refno[1].parse::<i32>()?;
            Ok(RefI32Tuple((refno0, refno1)).into())
        } else if refno.contains("_") {
            return match Self::from_url_refno(&refno) {
                None => Err(anyhow!("参考号错误!".to_string())),
                Some(refno) => Ok(refno),
            };
        } else {
            Err(anyhow!("参考号错误!".to_string()))
        }
    }

    #[inline]
    pub fn to_url_refno(&self) -> String {
        let refno: RefI32Tuple = self.into();
        format!("{}_{}", refno.get_0(), refno.get_1())
    }

    #[inline]
    pub fn from_url_refno(refno: &str) -> Option<Self> {
        let strs = refno.split('_').collect::<Vec<_>>();
        if strs.len() < 2 {
            return None;
        }
        let ref0 = strs[0].parse::<i32>();
        let ref1 = strs[1].parse::<i32>();
        if ref0.is_err() || ref1.is_err() {
            return None;
        }
        Some(RefI32Tuple((ref0.unwrap(), ref1.unwrap())).into())
    }

    #[inline]
    pub fn from_url_refno_default(refno: &str) -> Self {
        Self::from_url_refno(refno).unwrap_or_default()
    }

    #[inline]
    pub fn hash_with_another_refno(&self, another_refno: RefU64) -> u64 {
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        std::hash::Hash::hash(&self.0, &mut hash);
        std::hash::Hash::hash(&another_refno.0, &mut hash);
        std::hash::Hasher::finish(&hash)
    }

    #[inline]
    pub fn hash_with_str(&self, input: &str) -> u64 {
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        std::hash::Hash::hash(&self.0, &mut hash);
        std::hash::Hash::hash(&input, &mut hash);
        std::hash::Hasher::finish(&hash)
    }

    #[inline]
    pub fn from_arangodb_refno_str(refno_str: &str) -> Option<Self> {
        let mut refno_str = refno_str.split("/").collect::<Vec<_>>();
        if refno_str.len() <= 1 {
            return None;
        }
        let refno_url = refno_str.remove(1);
        RefU64::from_url_refno(refno_url)
    }

    /// 返回图数据库的id形式 例如 pdms_eles/1232_5445
    pub fn to_arangodb_ids(collection_name: &str, refnos: Vec<RefU64>) -> Vec<String> {
        refnos
            .into_iter()
            .map(|refno| format!("{}/{}", collection_name, refno.to_url_refno()))
            .collect()
    }
}

#[serde_as]
#[derive(
    Serialize,
    Deserialize,
    Clone,
    Debug,
    Default,
    Component,
    Deref,
    DerefMut,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub struct RefU64Vec(pub Vec<RefU64>);

impl BytesTrait for RefU64Vec {}

impl From<Vec<RefU64>> for RefU64Vec {
    fn from(d: Vec<RefU64>) -> Self {
        RefU64Vec(d)
    }
}

impl IntoIterator for RefU64Vec {
    type Item = RefU64;
    type IntoIter = IntoIter<RefU64>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl RefU64Vec {
    #[inline]
    pub fn push(&mut self, v: RefU64) {
        if !self.0.contains(&v) {
            self.0.push(v);
        }
    }
}

pub type NounHash = u32;

//untagged
///新的属性数据结构
#[derive(
    Serialize,
    Deserialize,
    Clone,
    Debug,
    Component,
    Default,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
#[serde(untagged)]
pub enum NamedAttrValue {
    #[default]
    InvalidType,
    IntegerType(i32),
    StringType(String),
    F32Type(f32),
    F32VecType(Vec<f32>),
    // DoubleArrayType(Vec<f64>),
    StringArrayType(Vec<String>),
    BoolArrayType(Vec<bool>),
    IntArrayType(Vec<i32>),
    BoolType(bool),
    // Vec3Type([f64; 3]),
    ElementType(String),
    WordType(String),
    // RefU64Type(RefU64),
    // StringHashType(AiosStrHash),
    // RefU64Array(RefU64Vec),
}

impl From<&AttrVal> for NamedAttrValue {
    fn from(v: &AttrVal) -> Self {
        match v.clone() {
            InvalidType => Self::InvalidType,
            IntegerType(d) => Self::IntegerType(d),
            StringType(d) => Self::StringType(d),
            DoubleType(d) => {
                if d > f32::MAX as f64 {
                    Self::StringType(d.to_string())
                } else {
                    Self::F32Type(d as f32)
                }
            }
            DoubleArrayType(d) => Self::F32VecType(d.into_iter().map(|x| x as f32).collect()),
            StringArrayType(d) => Self::StringArrayType(d),
            BoolArrayType(d) => Self::BoolArrayType(d),
            IntArrayType(d) => Self::IntArrayType(d),
            BoolType(d) => Self::BoolType(d),
            Vec3Type(d) => Self::F32VecType(d.into_iter().map(|x| x as f32).collect()),
            ElementType(d) => Self::ElementType(d),
            WordType(d) => Self::ElementType(d),
            RefU64Type(d) => Self::StringType(d.to_url_refno()),
            StringHashType(d) => Self::IntegerType(d as i32),
            RefU64Array(d) => {
                Self::StringArrayType(d.into_iter().map(|x| x.to_url_refno()).collect())
            }
        }
    }
}

///带名称的属性map
#[derive(
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
    Serialize,
    Deserialize,
    Deref,
    DerefMut,
    Clone,
    Default,
    Component,
)]
pub struct NamedAttrMap {
    #[serde(flatten)]
    pub map: BTreeMap<String, NamedAttrValue>,
}

impl From<&AttrMap> for NamedAttrMap {
    fn from(v: &AttrMap) -> Self {
        Self {
            map: v
                .map
                .iter()
                .map(|(h, v)| (db1_dehash(*h), NamedAttrValue::from(v)))
                .collect(),
        }
    }
}

///PDMS的属性数据Map
#[derive(
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
    Serialize,
    Deserialize,
    Deref,
    DerefMut,
    Clone,
    Default,
    Component,
)]
pub struct AttrMap {
    pub map: BHashMap<NounHash, AttrVal>,
}

impl Debug for AttrMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = self.to_string_hashmap();
        s.fmt(f)
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl BytesTrait for AttrMap {}

impl AttrMap {
    ///是否为负实体
    #[inline]
    pub fn is_neg(&self) -> bool {
        TOTAL_NEG_NOUN_NAMES.contains(&self.get_type())
    }

    ///是否为正实体
    #[inline]
    pub fn is_pos(&self) -> bool {
        GENRAL_POS_NOUN_NAMES.contains(&self.get_type())
    }

    ///是否为空
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.map.len() == 0
    }

    ///序列化成bincode
    #[inline]
    pub fn into_bincode_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }

    ///从bincode反序列化
    #[inline]
    pub fn from_bincode_bytes(bytes: &[u8]) -> Option<Self> {
        bincode::deserialize(bytes).ok()
    }

    #[inline]
    pub fn into_rkyv_bytes(&self) -> Vec<u8> {
        rkyv::to_bytes::<_, 1024>(self).unwrap().to_vec()
    }

    #[inline]
    pub fn into_rkyv_compress_bytes(&self) -> Vec<u8> {
        use flate2::write::DeflateEncoder;
        use flate2::Compression;
        let mut e = DeflateEncoder::new(Vec::new(), Compression::default());
        e.write_all(&self.into_rkyv_bytes());
        e.finish().unwrap_or_default()
    }

    #[inline]
    pub fn from_rkyv_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        use rkyv::Deserialize;
        let archived = unsafe { rkyv::archived_root::<Self>(bytes) };
        let r: Self = archived.deserialize(&mut rkyv::Infallible)?;
        Ok(r)
    }

    #[inline]
    pub fn from_rkvy_compress_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        use flate2::write::DeflateDecoder;
        let writer = Vec::new();
        let mut deflater = DeflateDecoder::new(writer);
        deflater.write_all(bytes)?;
        Self::from_rkyv_bytes(&deflater.finish()?)
    }

    #[inline]
    pub fn into_compress_bytes(&self) -> Vec<u8> {
        use flate2::write::DeflateEncoder;
        use flate2::Compression;
        let mut e = DeflateEncoder::new(Vec::new(), Compression::default());
        e.write_all(&self.into_bincode_bytes());
        e.finish().unwrap_or_default()
    }

    #[inline]
    pub fn from_compress_bytes(bytes: &[u8]) -> Option<Self> {
        use flate2::write::DeflateDecoder;
        let writer = Vec::new();
        let mut deflater = DeflateDecoder::new(writer);
        deflater.write_all(bytes).ok()?;
        bincode::deserialize(&deflater.finish().ok()?).ok()
    }

    //计算使用元件库的design 元件 hash
    pub fn cal_cata_hash(&self) -> Option<u64> {
        //todo 先只处理spref有值的情况，还需要处理 self.get_as_string("CATA")
        let type_name = self.get_type();
        if CATA_HAS_TUBI_GEO_NAMES.contains(&type_name) {
            return Some(*self.get_refno().unwrap_or_default());
        }
        //由于有ODESP这种，会导致复用出现问题，怎么解决这个问题
        //1、主动去判断是否CataRef是这个类型，即有ODESP这种字段，然后从复用的逻辑排除出来
        //2、解析的时候发现表达式有这些字段，主动去给catref加一个标记位，表示是不能复用的构件
        //3、把相关的数据都做一遍统计，owner、attach
        let ref_name = if type_name == "NOZZ" || type_name == "ELCONN" {
            "CATR"
        } else {
            "SPRE"
        };
        if let Some(spref) = self.get_as_string(ref_name) {
            if spref.starts_with('0') {
                return None;
            }
            if CATA_WITHOUT_REUSE_GEO_NAMES.contains(&type_name) {
                return Some(*self.get_refno().unwrap_or_default());
            }
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            std::hash::Hash::hash(&spref, &mut hasher);
            if let Some(des_para) = self.get_f64_vec("DESP") {
                hash_f64_slice(&des_para, &mut hasher);
            }
            let ref_strs = ["ANGL", "HEIG", "RADI"];
            let key_strs = self.get_as_strings(&ref_strs);
            for (ref_str, key_str) in ref_strs.iter().zip(key_strs) {
                std::hash::Hash::hash(*ref_str, &mut hasher);
                std::hash::Hash::hash(&key_str, &mut hasher);
            }

            //如果是土建模型 "DRNS", "DRNE"
            if let Some(drns) = self.get_as_string("DRNS") &&
                let Some(drne) = self.get_as_string("DRNE") {
                std::hash::Hash::hash(&drns, &mut hasher);
                std::hash::Hash::hash(&drne, &mut hasher);
                let poss = self.get_vec3("POSS").unwrap_or_default();
                let pose = self.get_vec3("POSE").unwrap_or_default();
                let v = (pose - poss).length();
                hash_f32(v, &mut hasher);
            }
            //JUSL is adjus in wire calculation, so here we should make hash unique by jusl
            let jusl = self.get_str_or_default("JUSL");
            std::hash::Hash::hash(jusl, &mut hasher);

            let val = std::hash::Hasher::finish(&hasher);

            return Some(val);
        }
        return None;
    }

    // 返回 DESI 、 CATA .. 等模块值
    pub fn get_db_stype(&self) -> Option<&'static str> {
        let val = self.map.get(&ATT_STYP)?;
        match val {
            AttrVal::IntegerType(v) => Some(match *v {
                1 => "DESI",
                2 => "CATA",
                8 => "DICT",
                _ => "UNSET",
            }),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WholeAttMap {
    pub implicit_attmap: AttrMap,
    pub explicit_attmap: AttrMap,
    pub uda_attmap: AttrMap,
}

impl WholeAttMap {
    pub fn refine(mut self, info_map: &DashMap<i32, AttrInfo>) -> Self {
        for (noun_hash, _v) in self.explicit_attmap.clone().map {
            if let Some(info) = info_map.get(&(noun_hash as i32)) {
                if info.offset > 0 && EXPR_ATT_SET.contains(&(noun_hash as i32)) {
                    let v = self.explicit_attmap.map.remove(&(noun_hash)).unwrap();
                    self.implicit_attmap.insert(noun_hash, v);
                }
            }
        }
        self
    }

    #[inline]
    pub fn into_bincode_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }

    #[inline]
    pub fn into_compress_bytes(&self) -> Vec<u8> {
        use flate2::write::DeflateEncoder;
        use flate2::Compression;
        let mut e = DeflateEncoder::new(Vec::new(), Compression::default());
        e.write_all(&self.into_bincode_bytes());
        e.finish().unwrap_or_default()
    }

    #[inline]
    pub fn from_compress_bytes(bytes: &[u8]) -> Option<Self> {
        use flate2::write::DeflateDecoder;
        let writer = Vec::new();
        let mut deflater = DeflateDecoder::new(writer);
        deflater.write_all(bytes).ok()?;
        // writer = ;
        bincode::deserialize(&deflater.finish().ok()?).ok()
    }

    /// 将隐式属性和显示属性放到一个attrmap中
    #[inline]
    pub fn change_implicit_explicit_into_attr(self) -> AttrMap {
        let mut map = self.implicit_attmap;
        for (k, v) in self.explicit_attmap.map {
            map.insert(k, v);
        }
        map
    }

    pub fn check_two_attr_difference(
        old_attr: WholeAttMap,
        new_attr: WholeAttMap,
    ) -> Vec<DifferenceValue> {
        let implicit_difference =
            get_two_attr_map_difference(old_attr.implicit_attmap, new_attr.implicit_attmap);
        let explicit_difference =
            get_two_attr_map_difference(old_attr.explicit_attmap, new_attr.explicit_attmap);
        [implicit_difference, explicit_difference].concat()
    }
}

fn get_two_attr_map_difference(old_map: AttrMap, mut new_map: AttrMap) -> Vec<DifferenceValue> {
    let mut result = vec![];
    for (k, v) in old_map.map.into_iter() {
        let new_value = new_map.map.remove(&k);
        result.push(DifferenceValue {
            noun: k,
            old_value: Some(v.clone()),
            new_value,
        });
        continue;
    }
    if !new_map.map.is_empty() {
        for (k, v) in new_map.map.into_iter() {
            result.push(DifferenceValue {
                noun: k,
                old_value: None,
                new_value: Some(v),
            })
        }
    }
    result
}

#[derive(Debug, Clone, Default, Deref, DerefMut, Serialize, Deserialize)]
pub struct NameAttrMap {
    pub map: BHashMap<String, AttrVal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifferenceValue {
    pub noun: NounHash,
    pub old_value: Option<AttrVal>,
    // 新增 old_value 为 none
    pub new_value: Option<AttrVal>, // 删除 new_value 为 none
}

// impl Inspectable for AttrMap {
//     type Attributes = ();
//
//     fn ui(
//         &mut self,
//         ui: &mut egui::Ui,
//         _options: Self::Attributes,
//         context: &mut bevy_inspector_egui::Context,
//     ) -> bool {
//         let mut changed = false;
//         ui.vertical_centered(|ui| {
//             egui::Grid::new(context.id()).show(ui, |ui| {
//                 let sort_keys = self.map.keys().cloned().sorted_by_key(|x| db1_dehash(x.0));
//                 //need sort
//                 for sort_key in sort_keys {
//                     ui.label(db1_dehash(sort_key.0));
//                     let v = self.map.get_mut(&sort_key).unwrap();
//                     ui.vertical(|ui| {
//                         changed |= v.ui(ui, Default::default(), context);
//                     });
//                     ui.end_row();
//                 }
//             });
//         });
//         changed
//     }
// }

pub const DEFAULT_NOUNS: [NounHash; 4] = [TYPE_HASH, NAME_HASH, REFNO_HASH, OWNER_HASH];

impl AttrMap {
    pub fn split_to_default_groups(&self) -> (AttrMap, AttrMap) {
        let mut default_att = AttrMap::default();
        let mut comp_att = AttrMap::default();

        for (k, v) in self.map.iter() {
            if DEFAULT_NOUNS.contains(k) {
                default_att.map.insert(k.clone(), v.clone());
            } else {
                comp_att.insert(k.clone(), v.clone());
            }
        }
        (default_att, comp_att)
    }
}

impl AttrMap {
    #[inline]
    pub fn get_att_by_name(&self, name: &str) -> Option<&AttrVal> {
        self.map.get(&db1_hash(name))
    }

    #[inline]
    pub fn insert(&mut self, k: NounHash, v: AttrVal) {
        self.map.insert(k, v);
    }

    #[inline]
    pub fn insert_by_att_name(&mut self, k: &str, v: AttrVal) {
        self.map.insert(db1_hash(k), v);
    }

    #[inline]
    pub fn contains_attr_name(&self, name: &str) -> bool {
        self.map.contains_key(&db1_hash(name))
    }

    #[inline]
    pub fn contains_attr_hash(&self, hash: u32) -> bool {
        self.map.contains_key(&hash)
    }

    pub fn to_string_hashmap(&self) -> BTreeMap<String, String> {
        let mut map = BTreeMap::new();
        for (k, v) in &self.map {
            map.insert(db1_dehash(*k), format!("{:?}", v));
        }
        map
    }

    #[inline]
    pub fn get_name_hash(&self) -> AiosStrHash {
        return if let Some(StringHashType(name_hash)) = self.get_val("NAME") {
            *name_hash
        } else {
            0
        };
    }

    #[inline]
    pub fn get_name_string(&self) -> String {
        return if let Some(StringType(name)) = self.get_val("NAME") {
            name.clone()
        } else {
            Default::default()
        };
    }

    #[inline]
    pub fn get_name(&self) -> Option<String> {
        return if let Some(StringType(name)) = self.get_val("NAME") {
            Some(name.clone())
        } else {
            None
        };
    }

    #[inline]
    pub fn get_main_db_in_mdb(&self) -> Option<RefU64> {
        if let Some(v) = self.map.get(&ATT_CURD) {
            match v {
                AttrVal::IntArrayType(v) => {
                    let refno = RefU64::from_two_nums(v[0] as u32, v[1] as u32);
                    return Some(refno);
                }
                _ => {}
            }
        }
        None
    }

    //获取spref
    #[inline]
    pub fn get_foreign_refno(&self, key: &str) -> Option<RefU64> {
        if let RefU64Type(d) = self.get_val(key)? {
            return Some(*d);
        }
        None
    }

    #[inline]
    pub fn get_refno_as_string(&self) -> Option<String> {
        self.get_as_smol_str("REFNO")
    }

    pub fn get_obstruction(&self) -> Option<u32> {
        self.get_u32("OBST")
    }

    pub fn get_level(&self) -> Option<[u32; 2]> {
        let v = self.get_i32_vec("LEVE")?;
        if v.len() >= 2 {
            return Some([v[0] as u32, v[1] as u32]);
        }
        None
    }

    ///判断构件是否可见
    pub fn is_visible_by_level(&self, level: Option<u32>) -> Option<bool> {
        let levels = self.get_level()?;
        let l = level.unwrap_or(LEVEL_VISBLE);
        Some(levels[0] <= l && l <= levels[1])
    }

    #[inline]
    pub fn get_refno(&self) -> Option<RefU64> {
        if let RefU64Type(d) = self.get_val("REFNO")? {
            return Some(*d);
        }
        None
    }

    #[inline]
    pub fn get_owner(&self) -> Option<RefU64> {
        if let RefU64Type(d) = self.get_val("OWNER")? {
            return Some(*d);
        }
        None
    }

    #[inline]
    pub fn get_owner_as_string(&self) -> String {
        self.get_as_string("OWNER").unwrap_or(UNSET_STR.into())
    }

    #[inline]
    pub fn get_type(&self) -> &str {
        self.get_str("TYPE").unwrap_or("unset")
    }

    #[inline]
    pub fn get_noun(&self) -> i32 {
        db1_hash_i32(self.get_type())
    }

    #[inline]
    pub fn get_typex(&self) -> &str {
        self.get_str("TYPEX").unwrap_or("unset")
    }

    #[inline]
    pub fn is_type(&self, type_name: &str) -> bool {
        self.get_type() == type_name
    }

    #[inline]
    pub fn get_type_cloned(&self) -> Option<String> {
        self.get_str("TYPE").map(|x| x.to_string())
    }

    #[inline]
    pub fn get_u32(&self, key: &str) -> Option<u32> {
        self.get_i32(key).map(|s| s as u32)
    }

    #[inline]
    pub fn get_i32(&self, key: &str) -> Option<i32> {
        let v = self.get_val(key)?;
        match v {
            IntegerType(d) => Some(*d as i32),
            _ => None,
        }
    }

    #[inline]
    pub fn get_refu64(&self, key: &str) -> Option<RefU64> {
        let v = self.get_val(key)?;
        match v {
            RefU64Type(d) => Some(*d),
            _ => None,
        }
    }

    #[inline]
    pub fn get_refu64_vec(&self, key: &str) -> Option<RefU64Vec> {
        let v = self.get_val(key)?;
        match v {
            RefU64Array(d) => Some(d.clone()),
            _ => None,
        }
    }

    #[inline]
    pub fn get_str(&self, key: &str) -> Option<&str> {
        let v = self.get_val(key)?;
        match v {
            StringType(s) | WordType(s) | ElementType(s) => Some(s.as_str()),
            _ => None,
        }
    }

    #[inline]
    pub fn get_str_or_default(&self, key: &str) -> &str {
        self.get_str(key).unwrap_or("unset")
    }

    #[inline]
    pub fn get_as_strings(&self, keys: &[&str]) -> Vec<String> {
        let mut result = vec![];
        for key in keys {
            result.push(self.get_as_string(*key).unwrap_or(UNSET_STR.to_string()));
        }
        result
    }

    #[inline]
    pub fn get_as_string(&self, key: &str) -> Option<String> {
        let v = self.get_val(key)?;
        let s = match v {
            StringType(s) | WordType(s) | ElementType(s) => s.to_string(),
            IntegerType(d) => d.to_string().into(),
            DoubleType(d) => d.to_string().into(),
            BoolType(d) => d.to_string().into(),
            DoubleArrayType(d) => d
                .iter()
                .map(|i| format!(" {:.3}", i))
                .collect::<String>()
                .into(),
            StringArrayType(d) => d
                .iter()
                .map(|i| format!(" {}", i))
                .collect::<String>()
                .into(),
            IntArrayType(d) => d
                .iter()
                .map(|i| format!(" {}", i))
                .collect::<String>()
                .into(),
            BoolArrayType(d) => d
                .iter()
                .map(|i| format!(" {}", i))
                .collect::<String>()
                .into(),
            Vec3Type(d) => d
                .iter()
                .map(|i| format!(" {:.3}", i))
                .collect::<String>()
                .into(),

            RefU64Type(d) => RefI32Tuple::from(d).into(),
            StringHashType(d) => format!("{d}").into(),

            _ => UNSET_STR.into(),
        };
        Some(s)
    }

    #[inline]
    pub fn get_as_smol_str(&self, key: &str) -> Option<String> {
        let v = self.get_val(key)?;
        let s = match v {
            StringType(s) | WordType(s) | ElementType(s) => s.clone(),
            IntegerType(d) => d.to_string().into(),
            DoubleType(d) => d.to_string().into(),
            BoolType(d) => d.to_string().into(),
            DoubleArrayType(d) => d
                .iter()
                .map(|i| format!(" {}", i))
                .collect::<String>()
                .into(),
            StringArrayType(d) => d
                .iter()
                .map(|i| format!(" {}", i))
                .collect::<String>()
                .into(),
            IntArrayType(d) => d
                .iter()
                .map(|i| format!(" {}", i))
                .collect::<String>()
                .into(),
            BoolArrayType(d) => d
                .iter()
                .map(|i| format!(" {}", i))
                .collect::<String>()
                .into(),
            Vec3Type(d) => d
                .iter()
                .map(|i| format!(" {}", i))
                .collect::<String>()
                .into(),

            RefU64Type(d) => RefI32Tuple::from(d).into(),
            StringHashType(d) => format!("{d}").into(),

            _ => UNSET_STR.into(),
        };
        Some(s)
    }

    #[inline]
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        if let AttrVal::BoolType(d) = self.get_val(key)? {
            return Some(*d);
        }
        None
    }

    #[inline]
    pub fn get_val(&self, key: &str) -> Option<&AttrVal> {
        self.map.get(&db1_hash(key).into())
    }

    #[inline]
    pub fn get_f64(&self, key: &str) -> Option<f64> {
        self.get_val(key)?.double_value()
    }

    #[inline]
    pub fn get_f32(&self, key: &str) -> Option<f32> {
        self.get_f64(key).map(|x| x as f32)
    }

    #[inline]
    pub fn get_f32_or_default(&self, key: &str) -> f32 {
        self.get_f64(key).map(|x| x as f32).unwrap_or_default()
    }

    #[inline]
    pub fn get_position(&self) -> Option<Vec3> {
        if let Some(pos) = self.get_f64_vec("POS") {
            return Some(Vec3::new(pos[0] as f32, pos[1] as f32, pos[2] as f32));
        } else {
            //如果没有POS，就以POSS来尝试
            self.get_poss()
        }
    }

    #[inline]
    pub fn get_posse_dist(&self) -> Option<f32> {
        Some(self.get_pose()?.distance(self.get_poss()?))
    }

    #[inline]
    pub fn get_poss(&self) -> Option<Vec3> {
        let pos = self.get_f64_vec("POSS")?;
        if pos.len() == 3 {
            return Some(Vec3::new(pos[0] as f32, pos[1] as f32, pos[2] as f32));
        }
        None
    }

    #[inline]
    pub fn get_pose(&self) -> Option<Vec3> {
        let pos = self.get_f64_vec("POSE")?;
        if pos.len() == 3 {
            return Some(Vec3::new(pos[0] as f32, pos[1] as f32, pos[2] as f32));
        }
        None
    }

    #[inline]
    pub fn get_rotation(&self) -> Option<Quat> {
        let type_name = self.get_type();
        let mut quat = Quat::IDENTITY;
        if self.contains_attr_name("ZDIR") {
            let axis_dir = self.get_vec3("ZDIR").unwrap_or_default().normalize();
            if axis_dir.is_normalized() {
                quat = Quat::from_mat3(&cal_mat3_by_zdir(axis_dir));
            }
        } else if self.contains_attr_name("OPDI") {
            //PJOI 的方向
            let axis_dir = self.get_vec3("OPDI").unwrap_or_default().normalize();
            if axis_dir.is_normalized() {
                quat = Quat::from_mat3(&cal_mat3_by_zdir(axis_dir));
                // dbg!(quat_to_pdms_ori_str(&quat));
            }
        } else {
            match type_name {
                "CMPF" | "PFIT" => {
                    let sjus = self.get_str("SJUS").unwrap_or("unset");
                    //unset 和 UBOT 一样的效果
                    //DTOP, DCEN, DBOT
                    if sjus.starts_with("D") {
                        quat = Quat::from_mat3(&Mat3::from_cols(Vec3::X, Vec3::NEG_Y, Vec3::NEG_Z));
                    }
                }
                _ => {
                    let ang = self.get_f64_vec("ORI")?;
                    let mat = glam::f32::Mat3::from_rotation_z(ang[2].to_radians() as f32)
                        * glam::f32::Mat3::from_rotation_y(ang[1].to_radians() as f32)
                        * glam::f32::Mat3::from_rotation_x(ang[0].to_radians() as f32);

                    quat = Quat::from_mat3(&mat);
                }
            }
        }
        return Some(quat);
    }

    // #[inline]
    // pub fn get_rotation(&self) -> Option<Quat> {
    //     let type_name = self.get_type();
    //     let mut quat = Quat::IDENTITY;
    //
    //     if self.contains_attr_name("SJUS"){
    //         //unset 和 UBOT 一样的效果
    //         //DTOP, DCEN, DBOT
    //         let sjus = self.get_str("SJUS").unwrap_or("unset");
    //         if sjus.starts_with("D") {
    //             quat = Quat::from_mat3(&Mat3::from_cols(
    //                 Vec3::X,
    //                 Vec3::NEG_Y,
    //                 Vec3::NEG_Z,
    //             ));
    //         }
    //     } else if self.contains_attr_name("ZDIR"){
    //         let mut axis_dir = self.get_vec3("ZDIR").unwrap_or_default().normalize();
    //         if axis_dir.is_normalized() {
    //             quat = Quat::from_mat3(&cal_mat3_by_zdir(axis_dir));
    //         }
    //     }else{
    //         let ang = self.get_f64_vec("ORI")?;
    //         let mat = (glam::f32::Mat3::from_rotation_z(ang[2].to_radians() as f32)
    //             * glam::f32::Mat3::from_rotation_y(ang[1].to_radians() as f32)
    //             * glam::f32::Mat3::from_rotation_x(ang[0].to_radians() as f32));
    //
    //         quat = Quat::from_mat3(&mat);
    //     }
    //
    //     return Some(quat);
    // }

    pub fn get_matrix(&self) -> Option<Affine3A> {
        let mut affine = Affine3A::IDENTITY;
        let pos = self.get_f64_vec("POS")?;
        affine.translation = glam::f32::Vec3A::new(pos[0] as f32, pos[1] as f32, pos[2] as f32);
        let ang = self.get_f64_vec("ORI")?;
        affine.matrix3 = glam::f32::Mat3A::from_rotation_z(ang[2].to_radians() as f32)
            * glam::f32::Mat3A::from_rotation_y(ang[1].to_radians() as f32)
            * glam::f32::Mat3A::from_rotation_x(ang[0].to_radians() as f32);
        Some(affine)
    }

    #[inline]
    pub fn get_mat4(&self) -> Option<Mat4> {
        Some(Mat4::from(self.get_matrix()?))
    }

    pub fn get_f64_vec(&self, key: &str) -> Option<Vec<f64>> {
        let val = self.get_val(key)?;
        return match val {
            AttrVal::DoubleArrayType(data) => Some(data.clone()),
            AttrVal::Vec3Type(data) => Some(data.to_vec()),
            _ => None,
        };
    }

    pub fn get_vec3(&self, key: &str) -> Option<Vec3> {
        if let AttrVal::Vec3Type(d) = self.get_val(key)? {
            return Some(Vec3::new(d[0] as f32, d[1] as f32, d[2] as f32));
        }
        None
    }

    pub fn get_dvec3(&self, key: &str) -> Option<DVec3> {
        if let AttrVal::Vec3Type(d) = self.get_val(key)? {
            return Some(DVec3::new(d[0], d[1], d[2]));
        }
        None
    }

    pub fn get_i32_vec(&self, key: &str) -> Option<Vec<i32>> {
        if let AttrVal::IntArrayType(d) = self.get_val(key)? {
            return Some(d.clone());
        }
        None
    }

    ///生成具有几何属性的element的shape
    pub fn create_brep_shape(&self, limit_size: Option<f32>) -> Option<Box<dyn BrepShapeTrait>> {
        let type_noun = self.get_type();
        let mut r: Option<Box<dyn BrepShapeTrait>> = match type_noun {
            "BOX" | "NBOX" => Some(Box::new(SBox::from(self))),
            "CYLI" | "SLCY" | "NCYL" => Some(Box::new(SCylinder::from(self))),
            "SPHE" => Some(Box::new(Sphere::from(self))),
            "CONE" | "NCON" | "SNOU" | "NSNO" => Some(Box::new(LSnout::from(self))),
            "DISH" | "NDIS" => Some(Box::new(Dish::from(self))),
            "CTOR" | "NCTO" => Some(Box::new(CTorus::from(self))),
            "RTOR" | "NRTO" => Some(Box::new(RTorus::from(self))),
            "PYRA" | "NPYR" => Some(Box::new(Pyramid::from(self))),
            _ => None,
        };
        if r.is_some() && limit_size.is_some() {
            r.as_mut().unwrap().apply_limit_by_size(limit_size.unwrap());
        }
        r
    }

    /// 获取string属性数组，忽略为空的值
    pub fn get_attr_strings_without_default(&self, keys: &[&str]) -> Vec<String> {
        let mut results = vec![];
        for &attr_name in keys {
            if let Some(result) = self.get_val(attr_name) {
                match result {
                    AttrVal::StringType(v) => {
                        if v != "" {
                            results.push(v.trim_matches('\0').to_owned().clone().into());
                        }
                    }
                    _ => {}
                }
            }
        }
        results
    }

    pub fn get_attr_strings(&self, keys: &[&str]) -> Vec<String> {
        let mut results = vec![];
        for &attr_name in keys {
            if let Some(result) = self.get_str(attr_name) {
                results.push(result.trim_matches('\0').to_owned().clone().into());
            } else {
                results.push("".to_string());
            }
        }
        results
    }
}

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

#[derive(
    Default,
    Serialize,
    Deserialize,
    Clone,
    Debug,
    Component,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub enum AttrVal {
    #[default]
    InvalidType,
    IntegerType(i32),
    StringType(String),
    DoubleType(f64),
    DoubleArrayType(Vec<f64>),
    StringArrayType(Vec<String>),
    BoolArrayType(Vec<bool>),
    IntArrayType(Vec<i32>),
    BoolType(bool),
    Vec3Type([f64; 3]),
    ElementType(String),
    WordType(String),

    RefU64Type(RefU64),
    StringHashType(AiosStrHash),
    RefU64Array(RefU64Vec),
}

impl From<AttrValAql> for AttrVal {
    fn from(value: AttrValAql) -> Self {
        match value {
            AttrValAql::InvalidType => InvalidType,
            AttrValAql::IntegerType(i) => IntegerType(i),
            AttrValAql::StringType(d) => StringType(d),
            AttrValAql::DoubleType(d) => DoubleType(d),
            AttrValAql::DoubleArrayType(d) => DoubleArrayType(d),
            AttrValAql::StringArrayType(d) => StringArrayType(d),
            AttrValAql::BoolArrayType(d) => BoolArrayType(d),
            AttrValAql::IntArrayType(d) => IntArrayType(d),
            AttrValAql::BoolType(d) => BoolType(d),
            AttrValAql::Vec3Type(d) => Vec3Type(d),
            AttrValAql::ElementType(d) => ElementType(d),
            AttrValAql::WordType(d) => WordType(d),
            // AttrValAql::RefU64Type(d) => { RefU64Type(d) }
            AttrValAql::StringHashType(d) => StringHashType(d),
            AttrValAql::RefU64Array(d) => RefU64Array(d),
        }
    }
}

impl AttrVal {
    #[inline]
    pub fn i32_value(&self) -> i32 {
        return match self {
            IntegerType(v) => *v,
            _ => 0,
        };
    }

    #[inline]
    pub fn i32_array_value(&self) -> Vec<i32> {
        return match self {
            IntArrayType(v) => v.to_vec(),
            _ => vec![],
        };
    }

    #[inline]
    pub fn double_value(&self) -> Option<f64> {
        return match self {
            DoubleType(v) => Some(*v),
            _ => None,
        };
    }

    #[inline]
    pub fn f32_value(&self) -> Option<f32> {
        return match self {
            DoubleType(v) => Some(*v as f32),
            _ => None,
        };
    }

    #[inline]
    pub fn vec3_value(&self) -> Option<[f64; 3]> {
        return match self {
            Vec3Type(v) => Some(*v),
            _ => None,
        };
    }

    #[inline]
    pub fn dvec_value(&self) -> Option<Vec<f64>> {
        return match self {
            DoubleArrayType(v) => Some(v.to_vec()),
            _ => None,
        };
    }

    #[inline]
    pub fn element_value(&self) -> Option<String> {
        return match self {
            ElementType(v) => Some(v.clone()),
            _ => None,
        };
    }

    #[inline]
    pub fn string_value(&self) -> String {
        return match self {
            StringType(v) => v.to_string(),
            WordType(v) => v.to_string(),
            _ => "unset".to_string(),
        };
    }

    #[inline]
    pub fn refno_value(&self) -> Option<RefU64> {
        return match self {
            RefU64Type(v) => Some(*v),
            _ => None,
        };
    }

    #[inline]
    pub fn string_hash_value(&self) -> Option<AiosStrHash> {
        return match self {
            StringHashType(v) => Some(v.clone()),
            _ => None,
        };
    }

    #[inline]
    pub fn refu64_vec_value(&self) -> Option<RefU64Vec> {
        return match self {
            RefU64Array(v) => Some(v.clone()),
            _ => None,
        };
    }

    #[inline]
    pub fn bool_value(&self) -> Option<bool> {
        return match self {
            BoolType(v) => Some(*v),
            _ => None,
        };
    }

    #[inline]
    pub fn get_val_as_reflect(&self) -> Box<dyn Reflect> {
        return match self {
            InvalidType => Box::new("unset".to_string()),
            StringType(v) | ElementType(v) | WordType(v) => Box::new(v.to_string()),
            RefU64Type(v) => Box::new(v.to_refno_string()),
            BoolArrayType(v) => Box::new(v.clone()),
            IntArrayType(v) => Box::new(v.clone()),
            IntegerType(v) => Box::new(*v),
            DoubleArrayType(v) => Box::new(v.clone()),
            DoubleType(v) => Box::new(*v),
            BoolType(v) => Box::new(*v),
            StringHashType(v) => Box::new(*v),
            StringArrayType(v) => Box::new(v.iter().map(|x| x.to_string()).collect::<Vec<_>>()),
            Vec3Type(v) => Box::new(Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32)),
            RefU64Array(v) => Box::new(v.iter().map(|x| x.to_refno_string()).collect::<Vec<_>>()),
        };
    }

    #[inline]
    pub fn get_val_as_string(&self) -> String {
        return match self {
            AttrVal::InvalidType => "unset".to_string(),
            IntegerType(v) => v.to_string(),
            StringType(v) => v.to_string(),
            DoubleType(v) => v.to_string(),
            DoubleArrayType(v) => serde_json::to_string(v).unwrap(),
            StringArrayType(v) => serde_json::to_string(v).unwrap(),
            BoolArrayType(v) => serde_json::to_string(v).unwrap(),
            IntArrayType(v) => serde_json::to_string(v).unwrap(),
            BoolType(v) => v.to_string(),
            Vec3Type(v) => serde_json::to_string(v).unwrap(),
            ElementType(v) => v.to_string(),
            WordType(v) => v.to_string(),
            RefU64Type(v) => v.to_refno_str().to_string(),
            StringHashType(v) => v.to_string(),
            RefU64Array(v) => serde_json::to_string(v).unwrap(),
        };
    }

    pub fn get_val_as_string_csv(&self) -> String {
        return match self {
            AttrVal::InvalidType => "unset".to_string(),
            IntegerType(v) => v.to_string().replace(",", ";"),
            StringType(v) => v.to_string().replace(",", ";"),
            DoubleType(v) => v.to_string().replace(",", ";"),
            DoubleArrayType(v) => serde_json::to_string(v).unwrap().replace(",", ";"),
            StringArrayType(v) => serde_json::to_string(v).unwrap().replace(",", ";"),
            BoolArrayType(v) => serde_json::to_string(v).unwrap().replace(",", ";"),
            IntArrayType(v) => serde_json::to_string(v).unwrap().replace(",", ";"),
            BoolType(v) => v.to_string().replace(",", ";"),
            Vec3Type(v) => serde_json::to_string(v).unwrap().replace(",", ";"),
            ElementType(v) => v.to_string().replace(",", ";"),
            WordType(v) => v.to_string().replace(",", ";"),
            RefU64Type(v) => v.to_refno_str().to_string().replace(",", ";"),
            StringHashType(v) => v.to_string().replace(",", ";"),
            RefU64Array(v) => serde_json::to_string(v).unwrap().replace(",", ";"),
        };
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Component)]
#[serde(untagged)]
pub enum AttrValAql {
    InvalidType,
    IntegerType(i32),
    StringType(String),
    DoubleType(f64),
    DoubleArrayType(Vec<f64>),
    StringArrayType(Vec<String>),
    BoolArrayType(Vec<bool>),
    IntArrayType(Vec<i32>),
    BoolType(bool),
    Vec3Type([f64; 3]),
    ElementType(String),
    WordType(String),
    // RefU64Type(RefU64),
    StringHashType(AiosStrHash),
    RefU64Array(RefU64Vec),
}

impl From<AttrVal> for AttrValAql {
    fn from(value: AttrVal) -> Self {
        match value {
            InvalidType => AttrValAql::InvalidType,
            IntegerType(i) => AttrValAql::IntegerType(i),
            StringType(d) => AttrValAql::StringType(d),
            DoubleType(d) => AttrValAql::DoubleType(d),
            DoubleArrayType(d) => AttrValAql::DoubleArrayType(d),
            StringArrayType(d) => AttrValAql::StringArrayType(d),
            BoolArrayType(d) => AttrValAql::BoolArrayType(d),
            IntArrayType(d) => AttrValAql::IntArrayType(d),
            BoolType(d) => AttrValAql::BoolType(d),
            Vec3Type(d) => AttrValAql::Vec3Type(d),
            ElementType(d) => AttrValAql::ElementType(d),
            WordType(d) => AttrValAql::WordType(d),
            RefU64Type(d) => AttrValAql::StringType(d.to_url_refno().into()),
            StringHashType(d) => AttrValAql::StringHashType(d),
            RefU64Array(d) => AttrValAql::RefU64Array(d),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct PdmsDatabaseInfo {
    // 第一个i32是type_hash ，第二个i32是属性的hash
    pub noun_attr_info_map: DashMap<i32, DashMap<i32, AttrInfo>>,
}

impl PdmsDatabaseInfo {
    ///获得所有的explicit nouns
    pub fn get_all_explicit_nouns(&self) -> DashMap<i32, AttrInfo> {
        let mut map = DashMap::default();
        for kv in &self.noun_attr_info_map {
            for info in kv.value() {
                if info.offset == 0 && !map.contains_key(info.key()) {
                    map.insert(*info.key(), info.value().clone());
                }
            }
        }
        map
    }

    pub fn fill_default_values(&self, att_map: &mut AttrMap) {
        let noun_hash = att_map.get_noun();
        if let Some(m) = self.noun_attr_info_map.get(&noun_hash) {
            for info in m.value() {
                if info.offset == 0 && !att_map.contains_attr_hash(noun_hash as _) {
                    att_map.insert(noun_hash as _, info.default_val.clone());
                }
            }
        }
    }

    ///合并
    pub fn merge(&self, new: &DashMap<i32, DashMap<i32, AttrInfo>>) {
        for kv in new {
            self.noun_attr_info_map
                .insert(*kv.key(), kv.value().clone());
        }
    }

    pub fn save(&self, path: Option<&str>) -> anyhow::Result<()> {
        let path = path.unwrap_or("all_attr_info.json");
        let bytes = serde_json::to_string(self)?;
        let mut file = std::fs::File::create(path)?;
        file.write_all(bytes.as_bytes());
        Ok(())
    }
}

unsafe impl Send for PdmsDatabaseInfo {}

unsafe impl Sync for PdmsDatabaseInfo {}

///可以缩放的类型
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ScaledGeom {
    Box(Vec3),
    Cylinder(Vec3),
    Sphere(f32),
}

//for json compatibility
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

// #[derive(Serialize, Deserialize, Clone, Debug, Default, Resource, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
// pub struct CachedInstanceMgr {
//     pub inst_data: ShapeInstancesData,
//     // pub level_shape_mgr: LevelShapeMgr,   //每个非叶子节点都知道自己的所有shape refno
// }

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

//元件库里的模型，需要两级来完成这个边，有一个代表的refno
//指向典型的例子

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

// fn de_geo_edge_from_key_str<'de, D>(deserializer: D) -> Result<RefU64, D::Error>
//     where D: Deserializer<'de> {
//     let s = String::deserialize(deserializer)?;
//     Ok(RefU64::from_url_refno(&s).unwrap_or_default())
// }
#[inline]
fn ser_inst_info_edge_as_key_str<S>(refno: &RefU64, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str(format!("pdms_inst_infos/{}", refno.to_url_refno()).as_str())
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
    //todo 这里的数据是重复的，需要复用
    //有哪一些 geo insts 组成
    //也可以通过edge 来组合
    #[serde(default)]
    pub cata_hash: Option<String>,
    //是否可见
    pub visible: bool,
    //所属一般类型，ROOM、STRU、PIPE等, 用枚举处理
    pub generic_type: PdmsGenericType,
    pub aabb: Option<Aabb>,
    //相对世界坐标系下的变换矩阵 rot, translation, scale
    pub world_transform: Transform,

    #[serde(default)]
    pub flow_pt_indexs: Vec<i32>,

    #[serde(default)]
    pub geo_type: GeoBasicType,
}

pub fn de_refno_from_key_str<'de, D>(deserializer: D) -> Result<RefU64, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(RefU64::from_url_refno(&s).unwrap_or_default())
}

pub fn ser_refno_as_key_str<S>(refno: &RefU64, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str(refno.to_url_refno().as_str())
}

impl EleGeosInfo {
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
            return c.parse::<u64>().unwrap_or(0).to_string();
        }
        self.refno.to_url_refno()
    }

    ///获取几何体数据的u64 key
    #[inline]
    pub fn get_inst_key_u64(&self) -> u64 {
        if let Some(c) = &self.cata_hash {
            return c.parse::<u64>().unwrap_or(0);
        }
        *self.refno
    }

    ///获得所有的geo hashes
    // #[inline]
    // pub fn get_all_geo_hashes(&self) -> Vec<u64>{
    //     self.geo_basics.iter().map(|x| x.get_hash()).collect()
    // }
    //
    // ///获得正实体的geo hashes
    // #[inline]
    // pub fn get_pos_geo_hashes(&self) -> Vec<u64>{
    //     self.geo_basics.iter().filter(|&x| x.is_pos()).map(|x| x.get_hash()).collect()
    // }
    //
    // ///获得负实体的geo hashes
    // #[inline]
    // pub fn get_neg_geo_hashes(&self) -> Vec<u64>{
    //     self.geo_basics.iter().filter(|&x| x.is_neg()).map(|x| x.get_hash()).collect()
    // }

    // #[inline]
    // pub fn has_neg(&self) -> bool{
    //     self.geo_basics.iter().any(|x| x.is_neg())
    // }
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

    //负实体运算的参考号组合记录
    // pub compound_refnos_map: std::collections::HashMap<RefU64, Vec<RefU64>>,
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

//凸面体的数据缓存，同时也是需要lod的
#[derive(Serialize, Deserialize, Default, Deref, DerefMut)]
pub struct ColliderShapeMgr {
    pub shapes_map: DashMap<RefU64, SharedShape>,
}

impl ColliderShapeMgr {
    //
    // pub fn get_collider(ele_geos_info: &EleGeosInfo, mesh_mgr: &PlantMeshesData) -> Vec<SharedShape> {
    //     let mut target_colliders = vec![];
    //     let mut colliders = vec![];
    //     let ele_trans = ele_geos_info.world_transform;
    //     for geo in &ele_geos_info.geo_insts {
    //         let t = ele_geos_info.get_geo_world_transform(geo);
    //         let s = t.scale;
    //         let mut local_rot = glam::Quat::IDENTITY;
    //         let shape = match geo.geo_hash {
    //             prim_geo::CUBE_GEO_HASH => {
    //                 SharedShape::cuboid(s.x / 2.0, s.y / 2.0, s.z / 2.0)
    //             }
    //             prim_geo::SPHERE_GEO_HASH => {
    //                 SharedShape::ball(s.x)
    //             }
    //             prim_geo::CYLINDER_GEO_HASH => {
    //                 local_rot = glam::Quat::from_rotation_x(PI / 2.0);
    //                 SharedShape::cylinder(s.z / 2.0, s.x / 2.0)
    //             }
    //             _ => {
    //                 let m = mesh_mgr.get_mesh(geo.geo_hash).unwrap();
    //                 SharedShape(Arc::new(m.get_tri_mesh(t.compute_matrix())))
    //             }
    //         };
    //         let rot = t.rotation * local_rot;
    //         if shape.as_composite_shape().is_none() {
    //             colliders.push((Isometry {
    //                 rotation: UnitQuaternion::from_quaternion(Quaternion::new(rot.w, rot.x, rot.y, rot.z)),
    //                 translation: Vector::new(t.translation.x, t.translation.y, t.translation.z).into(),
    //             }, shape));
    //         } else {
    //             target_colliders.push(shape);
    //         }
    //     }
    //     if !colliders.is_empty() {
    //         target_colliders.push(SharedShape::compound(colliders));
    //     }
    //     target_colliders
    // }

    pub fn serialize_to_bin_file(&self) -> bool {
        let mut file = File::create(format!("collider.shapes")).unwrap();
        let serialized = bincode::serialize(&self).unwrap();
        file.write_all(serialized.as_slice()).unwrap();
        true
    }

    pub fn deserialize_from_bin_file(file_path: &str) -> Option<Self> {
        let mut file = File::open(file_path).ok()?;
        let mut buf: Vec<u8> = Vec::new();
        file.read_to_end(&mut buf).ok()?;
        bincode::deserialize(buf.as_slice()).ok()
    }
}

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

use crate::shape::pdms_shape::{BrepShapeTrait, PlantMesh};
#[cfg(feature = "render")]
use bevy_render::prelude::*;
#[cfg(feature = "opencascade_rs")]
use opencascade::primitives::Shape;

impl PlantGeoData {
    ///返回三角模型 （tri_mesh, AABB）
    #[cfg(feature = "render")]
    pub fn gen_bevy_mesh_with_aabb(&self) -> Option<(Mesh, Option<Aabb>)> {
        let mut mesh = bevy_render::prelude::Mesh::new(TriangleList);
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
        mesh.set_indices(Some(Indices::U32(d.indices.clone())));

        Some((mesh, self.aabb))
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
            if let Some(mut d) = m.gen_unit(tol_ratio) {
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
    #[serde(rename = "_key")]
    // #[serde_as(as = "DisplayFromStr")]
    pub inst_key: String,
    #[serde(deserialize_with = "de_refno_from_str")]
    #[serde(serialize_with = "ser_refno_as_str")]
    pub refno: RefU64,
    pub insts: Vec<EleInstGeo>,

    pub aabb: Option<Aabb>,
    pub type_name: String,

    #[serde(default)]
    pub ptset_map: BTreeMap<i32, CateAxisParam>,
}

impl EleInstGeosData {
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
    #[cfg(feature = "opencascade_rs")]
    pub fn gen_ngmr_occ_shapes(&self, transform: &Transform) -> Vec<(Vec<RefU64>, Shape)> {
        let ngmr_shapes: Vec<_> = self
            .insts
            .iter()
            .filter(|x| x.geo_type == GeoBasicType::CateCrossNeg)
            .filter_map(|x| {
                if let Some(mut s) = x.gen_occ_shape() {
                    s.transform_by_mat(&transform.compute_matrix().as_dmat4());
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
    #[cfg(feature = "opencascade_rs")]
    pub fn gen_occ_shape(&self, transform: &Transform) -> Option<(Shape, Vec<RefU64>)> {
        let mut neg_shapes: Vec<(Shape, Vec<RefU64>)> = self
            .insts
            .iter()
            .filter(|x| x.geo_type == GeoBasicType::Neg)
            .filter_map(|x| {
                if let Some(mut s) = x.gen_occ_shape() {
                    s.transform_by_mat(&transform.compute_matrix().as_dmat4());
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

        let mut pos_shapes: HashMap<RefU64, Shape> = self
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
                        *pos_shape = pos_shape.subtract_shape(&neg_shape).0;
                    }
                }
            });
        }
        let mut compound = opencascade::primitives::Compound::from_shapes(pos_shapes.values());
        compound.transform_by_mat(&transform.compute_matrix().as_dmat4());
        Some((compound, vec![]))
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
    ///对应参考号
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

    #[cfg(feature = "opencascade_rs")]
    pub fn gen_occ_shape(&self) -> Option<Shape> {
        let mut shape: Option<Shape> = self.geo_param.gen_occ_shape();
        //scale 不能要，已经包含在OCC的真实参数里
        let mut new_transform = self.transform;
        new_transform.scale = Vec3::ONE;
        if let Some(s) = shape.as_mut() {
            s.transform_by_mat(&new_transform.compute_matrix().as_dmat4());
        }
        shape
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
    RefU64::from_refno_str(&s).map_err(de::Error::custom)
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
        .map(|x| x.to_refno_string())
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
    s.serialize_str(refno.to_refno_str().as_str())
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
    // #[serde(deserialize_with = "de_from_str")]
    // #[serde(serialize_with = "ser_u64_as_str")]
    // #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(default)]
    pub cata_hash: Option<String>,
    // #[serde_as(as = "DisplayFromStr")]
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
            NamedAttrValue::StringType(self.refno.to_url_refno()),
            NamedAttrValue::StringType(self.owner.to_url_refno()),
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

#[test]
fn test_refu64() {
    let refno = RefU64::from(RefI32Tuple((16477, 80)));
    println!("refno={}", refno.0);
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
