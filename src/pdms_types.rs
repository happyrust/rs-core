use std::collections::{BTreeMap, HashMap};
use std::default::Default;
use std::fmt;
use std::fmt::{Debug, Formatter, Pointer};
use std::fs::File;
use std::io::{Read, Write};
use std::ops::{Deref, DerefMut};
use std::panic::catch_unwind;
use std::result::Iter;
use std::sync::Arc;
use std::vec::IntoIter;
use bitflags::bitflags;

use anyhow::anyhow;
use bevy::ecs::reflect::ReflectComponent;
use bevy::prelude::*;
use bevy::reflect::Reflect;
use bevy::render::primitives::Aabb;
use dashmap::DashMap;
use dashmap::mapref::one::Ref;
use glam::{Affine3A, Mat4, Quat, Vec3, Vec4};
use hash32::Hasher;
use hash32_derive::Hash32;
use id_tree::{NodeId, Tree};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use sled::IVec;
use smallvec::SmallVec;
use smol_str::SmolStr;

use crate::BHashMap;
use crate::consts::*;
use crate::consts::{ATT_CURD, UNSET_STR};
use crate::parsed_data::CateAxisParam;
use crate::pdms_data::AxisParam;
use crate::pdms_types::AttrVal::*;
use crate::prim_geo::ctorus::CTorus;
use crate::prim_geo::cylinder::SCylinder;
use crate::prim_geo::dish::Dish;
use crate::prim_geo::pyramid::Pyramid;
use crate::prim_geo::rtorus::RTorus;
use crate::prim_geo::sbox::SBox;
use crate::prim_geo::snout::LSnout;
use crate::shape::pdms_shape::{BrepShapeTrait, PdmsMesh};
use crate::tool::db_tool::{db1_dehash, db1_hash};

pub const LEVEL_VISBLE: u32 = 6;


// 包装整数
#[derive(Serialize, Deserialize, Clone, Debug, Default, Copy, Eq, PartialEq, Hash)]
pub struct Integer(pub u32);


///pdms的参考号
#[derive(Serialize, Deserialize, Clone, Debug, Default, Copy, Eq, PartialEq, Hash)]
pub struct RefI32Tuple(pub (i32, i32));

impl Into<SmolStr> for RefI32Tuple {
    fn into(self) -> SmolStr {
        SmolStr::from(format!("{}/{}", self.get_0(), self.get_1()))
    }
}

impl Into<String> for RefI32Tuple {
    fn into(self) -> String {
        format!("{}/{}", self.get_0(), self.get_1())
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
        self.0.0
    }

    #[inline]
    pub fn get_1(&self) -> i32 {
        self.0.1
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
        where T: Display,
              S: Serializer
    {
        serializer.collect_str(value)
    }

    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
        where T: FromStr,
              T::Err: Display,
              D: Deserializer<'de>
    {
        String::deserialize(deserializer)?.parse().map_err(de::Error::custom)
    }
}


//把Refno当作u64
#[derive(Hash, Serialize, Deserialize, Clone, Copy, Default, Component, Eq, PartialEq, Hash32)]
pub struct RefU64(
    // #[serde(with = "string")]
    pub u64
);

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

impl Into<sled::IVec> for RefU64 {
    fn into(self) -> sled::IVec {
        self.0.to_be_bytes().to_vec().into()
    }
}

impl Into<sled::IVec> for &RefU64 {
    fn into(self) -> sled::IVec {
        // bincode::serialize(self).unwrap().into()
        self.0.to_be_bytes().to_vec().into()
    }
}

impl From<sled::IVec> for RefU64 {
    fn from(d: sled::IVec) -> Self {
        // bincode::deserialize(&d).unwrap()
        Self::from(d.as_ref())
    }
}

//IVec

// impl FromSkyhashBytes for RefU64 {
//     fn from_element(element: Element) -> SkyResult<Self> {
//         if let Element::Binstr(v) = element {
//             return Ok(bincode::deserialize::<RefU64>(&v).unwrap());
//         }
//         Err(skytable::error::Error::ParseError("Bad element type".to_string()))
//     }
// }

impl ToString for RefU64 {
    fn to_string(&self) -> String {
        let refno: RefI32Tuple = self.into();
        refno.into()
    }
}

impl RefU64 {
    #[inline]
    pub fn is_valid(&self) -> bool { self.get_0() > 0 && self.get_1() > 0 }

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
        use hash32::{FnvHasher, Hash, Hasher};
        let mut fnv = FnvHasher::default();
        self.hash(&mut fnv);
        fnv.finish()
    }

    #[inline]
    pub fn to_refno_str(&self) -> SmolStr {
        let refno: RefI32Tuple = self.into();
        refno.into()
    }

    #[inline]
    pub fn to_refno_string(&self) -> String {
        let refno: RefI32Tuple = self.into();
        let refno_str: SmolStr = refno.into();
        refno_str.to_string()
    }

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
        let split_refno = refno.split('/').collect::<Vec<_>>();
        if split_refno.len() != 2 {
            return Err(anyhow!("参考号错误, 没有斜线!".to_string()));
        }
        let refno0: i32 = split_refno[0].parse::<i32>()?;
        let refno1: i32 = split_refno[1].parse::<i32>()?;
        Ok(RefI32Tuple((refno0, refno1)).into())
    }

    #[inline]
    pub fn to_url_refno(&self) -> String {
        let refno: RefI32Tuple = self.into();
        format!("{}_{}", refno.get_0(), refno.get_1())
    }

    #[inline]
    pub fn from_url_refno(refno: String) -> Option<Self> {
        let strs = refno.as_str().split('_').collect::<Vec<_>>();
        if strs.len() < 2 { return None; }
        Some(RefU64::from_two_nums(strs[0].parse().unwrap(), strs[1].parse().unwrap()))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, Component)]
pub struct RefU64Vec(pub Vec<RefU64>);


impl Into<IVec> for RefU64Vec {
    fn into(self) -> IVec {
        bincode::serialize(&self).unwrap().into()
    }
}

impl Into<IVec> for &RefU64Vec {
    fn into(self) -> IVec {
        bincode::serialize(self).unwrap().into()
    }
}

impl From<IVec> for RefU64Vec {
    fn from(d: IVec) -> Self {
        bincode::deserialize(&d).unwrap()
    }
}

impl Deref for RefU64Vec {
    type Target = Vec<RefU64>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for RefU64Vec {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
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
        self.0.push(v);
    }
}


// #[derive(Serialize, Deserialize, Clone, Debug, Default, Component, Eq, Hash, PartialEq)]
#[derive(Serialize, Deserialize, Clone, Debug, Default, Component, Reflect, Eq, Hash,
PartialEq, Ord, PartialOrd)]
#[reflect(Component)]
pub struct NounHash(pub u32);

impl ToString for NounHash {
    fn to_string(&self) -> String {
        db1_dehash(self.0)
    }
}

impl Deref for NounHash {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<&SmolStr> for NounHash {
    fn from(s: &SmolStr) -> Self {
        Self(db1_hash(s.as_str()))
    }
}

impl From<SmolStr> for NounHash {
    fn from(s: SmolStr) -> Self {
        Self(db1_hash(s.as_str()))
    }
}

impl From<u32> for NounHash {
    fn from(n: u32) -> Self {
        Self(n)
    }
}

impl From<&str> for NounHash {
    fn from(s: &str) -> Self {
        Self(db1_hash(s))
    }
}

///PDMS的属性数据Map
#[derive(Serialize, Deserialize, Deref, DerefMut, Clone, Default, Component)]
pub struct AttrMap {
    pub map: BHashMap<NounHash, AttrVal>,
}

impl Debug for AttrMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = self.to_string_hashmap();
        s.fmt(f)
    }
}

impl Into<IVec> for AttrMap {
    fn into(self) -> IVec {
        bincode::serialize(&self).unwrap().into()
    }
}

impl Into<IVec> for &AttrMap {
    fn into(self) -> IVec {
        bincode::serialize(self).unwrap().into()
    }
}

impl From<IVec> for AttrMap {
    fn from(d: IVec) -> Self {
        bincode::deserialize(&d).unwrap()
    }
}

impl AttrMap {
    #[inline]
    pub fn is_null(&self) -> bool {
        self.map.len() == 0
    }

    #[inline]
    pub fn into_bincode_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }

    #[inline]
    pub fn from_bincode_bytes(bytes: &[u8]) -> Option<Self> {
        bincode::deserialize(bytes).ok()
    }

    #[inline]
    pub fn into_compress_bytes(&self) -> Vec<u8> {
        use flate2::Compression;
        use flate2::write::DeflateEncoder;
        let mut e = DeflateEncoder::new(Vec::new(), Compression::default());
        e.write_all(&self.into_bincode_bytes());
        e.finish().unwrap_or_default()
    }

    #[inline]
    pub fn from_compress_bytes(bytes: &[u8]) -> Option<Self> {
        use flate2::write::DeflateDecoder;
        let mut writer = Vec::new();
        let mut deflater = DeflateDecoder::new(writer);
        deflater.write_all(bytes).ok()?;
        bincode::deserialize(&deflater.finish().ok()?).ok()
    }

    // 返回 DESI 、 CATA .. 等模块值
    pub fn get_db_stype(&self) -> Option<&'static str> {
        let val = self.get(&NounHash(ATT_STYP as u32))?;
        match val {
            AttrVal::IntegerType(v) => {
                Some(match *v {
                    1 => "DESI",
                    2 => "CATA",
                    8 => "DICT",
                    _ => "UNSET"
                })
            }
            _ => None
        }
    }
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
    pub fn insert(&mut self, k: NounHash, v: AttrVal) {
        self.map.insert(k, v);
    }

    #[inline]
    pub fn insert_by_att_name(&mut self, k: &str, v: AttrVal) {
        self.map.insert(k.into(), v);
    }

    #[inline]
    pub fn contains_attr_name(&self, name: &str) -> bool {
        self.map.contains_key(&name.into())
    }

    #[inline]
    pub fn contains_attr_hash(&self, hash: u32) -> bool {
        self.map.contains_key(&(hash.into()))
    }

    pub fn to_string_hashmap(&self) -> BTreeMap<String, String> {
        let mut map = BTreeMap::new();
        for (k, v) in &self.map {
            map.insert(db1_dehash(k.0), format!("{:?}", v));
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
    pub fn get_name(&self) -> AiosStr {
        return if let Some(StringType(name)) = self.get_val("NAME") {
            AiosStr(name.clone())
        } else {
            AiosStr(SmolStr::new(""))
        };
    }

    #[inline]
    pub fn get_main_db_in_mdb(&self) -> Option<RefU64> {
        if let Some(v) = self.map.get(&NounHash(ATT_CURD)) {
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
    pub fn get_refno_as_string(&self) -> Option<SmolStr> {
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
        // Err(anyhow!("Level number is less than 2".to_string()))
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
        // return Err(anyhow!("Owner type not corrent".to_string()));
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
    pub fn get_type_cloned(&self) -> Option<SmolStr> {
        self.get_smol_str("TYPE").map(|x| x.clone())
    }

    #[inline]
    pub fn get_u32(&self, key: &str) -> Option<u32> {
        self.get_i32(key).map(|s| s as u32)
    }

    #[inline]
    pub fn get_i32(&self, key: &str) -> Option<i32> {
        let v = self.get_val(key)?;
        match v {
            IntegerType(d) => {
                Some(*d as i32)
            }
            _ => {
                None
            }
        }
    }

    #[inline]
    pub fn get_refu64(&self, key: &str) -> Option<RefU64> {
        let v = self.get_val(key)?;
        match v {
            RefU64Type(d) => {
                Some(*d)
            }
            _ => {
                None
            }
        }
    }

    #[inline]
    pub fn get_refu64_vec(&self, key: &str) -> Option<RefU64Vec> {
        let v = self.get_val(key)?;
        match v {
            RefU64Array(d) => {
                Some(d.clone())
            }
            _ => {
                None
            }
        }
    }

    #[inline]
    pub fn get_str(&self, key: &str) -> Option<&str> {
        let v = self.get_val(key)?;
        match v {
            StringType(s) | WordType(s) | ElementType(s) => {
                Some(s.as_str())
            }
            _ => {
                None
            }
        }
    }

    #[inline]
    pub fn get_smol_str(&self, key: &str) -> Option<&SmolStr> {
        let v = self.get_val(key)?;
        match v {
            StringType(s) | WordType(s) | ElementType(s) => {
                Some(s)
            }
            _ => {
                None
            }
        }
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
    pub fn get_as_smol_str(&self, key: &str) -> Option<SmolStr> {
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

    // #[inline]
    // pub fn get_as_vec_string(&self, key: &str) -> Vec<SmolStr> {
    //     if let Some(v) = self.map.get(&key.into()) {
    //         return match v {
    //             StringArrayType(d) => d.clone(),
    //             _ => {
    //                 vec![]
    //             }
    //         };
    //     }
    //     vec![]
    // }

    // #[inline]
    // pub fn get_as_vec_refnos(&self, key: &str) -> Vec<SmolStr> {
    //     if let Some(v) = self.map.get(&key.into()) {
    //         return match v {
    //             IntArrayType(d) => d
    //                 .chunks_exact(2)
    //                 .map(|x| format!("{}/{}", x[0], x[1]).into())
    //                 .collect(),
    //             _ => {
    //                 vec![]
    //             }
    //         };
    //     }
    //     vec![]
    // }

    #[inline]
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        if let AttrVal::BoolType(d) = self.get_val(key)? {
            return Some(*d);
        }
        None
        // Err(TypeNotCorrect(key.to_string(), "bool".to_string()).into())
    }

    #[inline]
    pub fn get_val(&self, key: &str) -> Option<&AttrVal> {
        self.map.get(&db1_hash(key).into())
    }

    #[inline]
    pub fn get_f64(&self, key: &str) -> Option<f64> {
        self.get_val(key)?.double_value() //.ok_or_else(|| TypeNotCorrect(key.to_string(), "double".to_string()).into() )
    }

    #[inline]
    pub fn get_f32(&self, key: &str) -> Option<f32> {
        self.get_f64(key).map(|x| x as f32)
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
        let ang = self.get_f64_vec("ORI")?;
        let mat = (glam::f32::Mat3::from_rotation_z(ang[2].to_radians() as f32)
            * glam::f32::Mat3::from_rotation_y(ang[1].to_radians() as f32)
            * glam::f32::Mat3::from_rotation_x(ang[0].to_radians() as f32));
        Some(Quat::from_mat3(&mat))
    }

    pub fn get_matrix(&self) -> Option<Affine3A> {
        let mut affine = Affine3A::IDENTITY;
        let pos = self.get_f64_vec("POS")?;
        affine.translation = glam::f32::Vec3A::new(pos[0] as f32, pos[1] as f32, pos[2] as f32);
        let ang = self.get_f64_vec("ORI")?;
        affine.matrix3 = (glam::f32::Mat3A::from_rotation_z(ang[2].to_radians() as f32)
            * glam::f32::Mat3A::from_rotation_y(ang[1].to_radians() as f32)
            * glam::f32::Mat3A::from_rotation_x(ang[0].to_radians() as f32));
        Some(affine)
    }

    #[inline]
    pub fn get_mat4(&self) -> Option<Mat4> {
        Some(Mat4::from(self.get_matrix()?))
    }

    pub fn get_f64_vec(&self, key: &str) -> Option<Vec<f64>> {
        let val = self.get_val(key)?;
        return match val {
            AttrVal::DoubleArrayType(data) => {
                Some(data.clone())
            }
            AttrVal::Vec3Type(data) => {
                Some(data.to_vec())
            }
            _ => {
                // Err(TypeNotCorrect(key.to_string(), "f64 vec".to_string()).into())
                None
            }
        };
    }

    pub fn get_vec3(&self, key: &str) -> Option<Vec3> {
        if let AttrVal::Vec3Type(d) = self.get_val(key)? {
            return Some(Vec3::new(d[0] as f32, d[1] as f32, d[2] as f32));
        }
        // Err(TypeNotCorrect(key.to_string(), "Vec3Type".to_string()).into())
        None
    }

    pub fn get_i32_vec(&self, key: &str) -> Option<Vec<i32>> {
        if let AttrVal::IntArrayType(d) = self.get_val(key)? {
            return Some(d.clone());
        }
        None
        // Err(TypeNotCorrect(key.to_string(), "i32 vec".to_string()).into())
    }

    ///生成具有几何属性的element的shape
    pub fn create_brep_shape(&self) -> Option<Box<dyn BrepShapeTrait>> {
        let type_noun = self.get_type_cloned()?;
        return match type_noun.as_str() {
            "BOX" => Some(Box::new(SBox::from(self))),
            "CYLI" => Some(Box::new(SCylinder::from(self))),
            // "SPHE" => Some(Box::new(Sphere::from(self))),
            "CONE" => Some(Box::new(LSnout::from(self))),
            "DISH" => Some(Box::new(Dish::from(self))),
            "CTOR" => Some(Box::new(CTorus::from(self))),
            "RTOR" => Some(Box::new(RTorus::from(self))),
            "PYRA" => Some(Box::new(Pyramid::from(self))),
            _ => None,
        };
    }

    /// 获取string属性数组，忽略为空的值
    pub fn get_attr_strings_without_default(&self, keys: &[&str]) -> Vec<SmolStr> {
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

    pub fn get_attr_strings(&self, keys: &[&str]) -> Vec<SmolStr> {
        let mut results = vec![];
        for &attr_name in keys {
            if let Some(result) = self.get_val(attr_name) {
                match result {
                    AttrVal::StringType(v) => {
                        results.push(v.trim_matches('\0').to_owned().clone().into());
                    }
                    _ => {}
                }
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefnoInfo {
    /// 参考号的ref0
    pub ref_0: u32,
    /// 对应db number
    pub db_no: u32,
}

// #[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[derive(Serialize, Deserialize, Clone, Debug, Component)]
pub enum AttrVal {
    InvalidType,
    IntegerType(i32),
    StringType(SmolStr),
    DoubleType(f64),
    DoubleArrayType(Vec<f64>),
    StringArrayType(Vec<SmolStr>),
    BoolArrayType(Vec<bool>),
    IntArrayType(Vec<i32>),
    BoolType(bool),
    Vec3Type([f64; 3]),
    ElementType(SmolStr),
    WordType(SmolStr),

    RefU64Type(RefU64),
    StringHashType(AiosStrHash),
    RefU64Array(RefU64Vec),
}


impl Default for AttrVal {
    fn default() -> Self {
        Self::InvalidType
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
            Vec3Type(v) => {
                Some(*v)
            }
            _ => { None }
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
    pub fn element_value(&self) -> Option<SmolStr> {
        return match self {
            ElementType(v) => Some(v.clone()),
            _ => None,
        };
    }

    #[inline]
    pub fn string_value(&self) -> String {
        return match self {
            StringType(v) => v.to_string(),
            _ => "unset".to_string(),
        };
    }

    #[inline]
    pub fn smol_str_value(&self) -> SmolStr {
        return match self {
            StringType(v) => v.clone(),
            _ => SmolStr::new("unset"),
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
            _ => None
        };
    }

    #[inline]
    pub fn bool_value(&self) -> Option<bool> {
        return match self {
            BoolType(v) => Some(*v),
            _ => None
        };
    }

    #[inline]
    pub fn get_val_as_reflect(&self) -> Box<dyn Reflect> {
        return match self {
            InvalidType => { Box::new("unset".to_string()) }
            // IntegerType(v) => { Box::new(*v) }
            StringType(v) | ElementType(v) | WordType(v) => { Box::new(v.to_string()) }
            RefU64Type(v) => { Box::new(v.to_string()) }
            BoolArrayType(v) => { Box::new(v.clone()) }
            IntArrayType(v) => { Box::new(v.clone()) }
            IntegerType(v) => { Box::new(*v) }
            DoubleArrayType(v) => { Box::new(v.clone()) }
            DoubleType(v) => { Box::new(*v) }
            BoolType(v) => { Box::new(*v) }
            StringHashType(v) => { Box::new(*v) }
            StringArrayType(v) => { Box::new(v.iter().map(|x| x.to_string()).collect::<Vec<_>>()) }
            Vec3Type(v) => { Box::new(Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32)) }
            RefU64Array(v) => { Box::new(v.iter().map(|x| x.to_string()).collect::<Vec<_>>()) }
        };
    }

    #[inline]
    pub fn get_val_as_string(&self) -> String {
        return match self {
            AttrVal::InvalidType => { "unset".to_string() }
            IntegerType(v) => { v.to_string() }
            StringType(v) => { v.to_string() }
            DoubleType(v) => { v.to_string() }
            DoubleArrayType(v) => { serde_json::to_string(v).unwrap() }
            StringArrayType(v) => { serde_json::to_string(v).unwrap() }
            BoolArrayType(v) => { serde_json::to_string(v).unwrap() }
            IntArrayType(v) => { serde_json::to_string(v).unwrap() }
            BoolType(v) => { v.to_string() }
            Vec3Type(v) => { serde_json::to_string(v).unwrap() }
            ElementType(v) => { v.to_string() }
            WordType(v) => { v.to_string() }
            RefU64Type(v) => { v.to_refno_str().to_string() }
            StringHashType(v) => { v.to_string() }
            RefU64Array(v) => { serde_json::to_string(v).unwrap() }
        };
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct PdmsDatabaseInfo {
    pub db_names_map: DashMap<i32, String>,
    // 第一个i32是refno ，第二个i32是type的hash
    pub noun_attr_info_map: DashMap<i32, DashMap<i32, AttrInfo>>,
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

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct AiosAABB {
    pub min: Vec3,
    pub max: Vec3,
}

impl AiosAABB {
    #[inline]
    pub fn new(v1: Vec3, v2: Vec3) -> Self {
        Self { min: v1, max: v2 }
    }

    #[inline]
    pub fn scaled(&mut self, scale: &Vec3) {
        self.min = Vec3::new(
            self.min.x * scale.x,
            self.min.y * scale.y,
            self.min.z * scale.z,
        );
        self.max = Vec3::new(
            self.max.x * scale.x,
            self.max.y * scale.y,
            self.max.z * scale.z,
        );
    }

    #[inline]
    pub fn get_half_extents(&self) -> Vec3 {
        let center = (self.min + self.max) / 2.0;
        self.max - center
    }

    #[inline]
    pub fn get_center(&self) -> Vec3 {
        let center = (self.min + self.max) / 2.0;
        center
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, Deref, DerefMut)]
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


#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct PdmsMeshInstanceMgr {
    pub inst_mgr: ShapeInstancesMgr,
    pub level_shape_mgr: LevelShapeMgr,   //每个非叶子节点都知道自己的所有shape refno
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct PdmsMeshInstanceMgrOld {
    pub inst_mgr: ShapeInstancesMgrOld,
    pub level_shape_mgr: LevelShapeMgr,   //每个非叶子节点都知道自己的所有shape refno
}

impl PdmsMeshInstanceMgrOld {
    #[inline]
    pub fn get_instants_data(&self, refno: RefU64) -> DashMap<RefU64, Ref<RefU64, EleGeosInfoOld>> {
        let mut results = DashMap::new();
        let inst_map = &self.inst_mgr.inst_map;
        if self.level_shape_mgr.contains_key(&refno) {
            for v in (*self.level_shape_mgr.get(&refno).unwrap()).iter(){
                if inst_map.contains_key(v) {
                    results.insert(v.clone(), inst_map.get(v).unwrap());
                }
            }
        } else {
            if inst_map.contains_key(&refno) {
                results.insert(refno.clone(), inst_map.get(&refno).unwrap());
            }
        }
        results
    }

    pub fn serialize_to_bin_file(&self, mdb: &str) -> bool {
        let mut file = File::create(format!(r"PdmsMeshMgr_{}.bin", mdb)).unwrap();
        let serialized = bincode::serialize(&self).unwrap();
        file.write_all(serialized.as_slice()).unwrap();
        true
    }

    pub fn serialize_to_specify_file(&self, file_path: &str) -> bool {
        let mut file = File::create(file_path).unwrap();
        let serialized = bincode::serialize(&self).unwrap();
        file.write_all(serialized.as_slice()).unwrap();
        true
    }

    pub fn deserialize_from_bin_file(mdb: &str) -> anyhow::Result<Self> {
        let mut file = File::open(format!("PdmsMeshMgr_{}.bin", mdb))?;
        let mut buf: Vec<u8> = Vec::new();
        file.read_to_end(&mut buf).ok();
        let r = bincode::deserialize(buf.as_slice())?;
        Ok(r)
    }

    pub fn serialize_to_json_file(&self) -> bool {
        let mut file = File::create(format!("PdmsMeshMgr.json")).unwrap();
        let serialized = serde_json::to_string(&self).unwrap();
        file.write_all(serialized.as_bytes()).unwrap();
        true
    }

    pub fn deserialize_from_json_file() -> anyhow::Result<Self> {
        let mut file = File::open(format!("PdmsMeshMgr.json"))?;
        let mut buf: Vec<u8> = Vec::new();
        file.read_to_end(&mut buf).ok();
        let r = serde_json::from_slice::<Self>(&buf)?;
        Ok(r)
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
        // const ABC = Self::A.bits | Self::B.bits | Self::C.bits;
    }
}

#[repr(C)]
#[derive(Component, Serialize, Deserialize, Clone, Debug, Copy, Eq, PartialEq, Hash)]
pub enum PdmsGenericType {
    UNKOWN = 0,
    PIPE,
    STRU,
    EQUI,
    ROOM,
    SCTN,
    GENSEC,
    HANG,
    HANDRA,
    PANE,
    CFLOOR,
    CWBRAN,
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
    CE,
}

impl Default for PdmsGenericType {
    fn default() -> Self { PdmsGenericType::UNKOWN }
}

//todo important 压缩transform的数据，存储在一个数据集合里，去索引数据，精确到小数点4位数

//todo 需要插入这一层的变换矩阵

/// 存储一个Element 包含的所有几何信息
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct EleGeosInfo {
    // 该 GeosInfo 的参考号 转换为 0_0样式
    // #[serde(skip_serializing)]
    pub _key: String,
    //索引的mesh instance
    pub data: Vec<EleGeoInstance>,
    //是否可见
    pub visible: bool,
    //所属一般类型，ROOM、STRU、PIPE等, 用枚举处理
    pub generic_type: PdmsGenericType,

    //相对世界坐标系下的变换矩阵 rot, translation, scale
    pub world_transform: (Quat, Vec3, Vec3),

    pub ptset_map: BTreeMap<i32, CateAxisParam>,
    pub flow_pt_indexs: Vec<Option<i32>>,

}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct EleGeosInfoOld {
    //索引的mesh instance
    pub data: Vec<EleGeoInstance>,
    //是否可见
    pub visible: bool,
    //所属一般类型，ROOM、STRU、PIPE等, 用枚举处理
    pub generic_type: PdmsGenericType,

    //相对世界坐标系下的变换矩阵 rot, translation, scale
    pub world_transform: (Quat, Vec3, Vec3),

    pub ptset_map: BTreeMap<i32, CateAxisParam>,
    pub flow_pt_indexs: Vec<Option<i32>>,

}

impl Deref for EleGeosInfo {
    type Target = Vec<EleGeoInstance>;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ShapeInstancesMgrOld {
    pub inst_map: DashMap<RefU64, EleGeosInfoOld>,   //todo replace with EleGeosInfo
    //可以用类型的信息去遍历
}

impl ShapeInstancesMgrOld {
    #[inline]
    pub fn get_translation(&self, refno: RefU64) -> Option<Vec3> {
        self.inst_map.get(&refno).map(|x| x.world_transform.1)
    }

    pub fn serialize_to_specify_file(&self, file_path: &str) -> bool {
        let mut file = File::create(file_path).unwrap();
        let serialized = bincode::serialize(&self).unwrap();
        file.write_all(serialized.as_slice()).unwrap();
        true
    }
}

impl Deref for ShapeInstancesMgrOld {
    type Target = DashMap<RefU64, EleGeosInfoOld>;

    fn deref(&self) -> &Self::Target {
        &self.inst_map
    }
}

impl DerefMut for ShapeInstancesMgrOld {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inst_map
    }
}


#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ShapeInstancesMgr {
    pub inst_map: DashMap<RefU64, EleGeosInfo>,   //todo replace with EleGeosInfo
    // pub inst_map: DashMap<RefU64, EleGeosInfoOld>,   //todo replace with EleGeosInfo
    //可以用类型的信息去遍历
}


impl ShapeInstancesMgr {
    #[inline]
    pub fn get_translation(&self, refno: RefU64) -> Option<Vec3> {
        self.inst_map.get(&refno).map(|x| x.world_transform.1)
    }

    pub fn serialize_to_specify_file(&self, file_path: &str) -> bool {
        let mut file = File::create(file_path).unwrap();
        let serialized = bincode::serialize(&self).unwrap();
        file.write_all(serialized.as_slice()).unwrap();
        true
    }
}

impl Deref for ShapeInstancesMgr {
    type Target = DashMap<RefU64, EleGeosInfo>;

    fn deref(&self) -> &Self::Target {
        &self.inst_map
    }
}

impl DerefMut for ShapeInstancesMgr {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inst_map
    }
}


pub type GeoHash = u64;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct CachedMeshesMgr {
    pub meshes: DashMap<GeoHash, PdmsMesh>, //世界坐标系的变换, 为了js兼容64位，暂时使用String
}

impl CachedMeshesMgr {
    /// 获得对应的bevy 三角模型和线框模型
    pub fn get_bevy_mesh(&self, mesh_hash: &u64) -> Option<(Mesh, Mesh, Aabb)> {
        if let Some(cached_msh) = self.get_mesh(mesh_hash) {
            let bevy_mesh = cached_msh.gen_bevy_mesh_with_aabb();
            return Some(bevy_mesh);
        }
        None
    }

    pub fn get_mesh(&self, mesh_hash: &u64) -> Option<Ref<u64, PdmsMesh>> {
        self.meshes.get(mesh_hash)
    }

    //get the mesh index, if not exist, try to create and insert, and return index
    pub fn get_pdms_mesh_hash_key(&self, m: Box<dyn BrepShapeTrait>) -> u64 {
        let hash = m.hash_mesh_params();
        if !self.meshes.contains_key(&hash) {
            let mesh = m.gen_unit_shape();
            self.meshes.insert(hash, mesh);
        }
        hash
    }

    pub fn get_bbox(&self, hash: &u64) -> Option<AiosAABB> {
        if self.meshes.contains_key(hash) {
            let mesh = self.meshes.get(hash).unwrap();
            return Some(mesh.aabb.clone());
        }
        None
    }

    pub fn serialize_to_specify_file(&self, file_path: &str) -> bool {
        let mut file = File::create(file_path).unwrap();
        let serialized = bincode::serialize(&self).unwrap();
        file.write_all(serialized.as_slice()).unwrap();
        true
    }

    pub fn serialize_to_bin_file(&self) -> bool {
        let mut file = File::create(format!("cached_meshes.bin")).unwrap();
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

    pub fn serialize_to_json_file(&self) -> bool {
        let mut file = File::create(format!("cached_meshes.json")).unwrap();
        let serialized = serde_json::to_string(&self).unwrap();
        file.write_all(serialized.as_bytes()).unwrap();
        true
    }

    pub fn deserialize_from_json_file() -> Self {
        let mut file = File::open(format!("cached_meshes.json")).unwrap();
        let mut buf: Vec<u8> = Vec::new();
        file.read_to_end(&mut buf).ok();
        serde_json::from_slice(&buf).unwrap()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EleGeoInstance {
    pub geo_hash: u64,
    //对应参考号
    pub refno: RefU64,
    pub pts: SmallVec<[i32; 3]>,
    pub bbox: AiosAABB,
    //相对owner坐标系的变换, rot, translation, scale
    pub transform: (Quat, Vec3, Vec3),
    pub visible: bool,
    pub is_tubi: bool,
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
    fn get_noun_hash(&self) -> u32 { 0 }

    #[inline]
    fn get_type_name(&self) -> &str { "" }

    #[inline]
    fn get_children_count(&self) -> usize { 0 }
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
    pub fn new(refno: RefU64, noun: String, name: String, owner: RefU64, children_count: usize) -> Self {
        Self {
            refno,
            noun,
            name,
            owner,
            children_count,
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

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct PdmsElement {
    pub refno: String,
    pub owner: RefU64,
    pub name: String,
    pub noun: String,
    pub version: u32,
    pub children_count: usize,
}

impl Into<sled::IVec> for PdmsElement {
    fn into(self) -> sled::IVec {
        bincode::serialize(&self).unwrap().into()
    }
}

impl Into<sled::IVec> for &PdmsElement {
    fn into(self) -> sled::IVec {
        bincode::serialize(self).unwrap().into()
    }
}

impl From<sled::IVec> for PdmsElement {
    fn from(d: sled::IVec) -> Self {
        bincode::deserialize(&d).unwrap()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, Deref, DerefMut)]
pub struct PdmsElementVec(pub Vec<PdmsElement>);

impl Into<sled::IVec> for PdmsElementVec {
    fn into(self) -> sled::IVec {
        bincode::serialize(&self).unwrap().into()
    }
}

impl Into<sled::IVec> for &PdmsElementVec {
    fn into(self) -> sled::IVec {
        bincode::serialize(self).unwrap().into()
    }
}

impl From<sled::IVec> for PdmsElementVec {
    fn from(d: sled::IVec) -> Self {
        bincode::deserialize(&d).unwrap()
    }
}

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
    let mut dashmap_1 = DashMap::new();
    dashmap_1.insert("1", "hello");
    let mut dashmap_2 = DashMap::new();
    dashmap_2.insert("2", "world");
    let mut dashmap_3 = DashMap::new();
    dashmap_1.iter().for_each(|m| {
        dashmap_3.insert(m.key().clone(), m.value().clone());
    });
    dashmap_2.iter().for_each(|m| {
        dashmap_3.insert(m.key().clone(), m.value().clone());
    });
}

#[test]
fn test_refu64() {
    let refno = RefU64::from(RefI32Tuple(((16477, 80))));
    println!("refno={}", refno.0);
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DbAttributeType {
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
    DOUBLEVEC,
    INTVEC,
    FLOATVEC,
    TYPEX,
    Vec3Type,
    RefU64Vec,
}

impl DbAttributeType {
    #[inline]
    pub fn to_sql_str(&self) -> &str {
        match self {
            Self::INTEGER => "INT",
            Self::BOOL => "TINYINT(1)",
            Self::DOUBLE => "DOUBLE",
            Self::INTEGER => "INT",
            Self::ELEMENT | Self::WORD => "BIGINT",
            Self::FLOATVEC | Self::DOUBLEVEC => "BLOB",
            _ => { "VARCHAR" }
        }
    }
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AttrInfo {
    pub name: SmolStr,
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

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct AiosStr(pub SmolStr);

impl AiosStr {
    #[inline]
    pub fn get_u32_hash(&self) -> u32 {
        use hash32::{FnvHasher, Hash, Hasher};
        let mut fnv = FnvHasher::default();
        self.hash(&mut fnv);
        fnv.finish()
    }
    pub fn take(mut self) -> SmolStr {
        self.0
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl Deref for AiosStr {
    type Target = SmolStr;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl hash32::Hash for AiosStr {
    fn hash<H>(&self, state: &mut H)
        where
            H: Hasher,
    {
        state.write(self.0.as_str().as_bytes());
        state.write(&[0xff]);
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
