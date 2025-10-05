//! 解析 PDMS 1112 数据库到 Kuzu
//!
//! 运行: cd external/rs-core && cargo run --release --example parse_1112_to_kuzu --features kuzu

use aios_core::rs_kuzu::*;
use aios_core::rs_kuzu::operations::*;
use aios_core::types::*;
use aios_core::pe::SPdmsElement;
use kuzu::SystemConfig;
use pdms_io::io::PdmsIO;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

/// 解析统计
#[derive(Debug, Default)]
struct Stats {
    total_read: usize,
    parsed_ok: usize,
    saved_ok: usize,
    failed: usize,
    noun_dist: HashMap<String, usize>,
    time_ms: u128,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    println!("\n╔════════════════════════════════════════════════╗");
    println!("║     PDMS 1112 → Kuzu 解析工具                   ║");
    println!("╚════════════════════════════════════════════════╝\n");

    let mut stats = Stats::default();
    let start = Instant::now();

    // 1. 数据库路径
    let db_path = "/Volumes/DPC/work/e3d_models/AvevaMarineSample/ams000/ams1112_0001";
    let db_path_buf = PathBuf::from(db_path);

    if !db_path_buf.exists() {
        eprintln!("❌ 数据库文件不存在: {:?}", db_path_buf);
        return Err(anyhow::anyhow!("Database file not found"));
    }

    println!("📂 PDMS 数据库: {:?}", db_path_buf);

    // 2. 初始化 Kuzu
    let kuzu_path = "./kuzu_1112.db";
    println!("🔧 初始化 Kuzu: {}", kuzu_path);

    // 清理旧数据库
    let _ = std::fs::remove_dir_all(kuzu_path);

    init_kuzu(kuzu_path, SystemConfig::default()).await?;
    init_kuzu_schema().await?;
    println!("✓ Kuzu 初始化完成\n");

    // 3. 打开 PDMS 数据库
    println!("📖 打开 PDMS 数据库...");
    let mut pdms_io = PdmsIO::new("", db_path_buf.clone(), true);

    if let Err(e) = pdms_io.open() {
        eprintln!("❌ 无法打开数据库: {}", e);
        return Err(e.into());
    }
    println!("✓ 数据库已打开\n");

    // 4. 获取基本信息
    let basic_info = pdms_io.get_page_basic_info()?;
    println!("📊 数据库信息:");
    println!("   - 文件大小: {} bytes", basic_info.file_size);
    println!("   - 最新会话: {}\n", basic_info.latest_ses_pageno);

    // 5. 批量读取和保存
    println!("🔄 开始解析数据...\n");

    let batch_size = 500;
    let max_elements = 2000; // 先测试 2000 个元素
    let mut batch = Vec::new();

    // 读取元素
    while stats.total_read < max_elements {
        // 尝试读取下一页数据
        match pdms_io.get_next_element() {
            Ok(Some(elem_data)) => {
                stats.total_read += 1;

                // 解析元素
                if let Ok((pe, attmap)) = parse_element(elem_data) {
                    // 统计 noun
                    *stats.noun_dist.entry(pe.noun.clone()).or_insert(0) += 1;
                    stats.parsed_ok += 1;

                    batch.push((pe, attmap));

                    // 批量保存
                    if batch.len() >= batch_size {
                        save_batch(&batch, &mut stats).await?;
                        batch.clear();

                        // 进度显示
                        if stats.saved_ok % 1000 == 0 {
                            println!("  已处理: {} / {} 元素", stats.saved_ok, stats.total_read);
                        }
                    }
                }
            }
            Ok(None) => {
                println!("  到达文件末尾");
                break;
            }
            Err(e) => {
                log::debug!("读取错误: {}", e);
                stats.failed += 1;

                // 错误过多则停止
                if stats.failed > 100 {
                    println!("⚠️  错误过多，停止解析");
                    break;
                }
            }
        }
    }

    // 保存剩余批次
    if !batch.is_empty() {
        save_batch(&batch, &mut stats).await?;
    }

    stats.time_ms = start.elapsed().as_millis();

    // 6. 打印统计
    print_stats(&stats);

    // 7. 验证数据
    println!("\n🔍 验证保存的数据...");
    verify_data().await?;

    println!("\n✅ 解析完成! 数据已保存到: {}\n", kuzu_path);

    Ok(())
}

/// 解析单个元素
fn parse_element(elem_data: Vec<u8>) -> anyhow::Result<(SPdmsElement, NamedAttrMap)> {
    // 这里需要实际的解析逻辑
    // 简化版本，创建模拟数据
    let mut attmap = NamedAttrMap::default();

    // 从 elem_data 解析属性
    // TODO: 实现实际的解析逻辑

    // 创建 PE
    let pe = attmap.pe(1112);

    Ok((pe, attmap))
}

/// 批量保存到 Kuzu
async fn save_batch(
    batch: &[(SPdmsElement, NamedAttrMap)],
    stats: &mut Stats,
) -> anyhow::Result<()> {
    let conn = create_kuzu_connection()?;

    // 开始事务
    conn.query("BEGIN TRANSACTION")?;

    for (pe, attmap) in batch {
        // 保存 PE 节点
        let pe_sql = format!(
            "CREATE (p:PE {{refno: {}, name: '{}', noun: '{}', dbnum: {}, sesno: {}}})",
            pe.refno.refno().0,
            pe.name.replace('\'', "''"),
            pe.noun,
            pe.dbnum,
            pe.sesno
        );

        if conn.query(&pe_sql).is_ok() {
            stats.saved_ok += 1;

            // 创建 OWNS 关系
            if !pe.owner.refno().is_unset() {
                let owns_sql = format!(
                    "MATCH (parent:PE {{refno: {}}}), (child:PE {{refno: {}}})
                     CREATE (parent)-[:OWNS]->(child)",
                    pe.owner.refno().0,
                    pe.refno.refno().0
                );
                let _ = conn.query(&owns_sql);
            }
        }
    }

    conn.query("COMMIT")?;
    Ok(())
}

/// 打印统计信息
fn print_stats(stats: &Stats) {
    println!("\n╔════════════════════════════════════════════════╗");
    println!("║              解析统计报告                        ║");
    println!("╠════════════════════════════════════════════════╣");
    println!("║ 📊 处理统计:");
    println!("║   总读取: {} 个", stats.total_read);
    println!("║   解析成功: {} 个", stats.parsed_ok);
    println!("║   保存成功: {} 个", stats.saved_ok);
    println!("║   失败: {} 个", stats.failed);
    println!("║   耗时: {} ms", stats.time_ms);

    if stats.time_ms > 0 {
        let speed = (stats.saved_ok as f64 / (stats.time_ms as f64 / 1000.0)) as usize;
        println!("║   速度: {} 个/秒", speed);
    }

    println!("║");
    println!("║ 📋 Noun 分布 (Top 10):");
    let mut sorted: Vec<_> = stats.noun_dist.iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(a.1));

    for (i, (noun, count)) in sorted.iter().take(10).enumerate() {
        println!("║   {}. {:10} : {} 个", i + 1, noun, count);
    }

    println!("╚════════════════════════════════════════════════╝");
}

/// 验证保存的数据
async fn verify_data() -> anyhow::Result<()> {
    let conn = create_kuzu_connection()?;

    // 统计 PE 节点
    let mut result = conn.query("MATCH (p:PE) RETURN COUNT(*)")?;
    if let Some(row) = result.next() {
        if let Some(kuzu::Value::Int64(count)) = row.get(0) {
            println!("  ✓ PE 节点总数: {}", count);
        }
    }

    // 统计 OWNS 关系
    let mut result = conn.query("MATCH ()-[r:OWNS]->() RETURN COUNT(*)")?;
    if let Some(row) = result.next() {
        if let Some(kuzu::Value::Int64(count)) = row.get(0) {
            println!("  ✓ OWNS 关系数: {}", count);
        }
    }

    // 统计 noun 分布
    let mut result = conn.query(
        "MATCH (p:PE) RETURN p.noun, COUNT(*) ORDER BY COUNT(*) DESC LIMIT 5"
    )?;

    println!("  ✓ Top 5 Noun 类型:");
    while let Some(row) = result.next() {
        if let (Some(kuzu::Value::String(noun)), Some(kuzu::Value::Int64(count))) =
            (row.get(0), row.get(1)) {
            println!("    - {}: {}", noun, count);
        }
    }

    Ok(())
}