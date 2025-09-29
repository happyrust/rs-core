//! Kuzu 连接管理测试

#[cfg(feature = "kuzu")]
use crate::rs_kuzu::*;
#[cfg(feature = "kuzu")]
use kuzu::SystemConfig;
#[cfg(feature = "kuzu")]
use std::fs;

#[tokio::test]
#[cfg(feature = "kuzu")]
async fn test_kuzu_init() {
    // 使用临时目录
    let test_db_path = "./test_data/kuzu_test_db";

    // 清理旧数据
    let _ = fs::remove_dir_all(test_db_path);

    // 创建目录
    fs::create_dir_all("./test_data").unwrap();

    // 初始化数据库
    let result = init_kuzu(test_db_path, SystemConfig::default()).await;
    assert!(result.is_ok(), "Kuzu 初始化失败: {:?}", result.err());

    // 检查是否已初始化
    assert!(is_kuzu_initialized(), "Kuzu 应该已初始化");

    println!("✓ Kuzu 数据库初始化成功");
}

#[tokio::test]
#[cfg(feature = "kuzu")]
async fn test_kuzu_connection() {
    // 使用临时目录
    let test_db_path = "./test_data/kuzu_conn_test";

    // 清理旧数据
    let _ = fs::remove_dir_all(test_db_path);
    fs::create_dir_all("./test_data").unwrap();

    // 初始化
    init_kuzu(test_db_path, SystemConfig::default()).await.unwrap();

    // 获取连接
    let conn_result = get_kuzu_connection();
    assert!(conn_result.is_ok(), "获取连接失败: {:?}", conn_result.err());

    println!("✓ Kuzu 连接获取成功");
}

#[test]
#[cfg(feature = "kuzu")]
fn test_connection_config() {
    let config = KuzuConnectionConfig::new("./test_db")
        .with_buffer_pool_size(1024 * 1024 * 1024)
        .with_max_threads(4);

    assert_eq!(config.database_path, "./test_db");
    assert_eq!(config.buffer_pool_size, Some(1024 * 1024 * 1024));
    assert_eq!(config.max_num_threads, Some(4));

    // 验证配置
    let validation = config.validate();
    assert!(validation.is_ok(), "配置验证失败: {:?}", validation.err());

    println!("✓ 连接配置测试通过");
}

#[test]
#[cfg(feature = "kuzu")]
fn test_connection_stats() {
    let mut stats = ConnectionStats::default();

    // 记录一些查询
    stats.record_query(100, true);
    stats.record_query(200, true);
    stats.record_query(150, false);

    assert_eq!(stats.total_queries, 3);
    assert_eq!(stats.failed_queries, 1);

    let success_rate = stats.success_rate();
    assert!((success_rate - 0.666).abs() < 0.01, "成功率计算错误");

    println!("✓ 连接统计测试通过");
    println!("  总查询数: {}", stats.total_queries);
    println!("  失败查询数: {}", stats.failed_queries);
    println!("  成功率: {:.2}%", success_rate * 100.0);
}