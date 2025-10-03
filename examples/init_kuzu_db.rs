//! 初始化 Kuzu 数据库并创建 schema
//!
//! 运行此程序来初始化 Kuzu 数据库并创建所有表结构

#[cfg(feature = "kuzu")]
use aios_core::rs_kuzu::{
    init_kuzu_schema, create_kuzu_connection,
    drop_all_tables, validate_schema, list_tables, init_kuzu
};
use anyhow::Result;

#[cfg(feature = "kuzu")]
#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    // env_logger::init();

    println!("=== 初始化 Kuzu 数据库 ===\n");

    // 1. 初始化数据库
    let db_path = "./data/kuzu_db";
    println!("1. 初始化 Kuzu 数据库: {}", db_path);

    use kuzu::SystemConfig;
    let config = SystemConfig::default();
    init_kuzu(db_path, config).await?;
    println!("   ✓ 数据库初始化成功\n");

    // 2. 创建连接测试
    println!("2. 测试数据库连接...");
    let conn = create_kuzu_connection()?;
    println!("   ✓ 连接成功\n");

    // 3. 列出现有表（如果有）
    println!("3. 检查现有表...");
    match list_tables().await {
        Ok(tables) => {
            if tables.is_empty() {
                println!("   数据库中暂无表");
            } else {
                println!("   发现 {} 个表:", tables.len());
                for table in tables.iter().take(10) {
                    println!("     - {}", table);
                }
                if tables.len() > 10 {
                    println!("     ... 还有 {} 个表", tables.len() - 10);
                }
            }
        }
        Err(e) => {
            println!("   无法列出表: {}", e);
        }
    }

    // 4. 询问是否重新初始化
    println!("\n4. 初始化 Schema");
    println!("   将从 all_attr_info.json 生成所有表结构");

    // 可选：删除所有表重新创建
    // println!("   清理旧表...");
    // drop_all_tables().await?;
    // println!("   ✓ 旧表已清理\n");

    println!("   创建新 Schema...");
    match init_kuzu_schema().await {
        Ok(_) => {
            println!("   ✓ Schema 创建成功\n");
        }
        Err(e) => {
            println!("   ✗ Schema 创建失败: {}", e);
            return Err(e);
        }
    }

    // 5. 验证 schema
    println!("5. 验证 Schema...");
    match validate_schema().await {
        Ok(_) => {
            println!("   ✓ Schema 验证通过\n");
        }
        Err(e) => {
            println!("   ⚠ Schema 验证失败: {}", e);
        }
    }

    // 6. 查询特定表信息
    println!("6. 查询 ELBO 相关表:");

    // 查询 Attr_ELBO 表
    match conn.query("MATCH (e:Attr_ELBO) RETURN COUNT(*) as count;") {
        Ok(mut result) => {
            if let Some(row) = result.next() {
                if let Some(count) = row.get(0) {
                    println!("   - Attr_ELBO 表存在，包含 {} 条记录", count.to_string());
                }
            }
        }
        Err(e) => {
            if e.to_string().contains("does not exist") {
                println!("   - Attr_ELBO 表不存在");
            } else {
                println!("   - 查询 Attr_ELBO 出错: {}", e);
            }
        }
    }

    // 查询 TO_ELBO 关系
    match conn.query("CALL table_info() WHERE name = 'TO_ELBO' RETURN *;") {
        Ok(mut result) => {
            if result.next().is_some() {
                println!("   - TO_ELBO 关系表存在");
            } else {
                println!("   - TO_ELBO 关系表不存在");
            }
        }
        Err(e) => {
            println!("   - 查询 TO_ELBO 出错: {}", e);
        }
    }

    // 7. 列出所有创建的表
    println!("\n7. 所有创建的表:");
    match list_tables().await {
        Ok(tables) => {
            println!("   共 {} 个表:", tables.len());

            // 统计不同类型的表
            let pe_table = tables.iter().filter(|t| *t == "PE").count();
            let attr_tables = tables.iter().filter(|t| t.starts_with("Attr_")).count();
            let to_tables = tables.iter().filter(|t| t.starts_with("TO_")).count();
            let other_tables = tables.len() - pe_table - attr_tables - to_tables;

            println!("   - PE 主表: {}", pe_table);
            println!("   - Attr_* 属性表: {}", attr_tables);
            println!("   - TO_* 关系表: {}", to_tables);
            println!("   - 其他表: {}", other_tables);

            // 显示一些 ELBO 相关的表
            println!("\n   ELBO 相关表:");
            for table in &tables {
                if table.contains("ELBO") {
                    println!("     - {}", table);
                }
            }
        }
        Err(e) => {
            println!("   列出表失败: {}", e);
        }
    }

    println!("\n=== 初始化完成 ===");
    println!("\n您现在可以使用以下命令连接数据库:");
    println!("  kuzu {}", db_path);
    println!("\n在 Kuzu CLI 中查询 ELBO 表:");
    println!("  kuzu> CALL table_info() RETURN *;");
    println!("  kuzu> MATCH (e:Attr_ELBO) RETURN e LIMIT 5;");

    Ok(())
}

#[cfg(not(feature = "kuzu"))]
fn main() {
    println!("请使用 --features kuzu 编译");
    println!("cargo run --example init_kuzu_db --features kuzu");
}