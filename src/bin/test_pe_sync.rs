//! 测试 PE 数据同步到 Kuzu
//!
//! 运行方式: cargo run --bin test_pe_sync --features kuzu

use aios_core::sync::PeSyncService;
use aios_core::{init_surreal, init_test_surreal};
use anyhow::Result;
use log::{error, info};
use simplelog::*;
use std::fs;

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
    info!("PE 数据同步测试");
    info!("========================================");

    // 1. 初始化 SurrealDB
    info!("步骤 1: 初始化 SurrealDB");
    match init_surreal().await {
        Ok(_) => info!("✓ SurrealDB 初始化成功"),
        Err(e) => {
            error!("✗ SurrealDB 初始化失败: {}", e);
            info!("尝试使用测试模式初始化...");
            init_test_surreal().await?;
            info!("✓ SurrealDB 测试模式初始化成功");
        }
    }

    // 2. 创建 Kuzu 数据库目录
    let kuzu_db_dir = "./data";
    let kuzu_db_path = "./data/kuzu_pe_sync_test";
    info!("步骤 2: 准备 Kuzu 数据库路径: {}", kuzu_db_path);

    // 确保数据目录存在
    fs::create_dir_all(kuzu_db_dir)?;

    // 清理旧数据（如果存在）
    if fs::metadata(kuzu_db_path).is_ok() {
        info!("  清理旧的 Kuzu 数据库...");
        fs::remove_dir_all(kuzu_db_path)?;
    }
    info!("✓ Kuzu 数据库路径已准备");

    // 3. 创建同步服务
    info!("步骤 3: 创建同步服务");
    let sync_service = PeSyncService::new(1000) // 批处理大小为 1000
        .with_dbnum_filter(1112); // 只同步 dbnum=1112 的数据
    info!("✓ 同步服务已创建 (批处理大小: 1000, 过滤 dbnum: 1112)");

    // 4. 初始化 Kuzu 数据库
    #[cfg(feature = "kuzu")]
    {
        info!("步骤 4: 初始化 Kuzu 数据库和模式");
        sync_service.init_kuzu_database(kuzu_db_path).await?;
        info!("✓ Kuzu 数据库和模式初始化完成");
    }

    #[cfg(not(feature = "kuzu"))]
    {
        error!("Kuzu feature 未启用！请使用 --features kuzu 运行");
        return Ok(());
    }

    // 5. 执行数据同步
    info!("步骤 5: 开始数据同步");
    info!("  正在从 SurrealDB 同步 dbnum=1112 的数据到 Kuzu...");

    let sync_stats = sync_service.sync_all().await?;

    info!("✓ 数据同步完成！");
    info!("  - PE 节点数量: {}", sync_stats.pe_count);
    info!("  - Owner 关系数量: {}", sync_stats.owner_count);
    info!("  - 耗时: {:?}", sync_stats.duration);

    // 6. 验证同步结果
    info!("");
    info!("步骤 6: 验证同步结果");
    info!("========================================");

    #[cfg(feature = "kuzu")]
    {
        let verification = sync_service.verify_sync(Some(1112)).await?;

        info!("PE 节点数量对比:");
        info!("  - SurrealDB: {} 个节点", verification.surreal_pe_count);
        info!("  - Kuzu:      {} 个节点", verification.kuzu_pe_count);
        info!(
            "  - 匹配状态:  {}",
            if verification.pe_count_match {
                "✓ 匹配"
            } else {
                "✗ 不匹配"
            }
        );

        info!("");
        info!("Owner 关系数量对比:");
        info!("  - SurrealDB: {} 个关系", verification.surreal_owner_count);
        info!("  - Kuzu:      {} 个关系", verification.kuzu_owner_count);
        info!(
            "  - 匹配状态:  {}",
            if verification.owner_count_match {
                "✓ 匹配"
            } else {
                "✗ 不匹配"
            }
        );

        info!("");
        info!("数据内容抽样验证:");
        info!(
            "  - 状态: {}",
            if verification.sample_verification {
                "✓ 通过"
            } else {
                "✗ 失败"
            }
        );

        // 7. 测试查询性能
        info!("");
        info!("步骤 7: 测试查询性能");
        info!("========================================");

        // 测试一些典型查询
        test_query_performance().await?;

        // 8. 总结
        info!("");
        info!("========================================");
        info!("测试总结");
        info!("========================================");

        if verification.pe_count_match
            && verification.owner_count_match
            && verification.sample_verification
        {
            info!("✓ 所有测试通过！");
            info!("  数据已成功从 SurrealDB 同步到 Kuzu");
            info!("  dbnum=1112 的所有数据已正确同步");
        } else {
            error!("✗ 部分测试失败，请检查日志");
            if !verification.pe_count_match {
                error!("  - PE 节点数量不匹配");
            }
            if !verification.owner_count_match {
                error!("  - Owner 关系数量不匹配");
            }
            if !verification.sample_verification {
                error!("  - 数据内容验证失败");
            }
        }
    }

    Ok(())
}

#[cfg(feature = "kuzu")]
async fn test_query_performance() -> Result<()> {
    use aios_core::rs_kuzu::create_kuzu_connection;
    use std::time::Instant;

    let conn = create_kuzu_connection()?;

    // 测试 1: 查询特定 dbnum 的节点数量
    let start = Instant::now();
    let _result = conn.query("MATCH (p:PE) WHERE p.dbnum = 1112 RETURN count(p)")?;
    let duration1 = start.elapsed();
    info!("查询 1 - 统计 dbnum=1112 的节点数量: {:?}", duration1);

    // 测试 2: 查询层次关系深度
    let start = Instant::now();
    let _result = conn.query(
        r#"
        MATCH (p1:PE)-[:OWNS*1..3]->(p2:PE)
        WHERE p1.dbnum = 1112
        RETURN count(DISTINCT p2) LIMIT 10
    "#,
    )?;
    let duration2 = start.elapsed();
    info!("查询 2 - 查询 3 层深度的子节点: {:?}", duration2);

    // 测试 3: 查询特定类型的节点
    let start = Instant::now();
    let _result = conn.query(
        r#"
        MATCH (p:PE)
        WHERE p.dbnum = 1112 AND p.noun = 'PIPE'
        RETURN count(p)
    "#,
    )?;
    let duration3 = start.elapsed();
    info!("查询 3 - 查询 PIPE 类型节点数量: {:?}", duration3);

    // 测试 4: 查询没有子节点的叶子节点
    let start = Instant::now();
    let _result = conn.query(
        r#"
        MATCH (p:PE)
        WHERE p.dbnum = 1112 AND NOT EXISTS((p)-[:OWNS]->(:PE))
        RETURN count(p)
    "#,
    )?;
    let duration4 = start.elapsed();
    info!("查询 4 - 查询叶子节点数量: {:?}", duration4);

    info!("");
    info!("查询性能总结:");
    info!(
        "  - 平均查询时间: {:?}",
        (duration1 + duration2 + duration3 + duration4) / 4
    );

    Ok(())
}

#[cfg(not(feature = "kuzu"))]
async fn test_query_performance() -> Result<()> {
    error!("Kuzu feature 未启用，无法测试查询性能");
    Ok(())
}
