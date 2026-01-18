//! 属性类型转换器
//!
//! 基于 all_attr_info.json 模板将字符串属性值转换为正确的 NamedAttrValue

use crate::pdms_data::ATTR_INFO_MAP;
use crate::tool::db_tool::db1_hash;
use crate::types::named_attmap::NamedAttrMap;
use crate::types::named_attvalue::NamedAttrValue;
use anyhow::Result;
use std::collections::BTreeMap;

/// 属性类型转换器
pub struct AttrConverter;

impl AttrConverter {
    /// 根据元素类型转换属性映射
    pub fn convert_attributes(
        &self,
        noun: &str,
        raw_attrs: &BTreeMap<String, String>,
    ) -> Result<NamedAttrMap> {
        let mut named_map = NamedAttrMap::default();

        // 获取元素类型的属性模板
        let type_hash = db1_hash(noun) as i32;

        if let Some(type_attrs) = ATTR_INFO_MAP.get(&type_hash) {
            // 根据模板转换每个属性
            for (attr_name, attr_value) in raw_attrs {
                let attr_hash = db1_hash(attr_name.as_str()) as i32;
                if let Some(attr_info) = type_attrs.get(&attr_hash) {
                    // 根据属性类型转换值
                    let converted_value = self.convert_value_by_type(
                        &format!("{:?}", attr_info.att_type),
                        attr_value,
                        &self.attrval_to_json(&attr_info.default_val)?,
                    )?;

                    named_map.map.insert(attr_name.clone(), converted_value);
                } else {
                    // 未知属性，作为字符串处理
                    named_map.map.insert(
                        attr_name.clone(),
                        NamedAttrValue::StringType(attr_value.clone()),
                    );
                }
            }
        } else {
            // 未知元素类型，所有属性作为字符串处理
            for (attr_name, attr_value) in raw_attrs {
                named_map.map.insert(
                    attr_name.clone(),
                    NamedAttrValue::StringType(attr_value.clone()),
                );
            }
        }

        Ok(named_map)
    }

    /// 将 AttrVal 转换为 serde_json::Value
    fn attrval_to_json(
        &self,
        attr_val: &crate::types::attval::AttrVal,
    ) -> Result<serde_json::Value> {
        match attr_val {
            crate::types::attval::AttrVal::StringType(s) => {
                Ok(serde_json::Value::String(s.clone()))
            }
            crate::types::attval::AttrVal::IntegerType(i) => {
                Ok(serde_json::Value::Number(serde_json::Number::from(*i)))
            }
            crate::types::attval::AttrVal::DoubleType(d) => Ok(serde_json::Value::Number(
                serde_json::Number::from_f64(*d).unwrap_or(serde_json::Number::from(0)),
            )),
            crate::types::attval::AttrVal::Vec3Type(arr) => {
                let vec = serde_json::Value::Array(
                    arr.iter()
                        .map(|&v| {
                            serde_json::Value::Number(
                                serde_json::Number::from_f64(v)
                                    .unwrap_or(serde_json::Number::from(0)),
                            )
                        })
                        .collect(),
                );
                Ok(vec)
            }
            _ => Ok(serde_json::Value::String("".to_string())),
        }
    }

    /// 根据属性类型转换单个值
    fn convert_value_by_type(
        &self,
        attr_type: &str,
        value: &str,
        default_val: &serde_json::Value,
    ) -> Result<NamedAttrValue> {
        match attr_type {
            "STRING" => Ok(NamedAttrValue::StringType(value.to_string())),

            "INTEGER" => {
                if let Ok(int_val) = value.parse::<i32>() {
                    Ok(NamedAttrValue::IntegerType(int_val))
                } else {
                    // 使用默认值
                    self.extract_default_integer(default_val)
                }
            }

            "DOUBLE" => {
                if let Ok(float_val) = value.parse::<f64>() {
                    Ok(NamedAttrValue::F32Type(float_val as f32))
                } else {
                    // 使用默认值
                    self.extract_default_f32(default_val)
                }
            }

            "WORD" => Ok(NamedAttrValue::WordType(value.to_string())),

            "ELEMENT" => Ok(NamedAttrValue::ElementType(value.to_string())),

            "ORIENTATION" => {
                // 解析方向向量，格式如 "0 0 1 0"
                self.parse_orientation(value)
            }

            "INTVEC" => {
                // 解析整数向量
                self.parse_int_vector(value)
            }

            "REFNO" => {
                // 解析引用号
                self.parse_refno(value)
            }

            "BOOL" => {
                // 解析布尔值
                let bool_val = value.to_lowercase() == "true" || value == "1";
                Ok(NamedAttrValue::BoolType(bool_val))
            }

            _ => {
                // 未知类型，作为字符串处理
                Ok(NamedAttrValue::StringType(value.to_string()))
            }
        }
    }

    /// 从默认值中提取整数
    fn extract_default_integer(&self, default_val: &serde_json::Value) -> Result<NamedAttrValue> {
        if let Some(int_val) = default_val.get("IntegerType") {
            if let Some(val) = int_val.as_i64() {
                return Ok(NamedAttrValue::IntegerType(val as i32));
            }
        }
        Ok(NamedAttrValue::IntegerType(0))
    }

    /// 从默认值中提取浮点数
    fn extract_default_f32(&self, default_val: &serde_json::Value) -> Result<NamedAttrValue> {
        if let Some(float_val) = default_val.get("DoubleType") {
            if let Some(val) = float_val.as_f64() {
                return Ok(NamedAttrValue::F32Type(val as f32));
            }
        }
        Ok(NamedAttrValue::F32Type(0.0))
    }

    /// 解析方向向量
    fn parse_orientation(&self, value: &str) -> Result<NamedAttrValue> {
        // 方向向量格式: "x y z w" 或 "x y z"
        let parts: Vec<&str> = value.split_whitespace().collect();
        if parts.len() >= 3 {
            let x = parts[0].parse::<f32>().unwrap_or(0.0);
            let y = parts[1].parse::<f32>().unwrap_or(0.0);
            let z = parts[2].parse::<f32>().unwrap_or(0.0);
            let w = if parts.len() > 3 {
                parts[3].parse::<f32>().unwrap_or(0.0)
            } else {
                0.0
            };

            // 转换为 Vec3（暂时只取 x,y,z）
            Ok(NamedAttrValue::Vec3Type(glam::Vec3::new(x, y, z)))
        } else {
            Ok(NamedAttrValue::Vec3Type(glam::Vec3::default()))
        }
    }

    /// 解析整数向量
    fn parse_int_vector(&self, value: &str) -> Result<NamedAttrValue> {
        // 整数向量格式: "1 2 3 4"
        let parts: Vec<&str> = value.split_whitespace().collect();
        let int_vec: Result<Vec<i32>, _> = parts.iter().map(|s| s.parse::<i32>()).collect();

        match int_vec {
            Ok(vec) => Ok(NamedAttrValue::IntArrayType(vec)),
            Err(_) => Ok(NamedAttrValue::IntArrayType(vec![])),
        }
    }

    /// 解析引用号
    fn parse_refno(&self, value: &str) -> Result<NamedAttrValue> {
        // 引用号格式: "dbno_elno" 或纯数字
        if let Ok(refno_val) = value.parse::<u64>() {
            Ok(NamedAttrValue::RefU64Type(crate::RefU64(refno_val)))
        } else if let Some((dbnum, elno)) = value.split_once('_') {
            if let (Ok(dbno_val), Ok(elno_val)) = (dbnum.parse::<i32>(), elno.parse::<i32>()) {
                // 构造 refno: (dbnum << 32) | elno as u64
                let refno_val = ((dbno_val as u64) << 32) | (elno_val as u64);
                Ok(NamedAttrValue::RefU64Type(crate::RefU64(refno_val)))
            } else {
                Err(anyhow::anyhow!("无效的引用号格式: {}", value))
            }
        } else {
            Err(anyhow::anyhow!("无效的引用号格式: {}", value))
        }
    }
}

impl Default for AttrConverter {
    fn default() -> Self {
        Self
    }
}
