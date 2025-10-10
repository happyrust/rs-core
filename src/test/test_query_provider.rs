//! 查询提供者测试

#[cfg(test)]
mod tests {
    use crate::query_provider::*;
    use crate::init_surreal;
    use crate::RefnoEnum;

    /// 测试 SurrealDB 查询提供者
    #[tokio::test]
    #[ignore] // 需要数据库环境
    async fn test_surreal_provider() {
        init_surreal().await.unwrap();

        let provider = SurrealQueryProvider::new().unwrap();

        // 测试查询类型
        let pipes = provider
            .query_by_type(&["PIPE"], 1112, None)
            .await
            .unwrap();

        println!("找到 {} 个 PIPE 元素", pipes.len());
        assert!(!pipes.is_empty());

        // 测试获取子节点
        if let Some(&first_pipe) = pipes.first() {
            let children = provider.get_children(first_pipe).await.unwrap();
            println!("第一个 PIPE 有 {} 个子节点", children.len());
        }
    }

    /// 测试 Kuzu 查询提供者
    #[cfg(feature = "kuzu")]
    #[tokio::test]
    #[ignore] // 需要数据库环境
    async fn test_kuzu_provider() {
        use crate::rs_kuzu::{init_kuzu, init_kuzu_schema};
        use kuzu::SystemConfig;

        // 初始化 Kuzu
        let db_path = "./test_output/query_provider_test";
        init_kuzu(db_path, SystemConfig::default()).await.unwrap();
        init_kuzu_schema().await.unwrap();

        let provider = KuzuQueryProvider::new().unwrap();

        // 测试健康检查
        let health = provider.health_check().await.unwrap();
        assert!(health);

        println!("Kuzu 查询提供者测试通过");
    }

    /// 测试查询路由器 - SurrealDB 模式
    #[tokio::test]
    #[ignore]
    async fn test_router_surreal_mode() {
        init_surreal().await.unwrap();

        let router = QueryRouter::new(QueryStrategy::surreal_only()).unwrap();

        // 测试查询
        let pipes = router.query_by_type(&["PIPE"], 1112, None).await.unwrap();
        println!("[Router-SurrealDB] 找到 {} 个 PIPE", pipes.len());

        assert!(!pipes.is_empty());
    }

    /// 测试查询路由器 - Auto 模式
    #[tokio::test]
    #[ignore]
    async fn test_router_auto_mode() {
        init_surreal().await.unwrap();

        // 创建自动选择模式的路由器
        let router = QueryRouter::auto().unwrap();

        // 测试基本查询
        let zones = router.query_by_type(&["ZONE"], 1112, None).await.unwrap();
        println!("[Router-Auto] 找到 {} 个 ZONE", zones.len());

        // 测试层级查询
        if let Some(&first_zone) = zones.first() {
            let children = router.get_children(first_zone).await.unwrap();
            println!("[Router-Auto] 第一个 ZONE 有 {} 个子节点", children.len());

            // 测试深层查询
            let descendants = router
                .get_descendants(first_zone, Some(3))
                .await
                .unwrap();
            println!("[Router-Auto] 深度3的子孙节点: {} 个", descendants.len());
        }
    }

    /// 测试回退机制
    #[tokio::test]
    #[ignore]
    async fn test_router_fallback() {
        init_surreal().await.unwrap();

        let router = QueryRouter::new(QueryStrategy {
            engine: QueryEngine::Auto,
            enable_fallback: true,
            timeout_ms: Some(5000),
            enable_performance_log: true,
        })
        .unwrap();

        // 即使 Kuzu 不可用或查询失败，也应该回退到 SurrealDB
        let result = router.query_by_type(&["PIPE"], 1112, None).await;

        assert!(result.is_ok(), "回退机制应该确保查询成功");
        let pipes = result.unwrap();
        println!("[Router-Fallback] 找到 {} 个 PIPE（可能使用了回退）", pipes.len());
    }

    /// 测试不同提供者的结果一致性
    #[cfg(feature = "kuzu")]
    #[tokio::test]
    #[ignore]
    async fn test_provider_consistency() {
        use std::collections::HashSet;

        init_surreal().await.unwrap();

        let surreal_provider = SurrealQueryProvider::new().unwrap();
        let kuzu_provider = KuzuQueryProvider::new().unwrap();

        // 查询相同的数据
        let surreal_pipes = surreal_provider
            .query_by_type(&["PIPE"], 1112, None)
            .await
            .unwrap();

        let kuzu_pipes = kuzu_provider
            .query_by_type(&["PIPE"], 1112, None)
            .await
            .unwrap();

        // 转换为 HashSet 进行比较
        let surreal_set: HashSet<_> = surreal_pipes.iter().collect();
        let kuzu_set: HashSet<_> = kuzu_pipes.iter().collect();

        println!(
            "一致性测试: SurrealDB {} 个, Kuzu {} 个",
            surreal_pipes.len(),
            kuzu_pipes.len()
        );

        // 验证结果一致性
        let diff = surreal_set.symmetric_difference(&kuzu_set).count();
        if diff > 0 {
            println!("⚠️  发现 {} 个差异", diff);
        }

        // 允许少量差异（可能是时序问题）
        assert!(diff < surreal_pipes.len() / 100); // 差异少于1%
    }

    /// 性能对比测试
    #[cfg(feature = "kuzu")]
    #[tokio::test]
    #[ignore]
    async fn test_performance_comparison() {
        use std::time::Instant;

        init_surreal().await.unwrap();

        let surreal_provider = SurrealQueryProvider::new().unwrap();
        let kuzu_provider = KuzuQueryProvider::new().unwrap();

        // 获取测试数据
        let zones = surreal_provider
            .query_by_type(&["ZONE"], 1112, Some(true))
            .await
            .unwrap();

        let test_zone = zones.first().copied().unwrap();

        println!("\n性能对比测试 - 深层递归查询:");

        // SurrealDB 性能测试
        let start = Instant::now();
        let surreal_result = surreal_provider
            .get_descendants(test_zone, Some(12))
            .await
            .unwrap();
        let surreal_time = start.elapsed();

        println!(
            "  SurrealDB: {} 个节点, 耗时: {:?}",
            surreal_result.len(),
            surreal_time
        );

        // Kuzu 性能测试
        let start = Instant::now();
        let kuzu_result = kuzu_provider
            .get_descendants(test_zone, Some(12))
            .await
            .unwrap();
        let kuzu_time = start.elapsed();

        println!(
            "  Kuzu:      {} 个节点, 耗时: {:?}",
            kuzu_result.len(),
            kuzu_time
        );

        // 计算提升倍数
        let speedup = surreal_time.as_secs_f64() / kuzu_time.as_secs_f64();
        println!("  性能提升: {:.2}x", speedup);

        // 验证 Kuzu 更快
        assert!(kuzu_time < surreal_time, "Kuzu 应该更快");
    }
}
