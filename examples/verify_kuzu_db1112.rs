//! Kuzu 数据库 1112 数据验证工具
//!
//! 功能:
//! 1. 检查 PE 节点总数
//! 2. 验证 noun 类型分布
//! 3. 检查层级关系完整性
//! 4. 分析孤立节点
//! 5. 验证引用关系
//!
//! 运行:
//! cd external/rs-core && cargo run --release --example verify_kuzu_db1112 --features kuzu

use aios_core::rs_kuzu::*;
use kuzu::{Connection, SystemConfig, Value};
use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Default)]
struct ValidationStats {
    total_pe_nodes: usize,
    noun_distribution: HashMap<String, usize>,
    nodes_with_owner: usize,
    nodes_without_owner: usize,
    total_owns_relationships: usize,
    max_hierarchy_depth: usize,
    orphan_nodes: Vec<u64>,
    root_nodes: Vec<(u64, String, String)>,
    total_attr_nodes: HashMap<String, usize>,
    dbnum_distribution: HashMap<i32, usize>,
    sesno_distribution: HashMap<i32, usize>,
}

impl ValidationStats {
    fn print_report(&self) {
        println!("\n╔══════════════════════════════════════════════════════════╗");
        println!("║        Kuzu 数据库 1112 数据验证报告                      ║");
        println!("╠══════════════════════════════════════════════════════════╣");

        println!("║ 📊 PE 节点统计:");
        println!("║   总数量: {} 个", self.total_pe_nodes);
        println!("║   有 owner: {} 个", self.nodes_with_owner);
        println!("║   无 owner (根节点): {} 个", self.nodes_without_owner);
        println!("║   孤立节点: {} 个", self.orphan_nodes.len());

        println!("║");
        println!("║ 🔗 关系统计:");
        println!("║   OWNS 关系总数: {} 个", self.total_owns_relationships);
        println!("║   最大层级深度: {} 层", self.max_hierarchy_depth);

        println!("║");
        println!("║ 📋 Noun 类型分布 (Top 15):");
        let mut sorted_nouns: Vec<_> = self.noun_distribution.iter().collect();
        sorted_nouns.sort_by(|a, b| b.1.cmp(a.1));
        for (i, (noun, count)) in sorted_nouns.iter().take(15).enumerate() {
            println!("║   {:2}. {:20} : {:>8} 个 ({:.1}%)",
                i + 1, noun, count,
                (**count as f64 / self.total_pe_nodes as f64) * 100.0);
        }

        if !self.total_attr_nodes.is_empty() {
            println!("║");
            println!("║ 📦 属性节点统计:");
            let mut sorted_attrs: Vec<_> = self.total_attr_nodes.iter().collect();
            sorted_attrs.sort_by(|a, b| b.1.cmp(a.1));
            for (table, count) in sorted_attrs.iter().take(10) {
                println!("║   {:25} : {:>8} 个", table, count);
            }
        }

        println!("║");
        println!("║ 🌳 层级结构:");
        println!("║   根节点数量: {} 个", self.root_nodes.len());
        if !self.root_nodes.is_empty() {
            println!("║   根节点示例 (前5个):");
            for (refno, name, noun) in self.root_nodes.iter().take(5) {
                println!("║     - {} ({}) [{}]", name, noun, refno);
            }
        }

        println!("║");
        println!("║ 📊 数据库版本分布:");
        println!("║   dbnum 分布:");
        for (dbnum, count) in &self.dbnum_distribution {
            println!("║     DB {}: {} 个", dbnum, count);
        }
        println!("║   sesno 分布:");
        let mut sorted_sesnos: Vec<_> = self.sesno_distribution.iter().collect();
        sorted_sesnos.sort_by(|a, b| a.0.cmp(b.0));
        for (sesno, count) in sorted_sesnos.iter().take(5) {
            println!("║     Session {}: {} 个", sesno, count);
        }

        if !self.orphan_nodes.is_empty() {
            println!("║");
            println!("║ ⚠️  警告: 发现 {} 个孤立节点", self.orphan_nodes.len());
            println!("║   示例 refno: {:?}", &self.orphan_nodes[..self.orphan_nodes.len().min(5)]);
        }

        println!("╚══════════════════════════════════════════════════════════╝\n");
    }
}

async fn validate_kuzu_database(conn: &Connection<'_>) -> anyhow::Result<ValidationStats> {
    let mut stats = ValidationStats::default();
    let start = Instant::now();

    println!("\n🔍 开始验证 Kuzu 数据库...\n");

    // 1. 检查 PE 节点总数
    print!("  1. 统计 PE 节点总数... ");
    let mut result = conn.query("MATCH (p:PE) RETURN COUNT(*)")?;
    if let Some(row) = result.next() {
        if let Some(Value::Int64(count)) = row.get(0) {
            stats.total_pe_nodes = *count as usize;
            println!("✓ {} 个", stats.total_pe_nodes);
        }
    }

    // 2. 统计 noun 类型分布
    print!("  2. 分析 noun 类型分布... ");
    let mut result = conn.query("MATCH (p:PE) RETURN p.noun, COUNT(*) ORDER BY COUNT(*) DESC")?;
    while let Some(row) = result.next() {
        if let (Some(Value::String(noun)), Some(Value::Int64(count))) = (row.get(0), row.get(1)) {
            stats.noun_distribution.insert(noun.clone(), *count as usize);
        }
    }
    println!("✓ {} 种类型", stats.noun_distribution.len());

    // 3. 检查 OWNS 关系
    print!("  3. 检查 OWNS 关系... ");
    let mut result = conn.query("MATCH ()-[r:OWNS]->() RETURN COUNT(*)")?;
    if let Some(row) = result.next() {
        if let Some(Value::Int64(count)) = row.get(0) {
            stats.total_owns_relationships = *count as usize;
            println!("✓ {} 个关系", stats.total_owns_relationships);
        }
    }

    // 4. 查找根节点（没有 owner 的节点）
    print!("  4. 查找根节点... ");
    let mut result = conn.query(
        "MATCH (p:PE)
         WHERE NOT EXISTS { MATCH ()-[:OWNS]->(p) }
         RETURN p.refno, p.name, p.noun
         LIMIT 100"
    )?;
    while let Some(row) = result.next() {
        if let (Some(Value::Int64(refno)), Some(Value::String(name)), Some(Value::String(noun))) =
            (row.get(0), row.get(1), row.get(2)) {
            stats.root_nodes.push((*refno as u64, name.clone(), noun.clone()));
        }
    }
    stats.nodes_without_owner = stats.root_nodes.len();
    println!("✓ {} 个", stats.nodes_without_owner);

    // 5. 统计有 owner 的节点
    print!("  5. 统计有 owner 的节点... ");
    let mut result = conn.query(
        "MATCH ()-[:OWNS]->(p:PE)
         RETURN COUNT(DISTINCT p)"
    )?;
    if let Some(row) = result.next() {
        if let Some(Value::Int64(count)) = row.get(0) {
            stats.nodes_with_owner = *count as usize;
            println!("✓ {} 个", stats.nodes_with_owner);
        }
    }

    // 6. 检查层级深度
    print!("  6. 分析层级深度... ");
    for depth in 1..=10 {
        let query = format!(
            "MATCH path = (root:PE)-[:OWNS*{}]->()
             WHERE NOT EXISTS {{ MATCH ()-[:OWNS]->(root) }}
             RETURN COUNT(path) LIMIT 1",
            depth
        );
        let mut result = conn.query(&query)?;
        if let Some(row) = result.next() {
            if let Some(Value::Int64(count)) = row.get(0) {
                if *count > 0 {
                    stats.max_hierarchy_depth = depth;
                }
            }
        }
    }
    println!("✓ 最大 {} 层", stats.max_hierarchy_depth);

    // 7. 检查属性节点
    print!("  7. 统计属性节点... ");
    let attr_tables = vec!["Attr_EQUI", "Attr_SUBE", "Attr_PIPE", "Attr_BRAN", "Attr_ELBO", "Attr_TEE"];
    for table in attr_tables {
        let query = format!("MATCH (a:{}) RETURN COUNT(*)", table);
        if let Ok(mut result) = conn.query(&query) {
            if let Some(row) = result.next() {
                if let Some(Value::Int64(count)) = row.get(0) {
                    if *count > 0 {
                        stats.total_attr_nodes.insert(table.to_string(), *count as usize);
                    }
                }
            }
        }
    }
    println!("✓ {} 种属性表有数据", stats.total_attr_nodes.len());

    // 8. 检查 dbnum 和 sesno 分布
    print!("  8. 分析数据库版本分布... ");
    let mut result = conn.query("MATCH (p:PE) RETURN DISTINCT p.dbnum, COUNT(*)")?;
    while let Some(row) = result.next() {
        if let (Some(Value::Int64(dbnum)), Some(Value::Int64(count))) = (row.get(0), row.get(1)) {
            stats.dbnum_distribution.insert(*dbnum as i32, *count as usize);
        }
    }

    let mut result = conn.query("MATCH (p:PE) RETURN DISTINCT p.sesno, COUNT(*) ORDER BY p.sesno")?;
    while let Some(row) = result.next() {
        if let (Some(Value::Int64(sesno)), Some(Value::Int64(count))) = (row.get(0), row.get(1)) {
            stats.sesno_distribution.insert(*sesno as i32, *count as usize);
        }
    }
    println!("✓");

    // 9. 查找孤立节点（既没有 owner 也没有 children）
    print!("  9. 查找孤立节点... ");
    let mut result = conn.query(
        "MATCH (p:PE)
         WHERE NOT EXISTS { MATCH ()-[:OWNS]->(p) }
           AND NOT EXISTS { MATCH (p)-[:OWNS]->() }
         RETURN p.refno
         LIMIT 100"
    )?;
    while let Some(row) = result.next() {
        if let Some(Value::Int64(refno)) = row.get(0) {
            stats.orphan_nodes.push(*refno as u64);
        }
    }
    println!("✓ {} 个", stats.orphan_nodes.len());

    println!("\n✅ 验证完成! (耗时: {:.2}秒)", start.elapsed().as_secs_f64());

    Ok(stats)
}

async fn run_sample_queries(conn: &Connection<'_>) -> anyhow::Result<()> {
    println!("\n🔍 运行示例查询:\n");

    // 查询1: 找出拥有最多子节点的元素
    println!("  查询 1: 拥有最多子节点的 PE 元素 (Top 5)");
    let mut result = conn.query(
        "MATCH (p:PE)-[:OWNS]->(child:PE)
         RETURN p.refno, p.name, p.noun, COUNT(child) as child_count
         ORDER BY child_count DESC
         LIMIT 5"
    )?;

    println!("  ┌─────────┬──────────────────────┬──────────┬───────────┐");
    println!("  │ RefNo   │ Name                 │ Noun     │ Children  │");
    println!("  ├─────────┼──────────────────────┼──────────┼───────────┤");
    while let Some(row) = result.next() {
        if let (Some(Value::Int64(refno)), Some(Value::String(name)), Some(Value::String(noun)), Some(Value::Int64(count))) =
            (row.get(0), row.get(1), row.get(2), row.get(3)) {
            println!("  │ {:7} │ {:20} │ {:8} │ {:9} │", refno, name, noun, count);
        }
    }
    println!("  └─────────┴──────────────────────┴──────────┴───────────┘");

    // 查询2: 查找特定类型的元素数量
    println!("\n  查询 2: 主要设备类型统计");
    let equipment_types = vec!["EQUI", "PUMP", "VALVE", "TANK", "VESSEL"];
    println!("  ┌──────────┬──────────┐");
    println!("  │ Type     │ Count    │");
    println!("  ├──────────┼──────────┤");
    for eq_type in equipment_types {
        let query = format!("MATCH (p:PE {{noun: '{}'}}) RETURN COUNT(*)", eq_type);
        if let Ok(mut result) = conn.query(&query) {
            if let Some(row) = result.next() {
                if let Some(Value::Int64(count)) = row.get(0) {
                    if *count > 0 {
                        println!("  │ {:8} │ {:8} │", eq_type, count);
                    }
                }
            }
        }
    }
    println!("  └──────────┴──────────┘");

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("\n╔══════════════════════════════════════════════════════════╗");
    println!("║          Kuzu 数据库 1112 数据验证工具                    ║");
    println!("╚══════════════════════════════════════════════════════════╝");

    // 尝试多个可能的数据库路径
    let possible_paths = vec![
        "./test_output/kuzu_1112_comparison.db",
        "../../test_output/kuzu_1112_comparison.db",
        "./kuzu_1112.db",
        "./data/kuzu_db",
        "../kuzu_db",
    ];

    let mut db_path = None;
    for path in &possible_paths {
        if std::path::Path::new(path).exists() {
            db_path = Some(path.to_string());
            println!("\n✓ 找到 Kuzu 数据库: {}", path);
            break;
        }
    }

    let db_path = db_path.ok_or_else(|| {
        anyhow::anyhow!("未找到 Kuzu 数据库文件。尝试过的路径:\n{:?}", possible_paths)
    })?;

    // 初始化 Kuzu
    init_kuzu(&db_path, SystemConfig::default()).await?;
    let conn = create_kuzu_connection()?;

    // 运行验证
    let stats = validate_kuzu_database(&conn).await?;

    // 打印报告
    stats.print_report();

    // 运行示例查询
    run_sample_queries(&conn).await?;

    println!("\n✅ 所有验证完成!\n");

    Ok(())
}