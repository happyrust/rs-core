//! 从 all_attr_info.json 直接生成 Kuzu Schema
//!
//! 直接读取 JSON 文件并生成强类型的 Kuzu 表结构

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use crate::pdms_types::{AttrInfo, DbAttributeType};
use crate::types::AttrVal;

/// JSON 文件中的属性信息映射
#[derive(Debug, Deserialize)]
pub struct AllAttrInfo {
    pub named_attr_info_map: HashMap<String, HashMap<String, AttrInfo>>,
    #[serde(default)]
    pub noun_attr_info_map: HashMap<String, HashMap<String, AttrInfo>>,
}

/// 加载 all_attr_info.json 文件
pub fn load_attr_info_json() -> Result<AllAttrInfo> {
    let json_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("all_attr_info.json");

    if !json_path.exists() {
        return Err(anyhow!("all_attr_info.json 文件不存在: {:?}", json_path));
    }

    let content =
        fs::read_to_string(&json_path).with_context(|| format!("读取文件失败: {:?}", json_path))?;

    let attr_info: AllAttrInfo =
        serde_json::from_str(&content).with_context(|| "解析 JSON 失败")?;

    Ok(attr_info)
}

/// 将 PDMS 属性类型转换为 Kuzu 数据类型
pub fn pdms_type_to_kuzu(attr_type: &DbAttributeType, default_val: &AttrVal) -> String {
    match attr_type {
        DbAttributeType::INTEGER => "INT32",
        DbAttributeType::DOUBLE => "DOUBLE",
        DbAttributeType::BOOL => "BOOLEAN",
        DbAttributeType::STRING | DbAttributeType::WORD => "STRING",
        DbAttributeType::ELEMENT => "INT64", // RefNo 类型
        DbAttributeType::ORIENTATION | DbAttributeType::DIRECTION | DbAttributeType::POSITION => {
            // 3D 向量
            "DOUBLE[]"
        }
        DbAttributeType::DOUBLEVEC | DbAttributeType::FLOATVEC => "DOUBLE[]",
        DbAttributeType::INTVEC => "INT32[]",
        DbAttributeType::Vec3Type => "DOUBLE[]",
        DbAttributeType::RefU64Vec => "INT64[]",
        _ => {
            // 根据默认值推断
            match default_val {
                AttrVal::IntArrayType(_) => "INT32[]",
                AttrVal::DoubleArrayType(_) => "DOUBLE[]",
                AttrVal::StringArrayType(_) => "STRING[]",
                AttrVal::BoolArrayType(_) => "BOOLEAN[]",
                AttrVal::Vec3Type(_) => "DOUBLE[]",
                AttrVal::RefU64Array(_) => "INT64[]",
                _ => "STRING", // 默认为字符串
            }
        }
    }
    .to_string()
}

/// 生成单个 noun 的建表 SQL
pub fn generate_noun_table_sql(noun: &str, attrs: &HashMap<String, AttrInfo>) -> Result<String> {
    let table_name = format!("Attr_{}", noun.to_uppercase());

    let mut columns = vec!["refno INT64".to_string()]; // 主键
    let mut processed_attrs = HashSet::new();

    // 按名称排序以保证稳定性
    let mut sorted_attrs: Vec<_> = attrs.iter().collect();
    sorted_attrs.sort_by_key(|(name, _)| *name);

    for (attr_name, attr_info) in sorted_attrs {
        // 避免重复属性
        let upper_name = attr_name.to_uppercase();
        if processed_attrs.contains(&upper_name) {
            continue;
        }
        processed_attrs.insert(upper_name.clone());

        let kuzu_type = pdms_type_to_kuzu(&attr_info.att_type, &attr_info.default_val);
        columns.push(format!("{} {}", upper_name, kuzu_type));
    }

    // 添加 PRIMARY KEY 约束
    columns[0] = "refno INT64 PRIMARY KEY".to_string();

    let columns_str = columns
        .into_iter()
        .map(|col| format!("            {}", col))
        .collect::<Vec<_>>()
        .join(",\n");

    Ok(format!(
        "CREATE NODE TABLE IF NOT EXISTS {}(\n{}\n        )",
        table_name, columns_str
    ))
}

/// 生成 PE 到属性表的关系
pub fn generate_pe_to_attr_rel_sql(noun: &str) -> String {
    format!(
        "CREATE REL TABLE IF NOT EXISTS TO_{}(
            FROM PE TO Attr_{},
            MANY_ONE
        )",
        noun.to_uppercase(),
        noun.to_uppercase()
    )
}

/// 识别并生成引用边表（用于 ELEMENT 类型的属性）
pub fn generate_reference_edge_sqls(noun: &str, attrs: &HashMap<String, AttrInfo>) -> Vec<String> {
    let mut edge_sqls = Vec::new();
    let mut processed_edges = HashSet::new();

    for (attr_name, attr_info) in attrs {
        // ELEMENT 类型表示引用其他元素
        if matches!(attr_info.att_type, DbAttributeType::ELEMENT) {
            let upper_name = attr_name.to_uppercase();

            // 推断目标 noun（基于属性名称的模式）
            let target_noun = infer_target_noun(&upper_name);
            let edge_name = format!("{}_{}", noun.to_uppercase(), upper_name);

            if processed_edges.contains(&edge_name) {
                continue;
            }
            processed_edges.insert(edge_name.clone());

            let sql = format!(
                "CREATE REL TABLE IF NOT EXISTS {}(
            FROM Attr_{} TO PE,
            field_name STRING DEFAULT '{}',
            target_noun STRING DEFAULT '{}'
        )",
                edge_name,
                noun.to_uppercase(),
                upper_name,
                target_noun
            );

            edge_sqls.push(sql);
        }
    }

    edge_sqls
}

/// 推断引用的目标 noun
fn infer_target_noun(attr_name: &str) -> String {
    // 基于属性名称推断目标类型
    if attr_name.ends_with("_REFNO") {
        let prefix = attr_name.trim_end_matches("_REFNO");
        return prefix.to_string();
    }

    if attr_name.ends_with("REF") || attr_name.ends_with("RF") {
        // 通用引用
        return "ELEMENT".to_string();
    }

    // 特殊情况处理
    match attr_name {
        "CREF" => "CATALOGUE".to_string(),
        "SREF" | "SPRE" => "SPEC".to_string(),
        "MREF" => "MATERIAL".to_string(),
        "OWNE" | "OWNER" => "ELEMENT".to_string(),
        _ => "ELEMENT".to_string(),
    }
}

/// 生成所有表的 SQL
pub fn generate_all_table_sqls() -> Result<Vec<String>> {
    let attr_info = load_attr_info_json()?;
    let mut sqls = Vec::new();

    // 1. 创建 PE 主表
    sqls.push(
        "CREATE NODE TABLE IF NOT EXISTS PE(
            refno INT64 PRIMARY KEY,
            name STRING,
            noun STRING,
            dbnum INT32,
            sesno INT32,
            cata_hash STRING,
            deleted BOOLEAN DEFAULT false,
            lock BOOLEAN DEFAULT false,
            typex INT32
        )"
        .to_string(),
    );

    // 1.1 创建 PE 表索引
    sqls.push("CREATE INDEX IF NOT EXISTS idx_pe_typex ON PE(typex)".to_string());
    sqls.push("CREATE INDEX IF NOT EXISTS idx_pe_noun ON PE(noun)".to_string());
    sqls.push("CREATE INDEX IF NOT EXISTS idx_pe_cata_hash ON PE(cata_hash)".to_string());

    // 2. 为每个 noun 创建属性表和关系
    for (noun, attrs) in &attr_info.named_attr_info_map {
        // 创建属性节点表
        let table_sql = generate_noun_table_sql(noun, attrs)?;
        sqls.push(table_sql);

        // 创建 PE -> Attr 关系
        let rel_sql = generate_pe_to_attr_rel_sql(noun);
        sqls.push(rel_sql);

        // 创建引用边表
        let edge_sqls = generate_reference_edge_sqls(noun, attrs);
        sqls.extend(edge_sqls);
    }

    // 3. 创建通用关系表
    sqls.push(
        "CREATE REL TABLE IF NOT EXISTS OWNS(
            FROM PE TO PE,
            MANY_ONE
        )"
        .to_string(),
    );

    Ok(sqls)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_json() {
        let result = load_attr_info_json();
        assert!(result.is_ok());

        let attr_info = result.unwrap();
        assert!(!attr_info.named_attr_info_map.is_empty());
    }

    #[test]
    fn test_generate_table_sql() {
        let attr_info = load_attr_info_json().unwrap();

        if let Some(elbo_attrs) = attr_info.named_attr_info_map.get("ELBO") {
            let sql = generate_noun_table_sql("ELBO", elbo_attrs).unwrap();
            println!("ELBO 表 SQL:\n{}", sql);
            assert!(sql.contains("Attr_ELBO"));
            assert!(sql.contains("refno INT64 PRIMARY KEY"));
        }
    }

    #[test]
    fn test_type_mapping() {
        assert_eq!(
            pdms_type_to_kuzu(&DbAttributeType::INTEGER, &AttrVal::IntegerType(0)),
            "INT32"
        );
        assert_eq!(
            pdms_type_to_kuzu(&DbAttributeType::DOUBLE, &AttrVal::DoubleType(0.0)),
            "DOUBLE"
        );
        assert_eq!(
            pdms_type_to_kuzu(&DbAttributeType::STRING, &AttrVal::StringType("".into())),
            "STRING"
        );
        assert_eq!(
            pdms_type_to_kuzu(&DbAttributeType::ELEMENT, &AttrVal::RefU64Type(0.into())),
            "INT64"
        );
    }
}
