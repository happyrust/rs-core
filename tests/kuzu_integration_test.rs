//! Kuzu 集成测试
//!
//! 这个测试文件用于验证 Kuzu 集成的基础功能

#[cfg(feature = "kuzu")]
#[cfg(test)]
mod kuzu_tests {
    use aios_core::rs_kuzu::*;
    use kuzu::SystemConfig;
    use std::fs;

    #[test]
    fn test_kuzu_config() {
        // 测试连接配置
        let config = KuzuConnectionConfig::new("./test_db")
            .with_buffer_pool_size(1024 * 1024 * 1024)
            .with_max_threads(4);

        assert_eq!(config.database_path, "./test_db");
        assert_eq!(config.buffer_pool_size, Some(1024 * 1024 * 1024));
        assert_eq!(config.max_num_threads, Some(4));

        println!("✓ Kuzu 配置测试通过");
    }

    #[test]
    fn test_kuzu_stats() {
        let mut stats = ConnectionStats::default();

        stats.record_query(100, true);
        stats.record_query(200, true);
        stats.record_query(150, false);

        assert_eq!(stats.total_queries, 3);
        assert_eq!(stats.failed_queries, 1);

        println!("✓ Kuzu 统计测试通过");
    }

    #[tokio::test]
    async fn test_kuzu_full_workflow() {
        // 完整工作流测试
        let test_db_path = "./test_data/kuzu_workflow";

        // 清理
        let _ = fs::remove_dir_all(test_db_path);
        fs::create_dir_all("./test_data").expect("无法创建测试目录");

        // 1. 初始化数据库
        init_kuzu(test_db_path, SystemConfig::default())
            .await
            .expect("Kuzu 初始化失败");

        assert!(is_kuzu_initialized(), "数据库应该已初始化");
        println!("✓ 步骤 1: 数据库初始化成功");

        // 2. 获取连接
        let conn = get_kuzu_connection().expect("无法获取连接");
        println!("✓ 步骤 2: 连接获取成功");

        // 3. 初始化模式
        init_kuzu_schema().await.expect("模式初始化失败");
        println!("✓ 步骤 3: 模式初始化成功");

        // 4. 验证模式
        let is_init = is_schema_initialized().await.unwrap_or(false);
        assert!(is_init, "模式应该已初始化");
        println!("✓ 步骤 4: 模式验证成功");

        // 5. 查询统计
        let stats = SchemaStats::query().await.expect("统计查询失败");
        println!("✓ 步骤 5: 统计查询成功");
        println!("  PE 节点数: {}", stats.pe_count);
        println!("  属性节点数: {}", stats.attribute_count);

        println!("\n🎉 Kuzu 完整工作流测试成功！");
    }
}
