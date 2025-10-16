use aios_core::{SUL_DB, SurlValue, init_surreal};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== 测试 fn::visible_geo_descendants 函数 ===\n");

    // 1. 初始化数据库连接（会自动加载 common.surql）
    println!("1. 正在初始化数据库连接...");
    init_surreal().await?;
    println!("   ✓ 数据库连接成功，函数定义已加载\n");

    // 2. 先查看 pe:17496_171099 的基本信息
    println!("2. 查看 pe:17496_171099 的基本信息...");
    let query = "SELECT * FROM pe:17496_171099 LIMIT 1;";
    let mut response = SUL_DB.query(query).await?;
    let pe_info: Vec<SurlValue> = response.take(0)?;
    println!("   PE信息: {:?}\n", pe_info);

    // 3. 查询直接子节点
    println!("3. 查询直接子节点...");
    let query = "SELECT * FROM pe:17496_171099.children;";
    let mut response = SUL_DB.query(query).await?;
    let children: Vec<SurlValue> = response.take(0)?;
    println!("   找到 {} 个直接子节点", children.len());
    for (i, child) in children.iter().take(5).enumerate() {
        println!("   [{}] {:?}", i + 1, child);
    }

    // 4. 使用 @.{..+collect}(.children) 递归收集所有后代节点（不包含自身）
    println!("\n4. 使用 @.{{..+collect}}(.children) 递归收集所有后代节点（不包含自身）...");
    let query = r#"
        (SELECT VALUE @.{..+collect}(.children).{id, noun} FROM ONLY pe:17496_171099 LIMIT 1) ?: []
    "#;

    let mut response = SUL_DB.query(query).await?;
    let all_descendants: Vec<SurlValue> = response.take(0)?;
    println!("   找到 {} 个后代节点", all_descendants.len());

    // 显示前几个节点
    for (i, node) in all_descendants.iter().take(10).enumerate() {
        println!("   [{}] {:?}", i + 1, node);
    }

    // 4b. 使用 +inclusive 包含根节点本身
    println!(
        "\n4b. 使用 @.{{..+collect+inclusive}}(.children) 递归收集所有后代节点（包含自身）..."
    );
    let query_inclusive = r#"
        (SELECT VALUE @.{..+collect+inclusive}(.children).{id, noun} FROM ONLY pe:17496_171099 LIMIT 1) ?: []
    "#;

    let mut response = SUL_DB.query(query_inclusive).await?;
    let all_with_self: Vec<SurlValue> = response.take(0)?;
    println!("   找到 {} 个节点（包含根节点）", all_with_self.len());

    // 显示前几个节点
    for (i, node) in all_with_self.iter().take(10).enumerate() {
        println!("   [{}] {:?}", i + 1, node);
    }

    // 5. 过滤出可见几何类型
    println!("\n5. 过滤可见几何类型...");
    let visible_types = vec![
        "BOX", "CYLI", "SLCY", "CONE", "DISH", "CTOR", "RTOR", "PYRA", "SNOU", "POHE", "POLYHE",
        "EXTR", "REVO", "FLOOR", "PANE", "ELCONN", "CMPF", "WALL", "GWALL", "SJOI", "FITT", "PFIT",
        "FIXING", "PJOI", "GENSEC", "RNODE", "PRTELE", "GPART", "SCREED", "PALJ", "CABLE", "BATT",
        "CMFI", "SCOJ", "SEVE", "SBFI", "STWALL", "SCTN", "NOZZ",
    ];

    let visible_nodes: Vec<_> = all_descendants
        .iter()
        .filter(|node| {
            if let SurlValue::Object(obj) = node {
                if let Some(SurlValue::String(noun)) = obj.get("noun") {
                    return visible_types.contains(&noun.as_str());
                }
            }
            false
        })
        .collect();

    println!("   找到 {} 个可见几何节点", visible_nodes.len());

    // 显示所有可见几何节点的详细信息
    for (i, node) in visible_nodes.iter().enumerate() {
        if let SurlValue::Object(obj) = node {
            let id = obj.get("id");
            let noun = obj
                .get("noun")
                .and_then(|v| {
                    if let SurlValue::String(s) = v {
                        Some(s.as_str())
                    } else {
                        None
                    }
                })
                .unwrap_or("Unknown");
            println!("   [{}] ID: {:?}, 类型: {}", i + 1, id, noun);
        }
    }

    // 6. 显示结果统计
    println!("\n6. 结果统计:");
    println!("   总计: {} 个可见几何节点", visible_nodes.len());

    // 按类型分组统计
    use std::collections::HashMap;
    let mut type_counts: HashMap<String, usize> = HashMap::new();
    for node in &visible_nodes {
        if let SurlValue::Object(obj) = node {
            if let Some(SurlValue::String(noun)) = obj.get("noun") {
                *type_counts.entry(noun.clone()).or_insert(0) += 1;
            }
        }
    }

    println!("\n   按类型分组:");
    let mut types: Vec<_> = type_counts.iter().collect();
    types.sort_by_key(|(_, count)| std::cmp::Reverse(**count));
    for (noun, count) in types.iter().take(10) {
        println!("     {}: {} 个", noun, count);
    }

    // 7. 测试修改后的 fn::visible_geo_descendants 函数
    println!("\n7. 测试修改后的 fn::visible_geo_descendants 函数...");

    // 7a. 不包含根节点
    println!("\n7a. 测试 fn::visible_geo_descendants(pe:17496_171099, false)...");
    let query = "SELECT * FROM fn::visible_geo_descendants(pe:17496_171099, false);";
    let mut response = SUL_DB.query(query).await?;
    let result_without_self: Vec<SurlValue> = response.take(0)?;
    println!(
        "   找到 {} 个可见几何节点（不包含根节点）",
        result_without_self.len()
    );
    for (i, node) in result_without_self.iter().enumerate() {
        println!("   [{}] {:?}", i + 1, node);
    }

    // 7b. 包含根节点
    println!("\n7b. 测试 fn::visible_geo_descendants(pe:17496_171099, true)...");
    let query = "SELECT * FROM fn::visible_geo_descendants(pe:17496_171099, true);";
    let mut response = SUL_DB.query(query).await?;
    let result_with_self: Vec<SurlValue> = response.take(0)?;
    println!(
        "   找到 {} 个可见几何节点（包含根节点）",
        result_with_self.len()
    );
    for (i, node) in result_with_self.iter().enumerate() {
        println!("   [{}] {:?}", i + 1, node);
    }

    println!("\n=== 测试完成 ===");
    Ok(())
}
