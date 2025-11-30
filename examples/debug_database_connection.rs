//! 调试数据库连接问题
//! 
//! 这个程序将检查我们连接的数据库是否包含预期的数据

use aios_core::{RefnoEnum, SUL_DB, SurrealQueryExt, init_surreal};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== 调试数据库连接问题 ===\n");

    // 初始化数据库连接
    println!("初始化数据库连接...");
    init_surreal().await?;
    println!("✓ 数据库连接成功\n");

    // 1. 检查数据库基本信息
    println!("1. 检查数据库基本信息...");
    let info_sql = "SELECT * FROM $auth";
    let auth_info: Vec<serde_json::Value> = SUL_DB.query_take(info_sql, 0).await.unwrap_or_default();
    println!("   认证信息: {:?}", auth_info);

    // 2. 检查所有表
    println!("\n2. 检查所有表...");
    let tables_sql = "SELECT * FROM information.tables";
    let tables: Vec<serde_json::Value> = SUL_DB.query_take(tables_sql, 0).await.unwrap_or_default();
    println!("   数据库中的表:");
    for table in tables.iter() {
        if let Some(name) = table.get("name") {
            println!("     - {}", name.as_str().unwrap_or("unknown"));
        }
    }

    // 3. 检查 pe 表
    println!("\n3. 检查 pe 表...");
    let pe_count_sql = "SELECT COUNT() as count FROM pe";
    let pe_count: Vec<i64> = SUL_DB.query_take(pe_count_sql, 0).await.unwrap_or_default();
    println!("   pe 表记录数: {}", pe_count.get(0).unwrap_or(&0));

    // 如果有记录，查看一些样本
    if pe_count.get(0).unwrap_or(&0) > &0 {
        let pe_sample_sql = "SELECT id, owner.noun FROM pe LIMIT 5";
        let pe_samples: Vec<serde_json::Value> = SUL_DB.query_take(pe_sample_sql, 0).await.unwrap_or_default();
        println!("   pe 表样本记录:");
        for (i, record) in pe_samples.iter().enumerate() {
            if let Some(id) = record.get("id") {
                if let Some(owner) = record.get("owner") {
                    if let Some(noun) = owner.get("noun") {
                        println!("     [{}] id: {}, noun: {}", i + 1, id.as_str().unwrap_or("null"), noun.as_str().unwrap_or("null"));
                    }
                }
            }
        }
    }

    // 4. 检查 tubi_relate 表
    println!("\n4. 检查 tubi_relate 表...");
    let tubi_count_sql = "SELECT COUNT() as count FROM tubi_relate";
    let tubi_count: Vec<i64> = SUL_DB.query_take(tubi_count_sql, 0).await.unwrap_or_default();
    println!("   tubi_relate 表记录数: {}", tubi_count.get(0).unwrap_or(&0));

    // 如果有记录，查看一些样本
    if tubi_count.get(0).unwrap_or(&0) > &0 {
        let tubi_sample_sql = "SELECT id, in FROM tubi_relate LIMIT 5";
        let tubi_samples: Vec<serde_json::Value> = SUL_DB.query_take(tubi_sample_sql, 0).await.unwrap_or_default();
        println!("   tubi_relate 表样本记录:");
        for (i, record) in tubi_samples.iter().enumerate() {
            if let Some(id) = record.get("id") {
                if let Some(in_field) = record.get("in") {
                    println!("     [{}] id: {}, in: {:?}", i + 1, id, in_field);
                }
            }
        }
    }

    // 5. 尝试直接查询用户提供的记录
    println!("\n5. 尝试直接查询用户提供的记录...");
    let test_refno = RefnoEnum::from("pe:21491_10000");
    println!("   测试查询: {:?}", test_refno);

    // 5.1 检查 pe 表中是否有这个记录
    let pe_check_sql = "SELECT id, owner.noun FROM pe WHERE id = $refno";
    let pe_check: Vec<serde_json::Value> = SUL_DB.query_take(pe_check_sql, 0).await.unwrap_or_default();
    println!("   pe 表中的记录:");
    for record in pe_check.iter() {
        println!("     {:?}", record);
    }

    // 5.2 检查 tubi_relate 表中是否有相关记录
    let tubi_check_sql = "SELECT id, in, id[0].old_pe FROM tubi_relate WHERE id[0] = $refno";
    let tubi_check: Vec<serde_json::Value> = SUL_DB.query_take(tubi_check_sql, 0).await.unwrap_or_default();
    println!("   tubi_relate 表中的相关记录:");
    for record in tubi_check.iter() {
        println!("     {:?}", record);
    }

    // 6. 尝试不使用参数化查询
    println!("\n6. 尝试不使用参数化查询...");
    let direct_pe_sql = format!("SELECT id, owner.noun FROM pe WHERE id = 'pe:21491_10000'");
    let direct_pe: Vec<serde_json::Value> = SUL_DB.query_take(&direct_pe_sql, 0).await.unwrap_or_default();
    println!("   直接查询 pe 表:");
    for record in direct_pe.iter() {
        println!("     {:?}", record);
    }

    let direct_tubi_sql = "SELECT id, in, id[0].old_pe FROM tubi_relate WHERE id[0] = 'pe:21491_10000'";
    let direct_tubi: Vec<serde_json::Value> = SUL_DB.query_take(direct_tubi_sql, 0).await.unwrap_or_default();
    println!("   直接查询 tubi_relate 表:");
    for record in direct_tubi.iter() {
        println!("     {:?}", record);
    }

    // 7. 检查数据库配置
    println!("\n7. 检查数据库配置...");
    println!("   当前连接的数据库信息应该显示在初始化日志中");
    println!("   请确认我们连接的是包含数据的正确数据库");

    println!("\n=== 调试完成 ===");
    Ok(())
}