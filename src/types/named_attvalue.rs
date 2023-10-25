use glam::{bool, f32, f64, i32, Vec3};
use serde_derive::{Deserialize, Serialize};
use bevy_ecs::component::Component;
use sea_query::Value;
use serde_json::json;
use bevy_reflect::Reflect;
use crate::attval::AttrVal;
use crate::attval::AttrVal::{BoolArrayType, BoolType, DoubleArrayType, DoubleType, ElementType, IntArrayType, IntegerType, InvalidType, RefU64Array, RefU64Type, StringArrayType, StringHashType, StringType, Vec3Type, WordType};
use crate::RefU64;
use crate::tool::float_tool::f32_round_3;

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
    // EmptyValue(String),  //String 指明是什么Type
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
}

impl Into<Value> for NamedAttrValue{
    fn into(self) -> Value{
        match self {
            NamedAttrValue::IntegerType(val) => Value::Int(val.into()),
            NamedAttrValue::StringType(val) | NamedAttrValue::WordType(val) |
                NamedAttrValue::ElementType(val)  => Value::String(Some(val.into())),
            NamedAttrValue::F32Type(val) => {
                //只保留3位有效数字
                let new_val = f32_round_3(val);
                Value::Float(val.into())
            },
            NamedAttrValue::BoolType(val) => Value::Bool(val.into()),
            NamedAttrValue::RefU64Type(val) => Value::String(Some(val.to_refno_string().into())),
            NamedAttrValue::F32VecType(val) => {
                let new_val = val.into_iter().map(|x| f32_round_3(x)).collect::<Vec<_>>();
                Value::Json(Some(json!(new_val).into()))
            },
            NamedAttrValue::Vec3Type(val) => Value::Json(Some(json!(val).into())),
            NamedAttrValue::StringArrayType(val) => Value::Json(Some(json!(val).into())),
            NamedAttrValue::BoolArrayType(val) => Value::Json(Some(json!(val).into())),
            NamedAttrValue::IntArrayType(val) => Value::Json(Some(json!(val).into())),

            _ => Value::String(None),
        }
    }
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
            ElementType(d) => Self::StringType(d),
            WordType(d) => Self::StringType(d),
            RefU64Type(d) => Self::RefU64Type(d),
            StringHashType(d) => Self::IntegerType(d as i32),
            RefU64Array(d) => {
                Self::StringArrayType(d.into_iter().map(|x| x.to_url_refno()).collect())
            }
        }
    }
}

impl NamedAttrValue {
    pub fn get_val_as_reflect(&self) -> Box<dyn Reflect> {
        return match self {
            NamedAttrValue::InvalidType => Box::new("unset".to_string()),
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
            NamedAttrValue::RefU64Type(r) => Box::new(r.to_refno_string()),
        };
    }
}

impl Into<serde_json::Value> for NamedAttrValue {
    fn into(self) -> serde_json::Value {
        match self {
            NamedAttrValue::IntegerType(d) => serde_json::Value::Number(d.into()),
            NamedAttrValue::F32Type(d) => {
                serde_json::Value::Number(serde_json::Number::from_f64(d as _).unwrap())
            }
            NamedAttrValue::BoolType(b) => serde_json::Value::Bool(b),
            NamedAttrValue::StringType(s) => serde_json::Value::String(s),
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
            NamedAttrValue::RefU64Type(d) => {
                //需要结合PdmsElement来跳转
                format!("PdmsElement/{}", d.to_string()).into()
            }
            NamedAttrValue::WordType(d) => d.into(),
            _ => serde_json::Value::Null,
            // NamedAttrValue::ElementType(d) => ds.insert(k.as_str(), d),
            // NamedAttrValue::WordType(d) => ds.insert(k.as_str(), d),
            // NamedAttrValue::RefU64Type(d) => ds.insert(k.as_str(), d),
        }
    }
}
