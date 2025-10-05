//! 1112 数据库快速测试 (轻量级版本)
//!
//! 运行方式: cargo run --release --example db1112_quick_test --features kuzu

use aios_core::rs_surreal::{get_pe, query_type_refnos_by_dbnum};
use aios_core::init_surreal;
use aios_core::rs_kuzu::*;
use aios_core::rs_kuzu::operations::*;
use aios_core::types::*;
use kuzu::SystemConfig;
use std::time::Instant;

fn main() -> anyhow::Result<()> {
    // 使用同步的 tokio runtime
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    runtime.block_on(async {
        run_test().await
    })
}

async fn run_test() -> anyhow::Result<()> {
    println!("╔════════════════════════════════════════════════════════╗");
    println!("║     1112 数据库快速测试 (Release 模式)               ║");
    println!("╚════════════════════════════════════════════════════════╝\n");

    // ========== 1. 初始化数据库 ==========
    println!("📋 步骤 1/5: 初始化数据库连接");

    // 初始化 SurrealDB
    let init_start = Instant::now();
    init_surreal().await?;
    println!("   ✅ SurrealDB 连接成功");

    // 初始化 Kuzu
    let kuzu_path = "./test_output/kuzu_1112_quick.db";
    let _ = std::fs::remove_dir_all(kuzu_path);
    std::fs::create_dir_all("./test_output")?;

    init_kuzu(kuzu_path, SystemConfig::default()).await?;
    init_kuzu_schema().await?;
    println!("   ✅ Kuzu 初始化完成 (耗时: {:?})\n", init_start.elapsed());

    // ========== 2. 从 SurrealDB 读取 1112 数据 ==========
    println!("📋 步骤 2/5: 从 SurrealDB 读取 1112 数据");

    let nouns = vec!["PIPE", "BRAN", "ELBO", "FLAN", "VALV", "TEE"];
    let test_limit = 200; // 限制测试数量

    let parse_start = Instant::now();
    let mut all_pes = Vec::new();

    for noun in &nouns {
        if all_pes.len() >= test_limit {
            break;
        }

        match query_type_refnos_by_dbnum(&[noun], 1112, None, false).await {
            Ok(refnos) => {
                println!("   {} - 找到 {} 个元素", noun, refnos.len());

                for refno in refnos.iter().take(test_limit - all_pes.len()) {
                    if let Ok(Some(pe)) = get_pe(*refno).await {
                        if !pe.deleted {
                            all_pes.push(pe);
                        }
                    }
                }
            }
            Err(e) => {
                println!("   ⚠️  {} 查询失败: {}", noun, e);
            }
        }
    }

    let parse_time = parse_start.elapsed();
    println!("   ✅ 读取完成: {} 个元素 (耗时: {:?})\n", all_pes.len(), parse_time);

    // ========== 3. 保存到 Kuzu ==========
    println!("📋 步骤 3/5: 保存到 Kuzu");

    let save_start = Instant::now();
    let conn = create_kuzu_connection()?;
    let batch_size = 100;
    let mut inserted_count = 0;

    for chunk in all_pes.chunks(batch_size) {
        for pe in chunk {
            let insert_sql = format!(
                r#"CREATE (p:PE {{refno: {}, name: '{}', noun: '{}', dbnum: {}, sesno: {}}})"#,
                pe.refno.refno().0,
                pe.name.replace('\'', "\\'"),
                pe.noun,
                pe.dbnum,
                pe.sesno
            );

            if let Err(e) = conn.query(&insert_sql) {
                println!("   ⚠️  插入失败: {}", e);
            } else {
                inserted_count += 1;
            }
        }
    }

    let save_time = save_start.elapsed();
    let save_speed = inserted_count as f64 / save_time.as_secs_f64();

    println!("   ✅ 保存完成: {} 个元素", inserted_count);
    println!("   ⏱️  保存耗时: {:?}", save_time);
    println!("   🚀 保存速度: {:.0} 个/秒\n", save_speed);

    // ========== 4. 数据统计 ==========
    println!("📋 步骤 4/5: 数据统计");

    let mut noun_counts = std::collections::HashMap::new();

    for pe in &all_pes {
        *noun_counts.entry(pe.noun.clone()).or_insert(0) += 1;
    }

    // 打印 Noun 分布
    println!("\n   📊 Noun 类型分布:");
    let mut sorted_nouns: Vec<_> = noun_counts.iter().collect();
    sorted_nouns.sort_by(|a, b| b.1.cmp(a.1));

    for (noun, count) in sorted_nouns.iter() {
        println!("      {:15} : {:4} 个", noun, count);
    }

    // ========== 5. 验证数据 ==========
    println!("\n📋 步骤 5/5: 验证 Kuzu 数据库");

    let conn = create_kuzu_connection()?;
    let pe_count = conn.query("MATCH (p:PE) RETURN COUNT(*)");

    if pe_count.is_ok() {
        println!("   ✅ Kuzu 数据库可查询");
    } else {
        println!("   ⚠️  Kuzu 数据库查询失败");
    }

    // ========== 总结 ==========
    let total_time = parse_time + save_time;

    println!("\n╔════════════════════════════════════════════════════════╗");
    println!("║                     测试总结                           ║");
    println!("╠════════════════════════════════════════════════════════╣");
    println!("║  元素总数:    {:>8} 个", all_pes.len());
    println!("║  读取耗时:    {:>8.2} 秒", parse_time.as_secs_f64());
    println!("║  保存耗时:    {:>8.2} 秒", save_time.as_secs_f64());
    println!("║  总耗时:      {:>8.2} 秒", total_time.as_secs_f64());
    println!("║  保存速度:    {:>8.0} 个/秒", save_speed);
    println!("║  Noun 种类:   {:>8} 种", noun_counts.len());
    println!("║  数据库路径:  {}", kuzu_path);
    println!("╚════════════════════════════════════════════════════════╝");

    println!("\n✅ 测试完成!");

    Ok(())
}
