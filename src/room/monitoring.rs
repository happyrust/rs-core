use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// 系统资源监控
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub cache_hit_rate: f64,
    pub active_queries: u64,
    pub total_queries: u64,
    pub avg_query_time_ms: f64,
    pub error_rate: f64,
    pub timestamp: SystemTime,
}

/// 缓存统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMetrics {
    pub geometry_cache_size: usize,
    pub geometry_cache_hit_rate: f64,
    pub query_cache_size: usize,
    pub query_cache_hit_rate: f64,
    pub total_cache_memory_mb: f64,
    pub cache_evictions: u64,
}

/// 查询性能统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryMetrics {
    pub total_queries: u64,
    pub successful_queries: u64,
    pub failed_queries: u64,
    pub avg_query_time_ms: f64,
    pub p95_query_time_ms: f64,
    pub p99_query_time_ms: f64,
    pub queries_per_second: f64,
}

/// 空间索引统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialIndexMetrics {
    pub memory_index_size: usize,
    pub sqlite_index_size: usize,
    pub index_hit_rate: f64,
    pub last_rebuild_time: SystemTime,
    pub rebuild_count: u64,
    pub index_memory_mb: f64,
}

/// 综合监控指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomSystemMetrics {
    pub system: SystemMetrics,
    pub cache: CacheMetrics,
    pub query: QueryMetrics,
    pub spatial_index: SpatialIndexMetrics,
    pub uptime_seconds: u64,
}

/// 性能监控器
pub struct RoomSystemMonitor {
    start_time: Instant,
    metrics_history: Arc<RwLock<Vec<RoomSystemMetrics>>>,
    query_times: Arc<RwLock<Vec<f64>>>,
    error_count: Arc<RwLock<u64>>,
    cache_stats: Arc<RwLock<CacheMetrics>>,

    // 配置
    max_history_size: usize,
    metrics_retention_duration: Duration,
}

impl Clone for RoomSystemMonitor {
    fn clone(&self) -> Self {
        Self {
            start_time: self.start_time,
            metrics_history: Arc::clone(&self.metrics_history),
            query_times: Arc::clone(&self.query_times),
            error_count: Arc::clone(&self.error_count),
            cache_stats: Arc::clone(&self.cache_stats),
            max_history_size: self.max_history_size,
            metrics_retention_duration: self.metrics_retention_duration,
        }
    }
}

impl RoomSystemMonitor {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            metrics_history: Arc::new(RwLock::new(Vec::new())),
            query_times: Arc::new(RwLock::new(Vec::new())),
            error_count: Arc::new(RwLock::new(0)),
            cache_stats: Arc::new(RwLock::new(CacheMetrics {
                geometry_cache_size: 0,
                geometry_cache_hit_rate: 0.0,
                query_cache_size: 0,
                query_cache_hit_rate: 0.0,
                total_cache_memory_mb: 0.0,
                cache_evictions: 0,
            })),
            max_history_size: 1000,
            metrics_retention_duration: Duration::from_hours(24),
        }
    }

    /// 记录查询时间
    pub async fn record_query_time(&self, duration: Duration, success: bool) {
        let mut query_times = self.query_times.write().await;
        query_times.push(duration.as_millis() as f64);

        // 限制历史记录大小
        if query_times.len() > self.max_history_size {
            let drain_count = query_times.len() - self.max_history_size;
            query_times.drain(0..drain_count);
        }

        if !success {
            let mut error_count = self.error_count.write().await;
            *error_count += 1;
        }
    }

    /// 更新缓存统计
    pub async fn update_cache_stats(&self, stats: CacheMetrics) {
        let mut cache_stats = self.cache_stats.write().await;
        *cache_stats = stats;
    }

    /// 获取当前系统指标
    pub async fn get_current_metrics(&self) -> RoomSystemMetrics {
        let query_times = self.query_times.read().await;
        let error_count = *self.error_count.read().await;
        let cache_stats = self.cache_stats.read().await.clone();

        // 计算查询统计
        let total_queries = query_times.len() as u64;
        let successful_queries = total_queries.saturating_sub(error_count);
        let failed_queries = error_count;

        let avg_query_time = if !query_times.is_empty() {
            query_times.iter().sum::<f64>() / query_times.len() as f64
        } else {
            0.0
        };

        let (p95_time, p99_time) = calculate_percentiles(&query_times);

        let uptime = self.start_time.elapsed().as_secs();
        let queries_per_second = if uptime > 0 {
            total_queries as f64 / uptime as f64
        } else {
            0.0
        };

        // 估算内存使用
        let memory_usage = estimate_memory_usage(&cache_stats).await;

        RoomSystemMetrics {
            system: SystemMetrics {
                memory_usage_mb: memory_usage,
                cpu_usage_percent: get_cpu_usage().await,
                cache_hit_rate: cache_stats.geometry_cache_hit_rate,
                active_queries: 0, // TODO: 实现活跃查询计数
                total_queries,
                avg_query_time_ms: avg_query_time,
                error_rate: if total_queries > 0 {
                    failed_queries as f64 / total_queries as f64
                } else {
                    0.0
                },
                timestamp: SystemTime::now(),
            },
            cache: cache_stats,
            query: QueryMetrics {
                total_queries,
                successful_queries,
                failed_queries,
                avg_query_time_ms: avg_query_time,
                p95_query_time_ms: p95_time,
                p99_query_time_ms: p99_time,
                queries_per_second,
            },
            spatial_index: get_spatial_index_metrics().await,
            uptime_seconds: uptime,
        }
    }

    /// 记录指标到历史记录
    pub async fn record_metrics(&self) {
        let current_metrics = self.get_current_metrics().await;
        let mut history = self.metrics_history.write().await;

        history.push(current_metrics);

        // 清理过期记录
        let cutoff_time = SystemTime::now() - self.metrics_retention_duration;
        history.retain(|m| m.system.timestamp > cutoff_time);

        // 限制历史记录大小
        if history.len() > self.max_history_size {
            let drain_count = history.len() - self.max_history_size;
            history.drain(0..drain_count);
        }
    }

    /// 获取历史指标
    pub async fn get_metrics_history(&self) -> Vec<RoomSystemMetrics> {
        let history = self.metrics_history.read().await;
        history.clone()
    }

    /// 检查系统健康状态
    pub async fn check_system_health(&self) -> SystemHealthStatus {
        let metrics = self.get_current_metrics().await;

        let mut issues = Vec::new();
        let mut warnings = Vec::new();

        // 检查内存使用
        if metrics.system.memory_usage_mb > 2048.0 {
            issues.push("内存使用过高 (>2GB)".to_string());
        } else if metrics.system.memory_usage_mb > 1024.0 {
            warnings.push("内存使用较高 (>1GB)".to_string());
        }

        // 检查错误率
        if metrics.system.error_rate > 0.1 {
            issues.push(format!(
                "错误率过高: {:.2}%",
                metrics.system.error_rate * 100.0
            ));
        } else if metrics.system.error_rate > 0.05 {
            warnings.push(format!(
                "错误率较高: {:.2}%",
                metrics.system.error_rate * 100.0
            ));
        }

        // 检查查询性能
        if metrics.query.avg_query_time_ms > 5000.0 {
            issues.push("平均查询时间过长 (>5s)".to_string());
        } else if metrics.query.avg_query_time_ms > 1000.0 {
            warnings.push("平均查询时间较长 (>1s)".to_string());
        }

        // 检查缓存命中率
        if metrics.cache.geometry_cache_hit_rate < 0.5 {
            warnings.push("几何缓存命中率较低 (<50%)".to_string());
        }

        let status = if !issues.is_empty() {
            HealthLevel::Critical
        } else if !warnings.is_empty() {
            HealthLevel::Warning
        } else {
            HealthLevel::Healthy
        };

        SystemHealthStatus {
            level: status,
            issues,
            warnings,
            metrics,
            timestamp: SystemTime::now(),
        }
    }

    /// 启动定期监控任务
    pub fn start_monitoring_task(self: Arc<Self>, interval: Duration) {
        let monitor = Arc::clone(&self);

        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);

            loop {
                interval_timer.tick().await;

                // 记录指标
                monitor.record_metrics().await;

                // 检查健康状态
                let health = monitor.check_system_health().await;

                match health.level {
                    HealthLevel::Critical => {
                        warn!("房间系统健康状态: 严重 - {:?}", health.issues);
                    }
                    HealthLevel::Warning => {
                        info!("房间系统健康状态: 警告 - {:?}", health.warnings);
                    }
                    HealthLevel::Healthy => {
                        debug!("房间系统健康状态: 正常");
                    }
                }
            }
        });
    }
}

/// 系统健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealthStatus {
    pub level: HealthLevel,
    pub issues: Vec<String>,
    pub warnings: Vec<String>,
    pub metrics: RoomSystemMetrics,
    pub timestamp: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthLevel {
    Healthy,
    Warning,
    Critical,
}

/// 计算百分位数
fn calculate_percentiles(values: &[f64]) -> (f64, f64) {
    if values.is_empty() {
        return (0.0, 0.0);
    }

    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let p95_idx = (sorted.len() as f64 * 0.95) as usize;
    let p99_idx = (sorted.len() as f64 * 0.99) as usize;

    let p95 = sorted
        .get(p95_idx.min(sorted.len() - 1))
        .copied()
        .unwrap_or(0.0);
    let p99 = sorted
        .get(p99_idx.min(sorted.len() - 1))
        .copied()
        .unwrap_or(0.0);

    (p95, p99)
}

/// 估算内存使用量
async fn estimate_memory_usage(cache_stats: &CacheMetrics) -> f64 {
    // 基础内存使用
    let base_memory = 50.0; // MB

    // 缓存内存使用
    let cache_memory = cache_stats.total_cache_memory_mb;

    // 查询历史内存使用
    let query_history_memory = 10.0; // MB

    base_memory + cache_memory + query_history_memory
}

/// 获取CPU使用率（简化实现）
async fn get_cpu_usage() -> f64 {
    // 在实际实现中，这里应该调用系统API获取真实的CPU使用率
    // 这里返回一个模拟值
    0.0
}

/// 获取空间索引指标
async fn get_spatial_index_metrics() -> SpatialIndexMetrics {
    #[cfg(all(not(target_arch = "wasm32"), feature = "sqlite"))]
    {
        use crate::spatial::hybrid_index::get_hybrid_index;

        if let Ok(index) = tokio::time::timeout(Duration::from_secs(1), get_hybrid_index()).await {
            let stats = index.get_stats().await;
            return SpatialIndexMetrics {
                memory_index_size: stats.memory_elements,
                sqlite_index_size: stats.sqlite_elements,
                index_hit_rate: stats.cache_hit_rate as f64,
                last_rebuild_time: stats.last_rebuild_time,
                rebuild_count: 0, // TODO: 实现重建计数
                index_memory_mb: stats.memory_elements as f64 * 0.001, // 估算
            };
        }
    }

    // 默认值
    SpatialIndexMetrics {
        memory_index_size: 0,
        sqlite_index_size: 0,
        index_hit_rate: 0.0,
        last_rebuild_time: SystemTime::now(),
        rebuild_count: 0,
        index_memory_mb: 0.0,
    }
}

/// 全局监控器实例
static GLOBAL_MONITOR: tokio::sync::OnceCell<Arc<RoomSystemMonitor>> =
    tokio::sync::OnceCell::const_new();

/// 获取全局监控器实例
pub async fn get_global_monitor() -> &'static Arc<RoomSystemMonitor> {
    GLOBAL_MONITOR
        .get_or_init(|| async {
            let monitor = Arc::new(RoomSystemMonitor::new());

            // 启动定期监控任务
            monitor
                .clone()
                .start_monitoring_task(Duration::from_secs(30));

            monitor
        })
        .await
}

/// 便捷函数：记录查询时间
pub async fn record_query_time(duration: Duration, success: bool) {
    let monitor = get_global_monitor().await;
    monitor.record_query_time(duration, success).await;
}

/// 便捷函数：获取当前系统指标
pub async fn get_current_system_metrics() -> RoomSystemMetrics {
    let monitor = get_global_monitor().await;
    monitor.get_current_metrics().await
}

/// 便捷函数：检查系统健康状态
pub async fn check_system_health() -> SystemHealthStatus {
    let monitor = get_global_monitor().await;
    monitor.check_system_health().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_monitor_creation() {
        let monitor = RoomSystemMonitor::new();
        let metrics = monitor.get_current_metrics().await;
        assert_eq!(metrics.query.total_queries, 0);
    }

    #[tokio::test]
    async fn test_query_time_recording() {
        let monitor = RoomSystemMonitor::new();

        monitor
            .record_query_time(Duration::from_millis(100), true)
            .await;
        monitor
            .record_query_time(Duration::from_millis(200), false)
            .await;

        let metrics = monitor.get_current_metrics().await;
        assert_eq!(metrics.query.total_queries, 2);
        assert_eq!(metrics.query.successful_queries, 1);
        assert_eq!(metrics.query.failed_queries, 1);
    }

    #[tokio::test]
    async fn test_percentile_calculation() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let (p95, p99) = calculate_percentiles(&values);
        assert!(p95 >= 9.0);
        assert!(p99 >= 9.0);
    }

    #[tokio::test]
    async fn test_health_check() {
        let monitor = RoomSystemMonitor::new();
        let health = monitor.check_system_health().await;

        // 新创建的监控器应该是健康的
        matches!(health.level, HealthLevel::Healthy);
    }
}
