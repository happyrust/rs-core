use crate::RefnoEnum;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// 上游解析后的 PE 基础信息
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PeRecord {
    pub refno: RefnoEnum,
    pub noun: String,
    pub dbnum: i32,
    pub name: Option<String>,
    pub sesno: Option<i32>,
    /// 若已提前构建缓存，可直接写入 `pe.named_attr_json`
    pub cache_json: Option<Value>,
}

/// 强类型属性投影
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TypedAttrRecord {
    pub noun: String,
    pub refno: RefnoEnum,
    /// 字段值遵循 `attr_table_specs` 定义
    pub fields: BTreeMap<String, Value>,
}

impl TypedAttrRecord {
    pub fn new<S: Into<String>>(noun: S, refno: RefnoEnum) -> Self {
        Self {
            noun: noun.into(),
            refno,
            fields: BTreeMap::new(),
        }
    }
}

/// 外部引用或关联关系
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EdgeRecord {
    pub from: RefnoEnum,
    pub to: RefnoEnum,
    pub edge_type: EdgeType,
}

/// 关系类型枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EdgeType {
    /// `(:PE)-[:REL_ATTR]->(:Attr_<NOUN>)`
    RelAttr,
    /// `(:PE)-[:TO_<TARGET>]->(:PE)` 等外部引用
    ToNoun { target_noun: String, field: String },
    /// 所属层级关系
    Owner,
}
