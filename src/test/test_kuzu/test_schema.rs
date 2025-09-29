//! Kuzu 图模式测试

#[cfg(feature = "kuzu")]
use crate::rs_kuzu::*;
#[cfg(feature = "kuzu")]
use kuzu::SystemConfig;
#[cfg(feature = "kuzu")]
use std::fs;

#[tokio::test]
#[cfg(feature = "kuzu")]
async fn test_schema_initialization() {
    // 使用临时目录
    let test_db_path = "./test_data/kuzu_schema_test";

    // 清理旧数据
    let _ = fs::remove_dir_all(test_db_path);
    fs::create_dir_all("./test_data").unwrap();

    // 初始化数据库
    init_kuzu(test_db_path, SystemConfig::default()).await.unwrap();

    // 初始化图模式
    let result = init_kuzu_schema().await;
    assert!(result.is_ok(), "图模式初始化失败: {:?}", result.err());

    println!("✓ 图模式初始化成功");
}

#[tokio::test]
#[cfg(feature = "kuzu")]
async fn test_schema_check() {
    // 使用临时目录
    let test_db_path = "./test_data/kuzu_schema_check";

    // 清理旧数据
    let _ = fs::remove_dir_all(test_db_path);
    fs::create_dir_all("./test_data").unwrap();

    // 初始化数据库
    init_kuzu(test_db_path, SystemConfig::default()).await.unwrap();

    // 检查模式（应该未初始化）
    let is_init = is_schema_initialized().await.unwrap_or(false);
    assert!(!is_init, "模式不应该已初始化");

    // 初始化模式
    init_kuzu_schema().await.unwrap();

    // 再次检查（应该已初始化）
    let is_init = is_schema_initialized().await.unwrap_or(false);
    assert!(is_init, "模式应该已初始化");

    println!("✓ 模式检查测试通过");
}

#[tokio::test]
#[cfg(feature = "kuzu")]
async fn test_schema_stats() {
    // 使用临时目录
    let test_db_path = "./test_data/kuzu_schema_stats";

    // 清理旧数据
    let _ = fs::remove_dir_all(test_db_path);
    fs::create_dir_all("./test_data").unwrap();

    // 初始化数据库和模式
    init_kuzu(test_db_path, SystemConfig::default()).await.unwrap();
    init_kuzu_schema().await.unwrap();

    // 查询统计
    let stats_result = SchemaStats::query().await;
    assert!(stats_result.is_ok(), "统计查询失败: {:?}", stats_result.err());

    let stats = stats_result.unwrap();
    println!("✓ 模式统计查询成功");
    println!("  PE 节点数: {}", stats.pe_count);
    println!("  属性节点数: {}", stats.attribute_count);
    println!("  UDA 节点数: {}", stats.uda_count);
    println!("  OWNS 关系数: {}", stats.owns_count);
    println!("  REFERS_TO 关系数: {}", stats.refers_to_count);

    // 初始状态应该都是 0
    assert_eq!(stats.pe_count, 0);
    assert_eq!(stats.attribute_count, 0);
}

#[tokio::test]
#[cfg(feature = "kuzu")]
#[ignore] // 谨慎使用，会删除所有表
async fn test_drop_all_tables() {
    let test_db_path = "./test_data/kuzu_drop_test";

    let _ = fs::remove_dir_all(test_db_path);
    fs::create_dir_all("./test_data").unwrap();

    init_kuzu(test_db_path, SystemConfig::default()).await.unwrap();
    init_kuzu_schema().await.unwrap();

    // 删除所有表
    let result = drop_all_tables().await;
    assert!(result.is_ok(), "删除表失败: {:?}", result.err());

    println!("✓ 表删除测试通过");
}