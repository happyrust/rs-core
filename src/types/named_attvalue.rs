use crate::attval::AttrVal;
use crate::tool::float_tool::f32_round_3;
use crate::{RefU64, RefU64Vec, SurlValue};
use bevy_ecs::component::Component;
use bevy_reflect::Reflect;
use glam::{bool, f32, f64, i32, Vec3};
use num_traits::{FromPrimitive, Num, One, Signed, ToPrimitive, Zero};
#[cfg(feature="sea-orm")]
use sea_query::Value as SeaValue;
use serde_derive::{Deserialize, Serialize};
use serde_json::json;

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
    Vec3Type(Vec3),
    StringArrayType(Vec<String>),
    BoolArrayType(Vec<bool>),
    IntArrayType(Vec<i32>),
    BoolType(bool),
    ElementType(String),
    WordType(String),
    RefU64Type(RefU64),
    RefU64Array(Vec<RefU64>),
}

#[cfg(feature="sea-orm")]
impl Into<Value> for NamedAttrValue {
    fn into(self) -> Value {
        match self {
            NamedAttrValue::IntegerType(val) => Value::Int(val.into()),
            NamedAttrValue::StringType(val)
            | NamedAttrValue::WordType(val)
            | NamedAttrValue::ElementType(val) => Value::String(Some(val.into())),
            NamedAttrValue::F32Type(val) => Value::Float(val.into()),
            NamedAttrValue::BoolType(val) => Value::Bool(val.into()),
            NamedAttrValue::RefU64Type(val) => Value::String(Some(val.to_refno_string().into())),
            NamedAttrValue::F32VecType(val) => {
                let new_val = val.into_iter().map(|x| f32_round_3(x)).collect::<Vec<_>>();
                Value::Json(Some(json!(new_val).into()))
            }
            NamedAttrValue::Vec3Type(val) => Value::Json(Some(json!(val).into())),
            NamedAttrValue::StringArrayType(val) => Value::Json(Some(json!(val).into())),
            NamedAttrValue::BoolArrayType(val) => Value::Json(Some(json!(val).into())),
            NamedAttrValue::IntArrayType(val) => Value::Json(Some(json!(val).into())),

            _ => Value::String(None),
            // NamedAttrValue::InvalidType => {}
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
            RefU64Array(d) => {
                Self::StringArrayType(d.into_iter().map(|x| x.to_url_refno()).collect())
            }
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

    // #[inline]
    // pub fn string_hash_value(&self) -> Option<AiosStrHash> {
    //     return match self {
    //         StringHashType(v) => Some(v.clone()),
    //         _ => None,
    //     };
    // }

    // #[inline]
    // pub fn refu64_vec_value(&self) -> Option<RefU64Vec> {
    //     return match self {
    //         NamedAttrValue::RefU64Array(v) => Some(v.clone()),
    //         _ => None,
    //     };
    // }

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
            _ => "unset".to_string(),
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
            Self::RefU64Type(v) => v.to_refno_str().to_string(),
            // Self::StringHashType(v) => v.to_string(),
            // Self::RefU64Array(v) => serde_json::to_string(v).unwrap(),
        };
    }
}

impl NamedAttrValue {
    pub fn get_val_as_reflect(&self) -> Box<dyn Reflect> {
        return match self {
            _ => Box::new("unset".to_string()),
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
                //todo fix 为什么会有出错的情况
                //infinite ??
                serde_json::Value::Number(
                    serde_json::Number::from_f64(d as _)
                        .unwrap_or(serde_json::Number::from_f64(0.0).unwrap()),
                )
            }
            NamedAttrValue::BoolType(b) => serde_json::Value::Bool(b),
            NamedAttrValue::StringType(s)
            | NamedAttrValue::WordType(s)
            | NamedAttrValue::ElementType(s) => {
                if s.contains('\0') || s.contains("u0000") {
                    return serde_json::Value::String("".into());
                }
                serde_json::Value::String(s)
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
            NamedAttrValue::RefU64Type(d) => {
                //需要结合PdmsElement来跳转
                // format!("PdmsElement/{}", d.to_string()).into()
                d.to_string().into()
            }
            _ => serde_json::Value::Null,
            // NamedAttrValue::ElementType(d) => serde_json::Value::String(d),
            // NamedAttrValue::WordType(d) => ds.insert(k.as_str(), d),
        }
    }
}
