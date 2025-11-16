use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use glam::Vec3;
use std::time::Duration;
use tokio::runtime::Runtime;

// 导入房间查询模块
use aios_core::room::query::query_room_number_by_point;
#[cfg(all(not(target_arch = "wasm32"), feature = "sqlite"))]
use aios_core::room::query_v2::{
    batch_query_room_numbers, clear_geometry_cache, get_room_query_stats,
    query_room_number_by_point_v2,
};

/// 生成测试点数据
fn generate_test_points(count: usize) -> Vec<Vec3> {
    let mut points = Vec::with_capacity(count);

    // 生成一些典型的工厂坐标点
    for i in 0..count {
        let x = (i as f32 * 100.0) % 10000.0;
        let y = (i as f32 * 50.0) % 5000.0;
        let z = (i as f32 * 10.0) % 100.0;
        points.push(Vec3::new(x, y, z));
    }

    points
}

/// 基准测试：单点房间查询（原版本）
fn bench_single_point_query_v1(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let test_points = generate_test_points(10);

    c.bench_function("single_point_query_v1", |b| {
        b.to_async(&rt).iter(|| async {
            let point = black_box(test_points[0]);
            let _ = query_room_number_by_point(point).await;
        });
    });
}

/// 基准测试：单点房间查询（改进版本）
#[cfg(all(not(target_arch = "wasm32"), feature = "sqlite"))]
fn bench_single_point_query_v2(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let test_points = generate_test_points(10);

    c.bench_function("single_point_query_v2", |b| {
        b.to_async(&rt).iter(|| async {
            let point = black_box(test_points[0]);
            let _ = query_room_number_by_point_v2(point).await;
        });
    });
}

/// 基准测试：批量房间查询
#[cfg(all(not(target_arch = "wasm32"), feature = "sqlite"))]
fn bench_batch_query(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("batch_query");
    group.measurement_time(Duration::from_secs(30));

    for batch_size in [10, 50, 100, 500].iter() {
        let test_points = generate_test_points(*batch_size);

        group.bench_with_input(
            BenchmarkId::new("batch_size", batch_size),
            batch_size,
            |b, &_batch_size| {
                b.to_async(&rt).iter(|| async {
                    let points = black_box(test_points.clone());
                    let _ = batch_query_room_numbers(points, 10).await;
                });
            },
        );
    }

    group.finish();
}

/// 基准测试：并发查询性能
#[cfg(all(not(target_arch = "wasm32"), feature = "sqlite"))]
fn bench_concurrent_query(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let test_points = generate_test_points(100);

    let mut group = c.benchmark_group("concurrent_query");
    group.measurement_time(Duration::from_secs(20));

    for concurrency in [1, 5, 10, 20].iter() {
        group.bench_with_input(
            BenchmarkId::new("concurrency", concurrency),
            concurrency,
            |b, &concurrency| {
                b.to_async(&rt).iter(|| async {
                    let points = black_box(test_points.clone());
                    let _ = batch_query_room_numbers(points, concurrency).await;
                });
            },
        );
    }

    group.finish();
}

/// 基准测试：缓存性能对比
#[cfg(all(not(target_arch = "wasm32"), feature = "sqlite"))]
fn bench_cache_performance(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let test_point = Vec3::new(1000.0, 500.0, 10.0);

    let mut group = c.benchmark_group("cache_performance");

    // 冷缓存测试
    group.bench_function("cold_cache", |b| {
        b.to_async(&rt).iter(|| async {
            clear_geometry_cache();
            let point = black_box(test_point);
            let _ = query_room_number_by_point_v2(point).await;
        });
    });

    // 热缓存测试
    group.bench_function("warm_cache", |b| {
        b.to_async(&rt).iter(|| async {
            // 不清理缓存，测试热缓存性能
            let point = black_box(test_point);
            let _ = query_room_number_by_point_v2(point).await;
        });
    });

    group.finish();
}

/// 基准测试：内存使用情况
#[cfg(all(not(target_arch = "wasm32"), feature = "sqlite"))]
fn bench_memory_usage(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("memory_usage_stats", |b| {
        b.to_async(&rt).iter(|| async {
            let stats = get_room_query_stats().await;
            black_box(stats);
        });
    });
}

/// 压力测试：大量查询
#[cfg(all(not(target_arch = "wasm32"), feature = "sqlite"))]
fn bench_stress_test(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let test_points = generate_test_points(1000);

    let mut group = c.benchmark_group("stress_test");
    group.measurement_time(Duration::from_secs(60));
    group.sample_size(10);

    group.bench_function("1000_points", |b| {
        b.to_async(&rt).iter(|| async {
            let points = black_box(test_points.clone());
            let _ = batch_query_room_numbers(points, 50).await;
        });
    });

    group.finish();
}

// 配置基准测试组
#[cfg(all(not(target_arch = "wasm32"), feature = "sqlite"))]
criterion_group!(
    benches,
    bench_single_point_query_v1,
    bench_single_point_query_v2,
    bench_batch_query,
    bench_concurrent_query,
    bench_cache_performance,
    bench_memory_usage,
    bench_stress_test
);

#[cfg(not(all(not(target_arch = "wasm32"), feature = "sqlite")))]
criterion_group!(benches, bench_single_point_query_v1);

criterion_main!(benches);
