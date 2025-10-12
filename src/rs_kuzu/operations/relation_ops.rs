//! 关系操作
//!
//! 提供关系的创建和管理操作

#[cfg(feature = "kuzu")]
use crate::rs_kuzu::create_kuzu_connection;
#[cfg(feature = "kuzu")]
use crate::types::*;
#[cfg(feature = "kuzu")]
use anyhow::Result;

#[cfg(feature = "kuzu")]
/// 创建 PE 到属性的关系
pub async fn create_pe_to_attr_relation(pe: &SPdmsElement) -> Result<()> {
    let conn = create_kuzu_connection()?;
    let refno = pe.refno.refno().0;
    let noun = &pe.noun.to_uppercase();

    // 使用 MERGE 避免重复边
    let query = format!(
        "MATCH (p:PE {{refno: {}}}), (a:Attr_{} {{refno: {}}})
         MERGE (p)-[:TO_{}]->(a)",
        refno, noun, refno, noun
    );

    conn.query(&query)?;
    log::debug!("创建 PE->Attr 关系: {} -> Attr_{}", refno, noun);

    Ok(())
}

#[cfg(feature = "kuzu")]
/// 创建所有权关系
pub async fn create_owns_relation(pe: &SPdmsElement) -> Result<()> {
    if pe.owner.refno().is_unset() {
        return Ok(());
    }

    let conn = create_kuzu_connection()?;
    let refno = pe.refno.refno().0;
    let owner_refno = pe.owner.refno().0;

    // 使用 MERGE 避免重复边
    let query = format!(
        "MATCH (p1:PE {{refno: {}}}), (p2:PE {{refno: {}}})
         MERGE (p2)-[:OWNS]->(p1)",
        refno, owner_refno
    );

    conn.query(&query)?;
    log::debug!("创建 OWNS 关系: {} -> {}", owner_refno, refno);

    Ok(())
}

#[cfg(feature = "kuzu")]
/// 创建属性引用关系
pub async fn create_attr_reference_relations(
    pe: &SPdmsElement,
    attmap: &NamedAttrMap
) -> Result<()> {
    let conn = create_kuzu_connection()?;
    let refno = pe.refno.refno().0;
    let noun = &pe.noun.to_uppercase();

    for (attr_name, attr_value) in &attmap.map {
        match attr_value {
            NamedAttrValue::RefU64Type(target_refno) => {
                if target_refno.is_unset() {
                    continue;
                }

                let edge_name = format!("{}_{}", noun, attr_name.to_uppercase());
                let target = target_refno.0;

                // 使用 MERGE 避免重复边
                let query = format!(
                    "MATCH (a:Attr_{} {{refno: {}}}), (p:PE {{refno: {}}})
                     MERGE (a)-[:{}]->(p)",
                    noun, refno, target, edge_name
                );

                match conn.query(&query) {
                    Ok(_) => {
                        log::debug!("创建属性引用关系: {} -> PE:{}", edge_name, target);
                    }
                    Err(e) => {
                        log::warn!("创建引用关系失败 {} ({}->{}): {}", edge_name, refno, target, e);
                    }
                }
            }
            NamedAttrValue::RefnoEnumType(target_refno) => {
                if target_refno.refno().is_unset() {
                    continue;
                }

                let edge_name = format!("{}_{}", noun, attr_name.to_uppercase());
                let target = target_refno.refno().0;

                // 使用 MERGE 避免重复边
                let query = format!(
                    "MATCH (a:Attr_{} {{refno: {}}}), (p:PE {{refno: {}}})
                     MERGE (a)-[:{}]->(p)",
                    noun, refno, target, edge_name
                );

                match conn.query(&query) {
                    Ok(_) => {
                        log::debug!("创建属性引用关系: {} -> PE:{}", edge_name, target);
                    }
                    Err(e) => {
                        log::warn!("创建引用关系失败 {} ({}->{}): {}", edge_name, refno, target, e);
                    }
                }
            }
            NamedAttrValue::RefU64Array(refnos) => {
                for (idx, refno_enum) in refnos.iter().enumerate() {
                    let target = refno_enum.refno().0;
                    if target == 0 {
                        continue;
                    }

                    let edge_name = format!("{}_{}_{}", noun, attr_name.to_uppercase(), idx);
                    let query = format!(
                        "MATCH (a:Attr_{} {{refno: {}}}), (p:PE {{refno: {}}})
                         CREATE (a)-[:{}]->(p)",
                        noun, refno, target, edge_name
                    );

                    match conn.query(&query) {
                        Ok(_) => {
                            log::debug!("创建数组引用关系: {} -> PE:{}", edge_name, target);
                        }
                        Err(e) => {
                            log::warn!("创建数组引用失败 {} ({}->{}): {}", edge_name, refno, target, e);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    Ok(())
}

#[cfg(feature = "kuzu")]
/// 创建所有关系
pub async fn create_all_relations(
    pe: &SPdmsElement,
    attmap: &NamedAttrMap
) -> Result<()> {
    create_pe_to_attr_relation(pe).await?;
    create_owns_relation(pe).await?;
    create_attr_reference_relations(pe, attmap).await?;

    Ok(())
}

#[cfg(feature = "kuzu")]
/// 批量创建关系
pub async fn create_relations_batch(
    models: &[(SPdmsElement, NamedAttrMap)]
) -> Result<()> {
    let conn = create_kuzu_connection()?;

    conn.query("BEGIN TRANSACTION")?;

    let result = (|| {
        for (pe, attmap) in models {
            // PE->Attr 关系
            let refno = pe.refno.refno().0;
            let noun = &pe.noun.to_uppercase();

            let query = format!(
                "MATCH (p:PE {{refno: {}}}), (a:Attr_{} {{refno: {}}})
                 CREATE (p)-[:TO_{}]->(a)",
                refno, noun, refno, noun
            );
            conn.query(&query)?;

            // OWNS 关系
            if !pe.owner.refno().is_unset() {
                let owner_refno = pe.owner.refno().0;
                let query = format!(
                    "MATCH (p1:PE {{refno: {}}}), (p2:PE {{refno: {}}})
                     CREATE (p2)-[:OWNS]->(p1)",
                    refno, owner_refno
                );
                conn.query(&query).ok(); // 忽略错误（owner 可能不存在）
            }

            // 属性引用关系
            for (attr_name, attr_value) in &attmap.map {
                match attr_value {
                    NamedAttrValue::RefU64Type(target_refno) |
                    NamedAttrValue::RefnoEnumType(crate::types::RefnoEnum::Refno(target_refno)) => {
                        if !target_refno.is_unset() {
                            let edge_name = format!("{}_{}", noun, attr_name.to_uppercase());
                            let target = target_refno.0;

                            let query = format!(
                                "MATCH (a:Attr_{} {{refno: {}}}), (p:PE {{refno: {}}})
                                 CREATE (a)-[:{}]->(p)",
                                noun, refno, target, edge_name
                            );
                            conn.query(&query).ok(); // 忽略错误
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok::<(), anyhow::Error>(())
    })();

    match result {
        Ok(_) => {
            conn.query("COMMIT")?;
            log::info!("批量创建关系成功: {} 个模型", models.len());
            Ok(())
        }
        Err(e) => {
            conn.query("ROLLBACK")?;
            log::error!("批量创建关系失败: {}", e);
            Err(e)
        }
    }
}

#[cfg(feature = "kuzu")]
/// 创建关系（兼容旧接口）
pub async fn create_relation_kuzu(from: RefnoEnum, to: RefnoEnum, rel_type: &str) -> Result<()> {
    let conn = create_kuzu_connection()?;

    let from_refno = from.refno().0;
    let to_refno = to.refno().0;

    let query = format!(
        "MATCH (p1:PE {{refno: {}}}), (p2:PE {{refno: {}}})
         CREATE (p1)-[:{}]->(p2)",
        from_refno, to_refno, rel_type.to_uppercase()
    );

    conn.query(&query)?;
    log::debug!("创建关系: {} -[{}]-> {}", from_refno, rel_type, to_refno);

    Ok(())
}
