//! PDMS 数据库解析到 Kuzu 图数据库
//!
//! 完整的解析流程：
//! 1. 使用 PdmsIO 读取 PDMS 数据库文件
//! 2. 解析为 NamedAttrMap 结构
//! 3. 转换为 SPdmsElement + 属性
//! 4. 批量保存到 Kuzu 图数据库
//! 5. 创建关系和索引
//!
//! 运行方式:
//! cd external/rs-core && cargo run --release --example parse_pdms_to_kuzu --features kuzu -- --db 1112

use aios_core::rs_kuzu::*;
use aios_core::types::*;
use aios_core::pe::SPdmsElement;
use clap::Parser;
use kuzu::SystemConfig;
use pdms_io::io::PdmsIO;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Parser, Debug)]
#[command(name = "parse_pdms_to_kuzu")]
#[command(about = "Parse PDMS database and save to Kuzu graph database")]
struct Args {
    /// Database number to parse (e.g., 1112)
    #[arg(short, long)]
    db: String,

    /// PDMS database path (default: auto-detect based on db number)
    #[arg(short, long)]
    path: Option<String>,

    /// Kuzu database output path
    #[arg(short, long, default_value = "./kuzu_pdms.db")]
    output: String,

    /// Batch size for processing
    #[arg(short, long, default_value = "1000")]
    batch_size: usize,

    /// Maximum elements to process (0 = all)
    #[arg(short, long, default_value = "0")]
    limit: usize,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

/// 解析统计信息
#[derive(Debug, Default)]
struct ParseStats {
    total_elements: usize,
    parsed_count: usize,
    saved_pe_nodes: usize,
    saved_attr_nodes: usize,
    saved_relations: usize,
    failed_count: usize,
    parse_time_ms: u128,
    save_time_ms: u128,
    noun_distribution: HashMap<String, usize>,
    error_messages: Vec<String>,
}

impl ParseStats {
    fn print_summary(&self) {
        println!("\n╔════════════════════════════════════════════════════════╗");
        println!("║           PDMS 到 Kuzu 解析报告                         ║");
        println!("╠════════════════════════════════════════════════════════╣");

        println!("║ 📊 解析统计:");
        println!("║   总元素数:        {:>10} 个", self.total_elements);
        println!("║   成功解析:        {:>10} 个", self.parsed_count);
        println!("║   解析失败:        {:>10} 个", self.failed_count);
        println!("║   解析耗时:        {:>10} ms", self.parse_time_ms);

        println!("║");
        println!("║ 💾 Kuzu 保存:");
        println!("║   PE 节点:         {:>10} 个", self.saved_pe_nodes);
        println!("║   属性节点:        {:>10} 个", self.saved_attr_nodes);
        println!("║   关系边:          {:>10} 个", self.saved_relations);
        println!("║   保存耗时:        {:>10} ms", self.save_time_ms);

        println!("║");
        println!("║ 📋 Noun 类型分布 (Top 10):");
        let mut sorted_nouns: Vec<_> = self.noun_distribution.iter().collect();
        sorted_nouns.sort_by(|a, b| b.1.cmp(a.1));
        for (i, (noun, count)) in sorted_nouns.iter().take(10).enumerate() {
            println!("║   {:2}. {:15} : {:>8} 个", i + 1, noun, count);
        }

        if !self.error_messages.is_empty() {
            println!("║");
            println!("║ ⚠️  错误信息 (前5条):");
            for msg in self.error_messages.iter().take(5) {
                println!("║   - {}", msg);
            }
        }

        println!("║");
        println!("║ ⏱️  总耗时:          {:>10} ms",
            self.parse_time_ms + self.save_time_ms);

        if self.parsed_count > 0 {
            let parse_speed = (self.parsed_count as f64 / (self.parse_time_ms as f64 / 1000.0)) as usize;
            let save_speed = (self.saved_pe_nodes as f64 / (self.save_time_ms as f64 / 1000.0)) as usize;
            println!("║ ⚡ 性能指标:");
            println!("║   解析速度:        {:>10} 个/秒", parse_speed);
            println!("║   保存速度:        {:>10} 个/秒", save_speed);
        }

        println!("╚════════════════════════════════════════════════════════╝\n");
    }
}

/// 获取 PDMS 数据库路径
fn get_pdms_path(db_number: &str) -> PathBuf {
    // 根据数据库编号构建路径
    let base_paths = vec![
        format!("/Volumes/DPC/work/e3d_models/AvevaMarineSample/ams000/ams{}_0001", db_number),
        format!("./data/pdms/ams{}_0001", db_number),
        format!("../pdms_data/ams{}_0001", db_number),
    ];

    for path in base_paths {
        let path_buf = PathBuf::from(&path);
        if path_buf.exists() {
            return path_buf;
        }
    }

    // 默认路径
    PathBuf::from(format!("./ams{}_0001", db_number))
}

/// 批量保存 PE 和属性到 Kuzu
async fn save_batch_to_kuzu(
    elements: &[(SPdmsElement, NamedAttrMap)],
    stats: &mut ParseStats,
) -> anyhow::Result<()> {
    let conn = create_kuzu_connection()?;

    // 开始事务
    conn.query("BEGIN TRANSACTION")?;

    let result = (|| {
        // 1. 保存所有 PE 节点
        for (pe, _) in elements {
            let query = format!(
                "CREATE (p:PE {{
                    refno: {},
                    name: '{}',
                    noun: '{}',
                    dbnum: {},
                    sesno: {},
                    deleted: {},
                    lock: {},
                    cata_hash: '{}'
                }})",
                pe.refno.refno().0,
                pe.name.replace('\'', "''"),
                pe.noun,
                pe.dbnum,
                pe.sesno,
                pe.deleted,
                pe.lock,
                pe.cata_hash.replace('\'', "''")
            );

            if let Err(e) = conn.query(&query) {
                stats.error_messages.push(format!("PE {} 保存失败: {}", pe.refno.refno().0, e));
                stats.failed_count += 1;
            } else {
                stats.saved_pe_nodes += 1;
            }
        }

        // 2. 保存属性节点（根据 noun 类型）
        for (pe, attmap) in elements {
            if let Err(e) = save_attributes_for_pe(&conn, pe, attmap, stats) {
                stats.error_messages.push(format!("属性保存失败: {}", e));
            }
        }

        // 3. 创建 OWNS 关系
        for (pe, _) in elements {
            if !pe.owner.refno().is_unset() {
                let query = format!(
                    "MATCH (parent:PE {{refno: {}}}), (child:PE {{refno: {}}})
                     CREATE (parent)-[:OWNS]->(child)",
                    pe.owner.refno().0,
                    pe.refno.refno().0
                );

                if conn.query(&query).is_ok() {
                    stats.saved_relations += 1;
                }
            }
        }

        Ok::<(), anyhow::Error>(())
    })();

    match result {
        Ok(_) => {
            conn.query("COMMIT")?;
            Ok(())
        }
        Err(e) => {
            conn.query("ROLLBACK")?;
            Err(e)
        }
    }
}

/// 保存属性节点
fn save_attributes_for_pe(
    conn: &kuzu::Connection,
    pe: &SPdmsElement,
    attmap: &NamedAttrMap,
    stats: &mut ParseStats,
) -> anyhow::Result<()> {
    let noun = pe.noun.to_uppercase();
    let table_name = format!("Attr_{}", noun);

    // 构建属性字段
    let mut fields = vec![format!("refno: {}", pe.refno.refno().0)];

    // 添加常见属性
    for (key, value) in &attmap.map {
        // 跳过特殊字段
        if key == "REFNO" || key == "TYPE" || key.starts_with("UDA:") {
            continue;
        }

        match value {
            NamedAttrValue::IntegerType(v) => {
                fields.push(format!("{}: {}", key.to_uppercase(), v));
            }
            NamedAttrValue::F32Type(v) => {
                fields.push(format!("{}: {}", key.to_uppercase(), v));
            }
            NamedAttrValue::StringType(s) | NamedAttrValue::WordType(s) => {
                fields.push(format!("{}: '{}'", key.to_uppercase(), s.replace('\'', "''")));
            }
            NamedAttrValue::BoolType(b) => {
                fields.push(format!("{}: {}", key.to_uppercase(), b));
            }
            NamedAttrValue::Vec3Type(v) => {
                fields.push(format!("{}: [{}, {}, {}]", key.to_uppercase(), v.x, v.y, v.z));
            }
            _ => {}
        }
    }

    // 尝试创建属性节点
    let query = format!(
        "CREATE (a:{} {{ {} }})",
        table_name,
        fields.join(", ")
    );

    match conn.query(&query) {
        Ok(_) => {
            stats.saved_attr_nodes += 1;

            // 创建 PE 到属性的关系
            let rel_query = format!(
                "MATCH (p:PE {{refno: {}}}), (a:{} {{refno: {}}})
                 CREATE (p)-[:TO_{}]->(a)",
                pe.refno.refno().0,
                table_name,
                pe.refno.refno().0,
                noun
            );

            if conn.query(&rel_query).is_ok() {
                stats.saved_relations += 1;
            }
        }
        Err(e) => {
            // 如果表不存在，记录但不中断
            if !e.to_string().contains("does not exist") {
                return Err(e.into());
            }
        }
    }

    Ok(())
}

/// 解析 PDMS 数据并保存到 Kuzu
async fn parse_and_save(args: &Args) -> anyhow::Result<ParseStats> {
    let mut stats = ParseStats::default();

    println!("\n🚀 开始解析 PDMS 数据库 {}...", args.db);

    // 1. 获取数据库路径
    let db_path = if let Some(path) = &args.path {
        PathBuf::from(path)
    } else {
        get_pdms_path(&args.db)
    };

    if !db_path.exists() {
        return Err(anyhow::anyhow!("数据库文件不存在: {:?}", db_path));
    }

    println!("📂 数据库路径: {:?}", db_path);

    // 2. 初始化 Kuzu 数据库
    println!("🔧 初始化 Kuzu 数据库: {}", args.output);

    // 删除旧数据库
    if std::path::Path::new(&args.output).exists() {
        std::fs::remove_dir_all(&args.output)?;
    }

    init_kuzu(&args.output, SystemConfig::default()).await?;
    init_kuzu_schema().await?;

    // 3. 打开 PDMS 数据库
    let parse_start = Instant::now();
    let mut pdms_io = PdmsIO::new("", db_path.clone(), true);

    if let Err(e) = pdms_io.open() {
        return Err(anyhow::anyhow!("无法打开数据库文件: {}", e));
    }

    println!("✓ PDMS 数据库已打开");

    // 4. 批量读取和处理数据
    let mut batch_buffer = Vec::new();
    let mut total_processed = 0;

    loop {
        // 读取一批元素
        match pdms_io.read_element() {
            Ok(Some(elem_data)) => {
                // 解析为 NamedAttrMap
                if let Ok(attmap) = parse_element_to_attmap(&elem_data) {
                    // 转换为 SPdmsElement
                    let pe = attmap.pe(args.db.parse::<i32>().unwrap_or(1112));

                    // 更新统计
                    *stats.noun_distribution.entry(pe.noun.clone()).or_insert(0) += 1;
                    stats.parsed_count += 1;

                    batch_buffer.push((pe, attmap));

                    // 达到批量大小时保存
                    if batch_buffer.len() >= args.batch_size {
                        let save_start = Instant::now();
                        save_batch_to_kuzu(&batch_buffer, &mut stats).await?;
                        stats.save_time_ms += save_start.elapsed().as_millis();

                        if args.verbose {
                            println!("  已处理 {} 个元素...", total_processed + batch_buffer.len());
                        }

                        total_processed += batch_buffer.len();
                        batch_buffer.clear();
                    }
                }

                stats.total_elements += 1;

                // 检查限制
                if args.limit > 0 && stats.total_elements >= args.limit {
                    break;
                }
            }
            Ok(None) => break, // 没有更多元素
            Err(e) => {
                stats.error_messages.push(format!("读取错误: {}", e));
                stats.failed_count += 1;

                // 继续处理下一个
                if stats.failed_count > 100 {
                    println!("⚠️  错误过多，停止处理");
                    break;
                }
            }
        }
    }

    // 5. 保存剩余的批次
    if !batch_buffer.is_empty() {
        let save_start = Instant::now();
        save_batch_to_kuzu(&batch_buffer, &mut stats).await?;
        stats.save_time_ms += save_start.elapsed().as_millis();
    }

    stats.parse_time_ms = parse_start.elapsed().as_millis();

    // 6. 创建索引优化查询
    println!("\n📊 创建索引...");
    create_indexes().await?;

    Ok(stats)
}

/// 解析元素数据为 NamedAttrMap
fn parse_element_to_attmap(elem_data: &[u8]) -> anyhow::Result<NamedAttrMap> {
    // 这里需要根据实际的 PDMS 数据格式进行解析
    // 简化示例，实际需要调用 PDMS 解析库
    let mut attmap = NamedAttrMap::default();

    // TODO: 实际解析逻辑
    // attmap.insert(...);

    Ok(attmap)
}

/// 创建索引
async fn create_indexes() -> anyhow::Result<()> {
    let conn = create_kuzu_connection()?;

    // PE 表索引
    let indexes = vec![
        "CREATE INDEX IF NOT EXISTS pe_refno_idx ON PE(refno)",
        "CREATE INDEX IF NOT EXISTS pe_noun_idx ON PE(noun)",
        "CREATE INDEX IF NOT EXISTS pe_name_idx ON PE(name)",
        "CREATE INDEX IF NOT EXISTS pe_dbnum_idx ON PE(dbnum)",
    ];

    for idx_sql in indexes {
        if let Err(e) = conn.query(idx_sql) {
            eprintln!("创建索引失败: {}", e);
        }
    }

    println!("✓ 索引创建完成");
    Ok(())
}

/// 验证保存的数据
async fn verify_saved_data() -> anyhow::Result<()> {
    let conn = create_kuzu_connection()?;

    // 检查 PE 节点数量
    let mut result = conn.query("MATCH (p:PE) RETURN COUNT(*)")?;
    if let Some(row) = result.next() {
        if let Some(kuzu::Value::Int64(count)) = row.get(0) {
            println!("✓ PE 节点总数: {}", count);
        }
    }

    // 检查关系数量
    let mut result = conn.query("MATCH ()-[r:OWNS]->() RETURN COUNT(*)")?;
    if let Some(row) = result.next() {
        if let Some(kuzu::Value::Int64(count)) = row.get(0) {
            println!("✓ OWNS 关系总数: {}", count);
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 解析命令行参数
    let args = Args::parse();

    // 设置日志级别
    if args.verbose {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Debug)
            .init();
    } else {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Info)
            .init();
    }

    println!("╔════════════════════════════════════════════════════════╗");
    println!("║          PDMS to Kuzu 解析工具                          ║");
    println!("╚════════════════════════════════════════════════════════╝");

    // 执行解析
    match parse_and_save(&args).await {
        Ok(stats) => {
            stats.print_summary();

            // 验证数据
            println!("\n🔍 验证保存的数据...");
            verify_saved_data().await?;

            println!("\n✅ 解析完成! 数据已保存到: {}", args.output);
        }
        Err(e) => {
            eprintln!("\n❌ 解析失败: {}", e);
            return Err(e);
        }
    }

    Ok(())
}