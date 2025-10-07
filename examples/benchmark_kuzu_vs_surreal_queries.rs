//! Kuzu vs SurrealDB 查询性能对比测试
//!
//! 测试场景:
//! 1. 单层子节点查询 (children)
//! 2. 深层递归查询 (deep children - 12层)
//! 3. 类型过滤查询 (type filter by dbnum)
//! 4. 祖先查询 (ancestors)
//! 5. 多条件组合查询 (multi-filter)
//!
//! 运行方式:
//! ```bash
//! cargo run --release --example benchmark_kuzu_vs_surreal_queries --features kuzu
//! ```

use aios_core::init_surreal;
use aios_core::rs_surreal::queries::hierarchy::{get_children_refnos, query_ancestor_refnos};
use aios_core::rs_surreal::graph::{query_deep_children_refnos, query_filter_deep_children};
use aios_core::rs_surreal::mdb::query_type_refnos_by_dbnum;
use aios_core::rs_kuzu::*;
use aios_core::rs_kuzu::queries::hierarchy::*;
use aios_core::rs_kuzu::queries::type_filter::*;
use aios_core::rs_kuzu::queries::multi_filter::*;
use aios_core::types::*;
use kuzu::SystemConfig;
use std::time::Instant;
use colored::Colorize;

/// 基准测试统计
#[derive(Debug, Default)]
struct BenchmarkStats {
    test_name: String,
    surreal_time_ms: u128,
    kuzu_time_ms: u128,
    surreal_count: usize,
    kuzu_count: usize,
    iterations: usize,
}

impl BenchmarkStats {
    fn new(name: &str, iterations: usize) -> Self {
        Self {
            test_name: name.to_string(),
            iterations,
            ..Default::default()
        }
    }

    fn speedup(&self) -> f64 {
        if self.kuzu_time_ms == 0 {
            return 0.0;
        }
        self.surreal_time_ms as f64 / self.kuzu_time_ms as f64
    }

    fn print_summary(&self) {
        let speedup = self.speedup();
        let speedup_color = if speedup > 5.0 {
            "green"
        } else if speedup > 2.0 {
            "yellow"
        } else {
            "red"
        };

        println!("\n  📊 {}", self.test_name.bold());
        println!("  ├─ 迭代次数:      {} 次", self.iterations);
        println!("  ├─ SurrealDB:     {} ms (平均 {:.2} ms/次) - {} 条结果",
            self.surreal_time_ms,
            self.surreal_time_ms as f64 / self.iterations as f64,
            self.surreal_count
        );
        println!("  ├─ Kuzu:          {} ms (平均 {:.2} ms/次) - {} 条结果",
            self.kuzu_time_ms,
            self.kuzu_time_ms as f64 / self.iterations as f64,
            self.kuzu_count
        );

        if speedup_color == "green" {
            println!("  └─ 性能提升:      {:.2}x ⚡", speedup.to_string().green().bold());
        } else if speedup_color == "yellow" {
            println!("  └─ 性能提升:      {:.2}x", speedup.to_string().yellow());
        } else {
            println!("  └─ 性能提升:      {:.2}x", speedup.to_string().red());
        }

        // 数据一致性检查
        if self.surreal_count != self.kuzu_count {
            println!("  ⚠️  警告: 结果数量不一致! SurrealDB={}, Kuzu={}",
                self.surreal_count, self.kuzu_count);
        }
    }
}

/// 整体统计报告
struct OverallStats {
    tests: Vec<BenchmarkStats>,
}

impl OverallStats {
    fn new() -> Self {
        Self { tests: Vec::new() }
    }

    fn add(&mut self, stats: BenchmarkStats) {
        self.tests.push(stats);
    }

    fn print_report(&self) {
        println!("\n╔════════════════════════════════════════════════════════════╗");
        println!("║          Kuzu vs SurrealDB 查询性能对比报告               ║");
        println!("╠════════════════════════════════════════════════════════════╣");

        for stats in &self.tests {
            stats.print_summary();
        }

        println!("\n╔════════════════════════════════════════════════════════════╗");
        println!("║                    总体统计                                ║");
        println!("╠════════════════════════════════════════════════════════════╣");

        let total_surreal: u128 = self.tests.iter().map(|s| s.surreal_time_ms).sum();
        let total_kuzu: u128 = self.tests.iter().map(|s| s.kuzu_time_ms).sum();
        let avg_speedup: f64 = self.tests.iter()
            .map(|s| s.speedup())
            .sum::<f64>() / self.tests.len() as f64;

        println!("  测试场景数:       {} 个", self.tests.len());
        println!("  SurrealDB 总耗时: {} ms", total_surreal);
        println!("  Kuzu 总耗时:      {} ms", total_kuzu);
        println!("  平均性能提升:     {:.2}x", avg_speedup.to_string().green().bold());

        if total_kuzu > 0 {
            let overall_speedup = total_surreal as f64 / total_kuzu as f64;
            println!("  总体性能提升:     {:.2}x", overall_speedup.to_string().green().bold());
        }

        println!("╚════════════════════════════════════════════════════════════╝\n");
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║       Kuzu vs SurrealDB 查询性能基准测试                  ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    // 初始化 SurrealDB
    println!("📋 步骤 1/3: 初始化 SurrealDB");
    init_surreal().await?;
    println!("   ✓ SurrealDB 连接成功\n");

    // 初始化 Kuzu
    println!("📋 步骤 2/3: 初始化 Kuzu");
    let kuzu_path = "./test_output/kuzu_1112_comparison.db";
    if !std::path::Path::new(kuzu_path).exists() {
        eprintln!("❌ Kuzu 数据库不存在: {}", kuzu_path);
        eprintln!("   请先运行 parse_1112_to_kuzu 示例来创建数据库");
        return Err(anyhow::anyhow!("Kuzu database not found"));
    }

    init_kuzu(kuzu_path, SystemConfig::default()).await?;
    println!("   ✓ Kuzu 数据库加载成功\n");

    // 准备测试数据
    println!("📋 步骤 3/3: 准备测试数据");
    let test_refnos = prepare_test_data().await?;
    println!("   ✓ 准备了 {} 个测试 refno\n", test_refnos.len());

    // 开始基准测试
    println!("🚀 开始性能基准测试...\n");
    let mut overall = OverallStats::new();

    // 测试 1: 单层子节点查询
    overall.add(benchmark_children_query(&test_refnos[0..5.min(test_refnos.len())]).await?);

    // 测试 2: 深层递归查询 (12层)
    overall.add(benchmark_deep_children_query(&test_refnos[0..3.min(test_refnos.len())]).await?);

    // 测试 3: 类型过滤查询
    overall.add(benchmark_type_filter_query().await?);

    // 测试 4: 祖先查询
    overall.add(benchmark_ancestor_query(&test_refnos[0..5.min(test_refnos.len())]).await?);

    // 测试 5: 深层类型过滤查询
    overall.add(benchmark_deep_filter_query(&test_refnos[0..3.min(test_refnos.len())]).await?);

    // 打印总体报告
    overall.print_report();

    println!("✅ 基准测试完成!\n");

    Ok(())
}

/// 准备测试数据 - 从数据库获取一些有子节点的 refno
async fn prepare_test_data() -> anyhow::Result<Vec<RefnoEnum>> {
    // 查询一些有子节点的元素
    let nouns = ["ZONE", "STRU", "EQUI", "PIPE"];
    let mut test_refnos = Vec::new();

    for noun in &nouns {
        match query_type_refnos_by_dbnum(&[noun], 1112, Some(true), false).await {
            Ok(refnos) => {
                test_refnos.extend(refnos.into_iter().take(3));
                if test_refnos.len() >= 10 {
                    break;
                }
            }
            Err(e) => {
                log::warn!("查询 {} 失败: {}", noun, e);
            }
        }
    }

    if test_refnos.is_empty() {
        return Err(anyhow::anyhow!("无法找到测试数据"));
    }

    Ok(test_refnos)
}

/// 测试 1: 单层子节点查询
async fn benchmark_children_query(test_refnos: &[RefnoEnum]) -> anyhow::Result<BenchmarkStats> {
    let iterations = test_refnos.len();
    let mut stats = BenchmarkStats::new("单层子节点查询 (children)", iterations);

    // SurrealDB
    let start = Instant::now();
    for refno in test_refnos {
        if let Ok(children) = get_children_refnos(*refno).await {
            stats.surreal_count += children.len();
        }
    }
    stats.surreal_time_ms = start.elapsed().as_millis();

    // Kuzu
    let start = Instant::now();
    for refno in test_refnos {
        if let Ok(children) = kuzu_get_children_refnos(*refno).await {
            stats.kuzu_count += children.len();
        }
    }
    stats.kuzu_time_ms = start.elapsed().as_millis();

    Ok(stats)
}

/// 测试 2: 深层递归查询 (12层)
async fn benchmark_deep_children_query(test_refnos: &[RefnoEnum]) -> anyhow::Result<BenchmarkStats> {
    let iterations = test_refnos.len();
    let mut stats = BenchmarkStats::new("深层递归查询 (12层)", iterations);

    // SurrealDB
    let start = Instant::now();
    for refno in test_refnos {
        if let Ok(children) = query_deep_children_refnos(*refno).await {
            stats.surreal_count += children.len();
        }
    }
    stats.surreal_time_ms = start.elapsed().as_millis();

    // Kuzu
    let start = Instant::now();
    for refno in test_refnos {
        if let Ok(children) = kuzu_query_deep_children_refnos(*refno).await {
            stats.kuzu_count += children.len();
        }
    }
    stats.kuzu_time_ms = start.elapsed().as_millis();

    Ok(stats)
}

/// 测试 3: 类型过滤查询
async fn benchmark_type_filter_query() -> anyhow::Result<BenchmarkStats> {
    let iterations = 5;
    let mut stats = BenchmarkStats::new("类型过滤查询 (dbnum + noun)", iterations);

    let test_cases = vec![
        (vec!["PIPE"], 1112),
        (vec!["EQUI"], 1112),
        (vec!["VALVE", "PUMP"], 1112),
        (vec!["ELBO", "TEE"], 1112),
        (vec!["ZONE"], 1112),
    ];

    // SurrealDB
    let start = Instant::now();
    for (nouns, dbnum) in &test_cases {
        let nouns_ref: Vec<&str> = nouns.iter().map(|s| s.as_str()).collect();
        if let Ok(results) = query_type_refnos_by_dbnum(&nouns_ref, *dbnum, None, false).await {
            stats.surreal_count += results.len();
        }
    }
    stats.surreal_time_ms = start.elapsed().as_millis();

    // Kuzu
    let start = Instant::now();
    for (nouns, dbnum) in &test_cases {
        let nouns_ref: Vec<&str> = nouns.iter().map(|s| s.as_str()).collect();
        if let Ok(results) = kuzu_query_type_refnos_by_dbnum(&nouns_ref, *dbnum, None).await {
            stats.kuzu_count += results.len();
        }
    }
    stats.kuzu_time_ms = start.elapsed().as_millis();

    Ok(stats)
}

/// 测试 4: 祖先查询
async fn benchmark_ancestor_query(test_refnos: &[RefnoEnum]) -> anyhow::Result<BenchmarkStats> {
    let iterations = test_refnos.len();
    let mut stats = BenchmarkStats::new("祖先查询 (ancestors)", iterations);

    // SurrealDB
    let start = Instant::now();
    for refno in test_refnos {
        if let Ok(ancestors) = query_ancestor_refnos(*refno).await {
            stats.surreal_count += ancestors.len();
        }
    }
    stats.surreal_time_ms = start.elapsed().as_millis();

    // Kuzu
    let start = Instant::now();
    for refno in test_refnos {
        if let Ok(ancestors) = kuzu_query_ancestor_refnos(*refno).await {
            stats.kuzu_count += ancestors.len();
        }
    }
    stats.kuzu_time_ms = start.elapsed().as_millis();

    Ok(stats)
}

/// 测试 5: 深层类型过滤查询
async fn benchmark_deep_filter_query(test_refnos: &[RefnoEnum]) -> anyhow::Result<BenchmarkStats> {
    let iterations = test_refnos.len();
    let mut stats = BenchmarkStats::new("深层类型过滤查询", iterations);

    let filter_nouns = ["PIPE", "EQUI"];

    // SurrealDB
    let start = Instant::now();
    for refno in test_refnos {
        if let Ok(results) = query_filter_deep_children(*refno, &filter_nouns).await {
            stats.surreal_count += results.len();
        }
    }
    stats.surreal_time_ms = start.elapsed().as_millis();

    // Kuzu
    let start = Instant::now();
    for refno in test_refnos {
        if let Ok(results) = kuzu_query_filter_deep_children(*refno, &filter_nouns).await {
            stats.kuzu_count += results.len();
        }
    }
    stats.kuzu_time_ms = start.elapsed().as_millis();

    Ok(stats)
}
