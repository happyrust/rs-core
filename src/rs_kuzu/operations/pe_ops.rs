//! PE 操作
//!
//! 提供 PE 节点的写入和更新操作

#[cfg(feature = "kuzu")]
use crate::rs_kuzu::create_kuzu_connection;
#[cfg(feature = "kuzu")]
use crate::types::*;
#[cfg(feature = "kuzu")]
use anyhow::Result;

#[cfg(feature = "kuzu")]
/// 转义字符串中的单引号
fn escape_string(s: &str) -> String {
    s.replace("'", "''")
}

#[cfg(feature = "kuzu")]
/// 保存单个 PE 节点
pub async fn save_pe_node(pe: &SPdmsElement) -> Result<()> {
    let conn = create_kuzu_connection()?;

    // 使用 MERGE 避免重复键错误
    let query = format!(
        "MERGE (p:PE {{refno: {}}})
         ON MATCH SET
            p.name = '{}',
            p.noun = '{}',
            p.dbnum = {},
            p.sesno = {},
            p.cata_hash = '{}',
            p.deleted = {},
            p.lock = {},
            p.typex = {}
         ON CREATE SET
            p.name = '{}',
            p.noun = '{}',
            p.dbnum = {},
            p.sesno = {},
            p.cata_hash = '{}',
            p.deleted = {},
            p.lock = {},
            p.typex = {}",
        pe.refno.refno().0,
        escape_string(&pe.name),
        escape_string(&pe.noun),
        pe.dbnum,
        pe.sesno,
        escape_string(&pe.cata_hash),
        pe.deleted,
        pe.lock,
        pe.typex.unwrap_or(0),
        escape_string(&pe.name),
        escape_string(&pe.noun),
        pe.dbnum,
        pe.sesno,
        escape_string(&pe.cata_hash),
        pe.deleted,
        pe.lock,
        pe.typex.unwrap_or(0)
    );

    conn.query(&query)?;
    log::debug!(
        "保存 PE 节点: {} ({}) refno={} typex={:?}",
        pe.name,
        pe.noun,
        pe.refno.refno(),
        pe.typex
    );

    Ok(())
}

#[cfg(feature = "kuzu")]
/// 批量保存 PE 节点
pub async fn save_pe_batch(pes: &[SPdmsElement]) -> Result<()> {
    let conn = create_kuzu_connection()?;

    conn.query("BEGIN TRANSACTION")?;

    let result = (|| {
        for pe in pes {
            // 使用 MERGE 避免重复键错误
            let query = format!(
                "MERGE (p:PE {{refno: {}}})
                 ON MATCH SET
                    p.name = '{}',
                    p.noun = '{}',
                    p.dbnum = {},
                    p.sesno = {},
                    p.cata_hash = '{}',
                    p.deleted = {},
                    p.lock = {},
                    p.typex = {}
                 ON CREATE SET
                    p.name = '{}',
                    p.noun = '{}',
                    p.dbnum = {},
                    p.sesno = {},
                    p.cata_hash = '{}',
                    p.deleted = {},
                    p.lock = {},
                    p.typex = {}",
                pe.refno.refno().0,
                escape_string(&pe.name),
                escape_string(&pe.noun),
                pe.dbnum,
                pe.sesno,
                escape_string(&pe.cata_hash),
                pe.deleted,
                pe.lock,
                pe.typex.unwrap_or(0),
                escape_string(&pe.name),
                escape_string(&pe.noun),
                pe.dbnum,
                pe.sesno,
                escape_string(&pe.cata_hash),
                pe.deleted,
                pe.lock,
                pe.typex.unwrap_or(0)
            );

            conn.query(&query)?;
        }
        Ok::<(), anyhow::Error>(())
    })();

    match result {
        Ok(_) => {
            conn.query("COMMIT")?;
            log::info!("批量保存 PE 节点成功: {} 个", pes.len());
            Ok(())
        }
        Err(e) => {
            conn.query("ROLLBACK")?;
            log::error!("批量保存 PE 节点失败: {}", e);
            Err(e)
        }
    }
}

#[cfg(feature = "kuzu")]
/// 保存 PE（兼容旧接口）
pub async fn save_pe_kuzu(pe: &SPdmsElement) -> Result<()> {
    save_pe_node(pe).await
}

#[cfg(feature = "kuzu")]
/// 批量保存 PE（兼容旧接口）
pub async fn save_pe_batch_kuzu(pes: Vec<SPdmsElement>) -> Result<()> {
    save_pe_batch(&pes).await
}

#[cfg(feature = "kuzu")]
/// 批量保存 PE 节点（仅节点，不创建关系）
///
/// 用于快速导入大规模数据，仅保存 PE 节点，不创建 OWNS 关系
/// 应该先调用此方法保存所有节点，然后再调用 save_pe_owns_batch 创建关系
///
/// # 参数
/// * `pe_batch` - PE 元素列表
///
/// # 返回值
/// * `Ok(())` - 保存成功
/// * `Err(...)` - 保存失败，包含错误信息
///
/// # 示例
/// ```rust
/// let pe1 = SPdmsElement { refno: RefU64(100).into(), name: "Site1".to_string(), noun: "SITE".to_string(), dbnum: 1112, sesno: 1, ..Default::default() };
/// let pe2 = SPdmsElement { refno: RefU64(200).into(), name: "Zone1".to_string(), noun: "ZONE".to_string(), dbnum: 1112, sesno: 1, ..Default::default() };
/// let batch = vec![pe1, pe2];
/// save_pe_nodes_batch(&batch).await?;
/// ```
pub async fn save_pe_nodes_batch(pe_batch: &[SPdmsElement]) -> Result<()> {
    let conn = create_kuzu_connection()?;

    // 开启事务
    conn.query("BEGIN TRANSACTION")?;

    let result = (|| {
        for pe in pe_batch {
            let create_pe_query = format!(
                "CREATE (p:PE {{
                    refno: {},
                    name: '{}',
                    noun: '{}',
                    dbnum: {},
                    sesno: {},
                    cata_hash: '{}',
                    deleted: {},
                    lock: {},
                    typex: {}
                }})",
                pe.refno.refno().0,
                escape_string(&pe.name),
                escape_string(&pe.noun),
                pe.dbnum,
                pe.sesno,
                escape_string(&pe.cata_hash),
                pe.deleted,
                pe.lock,
                pe.typex.unwrap_or(0)
            );

            // 如果节点已存在，忽略错误（INSERT IGNORE 语义）
            match conn.query(&create_pe_query) {
                Ok(_) => {},
                Err(e) => {
                    let err_msg = e.to_string();
                    if !err_msg.contains("already exists") && !err_msg.contains("duplicate") {
                        return Err(anyhow::anyhow!(
                            "创建 PE 节点失败 {} ({}): {}",
                            pe.refno.refno(),
                            pe.noun,
                            e
                        ));
                    }
                    log::trace!("PE 节点已存在，跳过: {}", pe.refno.refno());
                }
            }
        }
        Ok::<(), anyhow::Error>(())
    })();

    match result {
        Ok(_) => {
            conn.query("COMMIT")?;
            log::debug!("批量保存 PE 节点成功: {} 个", pe_batch.len());
            Ok(())
        }
        Err(e) => {
            conn.query("ROLLBACK")?;
            log::error!("批量保存 PE 节点失败: {}", e);
            Err(e)
        }
    }
}

#[cfg(feature = "kuzu")]
/// 批量创建 OWNS 关系（使用批量查询优化）
///
/// 在所有 PE 节点保存完成后，调用此方法创建 OWNS 关系
/// 必须先调用 save_pe_nodes_batch 保存所有节点
///
/// 使用 HashMap 分组后批量创建，提高性能
///
/// # 参数
/// * `owns_batch` - (child_refno, owner_refno) 的列表
///   - child_refno: 子节点的 refno
///   - owner_refno: 父节点的 refno（RefU64(0) 表示无 owner，会被跳过）
///
/// # 返回值
/// * `Ok(())` - 保存成功
/// * `Err(...)` - 保存失败，包含错误信息
///
/// # 示例
/// ```rust
/// // 先保存所有节点
/// save_pe_nodes_batch(&pe_batch).await?;
///
/// // 再创建关系
/// let owns = vec![(RefU64(200), RefU64(100))]; // Zone 200 的 owner 是 Site 100
/// save_pe_owns_batch(&owns).await?;
/// ```
pub async fn save_pe_owns_batch(owns_batch: &[(RefU64, RefU64)]) -> Result<()> {
    use std::collections::HashMap;

    let conn = create_kuzu_connection()?;

    // 使用 HashMap 按 owner 分组，收集所有子节点
    // owner_refno -> [child_refno1, child_refno2, ...]
    let mut owner_children_map: HashMap<u64, Vec<u64>> = HashMap::new();

    for (child_refno, owner_refno) in owns_batch {
        // 跳过没有 owner 的节点
        if owner_refno.0 == 0 {
            continue;
        }

        owner_children_map
            .entry(owner_refno.0)
            .or_insert_with(Vec::new)
            .push(child_refno.0);
    }

    if owner_children_map.is_empty() {
        log::debug!("没有需要创建的 OWNS 关系");
        return Ok(());
    }

    // 开启事务
    conn.query("BEGIN TRANSACTION")?;

    let result = (|| {
        // 对每个 owner，批量创建到其所有子节点的关系
        for (owner_refno, children_refnos) in owner_children_map.iter() {
            // 构建批量 MATCH + CREATE 查询
            // MATCH (owner:PE {refno: 100})
            // MATCH (c1:PE {refno: 200}), (c2:PE {refno: 201}), ...
            // CREATE (owner)-[:OWNS]->(c1), (owner)-[:OWNS]->(c2), ...

            let children_count = children_refnos.len();

            // 生成子节点的 MATCH 子句和别名
            let mut match_clauses = Vec::new();
            let mut create_patterns = Vec::new();

            match_clauses.push(format!("MATCH (owner:PE {{refno: {}}})", owner_refno));

            for (idx, child_refno) in children_refnos.iter().enumerate() {
                let child_alias = format!("c{}", idx);
                match_clauses.push(format!(
                    "MATCH ({}:PE {{refno: {}}})",
                    child_alias, child_refno
                ));
                create_patterns.push(format!("(owner)-[:OWNS]->({})", child_alias));
            }

            // 组合成完整查询
            let create_owns_query = format!(
                "{}\nCREATE {}",
                match_clauses.join("\n"),
                create_patterns.join(", ")
            );

            // 执行批量创建
            match conn.query(&create_owns_query) {
                Ok(_) => {
                    log::trace!(
                        "成功创建 OWNS 关系: owner {} -> {} 个子节点",
                        owner_refno,
                        children_count
                    );
                },
                Err(e) => {
                    let err_msg = e.to_string();
                    if err_msg.contains("already exists") || err_msg.contains("duplicate") {
                        log::trace!(
                            "部分 OWNS 关系已存在，跳过: owner {} -> {} 个子节点",
                            owner_refno,
                            children_count
                        );
                    } else if err_msg.contains("not found") || err_msg.contains("does not exist") {
                        // 节点不存在
                        return Err(anyhow::anyhow!(
                            "节点不存在，无法创建 OWNS 关系: owner {} -> {} 个子节点 (请先调用 save_pe_nodes_batch)",
                            owner_refno,
                            children_count
                        ));
                    } else {
                        return Err(anyhow::anyhow!(
                            "批量创建 OWNS 关系失败 owner {} -> {} 个子节点: {}",
                            owner_refno,
                            children_count,
                            e
                        ));
                    }
                }
            }
        }
        Ok::<(), anyhow::Error>(())
    })();

    match result {
        Ok(_) => {
            conn.query("COMMIT")?;
            log::debug!(
                "批量创建 OWNS 关系成功: {} 个唯一 owner，共 {} 条关系",
                owner_children_map.len(),
                owns_batch.len()
            );
            Ok(())
        }
        Err(e) => {
            conn.query("ROLLBACK")?;
            log::error!("批量创建 OWNS 关系失败: {}", e);
            Err(e)
        }
    }
}

#[cfg(feature = "kuzu")]
/// 从 HashMap 批量创建 OWNS 关系（最优化版本）
///
/// 直接接受已经构建好的 owner-children 映射，避免重复遍历
///
/// # 参数
/// * `owner_children_map` - owner_refno -> [child_refno1, child_refno2, ...] 的映射
///
/// # 返回值
/// * `Ok(relation_count)` - 成功创建的关系数量
/// * `Err(...)` - 保存失败，包含错误信息
///
/// # 示例
/// ```rust
/// use std::collections::HashMap;
///
/// let mut map = HashMap::new();
/// map.insert(100, vec![200, 201, 202]); // Site 100 -> Zone 200, 201, 202
/// map.insert(200, vec![300, 301]);      // Zone 200 -> Equi 300, 301
///
/// let count = save_pe_owns_from_map(&map).await?;
/// println!("创建了 {} 个关系", count);
/// ```
pub async fn save_pe_owns_from_map(
    owner_children_map: &std::collections::HashMap<u64, Vec<u64>>
) -> Result<usize> {
    let conn = create_kuzu_connection()?;

    if owner_children_map.is_empty() {
        log::debug!("没有需要创建的 OWNS 关系");
        return Ok(0);
    }

    // 开启事务
    conn.query("BEGIN TRANSACTION")?;

    let mut total_relations = 0;

    let result = (|| {
        // 对每个 owner，批量创建到其所有子节点的关系
        for (owner_refno, children_refnos) in owner_children_map.iter() {
            let children_count = children_refnos.len();
            total_relations += children_count;

            // 生成子节点的 MATCH 子句和别名
            let mut match_clauses = Vec::new();
            let mut create_patterns = Vec::new();

            match_clauses.push(format!("MATCH (owner:PE {{refno: {}}})", owner_refno));

            for (idx, child_refno) in children_refnos.iter().enumerate() {
                let child_alias = format!("c{}", idx);
                match_clauses.push(format!(
                    "MATCH ({}:PE {{refno: {}}})",
                    child_alias, child_refno
                ));
                create_patterns.push(format!("(owner)-[:OWNS]->({})", child_alias));
            }

            // 组合成完整查询
            let create_owns_query = format!(
                "{}\nCREATE {}",
                match_clauses.join("\n"),
                create_patterns.join(", ")
            );

            // 执行批量创建
            match conn.query(&create_owns_query) {
                Ok(_) => {
                    log::trace!(
                        "成功创建 OWNS 关系: owner {} -> {} 个子节点",
                        owner_refno,
                        children_count
                    );
                },
                Err(e) => {
                    let err_msg = e.to_string();
                    if err_msg.contains("already exists") || err_msg.contains("duplicate") {
                        log::trace!(
                            "部分 OWNS 关系已存在，跳过: owner {} -> {} 个子节点",
                            owner_refno,
                            children_count
                        );
                    } else if err_msg.contains("not found") || err_msg.contains("does not exist") {
                        return Err(anyhow::anyhow!(
                            "节点不存在，无法创建 OWNS 关系: owner {} -> {} 个子节点",
                            owner_refno,
                            children_count
                        ));
                    } else {
                        return Err(anyhow::anyhow!(
                            "批量创建 OWNS 关系失败 owner {} -> {} 个子节点: {}",
                            owner_refno,
                            children_count,
                            e
                        ));
                    }
                }
            }
        }
        Ok::<(), anyhow::Error>(())
    })();

    match result {
        Ok(_) => {
            conn.query("COMMIT")?;
            log::debug!(
                "批量创建 OWNS 关系成功: {} 个唯一 owner，共 {} 条关系",
                owner_children_map.len(),
                total_relations
            );
            Ok(total_relations)
        }
        Err(e) => {
            conn.query("ROLLBACK")?;
            log::error!("批量创建 OWNS 关系失败: {}", e);
            Err(e)
        }
    }
}

#[cfg(not(feature = "kuzu"))]
/// PE 节点批量保存（未启用 kuzu feature）
pub async fn save_pe_nodes_batch(_pe_batch: &[SPdmsElement]) -> Result<()> {
    Err(anyhow::anyhow!("kuzu feature 未启用"))
}

#[cfg(not(feature = "kuzu"))]
/// OWNS 关系批量保存（未启用 kuzu feature）
pub async fn save_pe_owns_batch(_owns_batch: &[(RefU64, RefU64)]) -> Result<()> {
    Err(anyhow::anyhow!("kuzu feature 未启用"))
}

#[cfg(not(feature = "kuzu"))]
/// HashMap 方式批量保存（未启用 kuzu feature）
pub async fn save_pe_owns_from_map(
    _owner_children_map: &std::collections::HashMap<u64, Vec<u64>>
) -> Result<usize> {
    Err(anyhow::anyhow!("kuzu feature 未启用"))
}
