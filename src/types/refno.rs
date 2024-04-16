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
Reflect,
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
        let nums = ts
            .split(['_', '/'])
            .filter_map(|x| x.parse::<u32>().ok())
            .collect::<Vec<_>>();
        if nums.len() == 2 {
            Ok(Self::from_two_nums(nums[0], nums[1]))
        } else if let Ok(d) = ts.parse::<u64>().map_err(|_| ParseRefU64Error) {
            Ok(Self(d))
        } else {
            Err(ParseRefU64Error)
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
        let string: String = self.to_string();
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
        f.write_str(self.to_string().as_str())
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
    pub fn to_pbs_key(&self) -> String {
        format!("pbs:{}", &self.to_string())
    }

    pub fn to_type_key(&self,noun:&str) -> String {
        format!("{}:{}", noun,&self.to_string())
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
    pub fn to_slash_string(&self) -> String {
        format!("{}/{}", self.get_0(), self.get_1())
    }

    #[inline]
    pub fn from_two_nums(n: u32, m: u32) -> Self {
        Self(((n as u64) << 32) + m as u64)
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

    /// 返回图数据库的id形式 例如 pdms_eles/1232_5445
    pub fn to_arangodb_ids(collection_name: &str, refnos: Vec<RefU64>) -> Vec<String> {
        refnos
            .into_iter()
            .map(|refno| format!("{}/{}", collection_name, refno.to_string()))
            .collect()
    }

    #[inline]
    pub fn format_url_name(&self, col: &str) -> String {
        format!("{}/{}", col, self.to_string())
    }

    /// 将参考号字符串类型集合转为 Vec<RefU64>
    pub fn from_refno_strs(refno_strs: &Vec<String>) -> Vec<Self> {
        refno_strs
            .iter()
            .filter_map(|refno| Self::from_str(refno).ok())
            .collect()
    }
    /// 转换为pdms的形式
    pub fn to_pdms_str(&self) -> String {
        format!("{}/{}", self.get_0(), self.get_1())
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
