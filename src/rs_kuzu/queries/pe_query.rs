//! PE 节点查询
//!
//! 提供 PE (Plant Element) 的图查询功能

#[cfg(feature = "kuzu")]
use crate::rs_kuzu::create_kuzu_connection;
#[cfg(feature = "kuzu")]
use crate::types::*;
#[cfg(feature = "kuzu")]
use anyhow::{Context, Result, anyhow};
#[cfg(feature = "kuzu")]
use kuzu::{NodeVal, Value};
#[cfg(feature = "kuzu")]
use std::convert::TryFrom;

#[cfg(feature = "kuzu")]
/// 查询单个 PE
pub async fn get_pe_from_kuzu(refno: RefnoEnum) -> Result<Option<SPdmsElement>> {
    let conn = create_kuzu_connection()?;

    let query = format!(
        "MATCH (pe:PE {{refno: {}}})
         OPTIONAL MATCH (owner:PE)-[:OWNS]->(pe)
         RETURN pe, owner.refno AS owner_refno",
        refno.refno().0
    );

    let mut result = conn.query(&query)?;

    if let Some(record) = result.next() {
        let pe_value = record
            .get(0)
            .ok_or_else(|| anyhow!("Kuzu 查询结果缺少 PE 列"))?;
        let owner_value = record.get(1);

        let owner_refno = match owner_value {
            Some(Value::Null(_)) | None => None,
            Some(value) => Some(value_as_refno(value, "owner_refno")?),
        };

        match pe_value {
            Value::Node(node) => {
                let pe = node_to_spdms(node, owner_refno)?;
                Ok(Some(pe))
            }
            other => Err(anyhow!(
                "Kuzu 查询返回的第一列不是节点类型，而是 {:?}",
                other
            )),
        }
    } else {
        Ok(None)
    }
}

#[cfg(feature = "kuzu")]
/// 查询子元素
pub async fn query_children_refnos_kuzu(refno: RefnoEnum) -> Result<Vec<RefnoEnum>> {
    let conn = create_kuzu_connection()?;

    let query = format!(
        "MATCH (parent:PE {{refno: {}}})-[:OWNS]->(child:PE)
         RETURN child.refno",
        refno.refno().0
    );

    let mut result = conn.query(&query)?;
    let mut children = Vec::new();

    while let Some(record) = result.next() {
        if let Some(value) = record.get(0) {
            if let kuzu::Value::Int64(child_refno) = value {
                children.push(RefnoEnum::from(RefU64(*child_refno as u64)));
            }
        }
    }

    log::debug!("PE {} 有 {} 个子元素", refno.refno().0, children.len());

    Ok(children)
}

#[cfg(feature = "kuzu")]
pub(crate) fn node_refno(node: &NodeVal) -> Result<RefnoEnum> {
    let value =
        node_property(node, "refno").with_context(|| "PE 节点缺少 refno 属性".to_string())?;
    value_as_refno(value, "refno")
}

#[cfg(feature = "kuzu")]
fn node_to_spdms(node: &NodeVal, owner_hint: Option<RefnoEnum>) -> Result<SPdmsElement> {
    let refno = node_refno(node)?;
    let name = value_as_string(
        node_property(node, "name").with_context(|| "PE 节点缺少 name 属性".to_string())?,
        "name",
    )?;
    let noun = value_as_string(
        node_property(node, "noun").with_context(|| "PE 节点缺少 noun 属性".to_string())?,
        "noun",
    )?;
    let dbnum = value_as_i32(
        node_property(node, "dbnum").with_context(|| "PE 节点缺少 dbnum 属性".to_string())?,
        "dbnum",
    )?;
    let sesno = value_as_i32(
        node_property(node, "sesno").with_context(|| "PE 节点缺少 sesno 属性".to_string())?,
        "sesno",
    )?;
    let cata_hash = value_as_string(
        node_property(node, "cata_hash")
            .with_context(|| "PE 节点缺少 cata_hash 属性".to_string())?,
        "cata_hash",
    )?;
    let deleted = value_as_bool(
        node_property(node, "deleted").with_context(|| "PE 节点缺少 deleted 属性".to_string())?,
        "deleted",
    )?;
    let lock = value_as_bool(
        node_property(node, "lock").with_context(|| "PE 节点缺少 lock 属性".to_string())?,
        "lock",
    )?;

    let status_code = if let Some(value) = node_property(node, "status_code") {
        value_as_optional_string(value, "status_code")?
    } else {
        None
    };

    let owner = if let Some(owner) = owner_hint {
        owner
    } else if let Some(value) = node_property(node, "owner_refno") {
        value_as_refno(value, "owner_refno")?
    } else if let Some(value) = node_property(node, "owner") {
        value_as_refno(value, "owner")?
    } else {
        RefnoEnum::default()
    };

    Ok(SPdmsElement {
        refno,
        owner,
        name,
        noun,
        dbnum,
        sesno,
        status_code,
        cata_hash,
        lock,
        deleted,
        ..Default::default()
    })
}

#[cfg(feature = "kuzu")]
fn node_property<'a>(node: &'a NodeVal, key: &str) -> Option<&'a Value> {
    node.get_properties()
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value)
}

#[cfg(feature = "kuzu")]
fn value_as_string(value: &Value, field: &str) -> Result<String> {
    match value {
        Value::String(s) => Ok(s.clone()),
        other => Err(anyhow!("字段 {} 期望为字符串，但得到 {:?}", field, other)),
    }
}

#[cfg(feature = "kuzu")]
fn value_as_optional_string(value: &Value, field: &str) -> Result<Option<String>> {
    match value {
        Value::Null(_) => Ok(None),
        Value::String(s) => Ok(Some(s.clone())),
        other => Err(anyhow!(
            "字段 {} 期望为可选字符串，但得到 {:?}",
            field,
            other
        )),
    }
}

#[cfg(feature = "kuzu")]
fn value_as_bool(value: &Value, field: &str) -> Result<bool> {
    match value {
        Value::Bool(v) => Ok(*v),
        Value::Int8(v) => Ok(*v != 0),
        Value::Int16(v) => Ok(*v != 0),
        Value::Int32(v) => Ok(*v != 0),
        Value::Int64(v) => Ok(*v != 0),
        Value::UInt8(v) => Ok(*v != 0),
        Value::UInt16(v) => Ok(*v != 0),
        Value::UInt32(v) => Ok(*v != 0),
        Value::UInt64(v) => Ok(*v != 0),
        other => Err(anyhow!("字段 {} 期望为布尔值，但得到 {:?}", field, other)),
    }
}

#[cfg(feature = "kuzu")]
fn value_as_i32(value: &Value, field: &str) -> Result<i32> {
    match value {
        Value::Int32(v) => Ok(*v),
        Value::Int16(v) => Ok(i32::from(*v)),
        Value::Int8(v) => Ok(i32::from(*v)),
        Value::Int64(v) => {
            i32::try_from(*v).map_err(|_| anyhow!("字段 {} 的值 {:?} 超出 i32 范围", field, v))
        }
        Value::UInt16(v) => Ok((*v).into()),
        Value::UInt32(v) => {
            i32::try_from(*v).map_err(|_| anyhow!("字段 {} 的值 {:?} 超出 i32 范围", field, v))
        }
        Value::UInt64(v) => {
            i32::try_from(*v).map_err(|_| anyhow!("字段 {} 的值 {:?} 超出 i32 范围", field, v))
        }
        other => Err(anyhow!("字段 {} 期望为整数，但得到 {:?}", field, other)),
    }
}

#[cfg(feature = "kuzu")]
fn value_as_refno(value: &Value, field: &str) -> Result<RefnoEnum> {
    let raw = match value {
        Value::Int64(v) => {
            if *v < 0 {
                return Err(anyhow!("字段 {} 不允许负值: {}", field, v));
            }
            *v as u64
        }
        Value::Int32(v) => {
            if *v < 0 {
                return Err(anyhow!("字段 {} 不允许负值: {}", field, v));
            }
            (*v) as u64
        }
        Value::UInt64(v) => *v,
        Value::UInt32(v) => (*v).into(),
        other => {
            return Err(anyhow!(
                "字段 {} 期望为 Refno 数值类型，但得到 {:?}",
                field,
                other
            ));
        }
    };

    Ok(RefnoEnum::from(RefU64(raw)))
}

// ============================================================================
// QueryProvider 兼容包装函数
// ============================================================================

#[cfg(feature = "kuzu")]
/// 查询单个 PE (QueryProvider 兼容函数)
pub async fn kuzu_get_pe(refno: RefnoEnum) -> Result<Option<SPdmsElement>> {
    get_pe_from_kuzu(refno).await
}

#[cfg(feature = "kuzu")]
/// 查询子元素的完整 PE (QueryProvider 兼容函数)
pub async fn kuzu_get_children_pes(refno: RefnoEnum) -> Result<Vec<SPdmsElement>> {
    use crate::rs_kuzu::queries::advanced_hierarchy;
    advanced_hierarchy::kuzu_get_children_pes(refno).await
}

#[cfg(feature = "kuzu")]
/// 批量查询 PE (QueryProvider 兼容函数)
pub async fn kuzu_get_pes_batch(refnos: &[RefnoEnum]) -> Result<Vec<SPdmsElement>> {
    if refnos.is_empty() {
        return Ok(vec![]);
    }

    let mut pes = Vec::new();
    for refno in refnos {
        if let Some(pe) = get_pe_from_kuzu(*refno).await? {
            pes.push(pe);
        }
    }

    log::debug!("批量查询 PE: 请求 {} 个，返回 {} 个", refnos.len(), pes.len());
    Ok(pes)
}
