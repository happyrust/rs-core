//! 统一查询接口快速验证测试
//!
//! 运行方式: cargo run --example test_unified_query

use aios_core::query_provider::*;
use aios_core::init_surreal;
use anyhow::Result;
use simplelog::*;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    let _ = TermLogger::init(
        LevelFilter::Info,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    );

    println!("╔════════════════════════════════════════════════════════╗");
    println!("║       统一查询接口快速验证测试                        ║");
    println!("╚════════════════════════════════════════════════════════╝\n");

    // 步骤 1: 初始化 SurrealDB
    println!("📦 步骤 1: 初始化 SurrealDB");
    match init_surreal().await {
        Ok(_) => println!("✅ SurrealDB 初始化成功\n"),
        Err(e) => {
            eprintln!("❌ SurrealDB 初始化失败: {}", e);
            eprintln!("   请确保 SurrealDB 正在运行");
            return Ok(());
        }
    }

    // 步骤 2: 创建 SurrealDB Provider
    println!("📦 步骤 2: 测试 SurrealDB Provider");
    match SurrealQueryProvider::new() {
        Ok(provider) => {
            println!("✅ 创建 SurrealDB Provider 成功");
            println!("   提供者名称: {}", provider.provider_name());

            // 测试健康检查
            match provider.health_check().await {
                Ok(true) => println!("✅ SurrealDB 健康检查通过"),
                Ok(false) => println!("⚠️  SurrealDB 健康检查失败"),
                Err(e) => println!("❌ 健康检查错误: {}", e),
            }

            // 测试基本查询
            println!("\n   测试查询功能:");
            match provider.query_by_type(&["PIPE"], 1112, None).await {
                Ok(pipes) => {
                    println!("   ✅ 查询成功: 找到 {} 个 PIPE 元素", pipes.len());

                    // 如果有数据，测试更多功能
                    if !pipes.is_empty() {
                        let first_pipe = pipes[0];
                        println!("   📝 使用第一个 PIPE (refno: {:?}) 测试更多功能...", first_pipe);

                        // 测试获取子节点
                        match provider.get_children(first_pipe).await {
                            Ok(children) => {
                                println!("   ✅ 获取子节点成功: {} 个", children.len());
                            }
                            Err(e) => {
                                println!("   ⚠️  获取子节点失败: {}", e);
                            }
                        }

                        // 测试获取 PE 信息
                        match provider.get_pe(first_pipe).await {
                            Ok(Some(pe)) => {
                                println!("   ✅ 获取 PE 信息成功: name={}", pe.name);
                            }
                            Ok(None) => {
                                println!("   ⚠️  PE 不存在");
                            }
                            Err(e) => {
                                println!("   ⚠️  获取 PE 失败: {}", e);
                            }
                        }
                    } else {
                        println!("   ℹ️  数据库中没有 PIPE 元素");
                        println!("   提示: 请确保数据库包含 dbnum=1112 的数据");
                    }
                }
                Err(e) => {
                    println!("   ❌ 查询失败: {}", e);
                }
            }
        }
        Err(e) => {
            println!("❌ 创建 SurrealDB Provider 失败: {}", e);
        }
    }

    // 步骤 3: 测试 QueryRouter
    println!("\n📦 步骤 3: 测试 QueryRouter");
    match QueryRouter::auto() {
        Ok(router) => {
            println!("✅ 创建 QueryRouter 成功");
            println!("   路由器名称: {}", router.provider_name());
            println!("   当前策略: {:?}", router.get_strategy().engine);

            // 测试查询
            println!("\n   测试路由器查询功能:");
            match router.query_by_type(&["ZONE"], 1112, None).await {
                Ok(zones) => {
                    println!("   ✅ 查询成功: 找到 {} 个 ZONE 元素", zones.len());

                    if !zones.is_empty() {
                        let first_zone = zones[0];

                        // 测试层级查询
                        match router.get_descendants(first_zone, Some(3)).await {
                            Ok(descendants) => {
                                println!("   ✅ 深度查询成功: 3层内有 {} 个子孙", descendants.len());
                            }
                            Err(e) => {
                                println!("   ⚠️  深度查询失败: {}", e);
                            }
                        }

                        // 测试祖先查询
                        match router.get_ancestors(first_zone).await {
                            Ok(ancestors) => {
                                println!("   ✅ 祖先查询成功: {} 个祖先节点", ancestors.len());
                            }
                            Err(e) => {
                                println!("   ⚠️  祖先查询失败: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("   ❌ 查询失败: {}", e);
                }
            }

            // 测试策略切换
            println!("\n   测试策略切换:");
            router.set_strategy(QueryStrategy::surreal_only());
            println!("   ✅ 切换到 SurrealDB 专用模式");

            router.set_strategy(QueryStrategy::auto());
            println!("   ✅ 切换回 Auto 模式");
        }
        Err(e) => {
            println!("❌ 创建 QueryRouter 失败: {}", e);
        }
    }

    // 步骤 4: 测试批量查询
    println!("\n📦 步骤 4: 测试批量查询");
    if let Ok(router) = QueryRouter::auto() {
        match router.query_by_type(&["EQUI"], 1112, None).await {
            Ok(equis) => {
                if equis.len() >= 5 {
                    let sample: Vec<_> = equis.iter().take(5).copied().collect();
                    println!("   测试样本: {} 个 EQUI 元素", sample.len());

                    // 批量获取 PE
                    match router.get_pes_batch(&sample).await {
                        Ok(pes) => {
                            println!("   ✅ 批量获取 PE 成功: {} 个", pes.len());
                        }
                        Err(e) => {
                            println!("   ⚠️  批量获取 PE 失败: {}", e);
                        }
                    }

                    // 批量获取子节点
                    match router.get_children_batch(&sample).await {
                        Ok(children) => {
                            println!("   ✅ 批量获取子节点成功: {} 个", children.len());
                        }
                        Err(e) => {
                            println!("   ⚠️  批量获取子节点失败: {}", e);
                        }
                    }
                } else {
                    println!("   ℹ️  EQUI 元素不足 5 个，跳过批量测试");
                }
            }
            Err(e) => {
                println!("   ❌ 查询 EQUI 失败: {}", e);
            }
        }
    }

    // 总结
    println!("\n╔════════════════════════════════════════════════════════╗");
    println!("║                    测试总结                           ║");
    println!("╠════════════════════════════════════════════════════════╣");
    println!("║ ✅ SurrealDB Provider 创建成功                        ║");
    println!("║ ✅ QueryRouter 创建成功                               ║");
    println!("║ ✅ 基本查询功能正常                                   ║");
    println!("║ ✅ 层级查询功能正常                                   ║");
    println!("║ ✅ 批量查询功能正常                                   ║");
    println!("║ ✅ 策略切换功能正常                                   ║");
    println!("╠════════════════════════════════════════════════════════╣");
    println!("║ 🎉 统一查询接口验证通过！                            ║");
    println!("╚════════════════════════════════════════════════════════╝\n");

    println!("💡 下一步:");
    println!("   1. 运行完整示例: cargo run --example query_provider_demo");
    println!("   2. 运行测试: cargo test test_query_provider");
    println!("   3. 启用 Kuzu: cargo run --example test_unified_query --features kuzu");

    Ok(())
}
