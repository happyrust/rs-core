//! 测试 query_tubi_insts_by_brans 函数的示例程序
//!
//! 这个示例程序演示如何使用 query_tubi_insts_by_brans 函数查询 Tubi 实例数据

use aios_core::rs_surreal::inst::{TubiInstQuery, query_tubi_insts_by_brans};
use aios_core::{RefnoEnum, init_surreal};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== 测试 query_tubi_insts_by_brans 函数 ===\n");

    // 初始化数据库连接
    println!("初始化数据库连接...");
    init_surreal().await?;
    println!("✓ 数据库连接成功\n");

    // 测试 pe:21491_10000 的查询
    let test_refno = RefnoEnum::from("pe:21491_10000");
    println!("测试查询: {:?}\n", test_refno);

    let results = query_tubi_insts_by_brans(&[test_refno]).await?;

    println!("找到 {} 条 Tubi 实例记录\n", results.len());

    if !results.is_empty() {
        println!("Tubi 实例记录详情:");
        for (i, result) in results.iter().enumerate() {
            println!("  [{}] refno: {:?}", i + 1, result.refno);
            println!("      leave: {:?}", result.leave);
            if let Some(old_refno) = &result.old_refno {
                println!("      old_refno: {:?}", old_refno);
            }
            if let Some(generic) = &result.generic {
                println!("      generic: {}", generic);
            }
            println!("      world_aabb: {:?}", result.world_aabb);
            println!("      geo_hash: {}", result.geo_hash);
            if let Some(date) = &result.date {
                println!("      date: {}", date);
            }
            println!();
        }
    } else {
        println!("⚠️ 未找到 Tubi 实例记录\n");
    }

    println!("=== 测试完成 ===");
    Ok(())
}
