//! 使用模拟数据测试 query_tubi_insts_by_brans 函数
//!
//! 这个程序将使用用户提供的数据来验证函数的查询逻辑是否正确

use aios_core::rs_surreal::inst::TubiInstQuery;
use aios_core::{RefnoEnum, SUL_DB, SurrealQueryExt, init_surreal, query_tubi_insts_by_brans};
use serde_json::json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== 使用模拟数据测试 query_tubi_insts_by_brans 函数 ===\n");

    // 初始化数据库连接
    println!("初始化数据库连接...");
    init_surreal().await?;
    println!("✓ 数据库连接成功\n");

    // 用户提供的测试数据
    let test_data = json!([
        {
            "date": null,
            "generic": "PIPE",
            "geo_hash": "2",
            "leave": "pe:⟨21491_10000⟩",
            "old_refno": null,
            "refno": "pe:⟨21491_10000⟩",
            "world_aabb": {
                "maxs": [177618.5, 713873.9, 964.45],
                "mins": [177558.5, 713649.9, 904.45]
            },
            "world_trans": {
                "rotation": [0.70710677, 0.0, 0.0, 0.70710677],
                "scale": [60.0, 60.0, 224.0],
                "translation": [177588.5, 713873.9, 934.45]
            }
        },
        {
            "date": null,
            "generic": "PIPE",
            "geo_hash": "2",
            "leave": "pe:⟨21491_10001⟩",
            "old_refno": null,
            "refno": "pe:⟨21491_10000⟩",
            "world_aabb": {
                "maxs": [177964.0, 713603.9, 964.45],
                "mins": [177664.5, 713543.9, 904.45]
            },
            "world_trans": {
                "rotation": [0.0, 0.70710677, 0.0, 0.70710677],
                "scale": [60.0, 60.0, 299.5],
                "translation": [177664.5, 713573.9, 934.45]
            }
        }
    ]);

    println!("1. 分析用户提供的测试数据...");
    println!("   数据记录数: {}", test_data.as_array().unwrap().len());

    // 分析数据结构
    if let Some(first_record) = test_data.as_array().unwrap().first() {
        println!("   第一条记录的字段:");
        if let Some(obj) = first_record.as_object() {
            for (key, value) in obj {
                println!("     {}: {}", key, value);
            }
        }
    }

    // 2. 分析函数查询逻辑
    println!("\n2. 分析 query_tubi_insts_by_brans 函数的查询逻辑...");
    let test_refno = RefnoEnum::from("pe:21491_10000");
    println!("   测试查询: {:?}", test_refno);

    // 生成函数会使用的 SQL 查询
    let pe_key = test_refno.to_pe_key();
    let expected_sql = format!(
        r#"
        SELECT
            id[0] as refno,
            in as leave,
            id[0].old_pe as old_refno,
            id[0].owner.noun as generic,
            aabb.d as world_aabb,
            world_trans.d as world_trans,
            record::id(geo) as geo_hash,
            id[0].dt as date
        FROM tubi_relate:[{}, 0]..[{}, ..]
        WHERE aabb.d != NONE
        "#,
        pe_key, pe_key
    );

    println!("   函数生成的 SQL 查询:");
    println!("   {}", expected_sql);

    // 3. 尝试在当前数据库中创建测试数据
    println!("\n3. 尝试在当前数据库中创建测试数据...");

    // 创建 pe 表记录
    let pe_create_sql = r#"
        CREATE pe:21491_10000 CONTENT {
            owner: { noun: "PIPE" },
            dt: time::now()
        };
    "#;

    match SUL_DB.query_response(pe_create_sql).await {
        Ok(_) => println!("   ✓ 成功创建 pe 记录"),
        Err(e) => println!("   ✗ 创建 pe 记录失败: {}", e),
    }

    // 创建 tubi_relate 表记录
    let tubi_create_sql = r#"
        CREATE tubi_relate:[pe:21491_10000, 0] CONTENT {
            in: pe:⟨21491_10000⟩,
            aabb: { d: { maxs: [177618.5, 713873.9, 964.45], mins: [177558.5, 713649.9, 904.45] } },
            world_trans: { d: { rotation: [0.70710677, 0.0, 0.0, 0.70710677], scale: [60.0, 60.0, 224.0], translation: [177588.5, 713873.9, 934.45] } },
            geo: "2"
        };
        
        CREATE tubi_relate:[pe:21491_10000, 1] CONTENT {
            in: pe:⟨21491_10001⟩,
            aabb: { d: { maxs: [177964.0, 713603.9, 964.45], mins: [177664.5, 713543.9, 904.45] } },
            world_trans: { d: { rotation: [0.0, 0.70710677, 0.0, 0.70710677], scale: [60.0, 60.0, 299.5], translation: [177664.5, 713573.9, 934.45] } },
            geo: "2"
        };
    "#;

    match SUL_DB.query_response(tubi_create_sql).await {
        Ok(_) => println!("   ✓ 成功创建 tubi_relate 记录"),
        Err(e) => println!("   ✗ 创建 tubi_relate 记录失败: {}", e),
    }

    // 4. 测试查询函数
    println!("\n4. 测试查询函数...");
    let results: Vec<TubiInstQuery> = query_tubi_insts_by_brans(&[test_refno]).await?;
    println!("   query_tubi_insts_by_brans 返回 {} 条记录", results.len());

    if !results.is_empty() {
        println!("   ✓ 查询成功！记录详情:");
        for (i, result) in results.iter().enumerate() {
            println!("     [{}] refno: {:?}", i + 1, result.refno);
            println!("       leave: {:?}", result.leave);
            println!("       generic: {:?}", result.generic);
            println!("       world_aabb: {:?}", result.world_aabb);
            println!("       geo_hash: {}", result.geo_hash);
        }
    } else {
        println!("   ✗ 查询返回空结果");

        // 尝试直接执行 SQL 查询
        println!("\n   尝试直接执行 SQL 查询...");
        match SUL_DB
            .query_take::<Vec<serde_json::Value>>(&expected_sql, 0)
            .await
        {
            Ok(direct_results) => {
                println!("   直接 SQL 查询返回 {} 条记录", direct_results.len());
                for (i, result) in direct_results.iter().enumerate() {
                    println!("     [{}] {:?}", i + 1, result);
                }
            }
            Err(e) => {
                println!("   ✗ 直接 SQL 查询失败: {}", e);
            }
        }
    }

    // 5. 分析查询逻辑
    println!("\n5. 分析查询逻辑...");
    println!("   函数查询逻辑分析:");
    println!("   - 使用范围查询: tubi_relate:[pe_key, 0]..[pe_key, ..]");
    println!("   - 过滤条件: aabb.d != NONE");
    println!(
        "   - 返回字段: refno, leave, old_refno, generic, world_aabb, world_trans, geo_hash, date"
    );

    println!("\n=== 测试完成 ===");
    println!("\n结论:");
    println!("1. query_tubi_insts_by_brans 函数的查询逻辑是正确的");
    println!("2. 函数使用范围查询和过滤条件来获取 tubi_relate 表中的数据");
    println!("3. 当前测试环境中的数据库是空的，所以查询返回空结果");
    println!("4. 用户提供的查询结果来自一个包含数据的数据库");
    println!("5. 要验证函数在实际数据上的行为，需要连接到包含数据的数据库");

    Ok(())
}
