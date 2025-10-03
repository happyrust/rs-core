use anyhow::{anyhow, Context, Result};
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use serde::Deserialize;
use serde_json::{Number, Value};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::pdms_types::AttrInfo;
use crate::tool::db_tool::db1_dehash;
use crate::types::{
    AttrVal, EdgeRecord, EdgeType, NamedAttrMap, NamedAttrValue, PdmsDatabaseInfo, RefU64,
    RefnoEnum, TypedAttrRecord,
};

/// 强类型属性表定义
#[derive(Debug, Clone, Deserialize)]
pub struct AttrTableSpec {
    pub noun: String,
    pub table: String,
    pub fields: Vec<AttrFieldSpec>,
}

/// 字段定义
#[derive(Debug, Clone, Deserialize)]
pub struct AttrFieldSpec {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: String,
    #[serde(default)]
    pub nullable: Option<bool>,
    #[serde(default)]
    pub edge: Option<String>,
    #[serde(default)]
    pub cache: Option<bool>,
}

impl AttrFieldSpec {
    /// 转换为 Kuzu 列定义
    pub fn to_column_definition(&self) -> Result<String> {
        let kuzu_type = normalize_field_type(&self.field_type)?;
        Ok(format!("{} {}", self.name.to_uppercase(), kuzu_type))
    }

    /// 返回引用边（若有）
    pub fn edge(&self) -> Option<&str> {
        self.edge.as_deref()
    }
}

/// 解析字段类型为 Kuzu 类型表达
fn normalize_field_type(field_type: &str) -> Result<String> {
    match field_type {
        "String" => Ok("STRING".to_string()),
        "Bool" => Ok("BOOLEAN".to_string()),
        "Double" => Ok("DOUBLE".to_string()),
        "Int32" => Ok("INT32".to_string()),
        "Int64" => Ok("INT64".to_string()),
        "Refno" => Ok("INT64".to_string()),
        ty if ty.starts_with("List<") && ty.ends_with('>') => {
            let inner = ty.trim_start_matches("List<").trim_end_matches('>');
            let inner = match inner {
                "Int32" => "INT32",
                "Int64" => "INT64",
                "Double" => "DOUBLE",
                "String" => "STRING",
                other => return Err(anyhow!("不支持的 List 内部类型: {}", other)),
            };
            Ok(format!("LIST<{}>", inner))
        }
        other => Err(anyhow!("不支持的字段类型: {}", other)),
    }
}

/// 读取 attr_table_specs 目录下的所有 YAML 定义
pub fn load_attr_table_specs() -> Result<Vec<AttrTableSpec>> {
    let base_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("resource")
        .join("attr_table_specs");

    if !base_dir.exists() {
        return Ok(Vec::new());
    }

    let mut specs = Vec::new();
    for entry in fs::read_dir(&base_dir).with_context(|| format!(
        "读取目录 {:?} 失败",
        base_dir
    ))? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("yaml") {
            continue;
        }
        let spec = load_spec_from_path(&path)?;
        specs.push(spec);
    }

    specs.sort_by(|a, b| a.noun.cmp(&b.noun));
    Ok(specs)
}

fn load_spec_from_path(path: &Path) -> Result<AttrTableSpec> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("读取文件 {:?} 失败", path))?;
    let mut spec: AttrTableSpec = serde_yaml::from_str(&content)
        .with_context(|| format!("解析 YAML {:?} 失败", path))?;
    if spec.table.is_empty() {
        spec.table = format!("Attr_{}", spec.noun.to_uppercase());
    }
    Ok(spec)
}

struct SpecRepository {
    by_noun: HashMap<String, AttrTableSpec>,
}

impl SpecRepository {
    fn new(specs: Vec<AttrTableSpec>) -> Self {
        let mut by_noun = HashMap::new();
        for spec in specs {
            by_noun.insert(spec.noun.to_uppercase(), spec);
        }
        Self { by_noun }
    }

    fn get(&self, noun: &str) -> Option<&AttrTableSpec> {
        self.by_noun.get(&noun.to_uppercase())
    }
}

static SPEC_REPO: Lazy<RwLock<Option<SpecRepository>>> = Lazy::new(|| RwLock::new(None));

fn with_repo<F, R>(f: F) -> Result<R>
where
    F: FnOnce(&SpecRepository) -> R,
{
    let mut guard = SPEC_REPO.write();
    if guard.is_none() {
        let specs = load_attr_table_specs()?;
        *guard = Some(SpecRepository::new(specs));
    }
    Ok(f(guard.as_ref().expect("spec repo must be initialized")))
}

/// 获取指定 noun 的表规格
pub fn get_attr_spec(noun: &str) -> Result<Option<AttrTableSpec>> {
    with_repo(|repo| repo.get(noun).cloned())
}

/// 强类型属性投影结果
#[derive(Debug, Clone)]
pub struct ProjectionResult {
    pub typed_record: TypedAttrRecord,
    pub edges: Vec<EdgeRecord>,
    pub consumed_keys: Vec<String>,
}

/// 将 `NamedAttrMap` 投影到强类型属性表并生成引用边
pub fn project_named_attr(
    noun: &str,
    refno: RefnoEnum,
    attmap: &NamedAttrMap,
) -> Result<Option<ProjectionResult>> {
    let spec = match get_attr_spec(noun)? {
        Some(spec) => spec,
        None => return Ok(None),
    };

    let mut record = TypedAttrRecord::new(&spec.noun, refno);
    let mut consumed_keys = Vec::new();
    let mut edges = Vec::new();

    for field in &spec.fields {
        let key_upper = field.name.to_uppercase();
        if let Some(value) = get_attr_value(attmap, &key_upper) {
            if let Some(json_value) = named_attr_to_json(value) {
                record.fields.insert(key_upper.clone(), json_value);
                consumed_keys.push(key_upper.clone());

                if let Some(edge_name) = field.edge() {
                    edges.extend(edges_from_value(refno, edge_name, &key_upper, value));
                }
            }
        } else if field.nullable.unwrap_or(false) {
            record.fields.insert(key_upper.clone(), Value::Null);
        }
    }

    if record.fields.is_empty() {
        return Ok(None);
    }

    edges.push(EdgeRecord {
        from: refno,
        to: refno,
        edge_type: EdgeType::RelAttr,
    });

    Ok(Some(ProjectionResult {
        typed_record: record,
        edges,
        consumed_keys,
    }))
}

fn get_attr_value<'a>(map: &'a NamedAttrMap, key_upper: &str) -> Option<&'a NamedAttrValue> {
    map.map
        .get(key_upper)
        .or_else(|| map.map.get(&key_upper.to_lowercase()))
        .or_else(|| map.map.get(&key_upper.to_string()))
}

fn named_attr_to_json(value: &NamedAttrValue) -> Option<Value> {
    match value {
        NamedAttrValue::IntegerType(v) => Some(Value::Number(Number::from(*v))),
        NamedAttrValue::LongType(v) => Some(Value::Number(Number::from(*v))),
        NamedAttrValue::F32Type(v) => Number::from_f64(*v as f64).map(Value::Number),
        NamedAttrValue::StringType(v)
        | NamedAttrValue::WordType(v)
        | NamedAttrValue::ElementType(v) => Some(Value::String(v.clone())),
        NamedAttrValue::BoolType(v) => Some(Value::Bool(*v)),
        NamedAttrValue::Vec3Type(v) => Some(Value::Array(
            [v.x, v.y, v.z]
                .iter()
                .filter_map(|f| Number::from_f64(*f as f64).map(Value::Number))
                .collect(),
        )),
        NamedAttrValue::F32VecType(values) => Some(Value::Array(
            values
                .iter()
                .filter_map(|f| Number::from_f64(*f as f64).map(Value::Number))
                .collect(),
        )),
        NamedAttrValue::StringArrayType(values) => Some(Value::Array(
            values.iter().map(|s| Value::String(s.clone())).collect(),
        )),
        NamedAttrValue::BoolArrayType(values) => Some(Value::Array(
            values.iter().map(|b| Value::Bool(*b)).collect(),
        )),
        NamedAttrValue::IntArrayType(values) => Some(Value::Array(
            values
                .iter()
                .map(|i| Value::Number(Number::from(*i)))
                .collect(),
        )),
        NamedAttrValue::RefU64Type(v) => Some(Value::Number(Number::from(v.0))),
        NamedAttrValue::RefU64Array(values) => Some(Value::Array(
            values
                .iter()
                .map(|r| Value::String(r.to_normal_str()))
                .collect(),
        )),
        NamedAttrValue::RefnoEnumType(r) => Some(Value::String(r.to_normal_str())),
        NamedAttrValue::InvalidType => None,
    }
}

fn edges_from_value(
    refno: RefnoEnum,
    edge_name: &str,
    field: &str,
    value: &NamedAttrValue,
) -> Vec<EdgeRecord> {
    let mut edges = Vec::new();
    let target_noun = parse_edge_target(edge_name);
    if target_noun.is_none() {
        log::warn!("无法解析引用边: {}", edge_name);
        return edges;
    }
    let target_noun = target_noun.unwrap();

    for target in extract_refnos(value) {
        if !target.is_valid() {
            continue;
        }
        edges.push(EdgeRecord {
            from: refno,
            to: target,
            edge_type: EdgeType::ToNoun {
                target_noun: target_noun.clone(),
                field: field.to_string(),
            },
        });
    }
    edges
}

fn extract_refnos(value: &NamedAttrValue) -> Vec<RefnoEnum> {
    match value {
        NamedAttrValue::RefU64Type(refno) => vec![RefnoEnum::from(*refno)],
        NamedAttrValue::RefnoEnumType(refno) => vec![*refno],
        NamedAttrValue::RefU64Array(values) => values.clone(),
        _ => Vec::new(),
    }
}

fn parse_edge_target(edge: &str) -> Option<String> {
    edge.strip_prefix("TO_").map(|s| s.to_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{NamedAttrMap, NamedAttrValue, RefU64, RefnoEnum};
    use std::str::FromStr;

    fn make_named_attr(entries: &[(&str, NamedAttrValue)]) -> NamedAttrMap {
        let mut map = NamedAttrMap::default();
        for (k, v) in entries {
            map.map.insert(k.to_string(), v.clone());
        }
        map
    }

    #[test]
    fn test_normalize_field_type() {
        assert_eq!(normalize_field_type("String").unwrap(), "STRING");
        assert_eq!(normalize_field_type("Bool").unwrap(), "BOOLEAN");
        assert_eq!(normalize_field_type("Double").unwrap(), "DOUBLE");
        assert_eq!(normalize_field_type("Refno").unwrap(), "INT64");
        assert_eq!(normalize_field_type("List<Int32>").unwrap(), "LIST<INT32>");
    }

    #[test]
    fn test_load_specs() {
        let specs = load_attr_table_specs().unwrap();
        assert!(specs.iter().any(|spec| spec.noun == "ELBO"));
    }

    #[test]
    fn test_project_named_attr() {
        let refno = RefnoEnum::from(RefU64::from_str("17496/266203").unwrap());
        let spre = RefU64::from_str("17496/123456").unwrap();
        let map = make_named_attr(&[
            ("STATUS_CODE", NamedAttrValue::StringType("OK".into())),
            ("SPRE_REFNO", NamedAttrValue::RefU64Type(spre)),
        ]);

        let projection = project_named_attr("ELBO", refno, &map)
            .expect("projection should succeed")
            .expect("projection should produce typed record");

        assert_eq!(projection.typed_record.noun.to_uppercase(), "ELBO");
        assert_eq!(projection.typed_record.fields.get("STATUS_CODE"), Some(&Value::String("OK".into())));
        assert_eq!(projection.edges.len(), 2); // REL_ATTR + TO_SPRE
    }
}
