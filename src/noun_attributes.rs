//! PDMS Noun 属性查询模块
//!
//! 提供 noun → attributes 的映射查询功能。
//! 数据来源：
//! - attlib.dat: 属性定义（类型、默认值）
//! - JSON 文件: 预导出的 noun 属性映射

use crate::attlib_parser::{decode_base27, AttlibAttribute, AttlibDataType, AttlibDefaultValue, AttlibParser};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// 属性类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum AttributeType {
    Integer,
    Double,
    Bool,
    String,
    Word,
    Element,
    Position,
    Orientation,
    Direction,
    /// 整数数组
    Intvec,
    /// 实数数组
    Realvec,
    /// 文本
    Text,
    /// 引用
    Ref,
    #[serde(other)]
    Unknown,
}

impl std::fmt::Display for AttributeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AttributeType::Integer => write!(f, "INTEGER"),
            AttributeType::Double => write!(f, "DOUBLE"),
            AttributeType::Bool => write!(f, "BOOL"),
            AttributeType::String => write!(f, "STRING"),
            AttributeType::Word => write!(f, "WORD"),
            AttributeType::Element => write!(f, "ELEMENT"),
            AttributeType::Position => write!(f, "POSITION"),
            AttributeType::Orientation => write!(f, "ORIENTATION"),
            AttributeType::Direction => write!(f, "DIRECTION"),
            AttributeType::Intvec => write!(f, "INTVEC"),
            AttributeType::Realvec => write!(f, "REALVEC"),
            AttributeType::Text => write!(f, "TEXT"),
            AttributeType::Ref => write!(f, "REF"),
            AttributeType::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

/// 属性信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeInfo {
    pub name: String,
    pub hash: u32,
    pub offset: u32,
    pub att_type: AttributeType,
    #[serde(default)]
    pub default_val: serde_json::Value,
}

/// 属性描述信息（叠加 attlib.dat 元数据）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeDesc {
    pub name: String,
    pub hash: u32,
    pub offset: u32,
    pub att_type: AttributeType,
    #[serde(default)]
    pub default_val: serde_json::Value,
    /// attlib.dat 中的基础数据类型（LOG/REAL/INT/TEXT）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attlib_type: Option<String>,
    /// attlib.dat 中的默认值（已格式化）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attlib_default: Option<String>,
}

impl From<&AttributeInfo> for AttributeDesc {
    fn from(info: &AttributeInfo) -> Self {
        Self {
            name: info.name.clone(),
            hash: info.hash,
            offset: info.offset,
            att_type: info.att_type,
            default_val: info.default_val.clone(),
            attlib_type: None,
            attlib_default: None,
        }
    }
}

impl AttributeDesc {
    fn with_attlib(mut self, att: &AttlibAttribute) -> Self {
        self.attlib_type = Some(att.data_type.name().to_string());
        self.attlib_default = format_attlib_default(att);
        self
    }
}

fn format_attlib_default(att: &AttlibAttribute) -> Option<String> {
    match (&att.data_type, &att.default_value) {
        (_, AttlibDefaultValue::None) => None,
        (AttlibDataType::Log, AttlibDefaultValue::Scalar(v)) => Some(((*v != 0) as i32).to_string()),
        (AttlibDataType::Int, AttlibDefaultValue::Scalar(v)) => Some((*v as i32).to_string()),
        (AttlibDataType::Real, AttlibDefaultValue::Scalar(v)) => {
            Some(format!("{:.6}", f32::from_bits(*v) as f64))
        }
        (AttlibDataType::Text, AttlibDefaultValue::Text(words)) => att
            .default_text
            .clone()
            .or_else(|| Some(decode_base27(words))),
        // 兜底：保留原始数值
        (_, AttlibDefaultValue::Scalar(v)) => Some(v.to_string()),
        // 非文本类型但有文本默认值时，返回 None
        (_, AttlibDefaultValue::Text(_)) => None,
    }
}

/// JSON 文件中的属性格式
#[derive(Debug, Deserialize)]
struct JsonAttributeInfo {
    name: String,
    hash: u32,
    offset: u32,
    att_type: String,
    #[serde(default)]
    default_val: serde_json::Value,
}

impl From<JsonAttributeInfo> for AttributeInfo {
    fn from(json: JsonAttributeInfo) -> Self {
        let att_type = match json.att_type.to_uppercase().as_str() {
            "INTEGER" => AttributeType::Integer,
            "DOUBLE" => AttributeType::Double,
            "BOOL" => AttributeType::Bool,
            "STRING" => AttributeType::String,
            "WORD" => AttributeType::Word,
            "ELEMENT" => AttributeType::Element,
            "POSITION" => AttributeType::Position,
            "ORIENTATION" => AttributeType::Orientation,
            "DIRECTION" => AttributeType::Direction,
            "INTVEC" => AttributeType::Intvec,
            "REALVEC" => AttributeType::Realvec,
            "TEXT" => AttributeType::Text,
            "REF" | "REFERENCE" => AttributeType::Ref,
            _ => AttributeType::Unknown,
        };
        AttributeInfo {
            name: json.name,
            hash: json.hash,
            offset: json.offset,
            att_type,
            default_val: json.default_val,
        }
    }
}

/// all_attr_info.json 的格式
#[derive(Debug, Deserialize)]
struct AllAttrInfoFile {
    noun_attr_info_map: HashMap<String, HashMap<String, JsonAttributeInfo>>,
}

/// Noun 属性查询器
pub struct NounAttributeStore {
    /// noun_name -> attributes
    noun_attributes: HashMap<String, Vec<AttributeInfo>>,
    /// noun_hash -> noun_name
    noun_hash_to_name: HashMap<u32, String>,
    /// attr_hash -> attr_name
    all_attributes: HashMap<u32, String>,
}

impl NounAttributeStore {
    /// 创建空的存储
    pub fn new() -> Self {
        Self {
            noun_attributes: HashMap::new(),
            noun_hash_to_name: HashMap::new(),
            all_attributes: HashMap::new(),
        }
    }

    /// 从目录加载所有 JSON 文件
    pub fn load_from_directory<P: AsRef<Path>>(dir: P) -> Result<Self> {
        let mut store = Self::new();
        let dir = dir.as_ref();

        if !dir.exists() {
            anyhow::bail!("目录不存在: {}", dir.display());
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Err(e) = store.load_json_file(&path) {
                    eprintln!("警告: 加载 {} 失败: {}", path.display(), e);
                }
            }
        }

        Ok(store)
    }

    /// 从 all_attr_info.json 加载（推荐）
    pub fn load_from_all_attr_info<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut store = Self::new();
        let path = path.as_ref();

        let content = fs::read_to_string(path)
            .with_context(|| format!("读取文件失败: {}", path.display()))?;

        let data: AllAttrInfoFile = serde_json::from_str(&content)
            .with_context(|| format!("解析 JSON 失败: {}", path.display()))?;

        for (noun_hash_str, attrs) in data.noun_attr_info_map {
            let noun_hash: u32 = noun_hash_str.parse().unwrap_or(0);
            if noun_hash == 0 {
                continue;
            }

            // 尝试解码 noun 名称
            let noun_name = db1_dehash(noun_hash).unwrap_or_else(|| format!("NOUN_{}", noun_hash));

            let mut attr_list: Vec<AttributeInfo> = attrs
                .into_values()
                .map(AttributeInfo::from)
                .collect();

            attr_list.sort_by(|a, b| a.name.cmp(&b.name));

            // 记录所有属性
            for attr in &attr_list {
                store.all_attributes.insert(attr.hash, attr.name.clone());
            }

            store.noun_hash_to_name.insert(noun_hash, noun_name.clone());
            store.noun_attributes.insert(noun_name, attr_list);
        }

        Ok(store)
    }

    /// 加载单个 JSON 文件
    pub fn load_json_file<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)
            .with_context(|| format!("读取文件失败: {}", path.display()))?;

        // JSON 格式: { "NOUN_NAME": { "ATTR_NAME": {...}, ... } }
        let data: HashMap<String, HashMap<String, JsonAttributeInfo>> =
            serde_json::from_str(&content)
                .with_context(|| format!("解析 JSON 失败: {}", path.display()))?;

        for (noun_name, attrs) in data {
            let mut attr_list: Vec<AttributeInfo> = attrs
                .into_values()
                .map(AttributeInfo::from)
                .collect();

            // 按名称排序
            attr_list.sort_by(|a, b| a.name.cmp(&b.name));

            // 记录所有属性
            for attr in &attr_list {
                self.all_attributes.insert(attr.hash, attr.name.clone());
            }

            self.noun_attributes.insert(noun_name.to_uppercase(), attr_list);
        }

        Ok(())
    }

    /// 获取 noun 的属性列表
    pub fn get_attributes(&self, noun: &str) -> Option<&Vec<AttributeInfo>> {
        self.noun_attributes.get(&noun.to_uppercase())
    }

    /// 获取 noun 的属性名称列表
    pub fn get_attribute_names(&self, noun: &str) -> Option<Vec<&str>> {
        self.noun_attributes
            .get(&noun.to_uppercase())
            .map(|attrs| attrs.iter().map(|a| a.name.as_str()).collect())
    }

    /// 获取 noun 的特定属性
    pub fn get_attribute(&self, noun: &str, attr_name: &str) -> Option<&AttributeInfo> {
        self.noun_attributes
            .get(&noun.to_uppercase())
            .and_then(|attrs| {
                attrs
                    .iter()
                    .find(|a| a.name.eq_ignore_ascii_case(attr_name))
            })
    }

    /// 获取所有已加载的 noun 列表
    pub fn get_loaded_nouns(&self) -> Vec<&str> {
        self.noun_attributes.keys().map(|s| s.as_str()).collect()
    }

    /// 检查 noun 是否已加载
    pub fn has_noun(&self, noun: &str) -> bool {
        self.noun_attributes.contains_key(&noun.to_uppercase())
    }

    /// 通过 hash 查找属性名称
    pub fn get_attribute_name_by_hash(&self, hash: u32) -> Option<&str> {
        self.all_attributes.get(&hash).map(|s| s.as_str())
    }

    /// 获取 noun 的属性数量
    pub fn get_attribute_count(&self, noun: &str) -> usize {
        self.noun_attributes
            .get(&noun.to_uppercase())
            .map(|attrs| attrs.len())
            .unwrap_or(0)
    }

    /// 获取 noun 的属性描述信息
    ///
    /// - 属性列表来源：已加载的 JSON（all_attr_info 或 data 目录）
    /// - 元数据来源：可选的 attlib.dat（若提供路径则解析并匹配 hash）
    pub fn describe_noun(
        &self,
        noun: &str,
        attlib_path: Option<&Path>,
    ) -> Result<Vec<AttributeDesc>> {
        let attrs = self
            .get_attributes(&noun.to_uppercase())
            .with_context(|| format!("未找到 noun: {}", noun))?;

        // 可选加载 attlib.dat 以补充类型/默认值
        let mut attlib_parser = if let Some(path) = attlib_path {
            let path_str = path.to_string_lossy();
            let mut parser = AttlibParser::new(path_str.as_ref())
                .with_context(|| format!("打开 attlib.dat 失败: {}", path.display()))?;
            parser
                .load_all()
                .with_context(|| format!("解析 attlib.dat 失败: {}", path.display()))?;
            Some(parser)
        } else {
            None
        };

        let mut result = Vec::with_capacity(attrs.len());
        for attr in attrs {
            let mut desc = AttributeDesc::from(attr);
            if let Some(parser) = attlib_parser.as_ref() {
                if let Some(attlib_attr) = parser.get_full_attribute(attr.hash) {
                    desc = desc.with_attlib(&attlib_attr);
                }
            }
            result.push(desc);
        }

        Ok(result)
    }
}

impl Default for NounAttributeStore {
    fn default() -> Self {
        Self::new()
    }
}

/// PDMS 名称哈希编码
pub fn db1_hash(name: &str) -> u32 {
    let mut val: u32 = 0;
    for c in name.to_uppercase().chars().rev() {
        if c.is_ascii_uppercase() {
            val = val.wrapping_mul(27).wrapping_add((c as u32) - 64);
        }
    }
    val.wrapping_add(0x81BF1)
}

/// PDMS 哈希解码为名称
pub fn db1_dehash(hash: u32) -> Option<String> {
    const BASE27_MIN: u32 = 0x81BF2;
    const BASE27_MAX: u32 = 0x171FAD39;

    if hash < BASE27_MIN || hash > BASE27_MAX {
        return None;
    }

    let mut k = hash - 0x81BF1;
    let mut result = String::new();

    while k > 0 {
        let c = ((k % 27) + 64) as u8 as char;
        result.push(c);
        k /= 27;
    }

    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

// ============================================================================
// 全局访问
// ============================================================================

use once_cell::sync::OnceCell;

static GLOBAL_STORE: OnceCell<NounAttributeStore> = OnceCell::new();

/// 获取全局 NounAttributeStore（懒加载）
///
/// 自动从 all_attr_info.json 加载数据
pub fn get_noun_attribute_store() -> &'static NounAttributeStore {
    GLOBAL_STORE.get_or_init(|| {
        // 尝试从 all_attr_info.json 加载
        let path = "all_attr_info.json";
        if Path::new(path).exists() {
            if let Ok(store) = NounAttributeStore::load_from_all_attr_info(path) {
                return store;
            }
        }
        // 返回空 store
        NounAttributeStore::new()
    })
}

/// 查询指定 noun 的所有属性
///
/// # Example
/// ```ignore
/// let attrs = query_noun_attributes("ELBO");
/// for attr in attrs {
///     println!("{}: {}", attr.name, attr.att_type);
/// }
/// ```
pub fn query_noun_attributes(noun: &str) -> Option<&'static Vec<AttributeInfo>> {
    get_noun_attribute_store().get_attributes(noun)
}

/// 查询指定 noun 的特定属性
pub fn query_noun_attribute(noun: &str, attr_name: &str) -> Option<&'static AttributeInfo> {
    get_noun_attribute_store().get_attribute(noun, attr_name)
}

/// 获取所有已知的 noun 列表
pub fn list_all_nouns() -> Vec<&'static str> {
    get_noun_attribute_store().get_loaded_nouns()
}

/// 获取 noun 的属性描述信息（可选 attlib.dat 路径）
pub fn describe_noun_attributes(
    noun: &str,
    attlib_path: Option<&Path>,
) -> Result<Vec<AttributeDesc>> {
    get_noun_attribute_store().describe_noun(noun, attlib_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash() {
        assert_eq!(db1_hash("ELBO"), 828473);
        assert_eq!(db1_hash("PIPE"), 641779);
        assert_eq!(db1_hash("NAME"), 639374);
    }

    #[test]
    fn test_dehash() {
        assert_eq!(db1_dehash(828473), Some("ELBO".to_string()));
        assert_eq!(db1_dehash(641779), Some("PIPE".to_string()));
        assert_eq!(db1_dehash(639374), Some("NAME".to_string()));
    }

    #[test]
    fn test_load_from_all_attr_info() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/all_attr_info.json");
        if std::path::Path::new(path).exists() {
            let store = NounAttributeStore::load_from_all_attr_info(path).unwrap();
            
            // 应该有很多 noun
            assert!(store.get_loaded_nouns().len() > 100);
            
            // 检查常见 noun
            assert!(store.has_noun("ELBO"));
            assert!(store.has_noun("EQUI"));
            assert!(store.has_noun("CYLI"));
            assert!(store.has_noun("NOZZ"));
            
            // 检查属性
            let elbo_attrs = store.get_attributes("ELBO").unwrap();
            assert!(!elbo_attrs.is_empty());
            
            let name_attr = store.get_attribute("ELBO", "NAME");
            assert!(name_attr.is_some());
            assert_eq!(name_attr.unwrap().att_type, AttributeType::String);
        }
    }
}
