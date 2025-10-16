use crate::utils::{IntoRecordId, RecordIdExt};
use bevy_ecs::component::Component;
use bevy_reflect::Reflect;
#[cfg(feature = "sea-orm")]
use sea_orm::entity::prelude::*;
use serde::de;
use serde::de::value::MapAccessDeserializer;
use serde::{Deserialize, Serialize};
use serde::{Deserializer, Serializer};
use serde_json::Value as JsonValue;
use serde_with::DisplayFromStr;
use serde_with::serde_as;
use std::fmt::{Debug, Display, Formatter, Write};
use std::hash::Hash;
use std::ops::Deref;
use std::str::FromStr;
use std::{default, fmt, hash};
use surrealdb::types as surrealdb_types;

//todo change to this struct
#[derive(
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
    Hash,
    Clone,
    Default,
    Component,
    Eq,
    PartialEq,
    PartialOrd,
    Ord,
    Reflect,
    SurrealValue,
)]
pub struct RefNo {
    id: String,
    sesno: Option<u16>,
}

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
    // SurrealValue,
)]
pub struct RefU64(pub u64);

impl SurrealValue for RefU64 {
    fn kind_of() -> surrealdb_types::Kind {
        surrealdb_types::Kind::Record(vec!["pe".to_string()])
    }

    fn into_value(self) -> surrealdb_types::Value {
        surrealdb_types::Value::RecordId(self.to_pe_thing())
    }

    fn from_value(value: surrealdb_types::Value) -> anyhow::Result<Self> {
        match value {
            surrealdb_types::Value::RecordId(rid) => Ok(Self::from(rid)),
            surrealdb_types::Value::String(s) => {
                Self::from_str(&s).map_err(|_| anyhow::anyhow!("无法解析字符串为 RefU64"))
            }
            surrealdb_types::Value::Number(n) => Ok(Self(n.to_int().unwrap_or(0) as u64)),
            _ => Err(anyhow::anyhow!("不支持的值类型转换为 RefU64")),
        }
    }
}

impl RefU64 {
    // 自定义序列化方法
    pub fn serialize_as_u64<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u64(self.0)
    }

    // 自定义反序列化方法
    pub fn deserialize_from_u64<'de, D>(deserializer: D) -> Result<RefU64, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = u64::deserialize(deserializer)?;
        Ok(RefU64(value))
    }
}

impl From<u64> for RefU64 {
    fn from(d: u64) -> Self {
        Self(d)
    }
}

impl Serialize for RefU64 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.to_string().as_str())
    }
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum RefnoVariant {
    RefThing(RecordId),
    Str(String),
    Num(u64),
}

impl<'de> Deserialize<'de> for RefnoVariant {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de;
        use serde::de::Visitor;
        struct RefnoVariantVisitor;

        impl<'de> Visitor<'de> for RefnoVariantVisitor {
            type Value = RefnoVariant;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a RecordId, a string, or an unsigned integer")
            }

            fn visit_str<E>(self, value: &str) -> Result<RefnoVariant, E>
            where
                E: de::Error,
            {
                Ok(RefnoVariant::Str(value.to_owned()))
            }

            fn visit_string<E>(self, value: String) -> Result<RefnoVariant, E>
            where
                E: de::Error,
            {
                Ok(RefnoVariant::Str(value))
            }

            fn visit_u64<E>(self, value: u64) -> Result<RefnoVariant, E>
            where
                E: de::Error,
            {
                Ok(RefnoVariant::Num(value))
            }

            fn visit_map<A>(self, map: A) -> Result<RefnoVariant, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                // 尝试将map反序列化为Thing
                let thing = RecordId::deserialize(de::value::MapAccessDeserializer::new(map))?;
                dbg!(&thing);
                Ok(RefnoVariant::RefThing(thing))
            }
        }

        deserializer.deserialize_any(RefnoVariantVisitor)
    }
}

impl<'de> Deserialize<'de> for RefU64 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        if let Ok(s) = RefnoVariant::deserialize(deserializer) {
            dbg!(&s);
            match s {
                RefnoVariant::RefThing(s) => Ok(s.into()),
                RefnoVariant::Str(s) => Self::from_str(s.as_str())
                    .map_err(|_| serde::de::Error::custom("refno parse string error")),
                RefnoVariant::Num(d) => Ok(Self(d)),
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

impl From<RecordId> for RefU64 {
    fn from(record: RecordId) -> Self {
        let raw = record.to_raw();
        match record.key {
            RecordIdKey::String(key) => RefU64::from(key),
            RecordIdKey::Number(num) => RefU64(num as u64),
            _ => raw.as_str().into(),
        }
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

impl Into<u64> for RefU64 {
    fn into(self) -> u64 {
        self.0
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

// impl BytesTrait for RefU64 {
//     fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
//         Ok(self.0.to_be_bytes().to_vec().into())
//     }
//
//     fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
//         Ok(Self(u64::from_be_bytes(bytes[..8].try_into()?)))
//     }
// }

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
        self.to_table_key("pe")
    }

    #[inline]
    pub fn to_pe_thing(&self) -> RecordId {
        ("pe".to_string(), self.to_string()).into_record_id()
    }

    #[inline]
    pub fn to_pe_versioned_key(&self, version: i32) -> String {
        format!("pe_v:{}_{}", &self.to_string(), version)
    }

    #[inline]
    pub fn to_pbs_key(&self) -> String {
        format!("pbs:{}", &self.0.to_string())
    }

    #[inline]
    pub fn to_pbs_thing(&self) -> RecordId {
        ("pbs".to_string(), self.to_string()).into_record_id()
    }

    pub fn to_type_key(&self, noun: &str) -> String {
        format!("{}:{}", noun, &self.to_string())
    }

    #[inline]
    pub fn to_inst_relate_key(&self) -> String {
        self.to_table_key("inst_relate")
    }

    #[inline]
    pub fn to_inst_relate_history_key(&self, version: i32) -> String {
        format!("inst_relate:{}_{}", &self.to_string(), version)
    }

    #[inline]
    pub fn to_table_key(&self, tbl: &str) -> String {
        format!("{tbl}:{}", &self.to_string())
    }

    #[inline]
    pub fn to_table_history_key(&self, tbl: &str, version: i32) -> String {
        format!("{tbl}_v:{}_{}", &self.to_string(), version)
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
    pub fn to_e3d_id(&self) -> String {
        format!("={}/{}", self.get_0(), self.get_1())
    }

    #[inline]
    pub fn from_two_nums(n: u32, m: u32) -> Self {
        Self(((n as u64) << 32) + m as u64)
    }

    #[inline]
    pub fn to_array_id(&self) -> String {
        format!("['{}']", self.to_string())
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

use anyhow::anyhow;
#[cfg(feature = "sea-orm")]
use sea_orm::sea_query::ValueType;
use std::string::String;
use surrealdb::types::{RecordId, RecordIdKey, SurrealValue, ToSql, Value};

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

/// 参考号和 sesno 的对应关系
#[derive(
    Default,
    Debug,
    Clone,
    Copy,
    Eq,
    PartialEq,
    Hash,
    Serialize,
    Deserialize,
    Ord,
    PartialOrd,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
    SurrealValue,
)]
pub struct RefnoSesno {
    pub refno: RefU64,
    pub sesno: u32,
}

impl RefnoSesno {
    pub fn new(refno: RefU64, sesno: u32) -> Self {
        Self { refno, sesno }
    }

    pub fn to_pe_key(&self) -> String {
        if self.sesno == 0 {
            self.refno.to_pe_key()
        } else {
            format!("pe:{}", self.to_string())
        }
    }

    #[inline]
    pub fn to_table_key(&self, tbl: &str) -> String {
        format!("{tbl}:{}", self.to_string())
    }
}

impl From<&str> for RefnoSesno {
    fn from(value: &str) -> Self {
        serde_json::from_str(value).unwrap_or_default()
    }
}

impl ToString for RefnoSesno {
    fn to_string(&self) -> String {
        format!("['{}',{}]", self.refno.to_string(), self.sesno)
    }
}

impl From<(String, u32)> for RefnoSesno {
    fn from(value: (String, u32)) -> Self {
        let refno = RefU64::from_str(&value.0).unwrap_or_default();
        Self::new(refno, value.1)
    }
}

impl Into<RefU64> for RefnoSesno {
    fn into(self) -> RefU64 {
        self.refno
    }
}

impl Into<u32> for RefnoSesno {
    fn into(self) -> u32 {
        self.sesno
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    Eq,
    PartialEq,
    Hash,
    Serialize,
    Ord,
    PartialOrd,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
    Component,
)]
#[serde(untagged)]
pub enum RefnoEnum {
    Refno(RefU64),
    SesRef(RefnoSesno),
}

impl Default for RefnoEnum {
    fn default() -> Self {
        RefnoEnum::Refno(Default::default())
    }
}

impl SurrealValue for RefnoEnum {
    fn kind_of() -> surrealdb_types::Kind {
        // RefnoEnum can be either a RecordId or an Array [RecordId, sesno]
        // Using Kind::Record to indicate it's primarily stored as pe table records
        surrealdb_types::Kind::Record(vec!["pe".to_string()])
    }

    fn into_value(self) -> surrealdb_types::Value {
        match self {
            RefnoEnum::Refno(refno) => {
                // Convert to RecordId for simple refno
                surrealdb_types::Value::RecordId(refno.to_pe_thing())
            }
            RefnoEnum::SesRef(ses_ref) => {
                // Convert to Array [RecordId, sesno] for versioned refno
                let refno_value = surrealdb_types::Value::RecordId(ses_ref.refno.to_pe_thing());
                let sesno_value = surrealdb_types::Value::Number(surrealdb_types::Number::Int(
                    ses_ref.sesno as i64,
                ));
                surrealdb_types::Value::Array(vec![refno_value, sesno_value].into())
            }
        }
    }

    fn from_value(value: surrealdb_types::Value) -> anyhow::Result<Self> {
        match value {
            surrealdb_types::Value::RecordId(rid) => {
                // RecordId can be: pe:123_456 or pe:["123_456", 12]
                match rid.key {
                    RecordIdKey::String(key) => {
                        // Direct string key: pe:123_456 -> use from_str
                        RefnoEnum::from_str(&key)
                            .map_err(|_| anyhow::anyhow!("无法解析 RecordId 字符串 key: {}", key))
                    }
                    RecordIdKey::Number(num) => {
                        // Number key -> Refno variant
                        Ok(RefnoEnum::Refno(RefU64(num as u64)))
                    }
                    RecordIdKey::Array(arr) => {
                        // Array key: pe:["123_456", 12] -> join with comma and use from_str
                        if arr.is_empty() {
                            return Err(anyhow::anyhow!("RecordId Array key 为空"));
                        }
                        
                        // Convert array elements to strings and join with comma
                        let parts: Vec<String> = arr
                            .iter()
                            .map(|v| match v {
                                surrealdb_types::Value::String(s) => s.clone(),
                                surrealdb_types::Value::Number(n) => n.to_string(),
                                _ => {
                                    let mut s = String::new();
                                    v.fmt_sql(&mut s);
                                    s
                                }
                            })
                            .collect();
                        
                        let joined = parts.join(",");
                        RefnoEnum::from_str(&joined)
                            .map_err(|_| anyhow::anyhow!("无法解析 RecordId Array key: {}", joined))
                    }
                    _ => {
                        // Fallback: use to_raw()
                        let raw = rid.to_raw();
                        RefnoEnum::from_str(&raw)
                            .map_err(|_| anyhow::anyhow!("无法解析 RecordId: {}", raw))
                    }
                }
            }
            surrealdb_types::Value::String(s) => {
                // Parse string to RefnoEnum
                RefnoEnum::from_str(&s)
                    .map_err(|_| anyhow::anyhow!("无法解析字符串为 RefnoEnum: {}", s))
            }
            surrealdb_types::Value::Number(n) => {
                // Number -> Refno variant
                Ok(RefnoEnum::Refno(RefU64(n.to_int().unwrap_or(0) as u64)))
            }
            surrealdb_types::Value::Array(arr) => {
                // Array [refno, sesno] -> SesRef variant
                if arr.is_empty() {
                    return Err(anyhow::anyhow!("空数组无法转换为 RefnoEnum"));
                }

                // Parse first element as refno
                let refno = RefU64::from_value(arr[0].clone())
                    .map_err(|e| anyhow::anyhow!("无法解析数组第一个元素为 RefU64: {}", e))?;

                // If array has second element, parse it as sesno
                if arr.len() >= 2 {
                    let sesno = match &arr[1] {
                        surrealdb_types::Value::Number(n) => n.to_int().unwrap_or(0) as u32,
                        surrealdb_types::Value::String(s) => s.parse::<u32>().unwrap_or(0),
                        _ => 0,
                    };

                    if sesno > 0 {
                        Ok(RefnoEnum::SesRef(RefnoSesno::new(refno, sesno)))
                    } else {
                        Ok(RefnoEnum::Refno(refno))
                    }
                } else {
                    // Single element array -> Refno variant
                    Ok(RefnoEnum::Refno(refno))
                }
            }
            _ => Err(anyhow::anyhow!(
                "不支持的值类型转换为 RefnoEnum: {:?}",
                value
            )),
        }
    }
}

impl From<&str> for RefnoEnum {
    fn from(value: &str) -> Self {
        let value = value.trim();
        if value.starts_with('[') {
            RefnoEnum::SesRef(RefnoSesno::from(value))
        } else if value.contains(",") {
            let v: Vec<&str> = value.split(',').collect();
            if v.len() == 2 {
                let refno = RefU64::from_str(v[0]).unwrap_or_default();
                let sesno: u32 = v[1].parse().unwrap_or_default();
                RefnoEnum::SesRef(RefnoSesno::new(refno, sesno))
            } else {
                RefnoEnum::default()
            }
        } else {
            RefnoEnum::Refno(RefU64::from_str(value).unwrap_or_default())
        }
    }
}

impl FromStr for RefnoEnum {
    type Err = ParseRefU64Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(RefnoEnum::from(s))
    }
}

//实现 deserialize
impl<'de> Deserialize<'de> for RefnoEnum {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de;
        use serde::de::Visitor;
        use serde_json::Value;

        struct RefnoEnumVisitor;

        impl<'de> Visitor<'de> for RefnoEnumVisitor {
            type Value = RefnoEnum;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str(
                    "a RefnoEnum (number, string, RecordId, or {refno: number, sesno: number})",
                )
            }

            fn visit_u64<E>(self, value: u64) -> Result<RefnoEnum, E>
            where
                E: de::Error,
            {
                Ok(RefnoEnum::Refno(RefU64(value)))
            }

            fn visit_i64<E>(self, value: i64) -> Result<RefnoEnum, E>
            where
                E: de::Error,
            {
                if value >= 0 {
                    Ok(RefnoEnum::Refno(RefU64(value as u64)))
                } else {
                    Err(de::Error::custom(
                        "negative numbers are not valid for RefnoEnum",
                    ))
                }
            }

            fn visit_str<E>(self, value: &str) -> Result<RefnoEnum, E>
            where
                E: de::Error,
            {
                RefU64::from_str(value).map(|x| x.into()).map_err(|_| {
                    de::Error::custom(format!("RefnoEnum parse string error: {}", value))
                })
            }

            fn visit_string<E>(self, value: String) -> Result<RefnoEnum, E>
            where
                E: de::Error,
            {
                self.visit_str(&value)
            }

            fn visit_map<A>(self, map: A) -> Result<RefnoEnum, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let value =
                    JsonValue::deserialize(MapAccessDeserializer::new(map)).map_err(|err| {
                        de::Error::custom(format!("failed to deserialize map: {err}"))
                    })?;

                if let JsonValue::Object(mut obj) = value {
                    let sesno = obj
                        .get("sesno")
                        .cloned()
                        .map(parse_sesno_value::<A::Error>)
                        .transpose()?
                        .unwrap_or(0);

                    if let Some(refno_value) = obj.get("refno").cloned() {
                        let refno = parse_refno_value::<A::Error>(refno_value)?;
                        return if sesno > 0 {
                            Ok(RefnoEnum::SesRef(RefnoSesno::new(refno, sesno)))
                        } else {
                            Ok(RefnoEnum::Refno(refno))
                        };
                    }

                    if obj.contains_key("tb") && obj.contains_key("id") {
                        let refno = parse_refno_value::<A::Error>(
                            obj.get("id").cloned().unwrap_or(JsonValue::Null),
                        )?;
                        return if sesno > 0 {
                            Ok(RefnoEnum::SesRef(RefnoSesno::new(refno, sesno)))
                        } else {
                            Ok(RefnoEnum::Refno(refno))
                        };
                    }

                    if let Some(id_value) = obj.get("id").cloned() {
                        let refno = parse_refno_value::<A::Error>(id_value)?;
                        return if sesno > 0 {
                            Ok(RefnoEnum::SesRef(RefnoSesno::new(refno, sesno)))
                        } else {
                            Ok(RefnoEnum::Refno(refno))
                        };
                    }

                    return Err(de::Error::missing_field("refno"));
                }

                Err(de::Error::custom(
                    "expected object to deserialize RefnoEnum",
                ))
            }

            fn visit_seq<A>(self, seq: A) -> Result<RefnoEnum, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut seq = seq;
                let mut values = Vec::new();

                while let Some(value) = seq.next_element::<JsonValue>()? {
                    values.push(value);
                }

                if values.len() == 2 {
                    // 尝试解析为 [refno, sesno] 格式
                    let refno_val = match &values[0] {
                        JsonValue::Number(n) => n
                            .as_u64()
                            .ok_or_else(|| de::Error::custom("refno must be a positive number"))?,
                        JsonValue::String(s) => {
                            RefU64::from_str(s)
                                .map_err(|_| de::Error::custom("invalid refno string"))?
                                .0
                        }
                        JsonValue::Object(obj) => {
                            // 处理 {refno: number, sesno: number} 格式
                            if let Some(refno_field) = obj.get("refno") {
                                match refno_field {
                                    JsonValue::Number(n) => n.as_u64().ok_or_else(|| {
                                        de::Error::custom("refno must be a positive number")
                                    })?,
                                    JsonValue::String(s) => {
                                        RefU64::from_str(s)
                                            .map_err(|_| de::Error::custom("invalid refno string"))?
                                            .0
                                    }
                                    _ => {
                                        return Err(de::Error::custom(
                                            "refno must be a number or string",
                                        ));
                                    }
                                }
                            } else {
                                return Err(de::Error::custom("missing refno field"));
                            }
                        }
                        _ => {
                            return Err(de::Error::custom(
                                "refno must be a number, string, or object",
                            ));
                        }
                    };

                    let sesno_val = match &values[1] {
                        JsonValue::Number(n) => n
                            .as_u64()
                            .ok_or_else(|| de::Error::custom("sesno must be a positive number"))?
                            as u32,
                        _ => return Err(de::Error::custom("sesno must be a number")),
                    };

                    Ok(RefnoEnum::SesRef(RefnoSesno::new(
                        RefU64(refno_val),
                        sesno_val,
                    )))
                } else if values.len() == 1 {
                    // 尝试解析为单个值
                    match &values[0] {
                        JsonValue::Number(n) => {
                            let val = n
                                .as_u64()
                                .ok_or_else(|| de::Error::custom("number must be positive"))?;
                            Ok(RefnoEnum::Refno(RefU64(val)))
                        }
                        JsonValue::String(s) => RefU64::from_str(s)
                            .map(|x| x.into())
                            .map_err(|_| de::Error::custom("invalid refno string")),
                        JsonValue::Object(obj) => {
                            // 处理 {refno: number, sesno: number} 格式
                            let refno_val = if let Some(refno_field) = obj.get("refno") {
                                match refno_field {
                                    JsonValue::Number(n) => n.as_u64().ok_or_else(|| {
                                        de::Error::custom("refno must be a positive number")
                                    })?,
                                    JsonValue::String(s) => {
                                        RefU64::from_str(s)
                                            .map_err(|_| de::Error::custom("invalid refno string"))?
                                            .0
                                    }
                                    _ => {
                                        return Err(de::Error::custom(
                                            "refno must be a number or string",
                                        ));
                                    }
                                }
                            } else {
                                return Err(de::Error::custom("missing refno field"));
                            };

                            let sesno_val = if let Some(sesno_field) = obj.get("sesno") {
                                match sesno_field {
                                    JsonValue::Number(n) => n.as_u64().ok_or_else(|| {
                                        de::Error::custom("sesno must be a positive number")
                                    })?
                                        as u32,
                                    _ => return Err(de::Error::custom("sesno must be a number")),
                                }
                            } else {
                                0 // 默认sesno为0
                            };

                            if sesno_val == 0 {
                                Ok(RefnoEnum::Refno(RefU64(refno_val)))
                            } else {
                                Ok(RefnoEnum::SesRef(RefnoSesno::new(
                                    RefU64(refno_val),
                                    sesno_val,
                                )))
                            }
                        }
                        _ => Err(de::Error::custom("unsupported value type in array")),
                    }
                } else {
                    Err(de::Error::custom(
                        "RefnoEnum array must have 1 or 2 elements",
                    ))
                }
            }
        }

        deserializer.deserialize_any(RefnoEnumVisitor)
    }
}

fn parse_refno_value<E>(value: JsonValue) -> Result<RefU64, E>
where
    E: de::Error,
{
    match value {
        JsonValue::Number(n) => {
            let raw = n
                .as_u64()
                .ok_or_else(|| E::custom("refno must be a positive number"))?;
            Ok(RefU64(raw))
        }
        JsonValue::String(s) => {
            RefU64::from_str(&s).map_err(|_| E::custom(format!("invalid refno string: {s}")))
        }
        JsonValue::Object(mut obj) => {
            if obj.contains_key("tb") && obj.contains_key("id") {
                let record_id: RecordId = serde_json::from_value(JsonValue::Object(obj))
                    .map_err(|err| E::custom(format!("invalid record id object: {err}")))?;
                Ok(RefnoEnum::from(record_id).refno())
            } else if let Some(inner) = obj.remove("refno") {
                parse_refno_value(inner)
            } else if let Some(inner) = obj.remove("id") {
                parse_refno_value(inner)
            } else {
                Err(E::custom("refno object missing required fields"))
            }
        }
        JsonValue::Array(mut arr) => {
            if let Some(first) = arr.into_iter().next() {
                parse_refno_value(first)
            } else {
                Err(E::custom("empty array cannot represent refno"))
            }
        }
        _ => Err(E::custom("unsupported refno value")),
    }
}

fn parse_sesno_value<E>(value: JsonValue) -> Result<u32, E>
where
    E: de::Error,
{
    match value {
        JsonValue::Number(n) => {
            let raw = n
                .as_u64()
                .ok_or_else(|| E::custom("sesno must be a non-negative integer"))?;
            if raw > u32::MAX as u64 {
                return Err(E::custom("sesno out of range"));
            }
            Ok(raw as u32)
        }
        JsonValue::String(s) => s
            .parse::<u32>()
            .map_err(|_| E::custom(format!("invalid sesno string: {s}"))),
        JsonValue::Null => Ok(0),
        _ => Err(E::custom("sesno must be a number or string")),
    }
}

impl RefnoEnum {
    #[inline]
    pub fn to_pe_key(&self) -> String {
        match self {
            RefnoEnum::Refno(refno) => refno.to_pe_key(),
            RefnoEnum::SesRef(ses_ref) => ses_ref.to_pe_key(),
        }
    }

    #[inline]
    pub fn sesno(&self) -> Option<u32> {
        match self {
            RefnoEnum::Refno(_) => None,
            RefnoEnum::SesRef(ses_ref) => Some(ses_ref.sesno),
        }
    }

    #[inline]
    pub fn is_history(&self) -> bool {
        self.sesno().is_some()
    }

    #[inline]
    pub fn refno(&self) -> RefU64 {
        match self {
            RefnoEnum::Refno(refno) => *refno,
            RefnoEnum::SesRef(ses_ref) => ses_ref.refno,
        }
    }

    #[inline]
    pub fn ref_refno(&self) -> &RefU64 {
        match self {
            RefnoEnum::Refno(refno) => refno,
            RefnoEnum::SesRef(ses_ref) => &ses_ref.refno,
        }
    }

    #[inline]
    pub fn to_table_key(&self, tbl: &str) -> String {
        match self {
            RefnoEnum::Refno(refno) => refno.to_table_key(tbl),
            RefnoEnum::SesRef(ses_ref) => ses_ref.to_table_key(tbl),
        }
    }

    #[inline]
    pub fn is_valid(&self) -> bool {
        self.refno().is_valid()
    }

    #[inline]
    pub fn to_inst_relate_key(&self) -> String {
        match self {
            RefnoEnum::Refno(refno) => refno.to_inst_relate_key(),
            RefnoEnum::SesRef(ses_ref) => format!("inst_relate:{}", ses_ref.to_string()),
        }
    }

    #[inline]
    pub fn latest(&self) -> Self {
        self.refno().into()
    }

    #[inline]
    pub fn is_latest(&self) -> bool {
        match self {
            RefnoEnum::Refno(_) => true,
            RefnoEnum::SesRef(_) => false,
        }
    }

    #[inline]
    pub fn to_e3d_id(&self) -> String {
        match self {
            RefnoEnum::Refno(refno) => refno.to_e3d_id(),
            RefnoEnum::SesRef(ses_ref) => ses_ref.refno.to_e3d_id(),
        }
    }

    #[inline]
    pub fn to_pdms_str(&self) -> String {
        format!("{}/{}", self.refno().get_0(), self.refno().get_1())
    }

    #[inline]
    pub fn hash_with_another_refno(&self, another_refno: RefnoEnum) -> u64 {
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        std::hash::Hash::hash(&self, &mut hash);
        std::hash::Hash::hash(&another_refno, &mut hash);
        std::hash::Hasher::finish(&hash)
    }

    #[inline]
    pub fn to_array_id(&self) -> String {
        match self {
            RefnoEnum::Refno(refno) => refno.to_array_id(),
            RefnoEnum::SesRef(ses_ref) => ses_ref.to_string(),
        }
    }

    #[inline]
    pub fn to_array_zero_id(&self) -> String {
        format!("['{}', 0]", self.refno().to_string())
    }

    #[inline]
    pub fn to_normal_str(&self) -> String {
        if self.sesno().is_some() {
            format!("{}_{}", self.refno().to_string(), self.sesno().unwrap())
        } else {
            self.refno().to_string()
        }
    }
}

impl From<RefU64> for RefnoEnum {
    fn from(value: RefU64) -> Self {
        Self::Refno(value)
    }
}

impl From<RefnoSesno> for RefnoEnum {
    fn from(value: RefnoSesno) -> Self {
        Self::SesRef(value)
    }
}

impl From<(RefU64, u32)> for RefnoEnum {
    fn from(v: (RefU64, u32)) -> Self {
        Self::SesRef(RefnoSesno::new(v.0, v.1))
    }
}

impl From<(String, u32)> for RefnoEnum {
    fn from(value: (String, u32)) -> Self {
        let refno = RefU64::from_str(&value.0).unwrap_or_default();
        Self::SesRef(RefnoSesno::new(refno, value.1))
    }
}

impl From<(&str, u32)> for RefnoEnum {
    fn from(value: (&str, u32)) -> Self {
        let refno = RefU64::from_str(value.0).unwrap_or_default();
        Self::SesRef(RefnoSesno::new(refno, value.1))
    }
}

impl From<RecordId> for RefnoEnum {
    fn from(value: RecordId) -> Self {
        match value.key.clone() {
            RecordIdKey::Array(array) => {
                let refno_raw = array
                    .get(0)
                    .map(|v| match v {
                        Value::String(s) => s.clone(),
                        Value::Number(n) => n.to_string(),
                        Value::RecordId(r) => r.to_raw(),
                        other => {
                            let mut raw = String::new();
                            other.fmt_sql(&mut raw);
                            raw
                        }
                    })
                    .unwrap_or_default();
                let refno = RefU64::from(refno_raw.as_str());
                let sesno = array
                    .get(1)
                    .and_then(|v| match v {
                        Value::Number(n) => n.to_int().map(|n| n as u32),
                        Value::String(s) => s.parse().ok(),
                        _ => None,
                    })
                    .unwrap_or_default();
                RefnoEnum::SesRef(RefnoSesno::new(refno, sesno))
            }
            _ => {
                let refno: RefU64 = value.into();
                RefnoEnum::Refno(refno)
            }
        }
    }
}

impl Into<RefU64> for RefnoEnum {
    fn into(self) -> RefU64 {
        match self {
            RefnoEnum::Refno(refno) => refno,
            RefnoEnum::SesRef(ses_ref) => ses_ref.refno,
        }
    }
}

impl Display for RefnoEnum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RefnoEnum::Refno(refno) => write!(f, "{}_{}", refno.get_0(), refno.get_1()),
            RefnoEnum::SesRef(ses_ref) => write!(
                f,
                "['{}_{}', {}]",
                ses_ref.refno.get_0(),
                ses_ref.refno.get_1(),
                ses_ref.sesno
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_ref() -> RefU64 {
        RefU64::from_two_nums(17_496, 266_203)
    }

    mod refno_enum {
        use super::{RefU64, RefnoEnum, RefnoSesno, sample_ref};
        use std::convert::TryFrom;
        use surrealdb::types::{Kind, Number, RecordId, RecordIdKey, SurrealValue, Value};

        #[test]
        fn refno_enum_deserialize_from_string() {
            let parsed: RefnoEnum =
                serde_json::from_str("\"17496/266203\"").expect("string deserialization failed");
            assert_eq!(parsed, RefnoEnum::Refno(sample_ref()));
        }

        #[test]
        fn refno_enum_deserialize_from_single_element_array() {
            let parsed: RefnoEnum =
                serde_json::from_str("[\"17496/266203\"]").expect("array deserialization failed");
            assert_eq!(parsed, RefnoEnum::Refno(sample_ref()));
        }

        #[test]
        fn refno_enum_deserialize_from_number_pair_array() {
            let parsed: RefnoEnum =
                serde_json::from_str("[123456, 3]").expect("array number deserialization failed");
            assert_eq!(
                parsed,
                RefnoEnum::SesRef(RefnoSesno::new(RefU64(123_456), 3))
            );
        }

        #[test]
        fn refno_enum_deserialize_from_object_with_sesno() {
            let parsed: RefnoEnum =
                serde_json::from_str("{\"refno\": \"17496/266203\", \"sesno\": 4}")
                    .expect("object deserialization failed");
            assert_eq!(parsed, RefnoEnum::SesRef(RefnoSesno::new(sample_ref(), 4)));
        }

        #[test]
        fn refno_enum_deserialize_from_object_without_sesno() {
            let parsed: RefnoEnum = serde_json::from_str("{\"refno\": \"17496/266203\"}")
                .expect("object deserialization failed");
            assert_eq!(parsed, RefnoEnum::Refno(sample_ref()));
        }

        #[test]
        fn refno_enum_deserialize_negative_number_fails() {
            let result = serde_json::from_str::<RefnoEnum>("-1");
            assert!(result.is_err());
        }

        #[test]
        fn refno_enum_from_surreal_record_id() {
            let record_id = RecordId::parse_simple("pe:1_2").expect("record id parse failed");
            let refno = RefnoEnum::try_from(record_id).expect("RefnoEnum conversion failed");

            assert_eq!(refno, RefnoEnum::Refno(RefU64::from_two_nums(1, 2)));
        }

        #[test]
        fn refno_enum_from_surreal_json_payload() {
            let payload = serde_json::json!({ "tb": "pe", "id": "1_2" });
            let record_id = RecordId::parse_simple(&format!(
                "{}:{}",
                payload["tb"].as_str().expect("missing tb"),
                payload["id"].as_str().expect("missing id")
            ))
            .expect("record id parse failed");
            let refno = RefnoEnum::try_from(record_id).expect("RefnoEnum conversion failed");

            assert_eq!(refno, RefnoEnum::Refno(RefU64::from_two_nums(1, 2)));

            let refno_from_str: RefnoEnum =
                serde_json::from_str("\"pe:1_2\"").expect("string deserialize failed");
            assert_eq!(refno_from_str, refno);
        }

        /// 测试 RefnoEnum::Refno 转换为 SurrealDB Value::RecordId
        #[test]
        fn test_refno_enum_refno_into_value() {
            let refno_enum = RefnoEnum::Refno(RefU64::from_two_nums(17496, 169982));
            let value = refno_enum.into_value();

            match &value {
                Value::RecordId(rid) => {
                    assert_eq!(rid.table, "pe");
                    match &rid.key {
                        RecordIdKey::String(s) => {
                            assert_eq!(s, "17496_169982");
                        }
                        _ => panic!("Expected String key"),
                    }
                }
                _ => panic!("Expected RecordId value for Refno variant"),
            }
        }

        /// 测试 RefnoEnum::SesRef 转换为 SurrealDB Value::Array
        #[test]
        fn test_refno_enum_sesref_into_value() {
            let refno_enum = RefnoEnum::SesRef(RefnoSesno::new(RefU64::from_two_nums(100, 200), 5));
            let value = refno_enum.into_value();

            match &value {
                Value::Array(arr) => {
                    assert_eq!(arr.len(), 2);

                    // First element should be RecordId
                    match &arr[0] {
                        Value::RecordId(rid) => {
                            assert_eq!(rid.table, "pe");
                            match &rid.key {
                                RecordIdKey::String(s) => assert_eq!(s, "100_200"),
                                _ => panic!("Expected String key in array"),
                            }
                        }
                        _ => panic!("Expected RecordId as first array element"),
                    }

                    // Second element should be sesno
                    match &arr[1] {
                        Value::Number(n) => {
                            assert_eq!(n.to_int().unwrap(), 5);
                        }
                        _ => panic!("Expected Number as second array element"),
                    }
                }
                _ => panic!("Expected Array value for SesRef variant"),
            }
        }

        /// 测试从 Value::RecordId 转换为 RefnoEnum::Refno
        #[test]
        fn test_refno_enum_from_record_id_value() {
            let record_id = RecordId {
                table: "pe".to_string(),
                key: RecordIdKey::String("17496_169982".to_string()),
            };
            let value = Value::RecordId(record_id);

            let refno_enum =
                RefnoEnum::from_value(value).expect("Failed to convert RecordId to RefnoEnum");

            assert_eq!(
                refno_enum,
                RefnoEnum::Refno(RefU64::from_two_nums(17496, 169982))
            );
        }

        /// 测试从 Value::Array 转换为 RefnoEnum::SesRef
        #[test]
        fn test_refno_enum_from_array_value() {
            let refno_value = Value::RecordId(RecordId {
                table: "pe".to_string(),
                key: RecordIdKey::String("100_200".to_string()),
            });
            let sesno_value = Value::Number(Number::Int(5));
            let array_value = Value::Array(vec![refno_value, sesno_value].into());

            let refno_enum =
                RefnoEnum::from_value(array_value).expect("Failed to convert Array to RefnoEnum");

            assert_eq!(
                refno_enum,
                RefnoEnum::SesRef(RefnoSesno::new(RefU64::from_two_nums(100, 200), 5))
            );
        }

        /// 测试从 Value::String 转换为 RefnoEnum
        #[test]
        fn test_refno_enum_from_string_value() {
            let string_value = Value::String("17496_169982".to_owned());
            let refno_enum =
                RefnoEnum::from_value(string_value).expect("Failed to convert String to RefnoEnum");

            assert_eq!(
                refno_enum,
                RefnoEnum::Refno(RefU64::from_two_nums(17496, 169982))
            );
        }

        /// 测试从 Value::Number 转换为 RefnoEnum::Refno
        #[test]
        fn test_refno_enum_from_number_value() {
            let number_value = Value::Number(Number::from(123456789i64));
            let refno_enum =
                RefnoEnum::from_value(number_value).expect("Failed to convert Number to RefnoEnum");

            assert_eq!(refno_enum, RefnoEnum::Refno(RefU64(123456789)));
        }

        /// 测试 RefnoEnum 的往返转换
        #[test]
        fn test_refno_enum_roundtrip_refno() {
            let original = RefnoEnum::Refno(RefU64::from_two_nums(17496, 169982));
            let value = original.into_value();
            let restored = RefnoEnum::from_value(value).expect("Failed to restore RefnoEnum");

            assert_eq!(original, restored);
        }

        /// 测试 RefnoEnum::SesRef 的往返转换
        #[test]
        fn test_refno_enum_roundtrip_sesref() {
            let original = RefnoEnum::SesRef(RefnoSesno::new(RefU64::from_two_nums(100, 200), 5));
            let value = original.into_value();
            let restored = RefnoEnum::from_value(value).expect("Failed to restore RefnoEnum");

            assert_eq!(original, restored);
        }

        /// 测试从单元素数组转换（应该转换为 Refno 变体）
        #[test]
        fn test_refno_enum_from_single_element_array_value() {
            let refno_value = Value::RecordId(RecordId {
                table: "pe".to_string(),
                key: RecordIdKey::String("100_200".to_string()),
            });
            let array_value = Value::Array(vec![refno_value].into());

            let refno_enum = RefnoEnum::from_value(array_value)
                .expect("Failed to convert single-element array to RefnoEnum");

            assert_eq!(
                refno_enum,
                RefnoEnum::Refno(RefU64::from_two_nums(100, 200))
            );
        }

        /// 测试从包含 sesno=0 的数组转换（应该转换为 Refno 变体）
        #[test]
        fn test_refno_enum_from_array_with_zero_sesno() {
            let refno_value = Value::RecordId(RecordId {
                table: "pe".to_string(),
                key: RecordIdKey::String("100_200".to_string()),
            });
            let sesno_value = Value::Number(Number::Int(0));
            let array_value = Value::Array(vec![refno_value, sesno_value].into());

            let refno_enum = RefnoEnum::from_value(array_value)
                .expect("Failed to convert array with zero sesno");

            assert_eq!(
                refno_enum,
                RefnoEnum::Refno(RefU64::from_two_nums(100, 200))
            );
        }

        /// 测试 kind_of 方法
        #[test]
        fn test_refno_enum_kind_of() {
            let kind = RefnoEnum::kind_of();
            match kind {
                Kind::Record(tables) => {
                    assert_eq!(tables.len(), 1);
                    assert_eq!(tables[0], "pe");
                }
                _ => panic!("Expected Kind::Record, got {:?}", kind),
            }
        }

        /// 测试从 RecordId 的 Array key 转换为 RefnoEnum
        /// 例如: pe:["123_456", 12] -> RefnoEnum::SesRef
        #[test]
        fn test_refno_enum_from_record_id_with_array_key() {
            let record_id = RecordId {
                table: "pe".to_string(),
                key: RecordIdKey::Array(vec![
                    Value::String("100_200".to_string()),
                    Value::Number(Number::Int(5)),
                ].into()),
            };
            let value = Value::RecordId(record_id);

            let refno_enum = RefnoEnum::from_value(value)
                .expect("Failed to convert RecordId with Array key to RefnoEnum");

            // Should parse as "100_200,5" which becomes SesRef(100_200, 5)
            assert_eq!(
                refno_enum,
                RefnoEnum::SesRef(RefnoSesno::new(RefU64::from_two_nums(100, 200), 5))
            );
        }

        /// 测试从 RecordId 的 String key 转换为 RefnoEnum
        /// 例如: pe:"123_456" -> RefnoEnum::Refno
        #[test]
        fn test_refno_enum_from_record_id_with_string_key() {
            let record_id = RecordId {
                table: "pe".to_string(),
                key: RecordIdKey::String("100_200".to_string()),
            };
            let value = Value::RecordId(record_id);

            let refno_enum = RefnoEnum::from_value(value)
                .expect("Failed to convert RecordId with String key");

            assert_eq!(refno_enum, RefnoEnum::Refno(RefU64::from_two_nums(100, 200)));
        }

        /// 测试从 RecordId 的 Number key 转换为 RefnoEnum
        #[test]
        fn test_refno_enum_from_record_id_with_number_key() {
            let record_id = RecordId {
                table: "pe".to_string(),
                key: RecordIdKey::Number(123456789),
            };
            let value = Value::RecordId(record_id);

            let refno_enum = RefnoEnum::from_value(value)
                .expect("Failed to convert RecordId with Number key");

            assert_eq!(refno_enum, RefnoEnum::Refno(RefU64(123456789)));
        }
    }

    mod refu64 {
        use super::RefU64;
        use surrealdb::types::{Array, Kind, Number, RecordId, RecordIdKey, SurrealValue, Value};

        #[test]
        fn refu64_from_surrealdb_record_id_value() {
            let record_id = RecordId {
                table: "pe".to_string(),
                key: RecordIdKey::String("17496_169982".to_string()),
            };

            let surreal_value = Value::RecordId(record_id);

            let refu64 = RefU64::from_value(surreal_value)
                .expect("Failed to convert RecordId Value to RefU64");

            let expected = RefU64::from_two_nums(17496, 169982);
            assert_eq!(
                refu64, expected,
                "RefU64 conversion from RecordId Value failed"
            );

            assert_eq!(refu64.to_string(), "17496_169982");
        }

        #[test]
        fn refu64_from_surrealdb_string_value() {
            let string_value = Value::String("17496/169982".to_owned());
            let refu64 =
                RefU64::from_value(string_value).expect("Failed to convert String Value to RefU64");

            assert_eq!(refu64, RefU64::from_two_nums(17496, 169982));
        }

        #[test]
        fn refu64_from_surrealdb_number_value() {
            let number_value = Value::Number(Number::from(123456789i64));
            let refu64 =
                RefU64::from_value(number_value).expect("Failed to convert Number Value to RefU64");

            assert_eq!(refu64, RefU64(123456789));
        }

        /// 测试 RefU64 序列化为 Value::RecordId
        #[test]
        fn test_refu64_into_value_as_record_id() {
            let refu64 = RefU64::from_two_nums(17496, 169982);

            let value = refu64.into_value();

            match &value {
                Value::RecordId(rid) => {
                    assert_eq!(rid.table, "pe");
                    match &rid.key {
                        RecordIdKey::String(s) => {
                            assert_eq!(s, "17496_169982");
                        }
                        _ => panic!("Expected String key"),
                    }
                }
                _ => panic!("Expected RecordId value, got {:?}", value),
            }
        }

        /// 测试从不同表名的 RecordId 转换
        #[test]
        fn test_refu64_from_different_table_record_id() {
            let record_id = RecordId {
                table: "pbs".to_string(),
                key: RecordIdKey::String("100_200".to_string()),
            };
            let value = Value::RecordId(record_id);
            let refu64 = RefU64::from_value(value).expect("Failed to convert");

            assert_eq!(refu64, RefU64::from_two_nums(100, 200));
        }

        /// 测试从数字类型的 RecordId key 转换
        #[test]
        fn test_refu64_from_number_key_record_id() {
            let record_id = RecordId {
                table: "pe".to_string(),
                key: RecordIdKey::Number(123456789),
            };
            let value = Value::RecordId(record_id);
            let refu64 = RefU64::from_value(value).expect("Failed to convert");

            assert_eq!(refu64, RefU64(123456789));
        }

        /// 测试 RefU64 的往返转换（序列化 -> 反序列化）
        #[test]
        fn test_refu64_roundtrip_conversion() {
            let original = RefU64::from_two_nums(17496, 169982);

            let value = original.into_value();

            let restored = RefU64::from_value(value).expect("Failed to restore");

            assert_eq!(original, restored);
            assert_eq!(original.to_string(), restored.to_string());
        }

        /// 测试 kind_of 方法返回正确的类型信息
        #[test]
        fn test_refu64_kind_of() {
            let kind = RefU64::kind_of();

            match kind {
                Kind::Record(tables) => {
                    assert_eq!(tables.len(), 1);
                    assert_eq!(tables[0], "pe");
                }
                _ => panic!("Expected Kind::Record, got {:?}", kind),
            }
        }

        /// 测试从无效的 Value 类型转换应该失败
        #[test]
        fn test_refu64_from_invalid_value_types() {
            let bool_value = Value::Bool(true);
            assert!(RefU64::from_value(bool_value).is_err());

            let none_value = Value::None;
            assert!(RefU64::from_value(none_value).is_err());

            let array_value = Value::Array(Array::new());
            assert!(RefU64::from_value(array_value).is_err());
        }

        /// 测试从格式错误的字符串转换
        #[test]
        fn test_refu64_from_invalid_string_value() {
            let invalid_string = Value::String("invalid_format".to_owned());
            let result = RefU64::from_value(invalid_string);

            assert!(result.is_err() || result.unwrap() == RefU64::default());
        }

        /// 测试实际查询场景：模拟 SurrealDB 返回 RecordId
        #[test]
        fn test_refu64_query_result_scenario() {
            let query_result =
                RecordId::parse_simple("pe:17496_169982").expect("Failed to parse RecordId");

            let value = Value::RecordId(query_result);
            let refu64 = RefU64::from_value(value).expect("Failed to convert query result");

            assert_eq!(refu64.get_0(), 17496);
            assert_eq!(refu64.get_1(), 169982);
            assert_eq!(refu64.to_string(), "17496_169982");
        }
    }
}

#[macro_export]
macro_rules! pe_key {
    ($s:expr) => {
        crate::RefnoEnum::from($s)
    };
}

#[macro_export]
macro_rules! to_table_key {
    ($refno:expr, $table:expr) => {
        $refno.to_table_key($table)
    };
}

#[macro_export]
macro_rules! to_table_keys {
    ($refnos:expr, $table:expr) => {
        $refnos
            .into_iter()
            .map(|x| x.latest().to_table_key($table))
            .collect::<Vec<_>>()
    };
}
