//! 测试数据库连接和基本数据
use aios_core::{init_surreal, SUL_DB};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_surreal().await?;
    
    println!("=== 检查数据库 ===\n");
    
    // 1. 检查 pe 表
    println!("--- 1. pe 表记录数 ---");
    let sql = r#"SELECT count() FROM pe"#;
    let resp = SUL_DB.query(sql).await?;
    println!("{:?}\n", resp);
    
    // 2. 检查 inst_relate 表
    println!("--- 2. inst_relate 表记录数 ---");
    let sql = r#"SELECT count() FROM inst_relate"#;
    let resp = SUL_DB.query(sql).await?;
    println!("{:?}\n", resp);
    
    // 3. 检查 geo_relate 表
    println!("--- 3. geo_relate 表记录数 ---");
    let sql = r#"SELECT count() FROM geo_relate"#;
    let resp = SUL_DB.query(sql).await?;
    println!("{:?}\n", resp);
    
    // 4. 检查 inst_info 表
    println!("--- 4. inst_info 表记录数 ---");
    let sql = r#"SELECT count() FROM inst_info"#;
    let resp = SUL_DB.query(sql).await?;
    println!("{:?}\n", resp);
    
    // 5. 直接查看 inst_relate 表 (不带条件)
    println!("--- 5. inst_relate 原始数据 ---");
    let sql = r#"SELECT * FROM inst_relate LIMIT 2"#;
    let resp = SUL_DB.query(sql).await?;
    println!("{:?}\n", resp);

    Ok(())
}
