//! 同步性能基准测试

use aios_core::db_adapter::DatabaseAdapter;
use aios_core::sync::*;
use aios_core::types::*;
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;

/// 模拟数据库适配器用于测试
struct MockAdapter;

#[async_trait::async_trait]
impl DatabaseAdapter for MockAdapter {
    async fn get_pe(
        &self,
        _refno: RefnoEnum,
        _ctx: Option<aios_core::db_adapter::QueryContext>,
    ) -> anyhow::Result<Option<SPdmsElement>> {
        // 模拟数据库查询延迟
        tokio::time::sleep(Duration::from_micros(10)).await;
        Ok(Some(SPdmsElement::default()))
    }

    async fn save_pe(&self, _pe: &SPdmsElement) -> anyhow::Result<()> {
        // 模拟数据库写入延迟
        tokio::time::sleep(Duration::from_micros(20)).await;
        Ok(())
    }

    async fn get_attmap(
        &self,
        _refno: RefnoEnum,
        _ctx: Option<aios_core::db_adapter::QueryContext>,
    ) -> anyhow::Result<NamedAttrMap> {
        Ok(NamedAttrMap::default())
    }

    async fn save_attmap(&self, _refno: RefnoEnum, _attmap: &NamedAttrMap) -> anyhow::Result<()> {
        Ok(())
    }

    async fn query_children(
        &self,
        _refno: RefnoEnum,
        _ctx: Option<aios_core::db_adapter::QueryContext>,
    ) -> anyhow::Result<Vec<RefnoEnum>> {
        Ok(vec![])
    }

    async fn create_relation(
        &self,
        _from: RefnoEnum,
        _to: RefnoEnum,
        _rel_type: &str,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn delete_pe(&self, _refno: RefnoEnum) -> anyhow::Result<()> {
        Ok(())
    }

    async fn query_subtree(
        &self,
        _refno: RefnoEnum,
        _max_depth: usize,
        _ctx: Option<aios_core::db_adapter::QueryContext>,
    ) -> anyhow::Result<Vec<RefnoEnum>> {
        Ok(vec![])
    }
}

/// 并发执行器基准测试
fn concurrent_executor_benchmark(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("concurrent_executor");

    for batch_size in [10, 100, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            batch_size,
            |b, &batch_size| {
                b.iter(|| {
                    rt.block_on(async {
                        let config = ConcurrentConfig {
                            max_concurrency: 8,
                            batch_size: 100,
                            ..Default::default()
                        };
                        let executor = ConcurrentExecutor::new(config);

                        let refnos: Vec<RefnoEnum> = (0..batch_size)
                            .map(|i| RefnoEnum::from(RefU64(i as u64)))
                            .collect();

                        let source = Arc::new(MockAdapter);
                        let target = Arc::new(MockAdapter);

                        executor
                            .sync_batch_pes(black_box(refnos), source, target)
                            .await
                            .unwrap()
                    })
                });
            },
        );
    }

    group.finish();
}

/// 缓存层基准测试
fn cache_layer_benchmark(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("cache_layer");

    group.bench_function("cache_hit", |b| {
        b.iter(|| {
            rt.block_on(async {
                let cache = PECache::new(10000);
                let refno = RefnoEnum::from(RefU64(123));
                let pe = SPdmsElement::default();

                // 预热缓存
                cache.put(refno, pe.clone()).await;

                // 测试缓存命中
                for _ in 0..1000 {
                    black_box(cache.get(refno).await);
                }
            })
        });
    });

    group.bench_function("cache_miss", |b| {
        b.iter(|| {
            rt.block_on(async {
                let cache = PECache::new(10000);

                // 测试缓存未命中
                for i in 0..1000 {
                    let refno = RefnoEnum::from(RefU64(i));
                    black_box(cache.get(refno).await);
                }
            })
        });
    });

    group.finish();
}

/// 批量优化器基准测试
fn batch_optimizer_benchmark(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("batch_optimizer");

    group.bench_function("buffer_operations", |b| {
        b.iter(|| {
            rt.block_on(async {
                let config = BatchConfig {
                    pe_batch_size: 1000,
                    ..Default::default()
                };
                let optimizer = BatchOptimizer::new(config);

                // 测试缓冲操作
                for i in 0..100 {
                    let pe = SPdmsElement::default();
                    optimizer.buffer_pe(black_box(pe)).await.unwrap();

                    let refno = RefnoEnum::from(RefU64(i));
                    let attmap = NamedAttrMap::default();
                    optimizer
                        .buffer_attributes(black_box(refno), black_box(attmap))
                        .await
                        .unwrap();
                }

                optimizer.flush_all().await.unwrap();
            })
        });
    });

    group.finish();
}

/// 同步策略基准测试
fn sync_strategy_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("sync_strategy");

    group.bench_function("filter_matching", |b| {
        let filter = SyncFilter {
            include_types: vec!["PIPE".to_string(), "EQUI".to_string()],
            refno_range: Some((RefU64(100), RefU64(10000))),
            ..Default::default()
        };

        b.iter(|| {
            for i in 0..1000 {
                black_box(filter.matches_refno(RefU64(i)));
                black_box(filter.matches_type("PIPE"));
                black_box(filter.matches_attribute("NAME"));
            }
        });
    });

    group.finish();
}

/// 性能监控基准测试
fn performance_monitor_benchmark(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("performance_monitor");

    group.bench_function("record_statistics", |b| {
        b.iter(|| {
            rt.block_on(async {
                let monitor = PerformanceMonitor::new(1000);

                for i in 0..100 {
                    let mut stats = SyncStatistics::default();
                    stats.total_records = i * 10;
                    stats.successful_records = i * 9;
                    stats.failed_records = i;

                    monitor.record(black_box(stats)).await;
                }

                monitor.generate_report().await
            })
        });
    });

    group.finish();
}

/// 任务管理基准测试
fn task_management_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("task_management");

    group.bench_function("task_operations", |b| {
        b.iter(|| {
            let mut task = SyncTask::new(SyncTaskType::SyncAll);

            task.start();

            for i in 0..1000 {
                if i % 10 == 0 {
                    task.record_success();
                } else if i % 20 == 0 {
                    task.record_failure("test error".to_string());
                }
                task.update_progress(i, 1000);
            }

            task.complete();
            black_box(task.duration());
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    concurrent_executor_benchmark,
    cache_layer_benchmark,
    batch_optimizer_benchmark,
    sync_strategy_benchmark,
    performance_monitor_benchmark,
    task_management_benchmark
);

criterion_main!(benches);
