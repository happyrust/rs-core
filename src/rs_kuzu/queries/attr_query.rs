//! 属性查询
//!
//! 提供元素属性的图查询功能

#[cfg(feature = "kuzu")]
use crate::rs_kuzu::{create_kuzu_connection, types::kuzu_value_to_named_attr};
#[cfg(feature = "kuzu")]
use crate::types::*;
#[cfg(feature = "kuzu")]
use anyhow::Result;

#[cfg(feature = "kuzu")]
/// 查询元素属性
pub async fn get_named_attmap_kuzu(refno: RefnoEnum) -> Result<NamedAttrMap> {
    let conn = create_kuzu_connection()?;

    let query = format!(
        "MATCH (pe:PE {{refno: {}}})-[:HAS_ATTR]->(attr:Attribute)
         RETURN attr.name, attr.value, attr.type",
        refno.refno().0
    );

    let mut result = conn.query(&query)?;
    let mut attr_map = NamedAttrMap::default();

    while let Some(record) = result.next() {
        // 获取属性名、值和类型
        if let (Some(name_val), Some(value_val), Some(type_val)) =
            (record.get(0), record.get(1), record.get(2))
        {
            if let (
                kuzu::Value::String(name),
                kuzu::Value::String(value_str),
                kuzu::Value::String(attr_type),
            ) = (name_val, value_val, type_val)
            {
                // 直接将 value_str 解析为合适的类型
                // 创建一个简单的 String Value 并尝试转换
                let simple_value = kuzu::Value::String(value_str.clone());
                match kuzu_value_to_named_attr(&simple_value, attr_type) {
                    Ok(named_attr) => {
                        attr_map.insert(name.clone(), named_attr);
                    }
                    Err(e) => {
                        log::warn!("无法转换属性 {} 的值: {}", name, e);
                    }
                }
            }
        }
    }

    log::debug!("PE {} 有 {} 个属性", refno.refno().0, attr_map.len());

    Ok(attr_map)
}

#[cfg(feature = "kuzu")]
/// 批量查询属性
pub async fn get_batch_attmaps_kuzu(
    refnos: &[RefnoEnum],
) -> Result<Vec<(RefnoEnum, NamedAttrMap)>> {
    if refnos.is_empty() {
        return Ok(vec![]);
    }

    let conn = create_kuzu_connection()?;

    // 构建 refno 列表
    let refno_list = refnos
        .iter()
        .map(|r| r.refno().0.to_string())
        .collect::<Vec<_>>()
        .join(", ");

    let query = format!(
        "MATCH (pe:PE)-[:HAS_ATTR]->(attr:Attribute)
         WHERE pe.refno IN [{}]
         RETURN pe.refno, attr.name, attr.value, attr.type
         ORDER BY pe.refno",
        refno_list
    );

    let mut result = conn.query(&query)?;
    let mut attmaps = std::collections::HashMap::new();

    while let Some(record) = result.next() {
        if let (Some(refno_val), Some(name_val), Some(value_val), Some(type_val)) =
            (record.get(0), record.get(1), record.get(2), record.get(3))
        {
            if let (
                kuzu::Value::Int64(refno),
                kuzu::Value::String(name),
                kuzu::Value::String(value_str),
                kuzu::Value::String(attr_type),
            ) = (refno_val, name_val, value_val, type_val)
            {
                let refno_enum = RefnoEnum::from(RefU64(*refno as u64));
                let attr_map = attmaps
                    .entry(refno_enum)
                    .or_insert_with(NamedAttrMap::default);

                // 直接将 value_str 解析为合适的类型
                // 创建一个简单的 String Value 并尝试转换
                let simple_value = kuzu::Value::String(value_str.clone());
                match kuzu_value_to_named_attr(&simple_value, attr_type) {
                    Ok(named_attr) => {
                        attr_map.insert(name.clone(), named_attr);
                    }
                    Err(e) => {
                        log::warn!("无法转换属性: {}", e);
                    }
                }
            }
        }
    }

    let result: Vec<_> = attmaps.into_iter().collect();
    log::debug!("批量查询 {} 个PE的属性", result.len());

    Ok(result)
}
