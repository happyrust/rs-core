//! Kuzu JSON Schema 生成演示
//!
//! 演示如何从 all_attr_info.json 直接生成 Kuzu 表结构

#[cfg(feature = "kuzu")]
use aios_core::rs_kuzu::json_schema::{
    generate_all_table_sqls, generate_noun_table_sql, load_attr_info_json,
};
use anyhow::Result;

#[cfg(feature = "kuzu")]
fn main() -> Result<()> {
    println!("=== Kuzu JSON Schema 生成演示 ===\n");

    // 1. 加载 all_attr_info.json
    println!("1. 加载 all_attr_info.json...");
    let attr_info = load_attr_info_json()?;
    println!("   ✓ 成功加载 {} 个 noun", attr_info.named_attr_info_map.len());

    // 2. 展示一些重要的 noun
    let important_nouns = ["ELBO", "PIPE", "EQUIPMENT", "SITE", "ZONE", "BRANCH"];

    println!("\n2. 重要 Noun 的属性信息:");
    for noun in &important_nouns {
        if let Some(attrs) = attr_info.named_attr_info_map.get(*noun) {
            println!("   - {}: {} 个属性", noun, attrs.len());

            // 展示前5个属性
            let mut attr_names: Vec<_> = attrs.keys().collect();
            attr_names.sort();
            for name in attr_names.iter().take(5) {
                if let Some(info) = attrs.get(*name) {
                    println!("      • {} (类型: {:?})", name, info.att_type);
                }
            }
        }
    }

    // 3. 生成 ELBO 表的 SQL
    println!("\n3. 生成 ELBO 表的 SQL:");
    if let Some(elbo_attrs) = attr_info.named_attr_info_map.get("ELBO") {
        let sql = generate_noun_table_sql("ELBO", elbo_attrs)?;
        println!("{}", sql);
    }

    // 4. 统计生成的表数量
    println!("\n4. 生成所有表的 SQL...");
    let all_sqls = generate_all_table_sqls()?;

    let node_tables = all_sqls.iter().filter(|sql| sql.contains("CREATE NODE TABLE")).count();
    let rel_tables = all_sqls.iter().filter(|sql| sql.contains("CREATE REL TABLE")).count();

    println!("   ✓ 生成 {} 条 SQL 语句", all_sqls.len());
    println!("     - 节点表: {} 个", node_tables);
    println!("     - 关系表: {} 个", rel_tables);

    // 5. 展示关键表结构
    println!("\n5. 关键表结构:");

    // PE 主表
    for sql in &all_sqls {
        if sql.contains("CREATE NODE TABLE IF NOT EXISTS PE") {
            println!("\n[PE 主表]");
            println!("{}", sql);
            break;
        }
    }

    // OWNS 关系表
    for sql in &all_sqls {
        if sql.contains("CREATE REL TABLE IF NOT EXISTS OWNS") {
            println!("\n[OWNS 层次关系]");
            println!("{}", sql);
            break;
        }
    }

    // TO_ELBO 关系表
    for sql in &all_sqls {
        if sql.contains("CREATE REL TABLE IF NOT EXISTS TO_ELBO") {
            println!("\n[TO_ELBO 关系]");
            println!("{}", sql);
            break;
        }
    }

    println!("\n=== 演示完成 ===");

    Ok(())
}

#[cfg(not(feature = "kuzu"))]
fn main() {
    println!("请使用 --features kuzu 编译");
}