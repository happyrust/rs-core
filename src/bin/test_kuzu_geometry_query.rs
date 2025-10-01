//! 测试 Kuzu 查询几何元素
//!
//! 运行方式: cargo run --bin test_kuzu_geometry_query --features kuzu

use aios_core::{init_surreal, RefnoEnum, RefU64};
use aios_core::rs_surreal::{get_pe, get_named_attmap, get_children_refnos};
use anyhow::Result;
use log::{error, info};
use simplelog::*;
use std::collections::{HashSet, HashMap};

#[cfg(feature = "kuzu")]
use aios_core::rs_kuzu::{create_kuzu_connection, init_kuzu, init_kuzu_schema};
#[cfg(feature = "kuzu")]
use kuzu::SystemConfig;

/// 几何相关的属性名称
const GEOMETRY_ATTRS: &[&str] = &[
    "POS",     // 位置
    "ORI",     // 方向
    "XLEN",    // X长度
    "YLEN",    // Y长度
    "ZLEN",    // Z长度
    "DIAM",    // 直径
    "RADI",    // 半径
    "HEIG",    // 高度
    "WIDT",    // 宽度
    "LENG",    // 长度
    "PLEN",    // 管道长度
    "ALEN",    // 实际长度
    "PTAX",    // 点轴
    "PAAX",    // P轴
    "PBAX",    // B轴
    "PTCA",    // 点CA
    "PTCB",    // 点CB
];

/// 通常有几何的元素类型
const GEOMETRY_NOUNS: &[&str] = &[
    "PIPE",    // 管道
    "BRAN",    // 分支
    "EQUI",    // 设备
    "STRU",    // 结构
    "FITT",    // 管件
    "VALV",    // 阀门
    "FLAN",    // 法兰
    "INST",    // 仪表
    "PLOO",    // 管道环
    "LOOP",    // 环
    "CYLI",    // 圆柱
    "BOX",     // 箱体
    "PYRA",    // 金字塔
    "CONE",    // 圆锥
    "SPHE",    // 球体
    "NOZZ",    // 管嘴
    "DISH",    // 碟形封头
];

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
    info!("Kuzu 查询几何元素测试");
    info!("========================================");

    // 1. 初始化 SurrealDB
    info!("\n步骤 1: 初始化 SurrealDB");
    init_surreal().await?;
    info!("✓ SurrealDB 初始化成功");

    // 2. 解析参考号
    let refno_str = "17496/266203";
    let parts: Vec<&str> = refno_str.split('/').collect();
    let dbnum = parts[0].parse::<u32>()?;
    let refnum = parts[1].parse::<u32>()?;
    let target_refno = RefnoEnum::from(RefU64::from_two_nums(dbnum, refnum));

    info!("\n步骤 2: 查询参考号 {} 的信息", refno_str);

    // 获取目标元素的 PE 信息
    if let Ok(Some(pe)) = get_pe(target_refno).await {
        info!("找到目标元素:");
        info!("  - 名称: {}", pe.name);
        info!("  - 类型: {}", pe.noun);
        info!("  - DBNum: {}", pe.dbnum);
    } else {
        error!("未找到参考号 {} 的元素", refno_str);
        return Ok(());
    }

    // 3. 获取所有子元素
    info!("\n步骤 3: 查询所有子元素");

    let mut all_children = Vec::new();
    let mut to_process = vec![target_refno];
    let mut processed = HashSet::new();

    // 递归获取所有子元素
    while !to_process.is_empty() {
        let current = to_process.pop().unwrap();
        if processed.contains(&current) {
            continue;
        }
        processed.insert(current);

        if let Ok(children) = get_children_refnos(current).await {
            for child in children {
                all_children.push(child);
                to_process.push(child);
            }
        }
    }

    info!("  找到 {} 个子元素", all_children.len());

    // 4. 筛选有几何的元素
    info!("\n步骤 4: 筛选有几何属性的元素");

    let mut geometry_elements = Vec::new();
    let mut type_statistics: HashMap<String, usize> = HashMap::new();

    for child_refno in &all_children {
        // 获取元素信息
        if let Ok(Some(pe)) = get_pe(*child_refno).await {
            // 检查是否是几何相关的类型
            if GEOMETRY_NOUNS.contains(&pe.noun.as_str()) {
                // 获取属性
                if let Ok(attmap) = get_named_attmap(*child_refno).await {
                    // 检查是否有几何属性
                    let mut has_geometry = false;
                    let mut geometry_attrs = Vec::new();

                    for attr in GEOMETRY_ATTRS {
                        if attmap.map.contains_key(*attr) {
                            has_geometry = true;
                            geometry_attrs.push(attr.to_string());
                        }
                    }

                    if has_geometry {
                        geometry_elements.push((pe.clone(), geometry_attrs));
                        *type_statistics.entry(pe.noun.clone()).or_insert(0) += 1;
                    }
                }
            }
        }
    }

    info!("  找到 {} 个有几何属性的元素", geometry_elements.len());

    // 5. 统计结果
    info!("\n步骤 5: 统计分析");
    info!("按类型统计:");
    for (noun, count) in type_statistics.iter() {
        info!("  - {}: {} 个", noun, count);
    }

    // 6. 显示部分示例
    info!("\n步骤 6: 显示前 10 个有几何的元素");
    for (i, (pe, attrs)) in geometry_elements.iter().take(10).enumerate() {
        info!("  {}. {} ({})", i + 1, pe.name, pe.noun);
        info!("     几何属性: {:?}", attrs);
    }

    // 7. 如果启用了 Kuzu，同步数据并测试查询
    #[cfg(feature = "kuzu")]
    {
        use std::fs;

        info!("\n步骤 7: 同步到 Kuzu 并测试查询");

        let kuzu_db_path = "./data/kuzu_geometry_test";

        // 清理旧数据
        if fs::metadata(kuzu_db_path).is_ok() {
            info!("  清理旧的 Kuzu 数据库...");
            fs::remove_dir_all(kuzu_db_path)?;
        }

        // 初始化 Kuzu
        init_kuzu(kuzu_db_path, SystemConfig::default()).await?;
        init_kuzu_schema().await?;
        info!("✓ Kuzu 初始化成功");

        // 同步有几何的元素到 Kuzu
        info!("\n步骤 8: 同步数据到 Kuzu");
        let conn = create_kuzu_connection()?;

        let mut sync_count = 0;
        for (pe, _) in &geometry_elements {
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
                pe.status_code.as_ref().map_or("NULL".to_string(), |s| format!("'{}'", s.replace("'", "''"))),
                pe.lock
            );

            if conn.query(&insert_sql).is_ok() {
                sync_count += 1;
            }
        }

        info!("  成功同步 {} 个元素到 Kuzu", sync_count);

        // 9. 在 Kuzu 中查询测试
        info!("\n步骤 9: Kuzu 查询测试");

        // 查询 PIPE 类型的数量
        let query = "MATCH (p:PE) WHERE p.noun = 'PIPE' RETURN count(p) AS cnt";
        if let Ok(mut result) = conn.query(query) {
            if let Some(row) = result.next() {
                if let kuzu::Value::Int64(count) = row.get(0).unwrap() {
                    info!("  Kuzu 中 PIPE 元素数量: {}", count);
                }
            }
        }

        // 查询 EQUI 类型的数量
        let query = "MATCH (p:PE) WHERE p.noun = 'EQUI' RETURN count(p) AS cnt";
        if let Ok(mut result) = conn.query(query) {
            if let Some(row) = result.next() {
                if let kuzu::Value::Int64(count) = row.get(0).unwrap() {
                    info!("  Kuzu 中 EQUI 元素数量: {}", count);
                }
            }
        }

        // 查询所有类型
        let query = "MATCH (p:PE) RETURN DISTINCT p.noun AS noun ORDER BY noun";
        if let Ok(mut result) = conn.query(query) {
            info!("  Kuzu 中的所有元素类型:");
            while let Some(row) = result.next() {
                if let kuzu::Value::String(noun) = row.get(0).unwrap() {
                    info!("    - {}", noun);
                }
            }
        }

        // 10. 性能测试
        info!("\n步骤 10: 查询性能测试");
        use std::time::Instant;

        let start = Instant::now();
        let query = "MATCH (p:PE) RETURN count(p)";
        let _result = conn.query(query)?;
        info!("  统计所有节点: {:?}", start.elapsed());

        let start = Instant::now();
        let query = "MATCH (p:PE) WHERE p.noun IN ['PIPE', 'EQUI', 'VALV'] RETURN count(p)";
        let _result = conn.query(query)?;
        info!("  按类型过滤查询: {:?}", start.elapsed());
    }

    #[cfg(not(feature = "kuzu"))]
    {
        error!("Kuzu feature 未启用！请使用 --features kuzu 运行");
    }

    info!("\n========================================");
    info!("测试完成！");
    info!("========================================");
    info!("总结:");
    info!("  - 目标参考号: {}", refno_str);
    info!("  - 子元素总数: {}", all_children.len());
    info!("  - 有几何的元素: {}", geometry_elements.len());

    Ok(())
}