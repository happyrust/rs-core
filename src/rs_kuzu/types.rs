//! Kuzu 类型映射和转换
//!
//! 提供 aios_core 类型与 Kuzu 类型之间的转换

#[cfg(feature = "kuzu")]
use crate::types::*;
#[cfg(feature = "kuzu")]
use anyhow::Result;
#[cfg(feature = "kuzu")]
use kuzu::{LogicalType, Value as KuzuValue};

#[cfg(feature = "kuzu")]
/// 将 NamedAttrValue 转换为 Kuzu Value
pub fn named_attr_to_kuzu_value(attr: &NamedAttrValue) -> Result<KuzuValue> {
    match attr {
        NamedAttrValue::IntegerType(i) => Ok(KuzuValue::Int64(*i as i64)),
        NamedAttrValue::F32Type(f) => Ok(KuzuValue::Double(*f as f64)),
        NamedAttrValue::StringType(s) => Ok(KuzuValue::String(s.clone())),
        NamedAttrValue::WordType(w) => Ok(KuzuValue::String(w.clone())),
        NamedAttrValue::BoolType(b) => Ok(KuzuValue::Bool(*b)),
        NamedAttrValue::RefU64Type(r) => Ok(KuzuValue::Int64(r.0 as i64)),
        NamedAttrValue::RefnoEnumType(r) => Ok(KuzuValue::Int64(r.refno().0 as i64)),
        NamedAttrValue::Vec3Type(v) => {
            // Vec3 存储为字符串 "[x,y,z]"
            Ok(KuzuValue::String(format!("[{},{},{}]", v.x, v.y, v.z)))
        }
        NamedAttrValue::IntArrayType(arr) => {
            // 数组转换为 JSON 字符串
            Ok(KuzuValue::String(serde_json::to_string(arr)?))
        }
        NamedAttrValue::F32VecType(arr) => Ok(KuzuValue::String(serde_json::to_string(arr)?)),
        NamedAttrValue::StringArrayType(arr) => Ok(KuzuValue::String(serde_json::to_string(arr)?)),
        _ => Err(anyhow::anyhow!("不支持的属性类型转换")),
    }
}

#[cfg(feature = "kuzu")]
/// 将 Kuzu Value 转换为 NamedAttrValue
pub fn kuzu_value_to_named_attr(value: &KuzuValue, attr_type: &str) -> Result<NamedAttrValue> {
    match attr_type.to_uppercase().as_str() {
        "INT" | "INTEGER" | "I32" => {
            if let KuzuValue::Int64(i) = value {
                Ok(NamedAttrValue::IntegerType(*i as i32))
            } else {
                Err(anyhow::anyhow!("类型不匹配: 期望 INT"))
            }
        }
        "FLOAT" | "F32" | "REAL" => {
            if let KuzuValue::Double(f) = value {
                Ok(NamedAttrValue::F32Type(*f as f32))
            } else {
                Err(anyhow::anyhow!("类型不匹配: 期望 FLOAT"))
            }
        }
        "STRING" | "TEXT" => {
            if let KuzuValue::String(s) = value {
                Ok(NamedAttrValue::StringType(s.clone()))
            } else {
                Err(anyhow::anyhow!("类型不匹配: 期望 STRING"))
            }
        }
        "WORD" => {
            if let KuzuValue::String(s) = value {
                Ok(NamedAttrValue::WordType(s.clone()))
            } else {
                Err(anyhow::anyhow!("类型不匹配: 期望 WORD"))
            }
        }
        "BOOL" | "BOOLEAN" => {
            if let KuzuValue::Bool(b) = value {
                Ok(NamedAttrValue::BoolType(*b))
            } else {
                Err(anyhow::anyhow!("类型不匹配: 期望 BOOL"))
            }
        }
        "REFNO" | "REF" => {
            if let KuzuValue::Int64(i) = value {
                Ok(NamedAttrValue::RefU64Type(RefU64(*i as u64)))
            } else {
                Err(anyhow::anyhow!("类型不匹配: 期望 REFNO"))
            }
        }
        "VEC3" | "POSITION" => {
            if let KuzuValue::String(s) = value {
                // 解析 "[x,y,z]" 格式
                let s = s.trim_matches(|c| c == '[' || c == ']');
                let parts: Vec<&str> = s.split(',').collect();
                if parts.len() == 3 {
                    let x = parts[0].parse::<f32>()?;
                    let y = parts[1].parse::<f32>()?;
                    let z = parts[2].parse::<f32>()?;
                    Ok(NamedAttrValue::Vec3Type(glam::Vec3::new(x, y, z)))
                } else {
                    Err(anyhow::anyhow!("VEC3 格式错误"))
                }
            } else {
                Err(anyhow::anyhow!("类型不匹配: 期望 VEC3"))
            }
        }
        "I32ARRAY" => {
            if let KuzuValue::String(s) = value {
                let arr: Vec<i32> = serde_json::from_str(s)?;
                Ok(NamedAttrValue::IntArrayType(arr))
            } else {
                Err(anyhow::anyhow!("类型不匹配: 期望 I32ARRAY"))
            }
        }
        "F32ARRAY" => {
            if let KuzuValue::String(s) = value {
                let arr: Vec<f32> = serde_json::from_str(s)?;
                Ok(NamedAttrValue::F32VecType(arr))
            } else {
                Err(anyhow::anyhow!("类型不匹配: 期望 F32ARRAY"))
            }
        }
        _ => Err(anyhow::anyhow!("未知属性类型: {}", attr_type)),
    }
}

#[cfg(feature = "kuzu")]
/// 获取 NamedAttrValue 对应的 Kuzu LogicalType
pub fn get_kuzu_logical_type(attr: &NamedAttrValue) -> LogicalType {
    match attr {
        NamedAttrValue::IntegerType(_) => LogicalType::Int64,
        NamedAttrValue::F32Type(_) => LogicalType::Double,
        NamedAttrValue::StringType(_) => LogicalType::String,
        NamedAttrValue::WordType(_) => LogicalType::String,
        NamedAttrValue::BoolType(_) => LogicalType::Bool,
        NamedAttrValue::RefU64Type(_) => LogicalType::Int64,
        NamedAttrValue::RefnoEnumType(_) => LogicalType::Int64,
        _ => LogicalType::String, // 默认使用 String
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn test_attr_to_kuzu_conversion() {
        let attr = NamedAttrValue::IntegerType(42);
        let kuzu_val = named_attr_to_kuzu_value(&attr).unwrap();
        assert!(matches!(kuzu_val, KuzuValue::Int64(42)));

        let attr = NamedAttrValue::StringType("test".to_string());
        let kuzu_val = named_attr_to_kuzu_value(&attr).unwrap();
        assert!(matches!(kuzu_val, KuzuValue::String(s) if s == "test"));
    }

    #[test]
    fn test_kuzu_to_attr_conversion() {
        let kuzu_val = KuzuValue::Int64(42);
        let attr = kuzu_value_to_named_attr(&kuzu_val, "INT").unwrap();
        assert!(matches!(attr, NamedAttrValue::IntegerType(42)));

        let kuzu_val = KuzuValue::String("test".to_string());
        let attr = kuzu_value_to_named_attr(&kuzu_val, "STRING").unwrap();
        assert!(matches!(attr, NamedAttrValue::StringType(s) if s == "test"));
    }
}
