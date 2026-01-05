//! 调试 query_tubi_insts_by_brans 函数的查询逻辑
//!
//! 这个示例程序将生成完整的查询语句并执行它，以便分析为什么查询返回空结果

use aios_core::rs_surreal::inst::{TubiInstQuery, query_tubi_insts_by_brans};
use aios_core::{RefnoEnum, SUL_DB, SurrealQueryExt, init_surreal};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== 调试 query_tubi_insts_by_brans 函数 ===\n");

    // 初始化数据库连接
    println!("初始化数据库连接...");
    init_surreal().await?;
    println!("✓ 数据库连接成功\n");

    // 测试 pe:21491_10000 的查询
    let test_refno = RefnoEnum::from("pe:21491_10000");
    println!("测试查询: {:?}\n", test_refno);

    // 生成查询语句
    let pe_key = test_refno.to_pe_key();
    let sql = format!(
        r#"
        SELECT
            id[0] as refno,
            in as leave,
            id[0].old_pe as old_refno,
            id[0].owner.noun as generic,
            aabb.d as world_aabb,
            world_trans.d as world_trans,
            record::id(geo) as geo_hash,
            id[0].dt as date
        FROM tubi_relate:[{}, 0]..[{}, ..]
        WHERE aabb.d != NONE
        "#,
        pe_key, pe_key
    );

    println!("生成的查询语句:\n{}\n", sql);

    // 直接执行查询语句
    println!("直接执行查询语句...");
    let direct_results: Vec<TubiInstQuery> = SUL_DB.query_take(&sql, 0).await.unwrap_or_default();
    println!("直接查询返回 {} 条记录\n", direct_results.len());

    if !direct_results.is_empty() {
        println!("直接查询结果详情:");
        for (i, result) in direct_results.iter().enumerate() {
            println!("  [{}] refno: {:?}", i + 1, result.refno);
            println!("      leave: {:?}", result.leave);
            if let Some(old_refno) = &result.old_refno {
                println!("      old_refno: {:?}", old_refno);
            }
            if let Some(generic) = &result.generic {
                println!("      generic: {}", generic);
            }
            println!("      world_aabb: {:?}", result.world_aabb);
            println!("      world_trans: {:?}", result.world_trans);
            println!("      geo_hash: {}", result.geo_hash);
            if let Some(date) = &result.date {
                println!("      date: {}", date);
            }
            println!();
        }
    } else {
        println!("⚠️ 直接查询未找到记录\n");
    }

    // 检查 tubi_relate 表中是否有相关数据
    println!("\n检查 tubi_relate 表中的数据...");
    let check_sql = format!(
        r#"
        SELECT COUNT() as count FROM tubi_relate WHERE id[0] LIKE '{}_%'
        "#,
        pe_key
    );
    let count_result: Vec<i64> = SUL_DB.query_take(&check_sql, 0).await.unwrap_or_default();
    if !count_result.is_empty() {
        println!(
            "tubi_relate 表中有 {} 条以 '{}' 开头的记录",
            count_result[0], pe_key
        );
    } else {
        println!("⚠️ 无法获取 tubi_relate 表记录计数");
    }

    // 检查 pe 表中是否有相关数据
    println!("\n检查 pe 表中的数据...");
    let pe_check_sql = format!(
        r#"
        SELECT id, owner.noun as noun, old_pe FROM pe WHERE id = '{}'
        "#,
        test_refno.to_string()
    );
    let pe_results: Vec<serde_json::Value> = SUL_DB
        .query_take(&pe_check_sql, 0)
        .await
        .unwrap_or_default();
    if !pe_results.is_empty() {
        println!("pe 表记录:");
        for (i, result) in pe_results.iter().enumerate() {
            println!("  [{}] {:?}", i + 1, result);
        }
    } else {
        println!("⚠️ pe 表中未找到记录");
    }

    // 检查 tubi_relate 表结构
    println!("\n检查 tubi_relate 表结构...");
    let structure_sql = r#"
        SELECT * FROM tubi_relate LIMIT 5
        "#;
    let structure_results: Vec<serde_json::Value> = SUL_DB
        .query_take(structure_sql, 0)
        .await
        .unwrap_or_default();
    println!("tubi_relate 表结构（前5条）:");
    for (i, result) in structure_results.iter().enumerate() {
        println!("  [{}] {:?}", i + 1, result);
    }

    // 使用原始函数查询
    println!("\n使用原始函数查询...");
    let results = query_tubi_insts_by_brans(&[test_refno]).await?;
    println!("原始函数返回 {} 条记录\n", results.len());

    println!("=== 调试完成 ===");
    Ok(())
}
