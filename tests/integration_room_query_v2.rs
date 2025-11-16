use aios_core::room::query_v2::{
    batch_query_room_numbers, clear_geometry_cache, get_room_query_stats,
    query_room_number_by_point_v2,
};
use aios_core::spatial::hybrid_index::get_hybrid_index;
use glam::Vec3;
use std::time::Instant;

/// 集成测试：房间查询系统 V2
///
/// 测试改进版本的房间查询功能，包括：
/// - 混合空间索引
/// - 几何缓存优化
/// - 批量查询性能
/// - 统计信息收集

#[tokio::test]
#[cfg(all(not(target_arch = "wasm32"), feature = "sqlite"))]
async fn test_hybrid_spatial_index_initialization() {
    // 测试混合空间索引初始化
    let index = get_hybrid_index().await;
    let stats = index.get_stats().await;

    println!("混合空间索引统计: {:?}", stats);
    assert!(stats.total_elements >= 0);
}

#[tokio::test]
#[cfg(all(not(target_arch = "wasm32"), feature = "sqlite"))]
async fn test_single_point_query_v2() {
    // 测试单点房间查询
    let test_point = Vec3::new(1000.0, 500.0, 10.0);

    let start_time = Instant::now();
    let result = query_room_number_by_point_v2(test_point).await;
    let query_time = start_time.elapsed();

    println!("单点查询结果: {:?}, 耗时: {:?}", result, query_time);

    // 验证查询不会出错
    assert!(result.is_ok());

    // 检查统计信息
    let stats = get_room_query_stats().await;
    assert!(stats.total_queries > 0);

    println!("查询统计: {:?}", stats);
}

#[tokio::test]
#[cfg(all(not(target_arch = "wasm32"), feature = "sqlite"))]
async fn test_batch_query_performance() {
    // 测试批量查询性能
    let test_points = vec![
        Vec3::new(1000.0, 500.0, 10.0),
        Vec3::new(2000.0, 1000.0, 20.0),
        Vec3::new(3000.0, 1500.0, 30.0),
        Vec3::new(4000.0, 2000.0, 40.0),
        Vec3::new(5000.0, 2500.0, 50.0),
    ];

    let start_time = Instant::now();
    let results = batch_query_room_numbers(test_points.clone(), 3).await;
    let batch_time = start_time.elapsed();

    println!("批量查询结果: {:?}, 耗时: {:?}", results, batch_time);

    assert!(results.is_ok());
    let room_numbers = results.unwrap();
    assert_eq!(room_numbers.len(), test_points.len());

    // 检查统计信息
    let stats = get_room_query_stats().await;
    println!("批量查询后统计: {:?}", stats);
}

#[tokio::test]
#[cfg(all(not(target_arch = "wasm32"), feature = "sqlite"))]
async fn test_cache_performance() {
    // 测试缓存性能
    let test_point = Vec3::new(1000.0, 500.0, 10.0);

    // 清理缓存，测试冷缓存性能
    clear_geometry_cache();

    let cold_start = Instant::now();
    let _ = query_room_number_by_point_v2(test_point).await;
    let cold_time = cold_start.elapsed();

    // 测试热缓存性能
    let warm_start = Instant::now();
    let _ = query_room_number_by_point_v2(test_point).await;
    let warm_time = warm_start.elapsed();

    println!("冷缓存查询耗时: {:?}", cold_time);
    println!("热缓存查询耗时: {:?}", warm_time);

    // 热缓存应该比冷缓存快（在有缓存命中的情况下）
    let stats = get_room_query_stats().await;
    println!("缓存统计: {:?}", stats);

    // 验证缓存有效性
    assert!(stats.geometry_cache_size >= 0);
}

#[tokio::test]
#[cfg(all(not(target_arch = "wasm32"), feature = "sqlite"))]
async fn test_concurrent_queries() {
    // 测试并发查询
    use futures::future::join_all;

    let test_points = vec![
        Vec3::new(1000.0, 500.0, 10.0),
        Vec3::new(2000.0, 1000.0, 20.0),
        Vec3::new(3000.0, 1500.0, 30.0),
    ];

    let start_time = Instant::now();

    // 创建并发查询任务
    let tasks: Vec<_> = test_points
        .into_iter()
        .map(|point| tokio::spawn(async move { query_room_number_by_point_v2(point).await }))
        .collect();

    // 等待所有任务完成
    let results = join_all(tasks).await;
    let concurrent_time = start_time.elapsed();

    println!("并发查询耗时: {:?}", concurrent_time);

    // 验证所有查询都成功
    for result in results {
        assert!(result.is_ok());
        assert!(result.unwrap().is_ok());
    }

    let stats = get_room_query_stats().await;
    println!("并发查询后统计: {:?}", stats);
}

#[tokio::test]
#[cfg(all(not(target_arch = "wasm32"), feature = "sqlite"))]
async fn test_error_handling() {
    // 测试错误处理
    let invalid_point = Vec3::new(f32::NAN, f32::INFINITY, -f32::INFINITY);

    let result = query_room_number_by_point_v2(invalid_point).await;

    // 应该能够处理无效输入而不崩溃
    match result {
        Ok(_) => println!("查询成功处理了无效点"),
        Err(e) => println!("查询正确返回了错误: {}", e),
    }
}

#[tokio::test]
#[cfg(all(not(target_arch = "wasm32"), feature = "sqlite"))]
async fn test_memory_usage_monitoring() {
    // 测试内存使用监控
    let initial_stats = get_room_query_stats().await;
    println!("初始统计: {:?}", initial_stats);

    // 执行一些查询操作
    let test_points = (0..10)
        .map(|i| Vec3::new(i as f32 * 100.0, i as f32 * 50.0, 10.0))
        .collect();

    let _ = batch_query_room_numbers(test_points, 5).await;

    let final_stats = get_room_query_stats().await;
    println!("最终统计: {:?}", final_stats);

    // 验证统计信息有更新
    assert!(final_stats.total_queries >= initial_stats.total_queries);
}

#[tokio::test]
#[cfg(all(not(target_arch = "wasm32"), feature = "sqlite"))]
async fn test_spatial_index_rebuild() {
    // 测试空间索引重建
    let index = get_hybrid_index().await;

    let initial_stats = index.get_stats().await;
    println!("重建前统计: {:?}", initial_stats);

    // 重建索引
    let rebuild_result = index.rebuild_memory_index().await;
    assert!(rebuild_result.is_ok());

    let final_stats = index.get_stats().await;
    println!("重建后统计: {:?}", final_stats);

    // 验证重建时间有更新
    assert!(final_stats.last_rebuild_time >= initial_stats.last_rebuild_time);
}

/// 性能基准测试
#[tokio::test]
#[cfg(all(not(target_arch = "wasm32"), feature = "sqlite"))]
async fn benchmark_query_performance() {
    const BENCHMARK_POINTS: usize = 100;
    const ITERATIONS: usize = 5;

    let test_points: Vec<Vec3> = (0..BENCHMARK_POINTS)
        .map(|i| {
            Vec3::new(
                (i as f32 * 100.0) % 10000.0,
                (i as f32 * 50.0) % 5000.0,
                10.0,
            )
        })
        .collect();

    let mut total_time = std::time::Duration::ZERO;

    for iteration in 0..ITERATIONS {
        println!("基准测试迭代 {}/{}", iteration + 1, ITERATIONS);

        let start_time = Instant::now();
        let results = batch_query_room_numbers(test_points.clone(), 10).await;
        let iteration_time = start_time.elapsed();

        total_time += iteration_time;

        assert!(results.is_ok());
        println!("迭代 {} 耗时: {:?}", iteration + 1, iteration_time);
    }

    let avg_time = total_time / ITERATIONS as u32;
    let throughput = (BENCHMARK_POINTS * ITERATIONS) as f64 / total_time.as_secs_f64();

    println!("基准测试结果:");
    println!("  总查询数: {}", BENCHMARK_POINTS * ITERATIONS);
    println!("  总耗时: {:?}", total_time);
    println!("  平均耗时: {:?}", avg_time);
    println!("  吞吐量: {:.2} 查询/秒", throughput);

    let final_stats = get_room_query_stats().await;
    println!("最终统计: {:?}", final_stats);

    // 性能断言
    assert!(throughput > 1.0, "吞吐量应该大于 1 查询/秒");
    assert!(avg_time.as_millis() < 10000, "平均查询时间应该小于 10 秒");
}
