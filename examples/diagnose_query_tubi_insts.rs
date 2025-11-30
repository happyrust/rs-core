//! 全面诊断 query_tubi_insts_by_brans 查询问题
//! 
//! 这个示例程序将全面检查数据库中的表结构和数据，以便诊断查询问题

use aios_core::{RefnoEnum, SUL_DB, SurrealQueryExt, init_surreal, query_tubi_insts_by_brans};
use aios_core::rs_surreal::inst::TubiInstQuery;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== 全面诊断 query_tubi_insts_by_brans 查询问题 ===\n");

    // 初始化数据库连接
    println!("初始化数据库连接...");
    init_surreal().await?;
    println!("✓ 数据库连接成功\n");

    let test_refno = RefnoEnum::from("pe:21491_10000");
    println!("测试查询: {:?}\n", test_refno);

    // 1. 检查所有相关表的存在性
    println!("1. 检查表结构...");
    
    // 检查 pe 表
    let pe_check_sql = "SELECT COUNT() as count FROM pe";
    let pe_count: Vec<i64> = SUL_DB.query_take(pe_check_sql, 0).await.unwrap_or_default();
    println!("   pe 表记录数: {}", pe_count.get(0).unwrap_or(&0));
    
    // 检查 tubi_relate 表
    let tubi_check_sql = "SELECT COUNT() as count FROM tubi_relate";
    let tubi_count: Vec<i64> = SUL_DB.query_take(tubi_check_sql, 0).await.unwrap_or_default();
    println!("   tubi_relate 表记录数: {}", tubi_count.get(0).unwrap_or(&0));

    // 2. 检查是否有任何以 21491 开头的 pe 记录
    println!("\n2. 检查相关记录...");
    let pe_21491_sql = "SELECT id, owner.noun FROM pe WHERE id LIKE '21491_%' LIMIT 10";
    let pe_21491_records: Vec<serde_json::Value> = SUL_DB.query_take(pe_21491_sql, 0).await.unwrap_or_default();
    println!("   以 21491 开头的 pe 记录:");
    for (i, record) in pe_21491_records.iter().enumerate() {
        if let Some(id) = record.get("id") {
            if let Some(owner) = record.get("owner") {
                if let Some(noun) = owner.get("noun") {
                    println!("     [{}] id: {}, noun: {}", i + 1, id.as_str().unwrap_or("null"), noun.as_str().unwrap_or("null"));
                }
            }
        }
    }

    // 3. 检查是否有任何 tubi_relate 记录
    println!("\n3. 检查 tubi_relate 记录...");
    let tubi_sample_sql = "SELECT id, in, id[0].old_pe FROM tubi_relate LIMIT 5";
    let tubi_records: Vec<serde_json::Value> = SUL_DB.query_take(tubi_sample_sql, 0).await.unwrap_or_default();
    println!("   tubi_relate 记录样本:");
    for (i, record) in tubi_records.iter().enumerate() {
        println!("     [{}] {:?}", i + 1, record);
    }

    // 4. 检查 tubi_relate 表结构
    println!("\n4. 检查 tubi_relate 表结构...");
    let tubi_structure_sql = "SELECT * FROM tubi_relate LIMIT 1";
    let tubi_structure: Vec<serde_json::Value> = SUL_DB.query_take(tubi_structure_sql, 0).await.unwrap_or_default();
    println!("   tubi_relate 表结构:");
    for (key, value) in tubi_structure.get(0).unwrap_or(&serde_json::Value::Object(serde_json::Map::new())).as_object().unwrap_or(&serde_json::Map::new()) {
        println!("     {}: {}", key, value);
    }

    // 5. 尝试不同的查询方式
    println!("\n5. 尝试不同的查询方式...");
    
    // 5.1 检查记录ID格式
    let pe_format_check_sql = "SELECT id FROM pe WHERE id = 'pe:21491_10000'";
    let pe_format_check: Vec<serde_json::Value> = SUL_DB.query_take(pe_format_check_sql, 0).await.unwrap_or_default();
    println!("   pe 记录ID格式检查:");
    for record in pe_format_check.iter() {
        println!("     {:?}", record);
    }

    // 5.2 尝试简化查询
    let simple_sql = "SELECT COUNT() FROM tubi_relate WHERE id[0] = 'pe:21491_10000'";
    let simple_count: Vec<i64> = SUL_DB.query_take(simple_sql, 0).await.unwrap_or_default();
    println!("   简化查询匹配记录数: {}", simple_count.get(0).unwrap_or(&0));

    // 5.3 尝试不使用 aabb 过滤的查询
    let no_aabb_sql = r#"
        SELECT COUNT() FROM tubi_relate:[pe:21491_10000, 0]..[pe:21491_10000, ..]
        "#;
    let no_aabb_count: Vec<i64> = SUL_DB.query_take(no_aabb_sql, 0).await.unwrap_or_default();
    println!("   不使用 aabb 过滤的查询记录数: {}", no_aabb_count.get(0).unwrap_or(&0));

    // 6. 使用原始函数再次查询
    println!("\n6. 使用原始函数再次查询...");
    let results: Vec<TubiInstQuery> = query_tubi_insts_by_brans(&[test_refno]).await?;
    println!("   原始函数返回 {} 条记录", results.len());

    if !results.is_empty() {
        println!("   原始函数查询成功！记录详情:");
        for (i, result) in results.iter().enumerate() {
            println!("     [{}] refno: {:?}", i + 1, result.refno);
            println!("       leave: {:?}", result.leave);
            if let Some(old_refno) = &result.old_refno {
                println!("       old_refno: {:?}", old_refno);
            }
            if let Some(generic) = &result.generic {
                println!("       generic: {:?}", generic);
            }
            println!("       world_aabb: {:?}", result.world_aabb);
            println!("       world_trans: {:?}", result.world_trans);
            println!("       geo_hash: {}", result.geo_hash);
            if let Some(date) = &result.date {
                println!("       date: {}", date);
            }
        }
    }

    println!("\n=== 诊断完成 ===");
    Ok(())
}