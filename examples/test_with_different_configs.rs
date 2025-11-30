//! 使用不同的数据库配置测试查询
//! 
//! 这个程序将尝试使用不同的配置文件来连接数据库并测试查询

use aios_core::{RefnoEnum, SUL_DB, SurrealQueryExt, query_tubi_insts_by_brans};
use aios_core::rs_surreal::inst::TubiInstQuery;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== 使用不同的数据库配置测试查询 ===\n");

    // 测试的 refno
    let test_refno = RefnoEnum::from("pe:21491_10000");
    println!("测试查询: {:?}\n", test_refno);

    // 测试列表：配置文件名和描述
    let test_configs = vec![
        ("DbOption.toml", "默认配置 - AvevaMarineSample"),
        ("DbOption_ABA.toml", "ABA 项目配置"),
        ("DbOption-slyk.toml", "YCYK-E3D 项目配置"),
    ];

    for (config_file, description) in test_configs {
        println!("测试配置文件: {} ({})", config_file, description);
        
        // 注意：这里我们不能直接更改配置文件，因为 init_surreal() 函数在启动时已经读取了配置
        // 但我们可以记录下应该使用哪个配置文件
        
        // 直接查询当前数据库
        let pe_count_sql = "SELECT COUNT() as count FROM pe";
        let pe_count: Vec<i64> = SUL_DB.query_take(pe_count_sql, 0).await.unwrap_or_default();
        println!("   pe 表记录数: {}", pe_count.get(0).unwrap_or(&0));
        
        let tubi_count_sql = "SELECT COUNT() as count FROM tubi_relate";
        let tubi_count: Vec<i64> = SUL_DB.query_take(tubi_count_sql, 0).await.unwrap_or_default();
        println!("   tubi_relate 表记录数: {}", tubi_count.get(0).unwrap_or(&0));
        
        // 如果有数据，尝试查询
        if pe_count.get(0).unwrap_or(&0) > &0 || tubi_count.get(0).unwrap_or(&0) > &0 {
            println!("   尝试查询 pe:21491_10000...");
            
            // 直接查询 pe 表
            let pe_check_sql = "SELECT id, owner.noun FROM pe WHERE id = 'pe:21491_10000'";
            let pe_check: Vec<serde_json::Value> = SUL_DB.query_take(pe_check_sql, 0).await.unwrap_or_default();
            println!("   pe 表中的记录数: {}", pe_check.len());
            
            // 直接查询 tubi_relate 表
            let tubi_check_sql = "SELECT id, in FROM tubi_relate WHERE id[0] = 'pe:21491_10000'";
            let tubi_check: Vec<serde_json::Value> = SUL_DB.query_take(tubi_check_sql, 0).await.unwrap_or_default();
            println!("   tubi_relate 表中的相关记录数: {}", tubi_check.len());
            
            // 使用原始函数查询
            let results: Vec<TubiInstQuery> = query_tubi_insts_by_brans(&[test_refno]).await?;
            println!("   query_tubi_insts_by_brans 返回 {} 条记录", results.len());
            
            if !results.is_empty() {
                println!("   ✓ 找到数据！这个配置包含我们需要的记录");
                println!("   第一个记录的详情:");
                if let Some(first) = results.first() {
                    println!("     refno: {:?}", first.refno);
                    println!("     leave: {:?}", first.leave);
                    println!("     generic: {:?}", first.generic);
                    println!("     world_aabb: {:?}", first.world_aabb);
                }
            }
        } else {
            println!("   ✗ 这个数据库是空的");
        }
        
        println!();
    }

    println!("=== 测试完成 ===");
    println!("\n注意：");
    println!("1. 要使用不同的配置文件，需要重新启动程序并指定不同的配置文件");
    println!("2. 当前程序使用的是 DbOption.toml 配置");
    println!("3. 如果要测试其他配置，请将对应的配置文件重命名为 DbOption.toml 或修改程序以使用不同的配置文件");
    
    Ok(())
}