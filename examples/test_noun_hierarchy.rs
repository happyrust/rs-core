use aios_core::{init_surreal, query_noun_hierarchy};
use chrono::{Local, TimeZone};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化数据库连接
    init_surreal().await?;

    println!("查询名称包含 '107' 的 NOZZ 类型记录...\n");

    // 查询名称包含 "107" 的 NOZZ 类型
    let result = query_noun_hierarchy("NOZZ", Some("107")).await;

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

    Ok(())
}



