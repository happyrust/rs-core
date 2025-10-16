use crate::attval::AttrVal;
use crate::tool::float_tool::f32_round_3;
use crate::utils::{value_to_bool, value_to_f32, value_to_i32, value_to_string};
use crate::{RefU64, RefU64Vec, SurlValue};
use bevy_ecs::component::Component;
use bevy_reflect::Reflect;
use glam::{Vec3, bool, f32, f64, i32};
use num_traits::{FromPrimitive, Num, One, Signed, ToPrimitive, Zero};
#[cfg(feature = "sea-orm")]
use sea_query::Value as SeaValue;
use serde::Deserializer;
use serde::{Deserialize, Serialize};
use serde_json::json;

///新的属性数据结构
#[derive(
    Serialize,
    Deserialize,
    PartialEq,
    Clone,
    Debug,
    Component,
    Default,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
// #[serde(untagged)]
pub enum NamedAttrValue {
    #[default]
    InvalidType,
    IntegerType(i32),
    StringType(String),
    F32Type(f32),
    F32VecType(Vec<f32>),
    Vec3Type(Vec3),
    StringArrayType(Vec<String>),
    BoolArrayType(Vec<bool>),
    IntArrayType(Vec<i32>),
    BoolType(bool),
    ElementType(String),
    WordType(String),
    RefU64Type(RefU64),
    RefU64Array(Vec<RefnoEnum>),
    LongType(i64),
    RefnoEnumType(RefnoEnum),
}

use serde::de::{self, EnumAccess, MapAccess, SeqAccess, Visitor};
use std::fmt;
use std::str::FromStr;
use std::vec::Vec;
use surrealdb::types::{Array, RecordId, RecordIdKey, ToSql, Value};

use super::RefnoEnum;

impl From<(&str, Value)> for NamedAttrValue {
    fn from(tuple: (&str, Value)) -> Self {
        let (tn, value) = tuple;
        match value {
            Value::Number(val) => match tn {
                "REAL" => NamedAttrValue::F32Type(val.to_f64().unwrap_or_default() as f32),
                _ => NamedAttrValue::IntegerType(val.to_int().unwrap_or_default() as i32),
            },
            Value::Bool(val) => NamedAttrValue::BoolType(val),
            Value::String(val) => NamedAttrValue::StringType(val),
            Value::Array(val) => match tn {
                "REAL" | "DIR" | "POS" => {
                    let values = val.iter().map(value_to_f32).collect::<Vec<_>>();
                    NamedAttrValue::F32VecType(values)
                }
                "INT" => {
                    let values = val.iter().map(value_to_i32).collect::<Vec<_>>();
                    NamedAttrValue::IntArrayType(values)
                }
                "BOOL" => {
                    let values = val.iter().map(value_to_bool).collect::<Vec<_>>();
                    NamedAttrValue::BoolArrayType(values)
                }
                "TEXT" => {
                    let values = val.iter().map(value_to_string).collect::<Vec<_>>();
                    NamedAttrValue::StringArrayType(values)
                }
                "REF" => {
                    let values = val
                        .iter()
                        .map(|x| match x {
                            Value::RecordId(rid) => RefnoEnum::from(rid.clone()),
                            _ => {
                                let s = value_to_string(x);
                                RefnoEnum::from(s.as_str())
                            }
                        })
                        .collect::<Vec<_>>();
                    NamedAttrValue::RefU64Array(values)
                }
                _ => NamedAttrValue::InvalidType,
            },
            Value::RecordId(val) => match &val.key {
                RecordIdKey::Array(_) => NamedAttrValue::RefnoEnumType(RefnoEnum::from(val)),
                _ => NamedAttrValue::RefU64Type(RefU64::from(val)),
            },
            Value::Object(val) => {
                if let Some((key, v)) = val.into_iter().next() {
                    (tn, v).into()
                } else {
                    NamedAttrValue::InvalidType
                }
            }
            _ => NamedAttrValue::InvalidType,
        }
    }
}

impl From<&AttrVal> for NamedAttrValue {
    fn from(v: &AttrVal) -> Self {
        use crate::attval::AttrVal::*;
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
            ElementType(d) => Self::StringType(d),
            WordType(d) => Self::StringType(d),
            RefU64Type(d) => Self::RefU64Type(d),
            StringHashType(d) => Self::IntegerType(d as i32),
            RefU64Array(d) => Self::RefU64Array(d.into_iter().map(|x| x.into()).collect()),
        }
    }
}

impl From<AttrVal> for NamedAttrValue {
    fn from(v: AttrVal) -> Self {
        (&v).into()
    }
}

impl NamedAttrValue {
    #[inline]
    pub fn get_default_val(typ: &str) -> Self {
        match typ {
            "LOG" => Self::BoolType(false),
            "REAL" => Self::F32Type(0.0),
            "ELEMENT" => Self::RefU64Type(Default::default()),
            "TEXT" => Self::StringType("".into()),
            _ => Self::StringType("unset".into()),
        }
    }
}

impl NamedAttrValue {
    #[inline]
    pub fn i32_value(&self) -> i32 {
        return match self {
            Self::IntegerType(v) => *v,
            _ => 0,
        };
    }

    #[inline]
    pub fn i32_array_value(&self) -> Vec<i32> {
        return match self {
            Self::IntArrayType(v) => v.to_vec(),
            _ => vec![],
        };
    }

    // #[inline]
    // pub fn double_value(&self) -> Option<f64> {
    //     return match self {
    //         Self::F32Type(v) => Some(*v),
    //         _ => None,
    //     };
    // }

    #[inline]
    pub fn f32_value(&self) -> Option<f32> {
        return match self {
            Self::F32Type(v) => Some(*v),
            _ => None,
        };
    }

    #[inline]
    pub fn vec3_value(&self) -> Option<Vec3> {
        return match self {
            Self::Vec3Type(v) => Some(*v),
            _ => None,
        };
    }

    // #[inline]
    // pub fn dvec_value(&self) -> Option<Vec<f64>> {
    //     return match self {
    //         Self::DoubleArrayType(v) => Some(v.to_vec()),
    //         _ => None,
    //     };
    // }

    #[inline]
    pub fn element_value(&self) -> Option<String> {
        return match self {
            Self::ElementType(v) => Some(v.clone()),
            _ => None,
        };
    }

    #[inline]
    pub fn string_value(&self) -> String {
        return match self {
            Self::StringType(v) => v.to_string(),
            Self::WordType(v) => v.to_string(),
            _ => "unset".to_string(),
        };
    }

    #[inline]
    pub fn refno_value(&self) -> Option<RefU64> {
        return match self {
            Self::RefU64Type(v) => Some(*v),
            _ => None,
        };
    }

    #[inline]
    pub fn bool_value(&self) -> Option<bool> {
        return match self {
            Self::BoolType(v) => Some(*v),
            _ => None,
        };
    }

    #[inline]
    pub fn get_val_as_string(&self) -> String {
        return match self {
            Self::IntegerType(v) => v.to_string(),
            Self::StringType(v) => v.to_string(),
            Self::F32Type(v) => v.to_string(),
            Self::F32VecType(v) => serde_json::to_string(v).unwrap(),
            Self::StringArrayType(v) => serde_json::to_string(v).unwrap(),
            Self::BoolArrayType(v) => serde_json::to_string(v).unwrap(),
            Self::IntArrayType(v) => serde_json::to_string(v).unwrap(),
            Self::BoolType(v) => v.to_string(),
            Self::Vec3Type(v) => serde_json::to_string(v).unwrap(),
            Self::ElementType(v) => v.to_string(),
            Self::WordType(v) => v.to_string(),
            Self::RefU64Type(v) => v.to_string().to_string(),
            _ => "unset".to_string(),
        };
    }
}

impl NamedAttrValue {
    pub fn get_val_as_reflect(&self) -> Box<dyn Reflect> {
        return match self {
            NamedAttrValue::StringType(v)
            | NamedAttrValue::ElementType(v)
            | NamedAttrValue::WordType(v) => Box::new(v.to_string()),
            NamedAttrValue::BoolArrayType(v) => Box::new(v.clone()),
            NamedAttrValue::IntArrayType(v) => Box::new(v.clone()),
            NamedAttrValue::IntegerType(v) => Box::new(*v),
            NamedAttrValue::BoolType(v) => Box::new(*v),
            NamedAttrValue::StringArrayType(v) => {
                Box::new(v.iter().map(|x| x.to_string()).collect::<Vec<_>>())
            }
            NamedAttrValue::F32Type(v) => Box::new(*v),
            NamedAttrValue::F32VecType(v) => Box::new(v.clone()),
            NamedAttrValue::Vec3Type(v) => Box::new(vec![v.x, v.y, v.z]),
            NamedAttrValue::RefU64Type(r) => Box::new(r.to_slash_string()),
            _ => Box::new("unset".to_string()),
        };
    }
}

impl Into<serde_json::Value> for NamedAttrValue {
    fn into(self) -> serde_json::Value {
        match self {
            NamedAttrValue::IntegerType(d) => serde_json::Value::Number(d.into()),
            NamedAttrValue::F32Type(d) => serde_json::Value::Number(
                serde_json::Number::from_f64(d as _)
                    .unwrap_or(serde_json::Number::from_f64(0.0).unwrap()),
            ),
            NamedAttrValue::BoolType(b) => serde_json::Value::Bool(b),
            NamedAttrValue::StringType(s) | NamedAttrValue::WordType(s) => {
                if s.contains('\0') || s.contains("u0000") {
                    return serde_json::Value::String("".into());
                }
                serde_json::Value::String(s)
            }
            NamedAttrValue::ElementType(s) => {
                if s.contains('\0') || s.contains("u0000") {
                    return serde_json::Value::String("".into());
                }
                serde_json::Value::String(format!("pe:{}", s))
            }
            NamedAttrValue::RefU64Type(d) => {
                serde_json::Value::String(format!("pe:{}", d.to_string()))
            }
            NamedAttrValue::F32VecType(d) => {
                serde_json::Value::Array(d.into_iter().map(|x| x.into()).collect())
            }
            NamedAttrValue::Vec3Type(d) => {
                serde_json::Value::Array(d.to_array().into_iter().map(|x| x.into()).collect())
            }
            NamedAttrValue::StringArrayType(d) => {
                serde_json::Value::Array(d.into_iter().map(|x| x.into()).collect())
            }
            NamedAttrValue::BoolArrayType(d) => {
                serde_json::Value::Array(d.into_iter().map(|x| x.into()).collect())
            }
            NamedAttrValue::IntArrayType(d) => {
                serde_json::Value::Array(d.into_iter().map(|x| x.into()).collect())
            }
            _ => serde_json::Value::Null,
        }
    }
}

impl From<NamedAttrValue> for SurlValue {
    fn from(value: NamedAttrValue) -> Self {
        use surrealdb::types::Number;

        match value {
            NamedAttrValue::IntegerType(v) => SurlValue::Number(Number::Int(v as i64)),
            NamedAttrValue::LongType(v) => SurlValue::Number(Number::Int(v)),
            NamedAttrValue::StringType(v)
            | NamedAttrValue::WordType(v)
            | NamedAttrValue::ElementType(v) => SurlValue::String(v),
            NamedAttrValue::F32Type(v) => SurlValue::Number(Number::Float(v as f64)),
            NamedAttrValue::BoolType(v) => SurlValue::Bool(v),
            NamedAttrValue::F32VecType(v) => {
                let arr: Vec<SurlValue> = v
                    .into_iter()
                    .map(|x| SurlValue::Number(Number::Float(x as f64)))
                    .collect();
                SurlValue::Array(arr.into())
            }
            NamedAttrValue::Vec3Type(v) => {
                let arr: Vec<SurlValue> = v
                    .to_array()
                    .into_iter()
                    .map(|x| SurlValue::Number(Number::Float(x as f64)))
                    .collect();
                SurlValue::Array(arr.into())
            }
            NamedAttrValue::StringArrayType(v) => {
                let arr: Vec<SurlValue> = v.into_iter().map(SurlValue::String).collect();
                SurlValue::Array(arr.into())
            }
            NamedAttrValue::BoolArrayType(v) => {
                let arr: Vec<SurlValue> = v.into_iter().map(SurlValue::Bool).collect();
                SurlValue::Array(arr.into())
            }
            NamedAttrValue::IntArrayType(v) => {
                let arr: Vec<SurlValue> = v
                    .into_iter()
                    .map(|x| SurlValue::Number(Number::Int(x as i64)))
                    .collect();
                SurlValue::Array(arr.into())
            }
            NamedAttrValue::RefU64Type(v) => SurlValue::RecordId(v.to_pe_thing()),
            NamedAttrValue::RefnoEnumType(v) => SurlValue::RecordId(v.to_pe_thing()),
            NamedAttrValue::RefU64Array(v) => {
                let arr: Vec<SurlValue> = v
                    .into_iter()
                    .map(|x| SurlValue::RecordId(x.to_pe_thing()))
                    .collect();
                SurlValue::Array(arr.into())
            }
            NamedAttrValue::InvalidType => SurlValue::None,
        }
    }
}
