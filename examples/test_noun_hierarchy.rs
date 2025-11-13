use aios_core::{init_surreal, query_noun_hierarchy, RefnoEnum};
use chrono::{Local, TimeZone};

/// query_noun_hierarchy 函数完整测试案例
/// 
/// 本测试程序演示了 query_noun_hierarchy 函数的所有使用方式：
/// 1. 基础查询：根据名词类型和名称过滤查询
/// 2. 指定父节点查询：查询特定父节点下的子节点
/// 3. 组合查询：指定父节点 + 名称过滤
/// 4. 多父节点查询：同时查询多个父节点下的子节点
/// 
/// 运行方式：
/// ```bash
/// cargo run --example test_noun_hierarchy
/// ```
/// 
/// 注意事项：
/// - 指定父节点查询时，父节点ID必须是真实存在的节点
/// - 可以通过先运行基础查询获取一些节点ID，然后用这些ID作为父节点进行测试
/// - 如果父节点不存在或没有子节点，查询会返回空结果

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化数据库连接
    init_surreal().await?;

    println!("=== query_noun_hierarchy 完整测试案例 ===\n");
    println!("本测试演示 query_noun_hierarchy 函数的各种使用方式\n");

    println!("=== 测试1: 基础查询 - 查询名称包含 'B1' 的 PIPE 类型记录 ===\n");

    // 查询名称包含 "B1" 的 PIPE 类型
    let result = query_noun_hierarchy("PIPE", Some("B1"), None).await;

    match result {
        Ok(items) => {
            println!("找到 {} 条匹配的记录:\n", items.len());

            if items.is_empty() {
                println!("未找到匹配的记录。");
            } else {
                for (i, item) in items.iter().enumerate() {
                    println!("记录 {}:", i + 1);
                    println!("  名称: {}", item.name);
                    println!("  ID: {:?}", item.id);
                    println!("  类型: {}", item.noun);
                    println!("  所有者名称: {:?}", item.owner_name);
                    println!("  所有者: {:?}", item.owner);

                    // 转换为本地时间
                    if let Some(dt) = &item.last_modified_date {
                        // 使用 timestamp_millis() 获取毫秒时间戳，然后转换为本地时间
                        let timestamp_ms = dt.timestamp_millis();
                        let local_time = Local.timestamp_millis_opt(timestamp_ms).unwrap();
                        println!("  最后修改日期: {}", local_time.format("%Y-%m-%d %H:%M:%S"));
                    } else {
                        println!("  最后修改日期: 无");
                    }

                    println!();
                }
            }
        }
        Err(e) => {
            eprintln!("查询失败: {}", e);
            return Err(e);
        }
    }

    println!("=== 测试2: 指定父节点查询 - 查询特定父节点下的子节点 ===\n");

    // 获取一个真实的父节点ID用于测试
    // 在实际使用中，你可以：
    // 1. 先运行基础查询获取一些节点ID
    // 2. 使用已知的设备或区域ID作为父节点
    // 3. 从其他查询结果中获取父节点ID
    
    // 这里我们使用一个示例父节点ID（实际使用时请替换为真实存在的ID）
    let parent_refno = RefnoEnum::from("21491_18944"); // 示例：使用测试1中出现的所有者ID
    println!("🔍 查询父节点 {:} 下的所有 PIPE 类型记录...\n", parent_refno);
    println!("💡 提示：如果返回空结果，说明该父节点下没有 PIPE 类型的子节点\n");

    let result_with_parent = query_noun_hierarchy("PIPE", None, Some(vec![parent_refno])).await;

    match result_with_parent {
        Ok(items) => {
            println!("找到 {} 条匹配的记录:\n", items.len());

            if items.is_empty() {
                println!("未找到该父节点下的 PIPE 记录。");
            } else {
                for (i, item) in items.iter().enumerate() {
                    println!("记录 {}:", i + 1);
                    println!("  名称: {}", item.name);
                    println!("  ID: {:?}", item.id);
                    println!("  类型: {}", item.noun);
                    println!("  所有者名称: {:?}", item.owner_name);
                    println!("  所有者: {:?}", item.owner);

                    // 转换为本地时间
                    if let Some(dt) = &item.last_modified_date {
                        let timestamp_ms = dt.timestamp_millis();
                        let local_time = Local.timestamp_millis_opt(timestamp_ms).unwrap();
                        println!("  最后修改日期: {}", local_time.format("%Y-%m-%d %H:%M:%S"));
                    } else {
                        println!("  最后修改日期: 无");
                    }

                    println!();
                }
            }
        }
        Err(e) => {
            eprintln!("指定父节点查询失败: {}", e);
            println!("注意：这可能是因为父节点 ID 不存在或没有权限访问");
        }
    }

    println!("=== 测试3: 组合查询 - 指定父节点 + 名称过滤 ===\n");

    // 测试指定父节点 + 名称过滤的组合查询
    // 这种查询方式适用于：
    // 1. 在特定设备或区域内查找特定名称的组件
    // 2. 精确定位某个父节点下的特定子节点
    // 3. 减少查询结果范围，提高查询效率
    
    let parent_refno2 = RefnoEnum::from("21491_18944"); // 使用相同的父节点
    println!("🔍 查询父节点 {:} 下名称包含 'B1' 的记录...\n", parent_refno2);
    println!("💡 提示：这是最常用的查询方式之一，结合了层级关系和名称匹配\n");

    let result_with_filter = query_noun_hierarchy("PIPE", Some("火车"), Some(vec![parent_refno2])).await;

    match result_with_filter {
        Ok(items) => {
            println!("找到 {} 条匹配的记录:\n", items.len());

            if items.is_empty() {
                println!("未找到该父节点下名称包含 'B1' 的记录。");
            } else {
                for (i, item) in items.iter().enumerate() {
                    println!("记录 {}:", i + 1);
                    println!("  名称: {}", item.name);
                    println!("  ID: {:?}", item.id);
                    println!("  类型: {}", item.noun);
                    println!();
                }
            }
        }
        Err(e) => {
            eprintln!("组合查询失败: {}", e);
        }
    }

    println!("=== 测试4: 多父节点查询 - 同时查询多个父节点下的子节点 ===\n");

    // 测试多个父节点查询
    // 这种查询方式适用于：
    // 1. 批量查询多个设备或区域下的特定类型组件
    // 2. 汇总多个父节点的子节点信息
    // 3. 提高查询效率，避免多次单独查询
    
    let parent_refnos = vec![
        RefnoEnum::from("21900/1040"),  // 第一个父节点
        RefnoEnum::from("30101/21"),    // 第二个父节点
    ];
    println!("🔍 查询多个父节点下的所有 EQUIPMENT 记录...\n");
    println!("💡 提示：这种方式可以一次性获取多个父节点的子节点，提高查询效率\n");

    let result_multi_parent = query_noun_hierarchy("EQUIPMENT", None, Some(parent_refnos)).await;

    match result_multi_parent {
        Ok(items) => {
            println!("找到 {} 条匹配的记录:\n", items.len());

            if items.is_empty() {
                println!("未找到这些父节点下的 EQUIPMENT 记录。");
            } else {
                for (i, item) in items.iter().enumerate() {
                    println!("记录 {}:", i + 1);
                    println!("  名称: {}", item.name);
                    println!("  ID: {:?}", item.id);
                    println!("  类型: {}", item.noun);
                    println!("  所有者: {:?}", item.owner);
                    println!();
                }
            }
        }
        Err(e) => {
            eprintln!("多父节点查询失败: {}", e);
        }
    }

    println!("=== 测试总结 ===\n");
    println!("✅ query_noun_hierarchy 函数测试完成！");
    println!("\n📋 功能回顾：");
    println!("1. 基础查询：根据名词类型和名称过滤查询");
    println!("2. 指定父节点查询：查询特定父节点下的子节点");
    println!("3. 组合查询：指定父节点 + 名称过滤");
    println!("4. 多父节点查询：同时查询多个父节点下的子节点");
    
    println!("\n💡 使用建议：");
    println!("- 在实际使用时，请确保父节点ID真实存在");
    println!("- 可以通过基础查询先获取一些节点ID，然后作为父节点进行测试");
    println!("- 多父节点查询可以显著提高批量查询的效率");
    println!("- 名称过滤支持模糊匹配，不区分大小写");
    
    println!("\n🔧 调试提示：");
    println!("- 如果查询返回空结果，检查父节点ID是否存在");
    println!("- 查看控制台输出的SQL语句，了解实际的查询逻辑");
    println!("- 使用基础查询验证数据库连接和权限是否正常");

    Ok(())
}
