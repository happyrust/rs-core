//! 性能对比测试: SurrealDB vs Kuzu
//!
//! 运行方式: cargo run --bin test_query_performance_comparison --features kuzu

use aios_core::init_surreal;
use aios_core::rs_surreal::{get_children_refnos, query_type_refnos_by_dbnum};
use anyhow::Result;
use log::info;
use simplelog::*;
use std::collections::HashSet;
use std::time::Instant;

#[cfg(feature = "kuzu")]
use aios_core::rs_kuzu::{create_kuzu_connection, init_kuzu};
#[cfg(feature = "kuzu")]
use kuzu::SystemConfig;

/// 测试查询配置
struct QueryTest {
    name: String,
    iterations: usize,
}

/// 测试结果
#[derive(Debug)]
struct TestResult {
    test_name: String,
    surreal_time: std::time::Duration,
    kuzu_time: std::time::Duration,
    speedup: f64,
}

/// SurrealDB 查询测试
async fn test_surreal_queries(dbnum: u32) -> Result<Vec<(String, std::time::Duration, usize)>> {
    let mut results = Vec::new();

    // 测试1: 查询所有 PIPE 元素
    info!("\n[SurrealDB] 测试1: 查询所有 PIPE 元素");
    let start = Instant::now();
    let pipes = query_type_refnos_by_dbnum(&["PIPE"], dbnum, None, false).await?;
    let duration = start.elapsed();
    info!("  找到 {} 个 PIPE 元素, 耗时: {:?}", pipes.len(), duration);
    results.push(("查询 PIPE 元素".to_string(), duration, pipes.len()));

    // 测试2: 查询所有 EQUI 元素
    info!("[SurrealDB] 测试2: 查询所有 EQUI 元素");
    let start = Instant::now();
    let equis = query_type_refnos_by_dbnum(&["EQUI"], dbnum, None, false).await?;
    let duration = start.elapsed();
    info!("  找到 {} 个 EQUI 元素, 耗时: {:?}", equis.len(), duration);
    results.push(("查询 EQUI 元素".to_string(), duration, equis.len()));

    // 测试3: 查询所有 PLOO 元素
    info!("[SurrealDB] 测试3: 查询所有 PLOO 元素");
    let start = Instant::now();
    let ploos = query_type_refnos_by_dbnum(&["PLOO"], dbnum, None, false).await?;
    let duration = start.elapsed();
    info!("  找到 {} 个 PLOO 元素, 耗时: {:?}", ploos.len(), duration);
    results.push(("查询 PLOO 元素".to_string(), duration, ploos.len()));

    // 测试4: 查询特定元素的子元素（一层）
    if !pipes.is_empty() {
        info!("[SurrealDB] 测试4: 查询第一个 PIPE 的直接子元素");
        let start = Instant::now();
        let children = get_children_refnos(pipes[0]).await?;
        let duration = start.elapsed();
        info!("  找到 {} 个子元素, 耗时: {:?}", children.len(), duration);
        results.push(("查询直接子元素".to_string(), duration, children.len()));
    }

    // 测试5: 递归查询子元素（两层）
    if !equis.is_empty() {
        info!("[SurrealDB] 测试5: 递归查询第一个 EQUI 的两层子元素");
        let start = Instant::now();
        let mut all_children = HashSet::new();

        // 第一层
        let level1 = get_children_refnos(equis[0]).await?;
        for child in &level1 {
            all_children.insert(*child);
            // 第二层
            let level2 = get_children_refnos(*child).await?;
            for child2 in level2 {
                all_children.insert(child2);
            }
        }

        let duration = start.elapsed();
        info!("  找到 {} 个子元素（两层）, 耗时: {:?}", all_children.len(), duration);
        results.push(("递归查询两层子元素".to_string(), duration, all_children.len()));
    }

    // 测试6: 统计每种类型的元素数量
    info!("[SurrealDB] 测试6: 统计各类型元素数量");
    let types = ["PIPE", "BRAN", "EQUI", "STRU", "FITT", "VALV", "FLAN", "INST"];
    let start = Instant::now();
    let mut type_counts = Vec::new();
    for typ in types {
        let count = query_type_refnos_by_dbnum(&[typ], dbnum, None, false).await?.len();
        type_counts.push((typ, count));
    }
    let duration = start.elapsed();
    info!("  统计完成, 耗时: {:?}", duration);
    for (typ, count) in &type_counts {
        info!("    {}: {} 个", typ, count);
    }
    results.push(("统计8种类型".to_string(), duration, type_counts.iter().map(|(_, c)| c).sum()));

    Ok(results)
}

/// Kuzu 查询测试
#[cfg(feature = "kuzu")]
fn test_kuzu_queries(dbnum: u32) -> Result<Vec<(String, std::time::Duration, usize)>> {
    let conn = create_kuzu_connection()?;
    let mut results = Vec::new();

    // 测试1: 查询所有 PIPE 元素
    info!("\n[Kuzu] 测试1: 查询所有 PIPE 元素");
    let start = Instant::now();
    let query = format!("MATCH (p:PE) WHERE p.noun = 'PIPE' AND p.dbnum = {} RETURN count(p) AS cnt", dbnum);
    let mut result = conn.query(&query)?;
    let count = if let Some(row) = result.next() {
        if let kuzu::Value::Int64(c) = row.get(0).unwrap() {
*c as usize
        } else { 0 }
    } else { 0 };
    let duration = start.elapsed();
    info!("  找到 {} 个 PIPE 元素, 耗时: {:?}", count, duration);
    results.push(("查询 PIPE 元素".to_string(), duration, count));

    // 测试2: 查询所有 EQUI 元素
    info!("[Kuzu] 测试2: 查询所有 EQUI 元素");
    let start = Instant::now();
    let query = format!("MATCH (p:PE) WHERE p.noun = 'EQUI' AND p.dbnum = {} RETURN count(p) AS cnt", dbnum);
    let mut result = conn.query(&query)?;
    let count = if let Some(row) = result.next() {
        if let kuzu::Value::Int64(c) = row.get(0).unwrap() {
*c as usize
        } else { 0 }
    } else { 0 };
    let duration = start.elapsed();
    info!("  找到 {} 个 EQUI 元素, 耗时: {:?}", count, duration);
    results.push(("查询 EQUI 元素".to_string(), duration, count));

    // 测试3: 查询所有 PLOO 元素
    info!("[Kuzu] 测试3: 查询所有 PLOO 元素");
    let start = Instant::now();
    let query = format!("MATCH (p:PE) WHERE p.noun = 'PLOO' AND p.dbnum = {} RETURN count(p) AS cnt", dbnum);
    let mut result = conn.query(&query)?;
    let count = if let Some(row) = result.next() {
        if let kuzu::Value::Int64(c) = row.get(0).unwrap() {
*c as usize
        } else { 0 }
    } else { 0 };
    let duration = start.elapsed();
    info!("  找到 {} 个 PLOO 元素, 耗时: {:?}", count, duration);
    results.push(("查询 PLOO 元素".to_string(), duration, count));

    // 测试4: 查询特定元素的子元素（一层）
    info!("[Kuzu] 测试4: 查询第一个 PIPE 的直接子元素");
    let start = Instant::now();
    let query = format!(
        "MATCH (p:PE)-[:OWNS]->(c:PE) WHERE p.noun = 'PIPE' AND p.dbnum = {} RETURN count(DISTINCT c) AS cnt LIMIT 1",
        dbnum
    );
    let mut result = conn.query(&query)?;
    let count = if let Some(row) = result.next() {
        if let kuzu::Value::Int64(c) = row.get(0).unwrap() {
*c as usize
        } else { 0 }
    } else { 0 };
    let duration = start.elapsed();
    info!("  找到 {} 个子元素, 耗时: {:?}", count, duration);
    results.push(("查询直接子元素".to_string(), duration, count));

    // 测试5: 递归查询子元素（两层）
    info!("[Kuzu] 测试5: 递归查询第一个 EQUI 的两层子元素");
    let start = Instant::now();
    let query = format!(
        "MATCH (p:PE)-[:OWNS*1..2]->(c:PE) WHERE p.noun = 'EQUI' AND p.dbnum = {} RETURN count(DISTINCT c) AS cnt LIMIT 1",
        dbnum
    );
    let mut result = conn.query(&query)?;
    let count = if let Some(row) = result.next() {
        if let kuzu::Value::Int64(c) = row.get(0).unwrap() {
*c as usize
        } else { 0 }
    } else { 0 };
    let duration = start.elapsed();
    info!("  找到 {} 个子元素（两层）, 耗时: {:?}", count, duration);
    results.push(("递归查询两层子元素".to_string(), duration, count));

    // 测试6: 统计每种类型的元素数量
    info!("[Kuzu] 测试6: 统计各类型元素数量");
    let types = ["PIPE", "BRAN", "EQUI", "STRU", "FITT", "VALV", "FLAN", "INST"];
    let start = Instant::now();

    let mut total_count = 0;
    for typ in types {
        let query = format!(
            "MATCH (p:PE) WHERE p.noun = '{}' AND p.dbnum = {} RETURN count(p) AS cnt",
            typ, dbnum
        );
        let mut result = conn.query(&query)?;
        if let Some(row) = result.next() {
            if let kuzu::Value::Int64(cnt) = row.get(0).unwrap() {
                info!("    {}: {} 个", typ, cnt);
                total_count += *cnt as usize;
            }
        }
    }

    let duration = start.elapsed();
    info!("  统计完成, 耗时: {:?}", duration);
    results.push(("统计8种类型".to_string(), duration, total_count));

    Ok(results)
}

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
    info!("查询性能对比测试: SurrealDB vs Kuzu");
    info!("========================================");

    let dbnum: u32 = 1112;

    // 1. 初始化 SurrealDB
    info!("\n步骤 1: 初始化 SurrealDB");
    init_surreal().await?;
    info!("✓ SurrealDB 初始化成功");

    // 2. 运行 SurrealDB 查询测试
    info!("\n步骤 2: 运行 SurrealDB 查询测试");
    let surreal_results = test_surreal_queries(dbnum).await?;

    // 3. 初始化和运行 Kuzu 测试
    #[cfg(feature = "kuzu")]
    {
        use std::fs;

        info!("\n步骤 3: 初始化 Kuzu");
        let kuzu_db_path = "./data/kuzu_1112_sync";

        // 检查 Kuzu 数据库是否存在
        if !fs::metadata(kuzu_db_path).is_ok() {
            info!("  Kuzu 数据库不存在，需要先运行同步测试");
            info!("  请运行: cargo run --bin test_sync_1112 --features kuzu");
            return Ok(());
        }

        init_kuzu(kuzu_db_path, SystemConfig::default()).await?;
        info!("✓ Kuzu 初始化成功");

        info!("\n步骤 4: 运行 Kuzu 查询测试");
        let kuzu_results = test_kuzu_queries(dbnum)?;

        // 4. 对比结果
        info!("\n========================================");
        info!("性能对比结果");
        info!("========================================");

        let mut comparison_results = Vec::new();

        for (surreal_test, kuzu_test) in surreal_results.iter().zip(kuzu_results.iter()) {
            assert_eq!(surreal_test.0, kuzu_test.0, "测试名称不匹配");

            let speedup = surreal_test.1.as_secs_f64() / kuzu_test.1.as_secs_f64();

            comparison_results.push(TestResult {
                test_name: surreal_test.0.clone(),
                surreal_time: surreal_test.1,
                kuzu_time: kuzu_test.1,
                speedup,
            });
        }

        // 打印对比表格
        info!("\n{:<25} {:>15} {:>15} {:>10} {:>15}",
            "测试项目", "SurrealDB", "Kuzu", "加速比", "性能提升");
        info!("{}", "-".repeat(90));

        for result in &comparison_results {
            let improvement = if result.speedup > 1.0 {
                format!("Kuzu快 {:.1}x", result.speedup)
            } else {
                format!("SurrealDB快 {:.1}x", 1.0 / result.speedup)
            };

            info!("{:<25} {:>15.3?} {:>15.3?} {:>10.2}x {:>15}",
                result.test_name,
                result.surreal_time,
                result.kuzu_time,
                result.speedup,
                improvement
            );
        }

        // 计算平均加速比
        let avg_speedup = comparison_results.iter()
            .map(|r| r.speedup)
            .sum::<f64>() / comparison_results.len() as f64;

        info!("{}", "-".repeat(90));
        info!("平均加速比: {:.2}x", avg_speedup);

        if avg_speedup > 1.0 {
            info!("总结: Kuzu 在这些查询上平均比 SurrealDB 快 {:.1}x", avg_speedup);
        } else {
            info!("总结: SurrealDB 在这些查询上平均比 Kuzu 快 {:.1}x", 1.0 / avg_speedup);
        }

        // 分析结果
        info!("\n========================================");
        info!("性能分析");
        info!("========================================");

        info!("\n优势分析:");
        for result in &comparison_results {
            if result.speedup > 2.0 {
                info!("  ✓ Kuzu 在「{}」上表现优异 (快 {:.1}x)",
                    result.test_name, result.speedup);
            } else if result.speedup < 0.5 {
                info!("  ✓ SurrealDB 在「{}」上表现优异 (快 {:.1}x)",
                    result.test_name, 1.0 / result.speedup);
            }
        }

        info!("\n建议:");
        if avg_speedup > 1.5 {
            info!("  推荐使用 Kuzu 进行图遍历和关系查询");
        } else if avg_speedup < 0.67 {
            info!("  推荐继续使用 SurrealDB");
        } else {
            info!("  两个数据库性能相当，可根据具体需求选择");
        }
    }

    #[cfg(not(feature = "kuzu"))]
    {
        info!("\nKuzu feature 未启用！请使用 --features kuzu 运行完整对比测试");
    }

    info!("\n========================================");
    info!("测试完成！");
    info!("========================================");

    Ok(())
}