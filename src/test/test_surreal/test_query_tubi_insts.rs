use crate::rs_surreal::inst::{query_tubi_insts_by_brans, TubiInstQuery};
use crate::{RefnoEnum, SUL_DB, SurrealQueryExt, init_surreal};

use super::test_helpers::init_sul_db_with_memory;

/// 测试 query_tubi_insts_by_brans 函数在内存数据库上的基本功能
#[tokio::test]
async fn test_query_tubi_insts_by_brans_basic() -> anyhow::Result<()> {
    // 使用内存数据库初始化
    init_sul_db_with_memory().await?;

    // 创建测试数据
    let setup_sql = r#"
        -- 创建几何体记录
        CREATE geo:demo_geo1 CONTENT {
            visible: true,
            meshed: true,
            geo_type: 'Pos'
        };

        CREATE geo:demo_geo2 CONTENT {
            visible: true,
            meshed: true,
            geo_type: 'Pos'
        };

        -- 创建 tubi_relate 记录
        CREATE tubi_relate:[21491, 0] CONTENT {
            id: ["pe:21491_10000", "pe:21491_10001"],
            in: "pe:21491_10001",
            aabb: {
                d: {
                    mins: [0.0, 0.0, 0.0],
                    maxs: [1.0, 1.0, 1.0]
                }
            },
            world_trans: {
                d: {
                    translation: [0.0, 0.0, 0.0],
                    rotation: [0.0, 0.0, 0.0, 1.0],
                    scale: [1.0, 1.0, 1.0]
                }
            },
            geo: geo:demo_geo1
        };

        CREATE tubi_relate:[21491, 1] CONTENT {
            id: ["pe:21491_10000", "pe:21491_10002"],
            in: "pe:21491_10002",
            aabb: {
                d: {
                    mins: [1.0, 1.0, 1.0],
                    maxs: [2.0, 2.0, 2.0]
                }
            },
            world_trans: {
                d: {
                    translation: [1.0, 1.0, 1.0],
                    rotation: [0.0, 0.0, 0.0, 1.0],
                    scale: [1.0, 1.0, 1.0]
                }
            },
            geo: geo:demo_geo2
        };

        -- 创建没有 aabb 的记录（应该被过滤掉）
        CREATE tubi_relate:[21491, 2] CONTENT {
            id: ["pe:21491_10000", "pe:21491_10003"],
            in: "pe:21491_10003",
            aabb: NONE,
            world_trans: {
                d: {
                    translation: [2.0, 2.0, 2.0],
                    rotation: [0.0, 0.0, 0.0, 1.0],
                    scale: [1.0, 1.0, 1.0]
                }
            },
            geo: geo:demo_geo1
        };

        -- 创建 pe 表记录以支持关联查询
        CREATE pe:21491_10000 CONTENT {
            id: "pe:21491_10000",
            owner: { noun: "BRAN" },
            old_pe: "pe:21491_9999",
            dt: time::now()
        };

        CREATE pe:21491_10001 CONTENT {
            id: "pe:21491_10001",
            owner: { noun: "TUBI" },
            dt: time::now()
        };

        CREATE pe:21491_10002 CONTENT {
            id: "pe:21491_10002",
            owner: { noun: "TUBI" },
            dt: time::now()
        };

        CREATE pe:21491_10003 CONTENT {
            id: "pe:21491_10003",
            owner: { noun: "TUBI" },
            dt: time::now()
        };
    "#;

    SUL_DB.query_response(setup_sql).await?;

    // 测试查询 pe:21491_10000
    let test_refno = RefnoEnum::from("pe:21491_10000");
    let results = query_tubi_insts_by_brans(&[test_refno]).await?;

    // 验证结果
    assert_eq!(results.len(), 2, "应该返回2条记录（过滤掉没有aabb的记录）");

    // 验证第一条记录
    let first = &results[0];
    assert_eq!(first.refno.to_string(), "pe:21491_10000");
    assert_eq!(first.leave.to_string(), "pe:21491_10001");
    assert_eq!(first.old_refno.unwrap().to_string(), "pe:21491_9999");
    assert_eq!(first.generic.as_ref().unwrap(), "BRAN");
    assert_eq!(first.geo_hash, "geo:demo_geo1");

    // 验证第二条记录
    let second = &results[1];
    assert_eq!(second.refno.to_string(), "pe:21491_10000");
    assert_eq!(second.leave.to_string(), "pe:21491_10002");
    assert_eq!(second.old_refno.unwrap().to_string(), "pe:21491_9999");
    assert_eq!(second.generic.as_ref().unwrap(), "BRAN");
    assert_eq!(second.geo_hash, "geo:demo_geo2");

    println!("✓ 基本功能测试通过，返回 {} 条记录", results.len());
    for (i, result) in results.iter().enumerate() {
        println!("  [{}] refno: {}, leave: {}, geo_hash: {}", 
                 i, result.refno, result.leave, result.geo_hash);
    }

    Ok(())
}

/// 测试空输入数组的情况
#[tokio::test]
async fn test_query_tubi_insts_by_brans_empty_input() -> anyhow::Result<()> {
    init_sul_db_with_memory().await?;

    let results = query_tubi_insts_by_brans(&[]).await?;
    assert_eq!(results.len(), 0, "空输入应该返回空结果");

    println!("✓ 空输入测试通过");
    Ok(())
}

/// 测试多个分支构件编号的查询
#[tokio::test]
async fn test_query_tubi_insts_by_brans_multiple_brans() -> anyhow::Result<()> {
    init_sul_db_with_memory().await?;

    // 创建测试数据
    let setup_sql = r#"
        -- 创建几何体记录
        CREATE geo:demo_geo3 CONTENT {
            visible: true,
            meshed: true,
            geo_type: 'Pos'
        };

        -- 为第一个分支创建记录
        CREATE tubi_relate:[21491, 10] CONTENT {
            id: ["pe:21491_20000", "pe:21491_20001"],
            in: "pe:21491_20001",
            aabb: {
                d: {
                    mins: [0.0, 0.0, 0.0],
                    maxs: [1.0, 1.0, 1.0]
                }
            },
            world_trans: {
                d: {
                    translation: [0.0, 0.0, 0.0],
                    rotation: [0.0, 0.0, 0.0, 1.0],
                    scale: [1.0, 1.0, 1.0]
                }
            },
            geo: geo:demo_geo3
        };

        -- 为第二个分支创建记录
        CREATE tubi_relate:[21492, 0] CONTENT {
            id: ["pe:21492_10000", "pe:21492_10001"],
            in: "pe:21492_10001",
            aabb: {
                d: {
                    mins: [2.0, 2.0, 2.0],
                    maxs: [3.0, 3.0, 3.0]
                }
            },
            world_trans: {
                d: {
                    translation: [2.0, 2.0, 2.0],
                    rotation: [0.0, 0.0, 0.0, 1.0],
                    scale: [1.0, 1.0, 1.0]
                }
            },
            geo: geo:demo_geo3
        };

        -- 创建 pe 表记录
        CREATE pe:21491_20000 CONTENT {
            id: "pe:21491_20000",
            owner: { noun: "BRAN" },
            dt: time::now()
        };

        CREATE pe:21492_10000 CONTENT {
            id: "pe:21492_10000",
            owner: { noun: "BRAN" },
            dt: time::now()
        };
    "#;

    SUL_DB.query_response(setup_sql).await?;

    // 测试查询多个分支
    let refno1 = RefnoEnum::from("pe:21491_20000");
    let refno2 = RefnoEnum::from("pe:21492_10000");
    let results = query_tubi_insts_by_brans(&[refno1, refno2]).await?;

    assert_eq!(results.len(), 2, "应该返回2条记录，每个分支一条");

    println!("✓ 多分支查询测试通过，返回 {} 条记录", results.len());
    for (i, result) in results.iter().enumerate() {
        println!("  [{}] refno: {}, leave: {}", i, result.refno, result.leave);
    }

    Ok(())
}

/// 测试查询不存在的分支
#[tokio::test]
async fn test_query_tubi_insts_by_brans_nonexistent_bran() -> anyhow::Result<()> {
    init_sul_db_with_memory().await?;

    let test_refno = RefnoEnum::from("pe:99999_99999");
    let results = query_tubi_insts_by_brans(&[test_refno]).await?;
    
    assert_eq!(results.len(), 0, "不存在的分支应该返回空结果");

    println!("✓ 不存在分支测试通过");
    Ok(())
}

/// 测试真实数据库连接（需要真实数据）
#[tokio::test]
#[ignore] // 需要真实数据库连接，使用 cargo test test_query_tubi_insts_by_brans_real --lib -- --ignored --test-threads=1 --nocapture 运行
async fn test_query_tubi_insts_by_brans_real() -> anyhow::Result<()> {
    println!("\n=== 测试 query_tubi_insts_by_brans 函数（真实数据库） ===\n");

    // 初始化数据库连接
    println!("初始化数据库连接...");
    crate::init_surreal().await?;
    println!("✓ 数据库连接成功\n");

    // 测试 pe:21491_10000 的查询
    let test_refno = RefnoEnum::from("pe:21491_10000");
    println!("测试查询: {:?}\n", test_refno);

    let results = query_tubi_insts_by_brans(&[test_refno]).await?;

    println!("找到 {} 条 Tubi 实例记录\n", results.len());

    if !results.is_empty() {
        println!("Tubi 实例记录详情:");
        for (i, result) in results.iter().enumerate() {
            println!("  [{}] refno: {:?}", i + 1, result.refno);
            println!("      leave: {:?}", result.leave);
            if let Some(old_refno) = &result.old_refno {
                println!("      old_refno: {:?}", old_refno);
            }
            if let Some(generic) = &result.generic {
                println!("      generic: {}", generic);
            }
            println!("      world_aabb: {:?}", result.world_aabb);
            println!("      geo_hash: {}", result.geo_hash);
            if let Some(date) = &result.date {
                println!("      date: {}", date);
            }
            println!();
        }
    } else {
        println!("⚠️ 未找到 Tubi 实例记录\n");
    }

    println!("=== 测试完成 ===");
    Ok(())
}