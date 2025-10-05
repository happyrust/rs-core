//! 1112 数据库 SurrealDB vs Kuzu 性能对比测试
//!
//! 测试流程:
//! 1. 从现有 SurrealDB (8009) 读取 1112 数据
//! 2. 分别保存到新的 SurrealDB 测试实例 (8011) 和 Kuzu
//! 3. 对比两者的保存性能
//!
//! 运行方式:
//! 1. 先启动测试 SurrealDB: surreal start --bind 0.0.0.0:8011 memory
//! 2. cargo run --release --example db1112_surreal_kuzu_benchmark --features kuzu

use aios_core::init_surreal;
use aios_core::rs_surreal::{get_pe, query_type_refnos_by_dbnum};
use aios_core::rs_kuzu::*;
use aios_core::types::*;
use kuzu::SystemConfig;
use std::collections::HashMap;
use std::time::Instant;
use surrealdb::engine::remote::ws::Ws;
use surrealdb::Surreal;

/// 测试统计信息
#[derive(Debug, Default, Clone)]
struct BenchmarkStats {
    total_elements: usize,
    read_time_ms: u128,
    surreal_save_time_ms: u128,
    kuzu_save_time_ms: u128,
    surreal_saved: usize,
    kuzu_saved: usize,
    noun_distribution: HashMap<String, usize>,
}

impl BenchmarkStats {
    fn print_summary(&self) {
        println!("\n╔════════════════════════════════════════════════════════╗");
        println!("║      SurrealDB vs Kuzu 性能对比测试报告               ║");
        println!("╠════════════════════════════════════════════════════════╣");
        println!("║ 📊 数据读取:");
        println!("║   - 元素数量:        {:>8} 个", self.total_elements);
        println!("║   - 读取耗时:        {:>8} ms", self.read_time_ms);
        println!("║   - 读取速度:        {:>8} 个/秒",
            if self.read_time_ms > 0 {
                (self.total_elements as f64 / (self.read_time_ms as f64 / 1000.0)) as usize
            } else { 0 });
        println!("║");
        println!("║ 💾 SurrealDB 保存:");
        println!("║   - 保存数量:        {:>8} 个", self.surreal_saved);
        println!("║   - 保存耗时:        {:>8} ms", self.surreal_save_time_ms);
        println!("║   - 保存速度:        {:>8} 个/秒",
            if self.surreal_save_time_ms > 0 {
                (self.surreal_saved as f64 / (self.surreal_save_time_ms as f64 / 1000.0)) as usize
            } else { 0 });
        println!("║");
        println!("║ 📈 Kuzu 保存:");
        println!("║   - 保存数量:        {:>8} 个", self.kuzu_saved);
        println!("║   - 保存耗时:        {:>8} ms", self.kuzu_save_time_ms);
        println!("║   - 保存速度:        {:>8} 个/秒",
            if self.kuzu_save_time_ms > 0 {
                (self.kuzu_saved as f64 / (self.kuzu_save_time_ms as f64 / 1000.0)) as usize
            } else { 0 });
        println!("║");

        // 性能对比
        if self.surreal_save_time_ms > 0 && self.kuzu_save_time_ms > 0 {
            let ratio = self.surreal_save_time_ms as f64 / self.kuzu_save_time_ms as f64;
            println!("║ ⚡ 性能对比:");
            if ratio > 1.0 {
                println!("║   Kuzu 比 SurrealDB 快     {:.2}x", ratio);
            } else {
                println!("║   SurrealDB 比 Kuzu 快     {:.2}x", 1.0 / ratio);
            }
            println!("║   SurrealDB 耗时占比:      {:>6.1}%",
                self.surreal_save_time_ms as f64 / (self.surreal_save_time_ms + self.kuzu_save_time_ms) as f64 * 100.0);
            println!("║   Kuzu 耗时占比:           {:>6.1}%",
                self.kuzu_save_time_ms as f64 / (self.surreal_save_time_ms + self.kuzu_save_time_ms) as f64 * 100.0);
        }

        println!("║");
        println!("║ 📋 Noun 类型分布 (Top 10):");
        let mut sorted_nouns: Vec<_> = self.noun_distribution.iter().collect();
        sorted_nouns.sort_by(|a, b| b.1.cmp(a.1));
        for (noun, count) in sorted_nouns.iter().take(10) {
            println!("║   {:15} : {:>6} 个", noun, count);
        }

        println!("║");
        println!("║ ⏱️  总耗时:          {:>8} ms",
            self.read_time_ms + self.surreal_save_time_ms + self.kuzu_save_time_ms);
        println!("╚════════════════════════════════════════════════════════╝\n");
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut stats = BenchmarkStats::default();

    println!("╔════════════════════════════════════════════════════════╗");
    println!("║    SurrealDB vs Kuzu 性能对比测试 (1112数据库)       ║");
    println!("╚════════════════════════════════════════════════════════╝\n");

    // ========== 步骤 1: 从现有 SurrealDB 读取数据 ==========
    println!("📋 步骤 1/4: 从 SurrealDB (8009) 读取 1112 数据");

    init_surreal().await?;
    println!("   ✓ 连接到主 SurrealDB (8009)");

    let test_nouns = vec!["PIPE", "BRAN", "ELBO", "FLAN", "VALV", "TEE", "EQUI", "STRU"];
    let test_limit = 500;

    let read_start = Instant::now();
    let mut all_pes = Vec::new();

    for noun in &test_nouns {
        if all_pes.len() >= test_limit {
            break;
        }

        match query_type_refnos_by_dbnum(&[noun], 1112, None, false).await {
            Ok(refnos) => {
                println!("   {} - 找到 {} 个", noun, refnos.len());

                for refno in refnos.iter().take(test_limit - all_pes.len()) {
                    if let Ok(Some(pe)) = get_pe(*refno).await {
                        if !pe.deleted {
                            *stats.noun_distribution.entry(pe.noun.clone()).or_insert(0) += 1;
                            all_pes.push(pe);
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("   ⚠️  {} 查询失败: {}", noun, e);
            }
        }
    }

    stats.total_elements = all_pes.len();
    stats.read_time_ms = read_start.elapsed().as_millis();
    println!("   ✓ 读取完成: {} 个元素 (耗时: {} ms)\n", stats.total_elements, stats.read_time_ms);

    // ========== 步骤 2: 保存到测试 SurrealDB (8011) ==========
    println!("📋 步骤 2/4: 保存到测试 SurrealDB (8011)");

    let test_db = Surreal::new::<Ws>("127.0.0.1:8011").await?;
    test_db.signin(surrealdb::opt::auth::Root {
        username: "root",
        password: "root",
    }).await?;
    test_db.use_ns("test_1112").use_db("benchmark").await?;
    println!("   ✓ 连接到测试 SurrealDB");

    let surreal_start = Instant::now();

    for pe in &all_pes {
        let refno_val = pe.refno.refno().0 as i64;
        let data = serde_json::json!({
            "refno": refno_val,
            "name": &pe.name,
            "noun": &pe.noun,
            "dbnum": pe.dbnum as i64,
            "sesno": pe.sesno as i64,
            "deleted": pe.deleted,
            "lock": pe.lock,
        });

        match test_db.create::<Option<serde_json::Value>>("pe")
            .content(data).await {
            Ok(_) => stats.surreal_saved += 1,
            Err(e) => {
                if stats.surreal_saved == 0 {
                    eprintln!("   ⚠️  首次保存失败: {}", e);
                }
            }
        }
    }

    stats.surreal_save_time_ms = surreal_start.elapsed().as_millis();
    println!("   ✓ 保存完成: {} 个元素 (耗时: {} ms)\n", stats.surreal_saved, stats.surreal_save_time_ms);

    // ========== 步骤 3: 保存到 Kuzu ==========
    println!("📋 步骤 3/4: 保存到 Kuzu");

    let kuzu_path = "./test_output/kuzu_benchmark.db";
    let _ = std::fs::remove_dir_all(kuzu_path);
    std::fs::create_dir_all("./test_output")?;

    init_kuzu(kuzu_path, SystemConfig::default()).await?;
    init_kuzu_schema().await?;
    println!("   ✓ Kuzu 初始化完成");

    let kuzu_start = Instant::now();
    let conn = create_kuzu_connection()?;

    for pe in &all_pes {
        let insert_sql = format!(
            r#"CREATE (p:PE {{refno: {}, name: '{}', noun: '{}', dbnum: {}, sesno: {}}})"#,
            pe.refno.refno().0,
            pe.name.replace('\'', "\\'"),
            pe.noun,
            pe.dbnum,
            pe.sesno
        );

        match conn.query(&insert_sql) {
            Ok(_) => stats.kuzu_saved += 1,
            Err(e) => {
                if stats.kuzu_saved == 0 {
                    eprintln!("   ⚠️  首次保存失败: {}", e);
                }
            }
        }
    }

    stats.kuzu_save_time_ms = kuzu_start.elapsed().as_millis();
    println!("   ✓ 保存完成: {} 个元素 (耗时: {} ms)\n", stats.kuzu_saved, stats.kuzu_save_time_ms);

    // ========== 步骤 4: 验证数据 ==========
    println!("📋 步骤 4/4: 数据验证");

    // 验证 SurrealDB
    let surreal_query: Vec<serde_json::Value> = test_db.query("SELECT COUNT() as count FROM pe GROUP ALL").await?.take(0)?;
    if let Some(result) = surreal_query.first() {
        if let Some(count) = result.get("count") {
            println!("   ✓ SurrealDB 记录数: {}", count);
        }
    }

    // 验证 Kuzu
    match conn.query("MATCH (p:PE) RETURN COUNT(*)") {
        Ok(_) => println!("   ✓ Kuzu 数据库可查询\n"),
        Err(e) => eprintln!("   ✗ Kuzu 查询失败: {}\n", e),
    }

    // ========== 打印测试报告 ==========
    stats.print_summary();

    println!("✅ 测试完成!");
    println!("\n数据库位置:");
    println!("  - 主 SurrealDB:   ws://127.0.0.1:8009/rpc");
    println!("  - 测试 SurrealDB: ws://127.0.0.1:8011/rpc");
    println!("  - Kuzu:           {}", kuzu_path);

    Ok(())
}
