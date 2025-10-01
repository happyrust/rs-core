//! 简单测试 PE 数据同步
//!
//! 运行方式: cargo run --bin test_pe_sync_simple --features kuzu

use aios_core::rs_surreal::{SUL_DB, get_pe, query_type_refnos_by_dbnum};
use aios_core::{RefnoEnum, init_surreal};
use anyhow::Result;
use log::{error, info};
use simplelog::*;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    CombinedLogger::init(vec![TermLogger::new(
        LevelFilter::Debug,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )])?;

    info!("========================================");
    info!("简单 PE 数据同步测试");
    info!("========================================");

    // 1. 初始化 SurrealDB
    info!("步骤 1: 初始化 SurrealDB");
    init_surreal().await?;
    info!("✓ SurrealDB 初始化成功");

    // 2. 测试查询 dbnum=1112 的数据
    info!("步骤 2: 查询 dbnum=1112 的数据");

    // 查询 PIPE 类型的数据
    let pipe_refnos = query_type_refnos_by_dbnum(&["PIPE"], 1112, None, false).await?;
    info!("  找到 {} 个 PIPE 元素", pipe_refnos.len());

    // 查询前 5 个 PIPE 的详细信息
    info!("步骤 3: 获取前 5 个 PIPE 的详细信息");
    let sample_size = std::cmp::min(5, pipe_refnos.len());
    for i in 0..sample_size {
        let refno = pipe_refnos[i];
        if let Ok(Some(pe)) = get_pe(refno).await {
            info!(
                "  PE #{}: refno={}, name={}, owner={}, deleted={}",
                i + 1,
                pe.refno().0,
                pe.name,
                pe.owner.refno(),
                pe.deleted
            );
        }
    }

    // 3. 测试 Kuzu 初始化
    #[cfg(feature = "kuzu")]
    {
        use aios_core::rs_kuzu::{create_kuzu_connection, init_kuzu, init_kuzu_schema};
        use kuzu::SystemConfig;
        use std::fs;

        info!("");
        info!("步骤 4: 测试 Kuzu 数据库");

        let kuzu_db_path = "./data/test_kuzu_simple";

        // 清理旧数据
        if fs::metadata(kuzu_db_path).is_ok() {
            fs::remove_dir_all(kuzu_db_path)?;
        }

        // 初始化 Kuzu
        init_kuzu(kuzu_db_path, SystemConfig::default()).await?;
        info!("✓ Kuzu 数据库初始化成功");

        // 初始化模式
        init_kuzu_schema().await?;
        info!("✓ Kuzu 模式初始化成功");

        // 测试插入一个 PE 节点
        let conn = create_kuzu_connection()?;

        if let Some(first_refno) = pipe_refnos.first() {
            if let Ok(Some(pe)) = get_pe(*first_refno).await {
                let insert_sql = format!(
                    r#"
                    CREATE (p:PE {{
                        refno: {},
                        name: '{}',
                        noun: '{}',
                        dbnum: {},
                        sesno: {},
                        cata_hash: '{}',
                        deleted: {},
                        status_code: {},
                        lock: {}
                    }})
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
                    Ok(_) => info!("✓ 成功插入测试 PE 节点到 Kuzu"),
                    Err(e) => error!("✗ 插入失败: {}", e),
                }

                // 验证插入
                let query_sql = format!("MATCH (p:PE {{refno: {}}}) RETURN p.name", pe.refno().0);
                match conn.query(&query_sql) {
                    Ok(mut result) => {
                        if let Some(_row) = result.next() {
                            info!("✓ 成功从 Kuzu 查询到插入的节点");
                        }
                    }
                    Err(e) => error!("✗ 查询失败: {}", e),
                }
            }
        }
    }

    #[cfg(not(feature = "kuzu"))]
    {
        error!("Kuzu feature 未启用！请使用 --features kuzu 运行");
    }

    info!("");
    info!("========================================");
    info!("测试完成！");
    info!("========================================");

    Ok(())
}
