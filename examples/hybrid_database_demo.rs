//! 混合数据库使用示例
//!
//! 展示如何使用 HybridDatabaseManager 协调 SurrealDB 和 Kuzu

use aios_core::db_adapter::*;
use aios_core::types::*;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    simplelog::TermLogger::init(
        simplelog::LevelFilter::Info,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )
    .unwrap();

    println!("🚀 混合数据库示例\n");

    // 1. 创建 SurrealDB 适配器
    println!("📦 Step 1: 创建 SurrealDB 适配器");
    let surreal_adapter = Arc::new(SurrealAdapter::new());
    println!("   ✅ SurrealDB 适配器: {}", surreal_adapter.name());
    println!("   能力: {:?}\n", surreal_adapter.capabilities());

    // 2. 创建 Kuzu 适配器（如果启用）
    #[cfg(feature = "kuzu")]
    let kuzu_adapter = {
        println!("📦 Step 2: 创建 Kuzu 适配器");
        let adapter = Arc::new(KuzuAdapter::new());
        println!("   ✅ Kuzu 适配器: {}", adapter.name());
        println!("   能力: {:?}\n", adapter.capabilities());
        Some(adapter)
    };

    #[cfg(not(feature = "kuzu"))]
    let kuzu_adapter: Option<Arc<dyn DatabaseAdapter>> = None;

    // 3. 创建混合数据库管理器
    println!("🔧 Step 3: 创建混合数据库管理器");
    let config = HybridConfig {
        mode: HybridMode::DualKuzuPreferred,
        query_timeout_ms: 5000,
        fallback_on_error: true,
        enable_cache: true,
        cache_ttl_secs: 300,
    };

    let manager = HybridDatabaseManager::new(
        surreal_adapter,
        kuzu_adapter.clone(),
        config.clone(),
    );

    println!("   ✅ 混合管理器: {}", manager.name());
    println!("   模式: {:?}", config.mode);
    println!("   能力: {:?}\n", manager.capabilities());

    // 4. 健康检查
    println!("🏥 Step 4: 健康检查");
    match manager.health_check().await {
        Ok(healthy) => {
            if healthy {
                println!("   ✅ 数据库健康\n");
            } else {
                println!("   ⚠️  数据库不健康\n");
            }
        }
        Err(e) => {
            println!("   ❌ 健康检查失败: {}\n", e);
        }
    }

    // 5. 演示查询路由
    println!("🔄 Step 5: 查询路由演示");
    demonstrate_routing(&manager).await?;

    // 6. 演示不同模式
    println!("\n🎭 Step 6: 不同模式演示");
    demonstrate_modes(
        surreal_adapter.name(),
        kuzu_adapter.as_ref().map(|a| a.name()),
    );

    println!("\n✨ 示例完成！");

    Ok(())
}

async fn demonstrate_routing(
    manager: &HybridDatabaseManager,
) -> anyhow::Result<()> {
    println!("   查询路由决策：");

    // 创建不同的查询上下文
    let simple_ctx = QueryContext {
        requires_graph_traversal: false,
        ..Default::default()
    };

    let graph_ctx = QueryContext {
        requires_graph_traversal: true,
        ..Default::default()
    };

    println!("   - 简单查询 → 根据模式选择数据库");
    println!("   - 图遍历查询 → 优先 Kuzu（如果可用）");
    println!("   - 写入操作 → 根据模式进行单写或双写");

    Ok(())
}

fn demonstrate_modes(
    primary_name: &str,
    secondary_name: Option<&str>,
) {
    println!("\n   可用的混合模式：");
    println!("   1. SurrealPrimary - {} 为主", primary_name);
    if let Some(name) = secondary_name {
        println!("   2. KuzuPrimary - {} 为主", name);
        println!("   3. DualSurrealPreferred - 双写，优先 {}", primary_name);
        println!("   4. DualKuzuPreferred - 双写，优先 {} (推荐)", name);
        println!("   5. WriteToSurrealReadFromKuzu - 写 {}，读 {}", primary_name, name);
    }
}