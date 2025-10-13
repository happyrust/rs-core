//! 属性操作
//!
//! 提供属性的写入和更新操作

#[cfg(feature = "kuzu")]
use crate::rs_kuzu::create_kuzu_connection;
#[cfg(feature = "kuzu")]
use crate::types::*;
#[cfg(feature = "kuzu")]
use anyhow::Result;

#[cfg(feature = "kuzu")]
/// 转义字符串中的单引号
fn escape_string(s: &str) -> String {
    s.replace("'", "''").replace("\\", "\\\\")
}

#[cfg(feature = "kuzu")]
/// 格式化属性字段为 Cypher 格式
fn format_attr_field(name: &str, value: &NamedAttrValue) -> Result<Option<String>> {
    use NamedAttrValue::*;

    let field = match value {
        IntegerType(v) => format!("{}: {}", name.to_uppercase(), v),
        LongType(v) => format!("{}: {}", name.to_uppercase(), v),
        F32Type(v) => {
            if v.is_finite() {
                format!("{}: {}", name.to_uppercase(), v)
            } else {
                format!("{}: 0.0", name.to_uppercase())
            }
        }
        StringType(s) | WordType(s) => {
            format!("{}: '{}'", name.to_uppercase(), escape_string(s))
        }
        BoolType(b) => format!("{}: {}", name.to_uppercase(), b),
        Vec3Type(v) => {
            if v.x.is_finite() && v.y.is_finite() && v.z.is_finite() {
                format!("{}: [{}, {}, {}]", name.to_uppercase(), v.x, v.y, v.z)
            } else {
                format!("{}: [0.0, 0.0, 0.0]", name.to_uppercase())
            }
        }
        F32VecType(vec) => {
            let vals: Vec<String> = vec
                .iter()
                .map(|v| {
                    if v.is_finite() {
                        v.to_string()
                    } else {
                        "0.0".to_string()
                    }
                })
                .collect();
            format!("{}: [{}]", name.to_uppercase(), vals.join(", "))
        }
        IntArrayType(vec) => {
            let vals = vec
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            format!("{}: [{}]", name.to_uppercase(), vals)
        }
        StringArrayType(vec) => {
            let vals: Vec<String> = vec
                .iter()
                .map(|s| format!("'{}'", escape_string(s)))
                .collect();
            format!("{}: [{}]", name.to_uppercase(), vals.join(", "))
        }
        BoolArrayType(vec) => {
            let vals = vec
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            format!("{}: [{}]", name.to_uppercase(), vals)
        }
        RefU64Type(_) | RefnoEnumType(_) => {
            // 引用类型通过关系边处理，不存储在属性中
            return Ok(None);
        }
        RefU64Array(_) => {
            // 引用数组通过关系边处理
            return Ok(None);
        }
        ElementType(_) => return Ok(None),
        InvalidType => return Ok(None),
    };

    Ok(Some(field))
}

#[cfg(feature = "kuzu")]
/// 保存属性节点
pub async fn save_attr_node(pe: &SPdmsElement, attmap: &NamedAttrMap) -> Result<()> {
    let conn = create_kuzu_connection()?;
    let noun = &pe.noun;
    let table_name = format!("Attr_{}", noun.to_uppercase());

    // 构建属性字段
    let mut fields = vec![];
    let refno = pe.refno.refno().0;

    for (attr_name, attr_value) in &attmap.map {
        // 跳过特殊字段
        if attr_name == "REFNO"
            || attr_name == "TYPE"
            || attr_name == "OWNER"
            || attr_name.starts_with("UDA:")
        {
            continue;
        }

        if let Some(field) = format_attr_field(attr_name, attr_value)? {
            fields.push(field);
        }
    }

    // 使用 MERGE 避免重复键错误
    // Kuzu 的 MERGE 需要在匹配条件中包含所有要设置的属性
    fields.insert(0, format!("refno: {}", refno));

    let query = format!("MERGE (a:{} {{ {} }})", table_name, fields.join(", "));

    conn.query(&query)?;
    log::debug!("保存属性节点: {} refno={}", table_name, refno);

    Ok(())
}

#[cfg(feature = "kuzu")]
/// 批量保存属性节点
pub async fn save_attr_batch(models: &[(SPdmsElement, NamedAttrMap)]) -> Result<()> {
    let conn = create_kuzu_connection()?;

    conn.query("BEGIN TRANSACTION")?;

    let result = (|| {
        for (pe, attmap) in models {
            let noun = &pe.noun;
            let table_name = format!("Attr_{}", noun.to_uppercase());
            let refno = pe.refno.refno().0;

            let mut fields = vec![];

            for (attr_name, attr_value) in &attmap.map {
                if attr_name == "REFNO"
                    || attr_name == "TYPE"
                    || attr_name == "OWNER"
                    || attr_name.starts_with("UDA:")
                {
                    continue;
                }

                if let Some(field) = format_attr_field(attr_name, attr_value)? {
                    fields.push(field);
                }
            }

            // 使用 MERGE 避免重复键错误
            // Kuzu 的 MERGE 需要在匹配条件中包含所有要设置的属性
            fields.insert(0, format!("refno: {}", refno));

            let query = format!("MERGE (a:{} {{ {} }})", table_name, fields.join(", "));

            conn.query(&query)?;
        }
        Ok::<(), anyhow::Error>(())
    })();

    match result {
        Ok(_) => {
            conn.query("COMMIT")?;
            log::info!("批量保存属性节点成功: {} 个", models.len());
            Ok(())
        }
        Err(e) => {
            conn.query("ROLLBACK")?;
            log::error!("批量保存属性节点失败: {}", e);
            Err(e)
        }
    }
}

#[cfg(feature = "kuzu")]
/// 保存属性（兼容旧接口）
pub async fn save_attmap_kuzu(refno: RefnoEnum, attmap: &NamedAttrMap) -> Result<()> {
    // 需要从 NamedAttrMap 构建 SPdmsElement
    let pe = SPdmsElement {
        refno,
        noun: attmap.get_type(),
        name: attmap.get_name_or_default(),
        owner: attmap.get_owner(),
        dbnum: 0,
        sesno: attmap.sesno(),
        ..Default::default()
    };

    save_attr_node(&pe, attmap).await
}
