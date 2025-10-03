//! 模型生成相关查询
//!
//! 为 gen-model 项目提供的专门查询接口

#[cfg(feature = "kuzu")]
use crate::rs_kuzu::create_kuzu_connection;
#[cfg(feature = "kuzu")]
use crate::types::*;
#[cfg(feature = "kuzu")]
use anyhow::{Context, Result, anyhow};
#[cfg(feature = "kuzu")]
use kuzu::Value;
#[cfg(feature = "kuzu")]
use std::collections::HashMap;

#[cfg(feature = "kuzu")]
/// 查询单个元素的详细信息
///
/// # 参数
/// * `refno` - 元素参考号
///
/// # 返回
/// 包含元素所有属性的结构体
pub async fn query_element_kuzu(refno: RefU64) -> Result<Option<ElementData>> {
    let conn = create_kuzu_connection()?;

    let query = format!(
        "MATCH (e:PE {{refno: {}}})
         RETURN e.refno, e.dbnum, e.noun AS type_name, e.name,
                e.transform, e.aabb_min, e.aabb_max",
        refno.0
    );

    let mut result = conn.query(&query)?;

    if let Some(record) = result.next() {
        Ok(Some(ElementData {
            refno,
            dbnum: value_as_i32(record.get(1), "dbnum")?,
            type_name: value_as_string(record.get(2), "type_name")?,
            name: value_as_optional_string(record.get(3)),
            transform: value_as_transform(record.get(4)),
            aabb_min: value_as_vec3(record.get(5)),
            aabb_max: value_as_vec3(record.get(6)),
        }))
    } else {
        Ok(None)
    }
}

#[cfg(feature = "kuzu")]
/// 按类型和数据库编号查询元素
///
/// # 参数
/// * `type_name` - 元素类型名称 (如 "FRMW", "PRIM", "LOOP" 等)
/// * `dbnum` - 数据库编号
///
/// # 返回
/// 符合条件的元素参考号列表
pub async fn query_type_refnos_by_dbnum_kuzu(
    type_name: &str,
    dbnum: i32,
) -> Result<Vec<RefnoEnum>> {
    let conn = create_kuzu_connection()?;

    let query = format!(
        "MATCH (e:PE)
         WHERE e.noun = '{}' AND e.dbnum = {}
         RETURN e.refno
         ORDER BY e.refno",
        type_name, dbnum
    );

    let mut result = conn.query(&query)?;
    let mut refnos = Vec::new();

    while let Some(record) = result.next() {
        if let Some(Value::Int64(refno)) = record.get(0) {
            refnos.push(RefnoEnum::from(RefU64(*refno as u64)));
        }
    }

    log::debug!("找到 {} 个类型为 {} 的元素 (dbnum={})",
               refnos.len(), type_name, dbnum);

    Ok(refnos)
}

#[cfg(feature = "kuzu")]
/// 查询多个节点的所有子节点
///
/// # 参数
/// * `refnos` - 父节点参考号列表
///
/// # 返回
/// 所有子节点的参考号列表（去重）
pub async fn query_multi_children_refnos_kuzu(
    refnos: &[RefnoEnum],
) -> Result<Vec<RefnoEnum>> {
    if refnos.is_empty() {
        return Ok(Vec::new());
    }

    let conn = create_kuzu_connection()?;

    // 构建参考号列表
    let refno_list: Vec<String> = refnos
        .iter()
        .map(|r| r.refno().0.to_string())
        .collect();

    let query = format!(
        "UNWIND [{}] AS parent_refno
         MATCH (p:PE {{refno: parent_refno}})-[:OWNS]->(c:PE)
         RETURN DISTINCT c.refno
         ORDER BY c.refno",
        refno_list.join(", ")
    );

    let mut result = conn.query(&query)?;
    let mut children = Vec::new();

    while let Some(record) = result.next() {
        if let Some(Value::Int64(refno)) = record.get(0) {
            children.push(RefnoEnum::from(RefU64(*refno as u64)));
        }
    }

    log::debug!("从 {} 个父节点找到 {} 个子节点",
               refnos.len(), children.len());

    Ok(children)
}

#[cfg(feature = "kuzu")]
/// 批量查询元素
///
/// # 参数
/// * `refnos` - 元素参考号列表
///
/// # 返回
/// 元素数据列表
pub async fn query_batch_elements_kuzu(refnos: &[RefU64]) -> Result<Vec<ElementData>> {
    if refnos.is_empty() {
        return Ok(Vec::new());
    }

    let conn = create_kuzu_connection()?;

    // 构建参考号列表
    let refno_list: Vec<String> = refnos
        .iter()
        .map(|r| r.0.to_string())
        .collect();

    let query = format!(
        "UNWIND [{}] AS refno
         MATCH (e:PE {{refno: refno}})
         RETURN e.refno, e.dbnum, e.noun AS type_name, e.name,
                e.transform, e.aabb_min, e.aabb_max
         ORDER BY e.refno",
        refno_list.join(", ")
    );

    let mut result = conn.query(&query)?;
    let mut elements = Vec::new();

    while let Some(record) = result.next() {
        if let Some(Value::Int64(refno_val)) = record.get(0) {
            elements.push(ElementData {
                refno: RefU64(*refno_val as u64),
                dbnum: value_as_i32(record.get(1), "dbnum")?,
                type_name: value_as_string(record.get(2), "type_name")?,
                name: value_as_optional_string(record.get(3)),
                transform: value_as_transform(record.get(4)),
                aabb_min: value_as_vec3(record.get(5)),
                aabb_max: value_as_vec3(record.get(6)),
            });
        }
    }

    Ok(elements)
}

#[cfg(feature = "kuzu")]
/// 查询子树（递归查询所有后代）
///
/// # 参数
/// * `root` - 根节点参考号
/// * `max_depth` - 最大深度限制
///
/// # 返回
/// 子树中所有节点的数据
pub async fn query_subtree_kuzu(root: RefU64, max_depth: i32) -> Result<Vec<ElementData>> {
    let conn = create_kuzu_connection()?;

    let query = format!(
        "MATCH (root:PE {{refno: {}}})-[:OWNS*1..{}]->(child:PE)
         RETURN DISTINCT child.refno, child.dbnum, child.noun AS type_name,
                child.name, child.transform, child.aabb_min, child.aabb_max
         ORDER BY child.refno",
        root.0, max_depth
    );

    let mut result = conn.query(&query)?;
    let mut elements = Vec::new();

    // 先添加根节点
    if let Some(root_data) = query_element_kuzu(root).await? {
        elements.push(root_data);
    }

    // 添加所有子节点
    while let Some(record) = result.next() {
        if let Some(Value::Int64(refno_val)) = record.get(0) {
            elements.push(ElementData {
                refno: RefU64(*refno_val as u64),
                dbnum: value_as_i32(record.get(1), "dbnum")?,
                type_name: value_as_string(record.get(2), "type_name")?,
                name: value_as_optional_string(record.get(3)),
                transform: value_as_transform(record.get(4)),
                aabb_min: value_as_vec3(record.get(5)),
                aabb_max: value_as_vec3(record.get(6)),
            });
        }
    }

    Ok(elements)
}

#[cfg(feature = "kuzu")]
/// 查询父元素
///
/// # 参数
/// * `child` - 子元素参考号
///
/// # 返回
/// 父元素参考号（如果存在）
pub async fn query_parent_kuzu(child: RefU64) -> Result<Option<RefU64>> {
    let conn = create_kuzu_connection()?;

    let query = format!(
        "MATCH (p:PE)-[:OWNS]->(c:PE {{refno: {}}})
         RETURN p.refno
         LIMIT 1",
        child.0
    );

    let mut result = conn.query(&query)?;

    if let Some(record) = result.next() {
        if let Some(Value::Int64(parent_refno)) = record.get(0) {
            return Ok(Some(RefU64(*parent_refno as u64)));
        }
    }

    Ok(None)
}

// ============================================================================
// 辅助结构和函数
// ============================================================================

#[cfg(feature = "kuzu")]
/// 元素数据结构
#[derive(Clone, Debug)]
pub struct ElementData {
    pub refno: RefU64,
    pub dbnum: i32,
    pub type_name: String,
    pub name: Option<String>,
    pub transform: Option<[f64; 16]>,
    pub aabb_min: Option<[f64; 3]>,
    pub aabb_max: Option<[f64; 3]>,
}

#[cfg(feature = "kuzu")]
fn value_as_i32(value: Option<&Value>, field_name: &str) -> Result<i32> {
    match value {
        Some(Value::Int64(v)) => Ok(*v as i32),
        Some(Value::Int32(v)) => Ok(*v),
        _ => Err(anyhow!("无法将 {} 转换为 i32", field_name)),
    }
}

#[cfg(feature = "kuzu")]
fn value_as_string(value: Option<&Value>, field_name: &str) -> Result<String> {
    match value {
        Some(Value::String(s)) => Ok(s.clone()),
        _ => Err(anyhow!("无法将 {} 转换为 String", field_name)),
    }
}

#[cfg(feature = "kuzu")]
fn value_as_optional_string(value: Option<&Value>) -> Option<String> {
    match value {
        Some(Value::String(s)) => Some(s.clone()),
        _ => None,
    }
}

#[cfg(feature = "kuzu")]
fn value_as_transform(value: Option<&Value>) -> Option<[f64; 16]> {
    match value {
        Some(Value::List(list)) if list.len() == 16 => {
            let mut transform = [0.0; 16];
            for (i, v) in list.iter().enumerate() {
                if let Value::Double(d) = v {
                    transform[i] = *d;
                } else if let Value::Float(f) = v {
                    transform[i] = *f as f64;
                }
            }
            Some(transform)
        }
        _ => None,
    }
}

#[cfg(feature = "kuzu")]
fn value_as_vec3(value: Option<&Value>) -> Option<[f64; 3]> {
    match value {
        Some(Value::List(list)) if list.len() == 3 => {
            let mut vec3 = [0.0; 3];
            for (i, v) in list.iter().enumerate() {
                if let Value::Double(d) = v {
                    vec3[i] = *d;
                } else if let Value::Float(f) = v {
                    vec3[i] = *f as f64;
                }
            }
            Some(vec3)
        }
        _ => None,
    }
}