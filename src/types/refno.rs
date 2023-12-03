use bevy_ecs::component::Component;
use bevy_reflect::Reflect;
#[cfg(feature = "sea-orm")]
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use serde::{Deserializer, Serializer};
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use std::fmt::{Debug, Display, Formatter, Write};
use std::hash::Hash;
use std::ops::Deref;
use std::str::FromStr;
use std::{fmt, hash};

#[derive(Debug, PartialEq, Eq, derive_more::Display)]
pub struct ParseRefU64Error;

#[derive(
rkyv::Archive,
rkyv::Deserialize,
rkyv::Serialize,
Hash,
Clone,
Copy,
Default,
Component,
Eq,
PartialEq,
PartialOrd,
Ord,
Reflect, // DeriveValueType,
)]
pub struct RefU64(pub u64);

impl Serialize for RefU64 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        serializer.serialize_str(self.to_string().as_str())
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum StringOrU64 {
    Str(String),
    Num(u64),
}

impl<'de> Deserialize<'de> for RefU64 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
    {
        if let Ok(s) = StringOrU64::deserialize(deserializer) {
            match s {
                StringOrU64::Str(s) => Self::from_str(s.as_str())
                    .map_err(|_| serde::de::Error::custom("refno parse error")),
                StringOrU64::Num(d) => Ok(Self(d)),
            }
        } else {
            return Err(serde::de::Error::custom("refno parse error"));
        }
    }
}

impl FromStr for RefU64 {
    type Err = ParseRefU64Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let ts = s.split(['=', ':']).skip(1).next().unwrap_or(s);
        if ts.contains('_') {
            Self::from_url_refno(&ts).ok_or(ParseRefU64Error)
        } else if ts.contains('/') {
            Self::from_refno_str(&ts).map_err(|_| ParseRefU64Error)
        } else {
            let d = ts.parse::<u64>().map_err(|_| ParseRefU64Error)?;
            Ok(Self(d))
        }
    }
}

impl From<Thing> for RefU64 {
    fn from(thing: Thing) -> Self {
        thing.id.to_string().as_str().into()
    }
}

#[cfg(feature = "sea-orm")]
impl sea_orm::sea_query::ValueType for RefU64 {
    fn try_from(v: Value) -> Result<Self, sea_orm::sea_query::ValueTypeErr> {
        <String as sea_orm::sea_query::ValueType>::try_from(v)
            .map(|v| Self::from_str(&v).unwrap_or_default())
    }

    fn type_name() -> String {
        stringify!(StringVec).to_owned()
    }

    fn array_type() -> sea_orm::sea_query::ArrayType {
        sea_orm::sea_query::ArrayType::String
    }

    fn column_type() -> sea_orm::sea_query::ColumnType {
        sea_orm::sea_query::ColumnType::Text
    }
}

#[cfg(feature = "sea-orm")]
impl Into<sea_orm::Value> for RefU64 {
    fn into(self) -> sea_orm::Value {
        let string: String = self.to_refno_string();
        sea_orm::Value::String(Some(Box::new(string)))
    }
}

#[cfg(feature = "sea-orm")]
impl sea_orm::TryGetable for RefU64 {
    fn try_get_by<I: sea_orm::ColIdx>(
        res: &sea_orm::QueryResult,
        idx: I,
    ) -> Result<Self, sea_orm::TryGetError> {
        <String as sea_orm::TryGetable>::try_get_by(res, idx)
            .map(|v| Self::from_str(&v).unwrap_or_default())
    }
}

impl From<&str> for RefU64 {
    fn from(s: &str) -> Self {
        Self::from_str(s).unwrap_or_default()
    }
}

impl From<String> for RefU64 {
    fn from(s: String) -> Self {
        Self::from_str(s.as_str()).unwrap_or_default()
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
        write!(f, "{}_{}", self.get_0(), self.get_1())
    }
}

impl RefU64 {
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.get_0() > 0
    }

    pub fn is_unset(&self) -> bool {
        self.get_0() == 0
    }

    #[inline]
    pub fn get_sled_key(&self) -> [u8; 8] {
        self.0.to_be_bytes()
    }

    #[inline]
    pub fn to_pe_key(&self) -> String {
        format!("pe:{}", &self.to_string())
    }
    #[inline]
    pub fn to_pe_thing(&self) -> Thing {
        ("pe".to_owned(), self.to_string()).into()
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
        if let Ok(r0) = strs[0].parse::<i32>() && let Ok(r1) = strs[1].parse::<i32>() {
            Some(RefI32Tuple((r0, r1)).into())
        } else {
            None
        }
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

    /// 将参考号字符串类型集合转为 Vec<RefU64>
    pub fn from_refno_strs(refno_strs: &Vec<String>) -> Vec<Self> {
        refno_strs
            .iter()
            .filter_map(|refno| Self::from_refno_str(refno).ok())
            .collect()
    }
}

///pdms的参考号
#[derive(Serialize, Deserialize, Clone, Debug, Default, Copy, Eq, PartialEq, Hash)]
pub struct RefI32Tuple(pub (i32, i32));

use crate::cache::mgr::BytesTrait;
use anyhow::anyhow;
#[cfg(feature = "sea-orm")]
use sea_orm::sea_query::ValueType;
use std::string::String;
use surrealdb::sql::Thing;

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
        self.0.0
    }

    #[inline]
    pub fn get_1(&self) -> i32 {
        self.0.1
    }
}
