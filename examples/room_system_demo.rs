use aios_core::room::monitoring::{
    check_system_health, get_current_system_metrics, record_query_time,
};
use aios_core::room::query_v2::{
    batch_query_room_numbers, clear_geometry_cache, get_room_query_stats,
    query_room_number_by_point_v2,
};
use aios_core::spatial::hybrid_index::get_hybrid_index;
use glam::Vec3;
use std::time::{Duration, Instant};
use tracing::{Level, info, warn};
use tracing_subscriber;

/// 房间计算系统演示程序
///
/// 展示改进版本的房间计算系统的各项功能：
/// 1. 混合空间索引
/// 2. 优化的几何缓存
/// 3. 批量查询
/// 4. 性能监控
/// 5. 系统健康检查

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("🚀 房间计算系统演示程序启动");

    // 1. 初始化混合空间索引
    info!("📊 初始化混合空间索引...");
    let index = get_hybrid_index().await;
    let initial_stats = index.get_stats().await;
    info!("空间索引统计: {:?}", initial_stats);

    // 2. 演示单点查询
    info!("🔍 演示单点房间查询...");
    demo_single_point_query().await?;

    // 3. 演示批量查询
    info!("📦 演示批量房间查询...");
    demo_batch_query().await?;

    // 4. 演示缓存性能
    info!("💾 演示缓存性能对比...");
    demo_cache_performance().await?;

    // 5. 演示并发查询
    info!("🔄 演示并发查询性能...");
    demo_concurrent_queries().await?;

    // 6. 演示性能监控
    info!("📈 演示性能监控功能...");
    demo_performance_monitoring().await?;

    // 7. 演示系统健康检查
    info!("🏥 演示系统健康检查...");
    demo_health_check().await?;

    // 8. 性能基准测试
    info!("⚡ 执行性能基准测试...");
    run_performance_benchmark().await?;

    info!("✅ 房间计算系统演示完成");
    Ok(())
}

/// 演示单点查询
async fn demo_single_point_query() -> anyhow::Result<()> {
    let test_points = vec![
        Vec3::new(1000.0, 500.0, 10.0),
        Vec3::new(2000.0, 1000.0, 20.0),
        Vec3::new(3000.0, 1500.0, 30.0),
    ];

    for (i, point) in test_points.iter().enumerate() {
        let start_time = Instant::now();

        #[cfg(all(not(target_arch = "wasm32"), feature = "sqlite"))]
        let result = query_room_number_by_point_v2(*point).await;

        #[cfg(not(all(not(target_arch = "wasm32"), feature = "sqlite")))]
        let result: anyhow::Result<Option<String>> = Ok(None);

        let query_time = start_time.elapsed();

        match result {
            Ok(room_number) => {
                info!(
                    "查询 {}: 点 {:?} -> 房间号 {:?}, 耗时 {:?}",
                    i + 1,
                    point,
                    room_number,
                    query_time
                );
                record_query_time(query_time, true).await;
            }
            Err(e) => {
                warn!("查询 {} 失败: {}, 耗时 {:?}", i + 1, e, query_time);
                record_query_time(query_time, false).await;
            }
        }
    }

    Ok(())
}

/// 演示批量查询
async fn demo_batch_query() -> anyhow::Result<()> {
    let test_points: Vec<Vec3> = (0..20)
        .map(|i| {
            Vec3::new(
                (i as f32 * 200.0) % 5000.0,
                (i as f32 * 100.0) % 2500.0,
                10.0 + (i as f32 * 5.0) % 50.0,
            )
        })
        .collect();

    let start_time = Instant::now();

    #[cfg(all(not(target_arch = "wasm32"), feature = "sqlite"))]
    let results = batch_query_room_numbers(test_points.clone(), 5).await;

    #[cfg(not(all(not(target_arch = "wasm32"), feature = "sqlite")))]
    let results: anyhow::Result<Vec<Option<String>>> = Ok(vec![None; test_points.len()]);

    let batch_time = start_time.elapsed();

    match results {
        Ok(room_numbers) => {
            info!(
                "批量查询完成: {} 个点, {} 个结果, 耗时 {:?}",
                test_points.len(),
                room_numbers.len(),
                batch_time
            );

            let successful_queries = room_numbers.iter().filter(|r| r.is_some()).count();
            info!("成功查询: {}/{}", successful_queries, room_numbers.len());

            record_query_time(batch_time, true).await;
        }
        Err(e) => {
            warn!("批量查询失败: {}, 耗时 {:?}", e, batch_time);
            record_query_time(batch_time, false).await;
        }
    }

    Ok(())
}

/// 演示缓存性能
async fn demo_cache_performance() -> anyhow::Result<()> {
    let test_point = Vec3::new(1500.0, 750.0, 15.0);

    // 冷缓存测试
    #[cfg(all(not(target_arch = "wasm32"), feature = "sqlite"))]
    {
        clear_geometry_cache();

        let cold_start = Instant::now();
        let _ = query_room_number_by_point_v2(test_point).await;
        let cold_time = cold_start.elapsed();

        info!("冷缓存查询耗时: {:?}", cold_time);

        // 热缓存测试
        let warm_start = Instant::now();
        let _ = query_room_number_by_point_v2(test_point).await;
        let warm_time = warm_start.elapsed();

        info!("热缓存查询耗时: {:?}", warm_time);

        let speedup = if warm_time.as_nanos() > 0 {
            cold_time.as_nanos() as f64 / warm_time.as_nanos() as f64
        } else {
            1.0
        };

        info!("缓存加速比: {:.2}x", speedup);

        // 获取缓存统计
        let cache_stats = get_room_query_stats().await;
        info!("缓存统计: {:?}", cache_stats);
    }

    Ok(())
}

/// 演示并发查询
async fn demo_concurrent_queries() -> anyhow::Result<()> {
    use futures::future::join_all;

    let test_points: Vec<Vec3> = (0..10)
        .map(|i| {
            Vec3::new(
                1000.0 + (i as f32 * 300.0),
                500.0 + (i as f32 * 150.0),
                10.0 + (i as f32 * 2.0),
            )
        })
        .collect();

    let start_time = Instant::now();

    #[cfg(all(not(target_arch = "wasm32"), feature = "sqlite"))]
    let tasks: Vec<_> = test_points
        .into_iter()
        .map(|point| tokio::spawn(async move { query_room_number_by_point_v2(point).await }))
        .collect();

    #[cfg(not(all(not(target_arch = "wasm32"), feature = "sqlite")))]
    let tasks: Vec<_> = (0..10)
        .map(|_| tokio::spawn(async move { Ok::<Option<String>, anyhow::Error>(None) }))
        .collect();

    let results = join_all(tasks).await;
    let concurrent_time = start_time.elapsed();

    let successful_tasks = results.iter().filter(|r| r.is_ok()).count();
    let successful_queries = results
        .iter()
        .filter_map(|r| r.as_ref().ok())
        .filter(|r| r.is_ok())
        .count();

    info!(
        "并发查询完成: {} 个任务, {} 个成功任务, {} 个成功查询, 耗时 {:?}",
        results.len(),
        successful_tasks,
        successful_queries,
        concurrent_time
    );

    record_query_time(concurrent_time, successful_tasks == results.len()).await;

    Ok(())
}

/// 演示性能监控
async fn demo_performance_monitoring() -> anyhow::Result<()> {
    let metrics = get_current_system_metrics().await;

    info!("📊 当前系统指标:");
    info!("  内存使用: {:.2} MB", metrics.system.memory_usage_mb);
    info!("  总查询数: {}", metrics.query.total_queries);
    info!("  成功查询: {}", metrics.query.successful_queries);
    info!("  失败查询: {}", metrics.query.failed_queries);
    info!("  平均查询时间: {:.2} ms", metrics.query.avg_query_time_ms);
    info!("  P95 查询时间: {:.2} ms", metrics.query.p95_query_time_ms);
    info!("  P99 查询时间: {:.2} ms", metrics.query.p99_query_time_ms);
    info!(
        "  查询吞吐量: {:.2} 查询/秒",
        metrics.query.queries_per_second
    );
    info!("  错误率: {:.2}%", metrics.system.error_rate * 100.0);
    info!(
        "  缓存命中率: {:.2}%",
        metrics.system.cache_hit_rate * 100.0
    );
    info!("  运行时间: {} 秒", metrics.uptime_seconds);

    info!("💾 缓存指标:");
    info!("  几何缓存大小: {}", metrics.cache.geometry_cache_size);
    info!(
        "  几何缓存命中率: {:.2}%",
        metrics.cache.geometry_cache_hit_rate * 100.0
    );
    info!("  查询缓存大小: {}", metrics.cache.query_cache_size);
    info!(
        "  查询缓存命中率: {:.2}%",
        metrics.cache.query_cache_hit_rate * 100.0
    );
    info!(
        "  总缓存内存: {:.2} MB",
        metrics.cache.total_cache_memory_mb
    );

    info!("🗂️ 空间索引指标:");
    info!(
        "  内存索引大小: {}",
        metrics.spatial_index.memory_index_size
    );
    info!(
        "  SQLite索引大小: {}",
        metrics.spatial_index.sqlite_index_size
    );
    info!(
        "  索引命中率: {:.2}%",
        metrics.spatial_index.index_hit_rate * 100.0
    );
    info!(
        "  索引内存: {:.2} MB",
        metrics.spatial_index.index_memory_mb
    );

    Ok(())
}

/// 演示系统健康检查
async fn demo_health_check() -> anyhow::Result<()> {
    let health = check_system_health().await;

    info!("🏥 系统健康检查结果:");

    match health.level {
        aios_core::room::monitoring::HealthLevel::Healthy => {
            info!("  状态: ✅ 健康");
        }
        aios_core::room::monitoring::HealthLevel::Warning => {
            info!("  状态: ⚠️ 警告");
            for warning in &health.warnings {
                info!("    警告: {}", warning);
            }
        }
        aios_core::room::monitoring::HealthLevel::Critical => {
            info!("  状态: ❌ 严重");
            for issue in &health.issues {
                info!("    问题: {}", issue);
            }
        }
    }

    if !health.warnings.is_empty() {
        info!("  警告数量: {}", health.warnings.len());
    }

    if !health.issues.is_empty() {
        info!("  严重问题数量: {}", health.issues.len());
    }

    Ok(())
}

/// 性能基准测试
async fn run_performance_benchmark() -> anyhow::Result<()> {
    const BENCHMARK_POINTS: usize = 100;
    const ITERATIONS: usize = 3;

    info!(
        "开始性能基准测试: {} 个点, {} 次迭代",
        BENCHMARK_POINTS, ITERATIONS
    );

    let test_points: Vec<Vec3> = (0..BENCHMARK_POINTS)
        .map(|i| {
            Vec3::new(
                (i as f32 * 100.0) % 8000.0,
                (i as f32 * 50.0) % 4000.0,
                10.0 + (i as f32 * 2.0) % 40.0,
            )
        })
        .collect();

    let mut total_time = Duration::ZERO;
    let mut successful_iterations = 0;

    for iteration in 0..ITERATIONS {
        info!("执行基准测试迭代 {}/{}", iteration + 1, ITERATIONS);

        let start_time = Instant::now();

        #[cfg(all(not(target_arch = "wasm32"), feature = "sqlite"))]
        let result = batch_query_room_numbers(test_points.clone(), 10).await;

        #[cfg(not(all(not(target_arch = "wasm32"), feature = "sqlite")))]
        let result: anyhow::Result<Vec<Option<String>>> = {
            tokio::time::sleep(Duration::from_millis(100)).await;
            Ok(vec![None; test_points.len()])
        };

        let iteration_time = start_time.elapsed();

        match result {
            Ok(_) => {
                total_time += iteration_time;
                successful_iterations += 1;
                info!("迭代 {} 完成, 耗时: {:?}", iteration + 1, iteration_time);
            }
            Err(e) => {
                warn!("迭代 {} 失败: {}", iteration + 1, e);
            }
        }
    }

    if successful_iterations > 0 {
        let avg_time = total_time / successful_iterations as u32;
        let total_queries = BENCHMARK_POINTS * successful_iterations as usize;
        let throughput = total_queries as f64 / total_time.as_secs_f64();

        info!("📊 基准测试结果:");
        info!("  成功迭代: {}/{}", successful_iterations, ITERATIONS);
        info!("  总查询数: {}", total_queries);
        info!("  总耗时: {:?}", total_time);
        info!("  平均耗时: {:?}", avg_time);
        info!("  吞吐量: {:.2} 查询/秒", throughput);
        info!(
            "  平均单查询时间: {:.2} ms",
            avg_time.as_millis() as f64 / BENCHMARK_POINTS as f64
        );

        // 性能评估
        if throughput > 100.0 {
            info!("  性能评估: ✅ 优秀 (>100 查询/秒)");
        } else if throughput > 50.0 {
            info!("  性能评估: ✅ 良好 (>50 查询/秒)");
        } else if throughput > 10.0 {
            info!("  性能评估: ⚠️ 一般 (>10 查询/秒)");
        } else {
            info!("  性能评估: ❌ 需要优化 (<10 查询/秒)");
        }
    } else {
        warn!("所有基准测试迭代都失败了");
    }

    Ok(())
}
