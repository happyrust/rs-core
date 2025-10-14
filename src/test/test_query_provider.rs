//! 查询提供者测试

#[cfg(test)]
mod tests {
    use crate::RefnoEnum;
    use crate::init_surreal;
    use crate::query_provider::*;

    /// 测试 SurrealDB 查询提供者
    #[tokio::test]
    #[ignore] // 需要数据库环境
    async fn test_surreal_provider() {
        init_surreal().await.unwrap();

        let provider = SurrealQueryProvider::new().unwrap();

        // 测试查询类型
        let pipes = provider.query_by_type(&["PIPE"], 1112, None).await.unwrap();

        println!("找到 {} 个 PIPE 元素", pipes.len());
        assert!(!pipes.is_empty());

        // 测试获取子节点
        if let Some(&first_pipe) = pipes.first() {
            let children = provider.get_children(first_pipe).await.unwrap();
            println!("第一个 PIPE 有 {} 个子节点", children.len());
        }
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
            let descendants = router.get_descendants(first_zone, Some(3)).await.unwrap();
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

        // 即使出现失败也应该通过回退逻辑保证查询成功
        let result = router.query_by_type(&["PIPE"], 1112, None).await;

        assert!(result.is_ok(), "回退机制应该确保查询成功");
        let pipes = result.unwrap();
        println!(
            "[Router-Fallback] 找到 {} 个 PIPE（可能使用了回退）",
            pipes.len()
        );
    }
}
