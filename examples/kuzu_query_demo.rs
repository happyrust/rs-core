//! Kuzu 数据库查询演示
//!
//! 演示如何查询 Kuzu 数据库中的表信息和数据

#[cfg(feature = "kuzu")]
use aios_core::rs_kuzu::{create_kuzu_connection, init_kuzu_schema};
use anyhow::Result;

#[cfg(feature = "kuzu")]
fn main() -> Result<()> {
    println!("=== Kuzu 查询演示 ===\n");

    // 创建连接
    let conn = create_kuzu_connection()?;
    println!("✓ 成功连接到 Kuzu 数据库\n");

    // 1. 查询所有表信息
    println!("1. 查询数据库中的所有表:");
    println!("----------------------------------------");

    let mut result = conn.query("CALL table_info() RETURN *;")?;
    let mut table_count = 0;

    while let Some(row) = result.next() {
        if let (Some(name), Some(type_val)) = (row.get(0), row.get(1)) {
            println!("   表名: {:<30} 类型: {}",
                name.to_string(),
                type_val.to_string());
            table_count += 1;
        }
    }
    println!("   总计: {} 个表\n", table_count);

    // 2. 查询 ELBO 表的 schema
    println!("2. 查询 Attr_ELBO 表的结构:");
    println!("----------------------------------------");

    // 使用 SHOW 查询表结构
    match conn.query("CALL show_tables() RETURN *;") {
        Ok(mut result) => {
            while let Some(row) = result.next() {
                if let Some(name) = row.get(0) {
                    let name_str = name.to_string();
                    if name_str.contains("ELBO") {
                        println!("   找到 ELBO 相关表: {}", name_str);
                    }
                }
            }
        }
        Err(e) => {
            println!("   注意: show_tables 可能不可用: {}", e);
        }
    }

    // 3. 查询 ELBO 表的数据（如果有）
    println!("\n3. 查询 Attr_ELBO 表的数据:");
    println!("----------------------------------------");

    match conn.query("MATCH (e:Attr_ELBO) RETURN e LIMIT 5;") {
        Ok(mut result) => {
            let mut count = 0;
            while let Some(row) = result.next() {
                println!("   记录 {}: {:?}", count + 1, row.get(0));
                count += 1;
            }
            if count == 0 {
                println!("   表中暂无数据");
            } else {
                println!("   显示前 {} 条记录", count);
            }
        }
        Err(e) => {
            if e.to_string().contains("does not exist") {
                println!("   Attr_ELBO 表不存在，需要先运行 init_kuzu_schema");
            } else {
                println!("   查询错误: {}", e);
            }
        }
    }

    // 4. 查询 PE 主表信息
    println!("\n4. 查询 PE 主表:");
    println!("----------------------------------------");

    match conn.query("MATCH (p:PE) RETURN COUNT(*) as count;") {
        Ok(mut result) => {
            if let Some(row) = result.next() {
                if let Some(count) = row.get(0) {
                    println!("   PE 表中有 {} 条记录", count.to_string());
                }
            }
        }
        Err(e) => {
            if e.to_string().contains("does not exist") {
                println!("   PE 表不存在，需要先运行 init_kuzu_schema");
            } else {
                println!("   查询错误: {}", e);
            }
        }
    }

    // 5. 查询关系表
    println!("\n5. 查询关系表:");
    println!("----------------------------------------");

    // 查询 TO_ELBO 关系
    match conn.query("MATCH (p:PE)-[r:TO_ELBO]->(e:Attr_ELBO) RETURN COUNT(*) as count;") {
        Ok(mut result) => {
            if let Some(row) = result.next() {
                if let Some(count) = row.get(0) {
                    println!("   TO_ELBO 关系: {} 条", count.to_string());
                }
            }
        }
        Err(_) => {
            println!("   TO_ELBO 关系表不存在或无数据");
        }
    }

    // 查询 OWNS 关系
    match conn.query("MATCH (p1:PE)-[r:OWNS]->(p2:PE) RETURN COUNT(*) as count;") {
        Ok(mut result) => {
            if let Some(row) = result.next() {
                if let Some(count) = row.get(0) {
                    println!("   OWNS 层次关系: {} 条", count.to_string());
                }
            }
        }
        Err(_) => {
            println!("   OWNS 关系表不存在或无数据");
        }
    }

    // 6. 有用的 Kuzu 查询示例
    println!("\n6. 有用的 Kuzu 查询示例:");
    println!("----------------------------------------");
    println!("   // 查看所有表");
    println!("   CALL table_info() RETURN *;");
    println!();
    println!("   // 查询特定表的前10条数据");
    println!("   MATCH (n:Attr_ELBO) RETURN n LIMIT 10;");
    println!();
    println!("   // 查询表的记录数");
    println!("   MATCH (n:Attr_ELBO) RETURN COUNT(*);");
    println!();
    println!("   // 查询特定属性");
    println!("   MATCH (n:Attr_ELBO) RETURN n.refno, n.NAME, n.STATUS_CODE LIMIT 10;");
    println!();
    println!("   // 查询关联数据");
    println!("   MATCH (p:PE)-[:TO_ELBO]->(e:Attr_ELBO) ");
    println!("   WHERE p.noun = 'ELBO' ");
    println!("   RETURN p.refno, p.name, e.STATUS_CODE LIMIT 10;");

    println!("\n=== 演示完成 ===");

    Ok(())
}

#[cfg(not(feature = "kuzu"))]
fn main() {
    println!("请使用 --features kuzu 编译");
}