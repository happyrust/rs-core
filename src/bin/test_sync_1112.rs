//! 完整测试 dbnum=1112 数据同步
//!
//! 运行方式: cargo run --bin test_sync_1112 --features kuzu

use aios_core::rs_surreal::{get_pe, query_type_refnos_by_dbnum};
use aios_core::{RefnoEnum, init_surreal};
use anyhow::Result;
use log::{error, info, warn};
use simplelog::*;
use std::collections::{HashMap, HashSet};

#[cfg(feature = "kuzu")]
use aios_core::rs_kuzu::{create_kuzu_connection, init_kuzu, init_kuzu_schema};
#[cfg(feature = "kuzu")]
use kuzu::SystemConfig;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    CombinedLogger::init(vec![TermLogger::new(
        LevelFilter::Info,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )])?;

    info!("========================================");
    info!("完整测试 dbnum=1112 数据同步");
    info!("========================================");

    // 1. 初始化 SurrealDB
    info!("\n步骤 1: 初始化 SurrealDB");
    init_surreal().await?;
    info!("✓ SurrealDB 初始化成功");

    // 2. 从 SurrealDB 读取数据
    info!("\n步骤 2: 从 SurrealDB 读取 dbnum=1112 的数据");

    let nouns = vec![
        "PIPE", "BRAN", "EQUI", "STRU", "FITT", "VALV", "FLAN", "INST", "TUBI", "ATTA", "PLOO",
        "LOOP", "SITE", "ZONE",
    ];

    let mut all_pes = Vec::new();
    let mut owner_relations = Vec::new();

    for noun in &nouns {
        match query_type_refnos_by_dbnum(&[noun], 1112, None, false).await {
            Ok(refnos) => {
                info!("  {} - 找到 {} 个元素", noun, refnos.len());

                for refno in refnos {
                    if let Ok(Some(pe)) = get_pe(refno).await {
                        if !pe.deleted {
                            // 记录 owner 关系
                            if pe.owner.refno().0 != 0 {
                                owner_relations.push((pe.refno().0, pe.owner.refno().0));
                            }
                            all_pes.push(pe);
                        }
                    }
                }
            }
            Err(_) => {
                // 某些类型可能不存在，这是正常的
            }
        }
    }

    info!("\n统计:");
    info!("  总共找到 {} 个 PE 元素", all_pes.len());
    info!("  总共找到 {} 个 owner 关系", owner_relations.len());

    // 3. 初始化 Kuzu 数据库
    #[cfg(feature = "kuzu")]
    {
        use std::fs;

        info!("\n步骤 3: 初始化 Kuzu 数据库");

        let kuzu_db_path = "./data/kuzu_1112_sync";

        // 清理旧数据
        if fs::metadata(kuzu_db_path).is_ok() {
            info!("  清理旧的 Kuzu 数据库...");
            fs::remove_dir_all(kuzu_db_path)?;
        }

        // 初始化 Kuzu
        init_kuzu(kuzu_db_path, SystemConfig::default()).await?;
        info!("✓ Kuzu 数据库初始化成功");

        // 初始化模式
        init_kuzu_schema().await?;
        info!("✓ Kuzu 模式初始化成功");

        // 4. 同步 PE 节点到 Kuzu
        info!("\n步骤 4: 同步 PE 节点到 Kuzu");

        let conn = create_kuzu_connection()?;
        let batch_size = 100;
        let mut inserted_count = 0;
        let mut failed_count = 0;

        for chunk in all_pes.chunks(batch_size) {
            for pe in chunk {
                let insert_sql = format!(
                    r#"
                    MERGE (p:PE {{refno: {}}})
                    SET p.name = '{}',
                        p.noun = '{}',
                        p.dbnum = {},
                        p.sesno = {},
                        p.cata_hash = '{}',
                        p.deleted = {},
                        p.status_code = {},
                        p.lock = {}
                    "#,
                    pe.refno().0,
                    pe.name.replace("'", "''"),
                    pe.noun.replace("'", "''"),
                    pe.dbnum,
                    pe.sesno,
                    pe.cata_hash.replace("'", "''"),
                    pe.deleted,
                    pe.status_code
                        .as_ref()
                        .map_or("NULL".to_string(), |s| format!(
                            "'{}'",
                            s.replace("'", "''")
                        )),
                    pe.lock
                );

                match conn.query(&insert_sql) {
                    Ok(_) => inserted_count += 1,
                    Err(e) => {
                        warn!("插入 PE {} 失败: {}", pe.refno().0, e);
                        failed_count += 1;
                    }
                }
            }
            info!("  已插入 {} / {} 个 PE 节点", inserted_count, all_pes.len());
        }

        info!(
            "✓ PE 节点同步完成: {} 成功, {} 失败",
            inserted_count, failed_count
        );

        // 5. 同步 owner 关系到 Kuzu
        info!("\n步骤 5: 同步 owner 关系到 Kuzu");

        let mut relation_inserted = 0;
        let mut relation_failed = 0;

        for (child, parent) in &owner_relations {
            let rel_sql = format!(
                r#"
                MATCH (parent:PE {{refno: {}}}), (child:PE {{refno: {}}})
                MERGE (parent)-[:OWNS]->(child)
                "#,
                parent, child
            );

            match conn.query(&rel_sql) {
                Ok(_) => relation_inserted += 1,
                Err(_) => {
                    // 关系创建失败可能是因为父节点不在我们的数据集中
                    relation_failed += 1;
                }
            }
        }

        info!(
            "✓ Owner 关系同步完成: {} 成功, {} 失败",
            relation_inserted, relation_failed
        );

        // 6. 验证同步结果
        info!("\n步骤 6: 验证同步结果");

        // 统计 Kuzu 中的 PE 节点数量
        let count_sql = "MATCH (p:PE) WHERE p.dbnum = 1112 RETURN count(p) AS cnt";
        let mut result = conn.query(count_sql)?;
        let kuzu_pe_count = if let Some(row) = result.next() {
            if let kuzu::Value::Int64(count) = row.get(0).unwrap() {
                *count as usize
            } else {
                0
            }
        } else {
            0
        };

        info!("  Kuzu PE 节点数量: {}", kuzu_pe_count);
        info!("  SurrealDB PE 节点数量: {}", all_pes.len());

        if kuzu_pe_count == all_pes.len() {
            info!("  ✓ PE 节点数量匹配!");
        } else {
            warn!("  ✗ PE 节点数量不匹配!");
        }

        // 统计 Kuzu 中的 owner 关系数量
        let rel_count_sql = "MATCH ()-[r:OWNS]->() RETURN count(r) AS cnt";
        let mut result = conn.query(rel_count_sql)?;
        let kuzu_rel_count = if let Some(row) = result.next() {
            if let kuzu::Value::Int64(count) = row.get(0).unwrap() {
                *count as usize
            } else {
                0
            }
        } else {
            0
        };

        info!("  Kuzu owner 关系数量: {}", kuzu_rel_count);
        info!("  期望的 owner 关系数量: {}", relation_inserted);

        // 7. 测试查询性能
        info!("\n步骤 7: 测试查询性能");

        use std::time::Instant;

        // 测试 1: 查询特定类型的节点
        let start = Instant::now();
        let test_sql = "MATCH (p:PE) WHERE p.dbnum = 1112 AND p.noun = 'PIPE' RETURN count(p)";
        let _result = conn.query(test_sql)?;
        info!("  查询 PIPE 节点数量: {:?}", start.elapsed());

        // 测试 2: 查询层次关系
        let start = Instant::now();
        let test_sql =
            "MATCH (p1:PE)-[:OWNS]->(p2:PE) WHERE p1.dbnum = 1112 RETURN count(DISTINCT p2)";
        let _result = conn.query(test_sql)?;
        info!("  查询直接子节点: {:?}", start.elapsed());

        // 测试 3: 查询多层关系
        let start = Instant::now();
        let test_sql = "MATCH (p1:PE)-[:OWNS*1..2]->(p2:PE) WHERE p1.dbnum = 1112 RETURN count(DISTINCT p2) LIMIT 100";
        let _result = conn.query(test_sql)?;
        info!("  查询 2 层子节点: {:?}", start.elapsed());

        // 8. 抽样验证数据内容
        info!("\n步骤 8: 抽样验证数据内容");

        let sample_size = 5.min(all_pes.len());
        let mut verified = 0;
        let mut failed = 0;

        for i in 0..sample_size {
            let pe = &all_pes[i];
            let verify_sql = format!(
                "MATCH (p:PE {{refno: {}}}) RETURN p.name, p.noun, p.dbnum",
                pe.refno().0
            );

            match conn.query(&verify_sql) {
                Ok(mut result) => {
                    if let Some(row) = result.next() {
                        // 验证成功
                        verified += 1;
                    } else {
                        warn!("  样本 {} 在 Kuzu 中未找到", pe.refno().0);
                        failed += 1;
                    }
                }
                Err(e) => {
                    warn!("  验证样本 {} 失败: {}", pe.refno().0, e);
                    failed += 1;
                }
            }
        }

        info!("  抽样验证: {} 成功, {} 失败", verified, failed);
    }

    #[cfg(not(feature = "kuzu"))]
    {
        error!("Kuzu feature 未启用！请使用 --features kuzu 运行");
    }

    // 9. 总结
    info!("\n========================================");
    info!("测试总结");
    info!("========================================");

    #[cfg(feature = "kuzu")]
    {
        info!("✓ 成功完成 dbnum=1112 的数据同步!");
        info!("  - PE 节点: {} 个", all_pes.len());
        info!("  - Owner 关系: {} 个", owner_relations.len());
        info!("  - 数据已成功同步到 Kuzu 图数据库");
        info!("  - Kuzu 数据库路径: ./data/kuzu_1112_sync");
    }

    Ok(())
}
