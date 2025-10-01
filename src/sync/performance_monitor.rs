//! 性能监控模块
//!
//! 提供同步性能监控和指标收集

use super::SyncStatistics;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::RwLock;

/// 性能监控器
pub struct PerformanceMonitor {
    /// 历史统计数据
    history: Arc<RwLock<VecDeque<SyncStatistics>>>,
    /// 最大历史记录数
    max_history: usize,
    /// 监控开始时间
    start_time: Instant,
    /// 实时指标
    metrics: Arc<RwLock<Metrics>>,
}

/// 实时性能指标
#[derive(Debug, Clone, Default)]
pub struct Metrics {
    /// 总处理记录数
    pub total_processed: u64,
    /// 成功记录数
    pub total_success: u64,
    /// 失败记录数
    pub total_failed: u64,
    /// 平均处理时间（毫秒）
    pub avg_processing_time_ms: f64,
    /// 当前吞吐量（记录/秒）
    pub current_throughput: f64,
    /// 峰值吞吐量（记录/秒）
    pub peak_throughput: f64,
    /// 错误率
    pub error_rate: f64,
    /// 最后更新时间
    pub last_update: Option<SystemTime>,
}

impl PerformanceMonitor {
    /// 创建新的性能监控器
    pub fn new(max_history: usize) -> Self {
        Self {
            history: Arc::new(RwLock::new(VecDeque::with_capacity(max_history))),
            max_history,
            start_time: Instant::now(),
            metrics: Arc::new(RwLock::new(Metrics::default())),
        }
    }

    /// 记录同步统计
    pub async fn record(&self, stats: SyncStatistics) {
        // 更新历史记录
        {
            let mut history = self.history.write().await;
            if history.len() >= self.max_history {
                history.pop_front();
            }
            history.push_back(stats.clone());
        }

        // 更新实时指标
        self.update_metrics(stats).await;
    }

    /// 更新实时指标
    async fn update_metrics(&self, stats: SyncStatistics) {
        let mut metrics = self.metrics.write().await;

        // 累加统计
        metrics.total_processed += stats.total_records as u64;
        metrics.total_success += stats.successful_records as u64;
        metrics.total_failed += stats.failed_records as u64;

        // 计算错误率
        if metrics.total_processed > 0 {
            metrics.error_rate = metrics.total_failed as f64 / metrics.total_processed as f64;
        }

        // 计算处理时间
        if let (Some(start), Some(end)) = (stats.start_time, stats.end_time) {
            if let Ok(duration) = end.duration_since(start) {
                let processing_time_ms = duration.as_millis() as f64;

                // 更新平均处理时间（指数移动平均）
                if metrics.avg_processing_time_ms == 0.0 {
                    metrics.avg_processing_time_ms = processing_time_ms;
                } else {
                    metrics.avg_processing_time_ms =
                        metrics.avg_processing_time_ms * 0.9 + processing_time_ms * 0.1;
                }

                // 计算吞吐量
                if processing_time_ms > 0.0 {
                    let throughput = (stats.total_records as f64 * 1000.0) / processing_time_ms;
                    metrics.current_throughput = throughput;

                    if throughput > metrics.peak_throughput {
                        metrics.peak_throughput = throughput;
                    }
                }
            }
        }

        metrics.last_update = Some(SystemTime::now());
    }

    /// 获取当前指标
    pub async fn get_metrics(&self) -> Metrics {
        self.metrics.read().await.clone()
    }

    /// 获取历史统计
    pub async fn get_history(&self) -> Vec<SyncStatistics> {
        self.history.read().await.iter().cloned().collect()
    }

    /// 获取运行时长
    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// 生成性能报告
    pub async fn generate_report(&self) -> PerformanceReport {
        let metrics = self.get_metrics().await;
        let history = self.get_history().await;
        let uptime = self.uptime();

        PerformanceReport {
            uptime_seconds: uptime.as_secs(),
            total_processed: metrics.total_processed,
            total_success: metrics.total_success,
            total_failed: metrics.total_failed,
            avg_processing_time_ms: metrics.avg_processing_time_ms,
            current_throughput: metrics.current_throughput,
            peak_throughput: metrics.peak_throughput,
            error_rate: metrics.error_rate,
            history_count: history.len(),
            avg_success_rate: Self::calculate_avg_success_rate(&history),
        }
    }

    /// 计算平均成功率
    fn calculate_avg_success_rate(history: &[SyncStatistics]) -> f64 {
        if history.is_empty() {
            return 0.0;
        }

        let total: f64 = history.iter().map(|s| s.success_rate()).sum();

        total / history.len() as f64
    }

    /// 检测性能异常
    pub async fn detect_anomalies(&self) -> Vec<Anomaly> {
        let metrics = self.get_metrics().await;
        let mut anomalies = Vec::new();

        // 检查错误率
        if metrics.error_rate > 0.1 {
            anomalies.push(Anomaly {
                severity: Severity::High,
                message: format!("错误率过高: {:.2}%", metrics.error_rate * 100.0),
                timestamp: SystemTime::now(),
            });
        }

        // 检查吞吐量
        if metrics.current_throughput < metrics.peak_throughput * 0.5 {
            anomalies.push(Anomaly {
                severity: Severity::Medium,
                message: format!(
                    "吞吐量下降: 当前 {:.2}/s, 峰值 {:.2}/s",
                    metrics.current_throughput, metrics.peak_throughput
                ),
                timestamp: SystemTime::now(),
            });
        }

        // 检查处理时间
        if metrics.avg_processing_time_ms > 1000.0 {
            anomalies.push(Anomaly {
                severity: Severity::Low,
                message: format!("处理时间过长: {:.2}ms", metrics.avg_processing_time_ms),
                timestamp: SystemTime::now(),
            });
        }

        anomalies
    }
}

/// 性能报告
#[derive(Debug, Clone)]
pub struct PerformanceReport {
    /// 运行时长（秒）
    pub uptime_seconds: u64,
    /// 总处理记录数
    pub total_processed: u64,
    /// 成功记录数
    pub total_success: u64,
    /// 失败记录数
    pub total_failed: u64,
    /// 平均处理时间（毫秒）
    pub avg_processing_time_ms: f64,
    /// 当前吞吐量（记录/秒）
    pub current_throughput: f64,
    /// 峰值吞吐量（记录/秒）
    pub peak_throughput: f64,
    /// 错误率
    pub error_rate: f64,
    /// 历史记录数
    pub history_count: usize,
    /// 平均成功率
    pub avg_success_rate: f64,
}

impl PerformanceReport {
    /// 格式化输出报告
    pub fn format(&self) -> String {
        format!(
            r#"
=== 同步性能报告 ===
运行时长: {} 秒
总处理记录: {}
成功记录: {} ({:.2}%)
失败记录: {} ({:.2}%)
平均处理时间: {:.2} ms
当前吞吐量: {:.2} 记录/秒
峰值吞吐量: {:.2} 记录/秒
平均成功率: {:.2}%
历史样本数: {}
"#,
            self.uptime_seconds,
            self.total_processed,
            self.total_success,
            (self.total_success as f64 / self.total_processed.max(1) as f64) * 100.0,
            self.total_failed,
            self.error_rate * 100.0,
            self.avg_processing_time_ms,
            self.current_throughput,
            self.peak_throughput,
            self.avg_success_rate,
            self.history_count
        )
    }
}

/// 异常信息
#[derive(Debug, Clone)]
pub struct Anomaly {
    /// 严重程度
    pub severity: Severity,
    /// 异常消息
    pub message: String,
    /// 时间戳
    pub timestamp: SystemTime,
}

/// 严重程度
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

/// Prometheus 指标导出器
pub struct PrometheusExporter {
    monitor: Arc<PerformanceMonitor>,
}

impl PrometheusExporter {
    /// 创建导出器
    pub fn new(monitor: Arc<PerformanceMonitor>) -> Self {
        Self { monitor }
    }

    /// 导出 Prometheus 格式的指标
    pub async fn export(&self) -> String {
        let metrics = self.monitor.get_metrics().await;
        let uptime = self.monitor.uptime();

        format!(
            r#"# HELP sync_total_processed Total number of processed records
# TYPE sync_total_processed counter
sync_total_processed {}

# HELP sync_total_success Total number of successful records
# TYPE sync_total_success counter
sync_total_success {}

# HELP sync_total_failed Total number of failed records
# TYPE sync_total_failed counter
sync_total_failed {}

# HELP sync_error_rate Current error rate
# TYPE sync_error_rate gauge
sync_error_rate {}

# HELP sync_avg_processing_time_ms Average processing time in milliseconds
# TYPE sync_avg_processing_time_ms gauge
sync_avg_processing_time_ms {}

# HELP sync_current_throughput Current throughput in records per second
# TYPE sync_current_throughput gauge
sync_current_throughput {}

# HELP sync_peak_throughput Peak throughput in records per second
# TYPE sync_peak_throughput gauge
sync_peak_throughput {}

# HELP sync_uptime_seconds Uptime in seconds
# TYPE sync_uptime_seconds counter
sync_uptime_seconds {}
"#,
            metrics.total_processed,
            metrics.total_success,
            metrics.total_failed,
            metrics.error_rate,
            metrics.avg_processing_time_ms,
            metrics.current_throughput,
            metrics.peak_throughput,
            uptime.as_secs()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_performance_monitor() {
        let monitor = PerformanceMonitor::new(100);

        let mut stats = SyncStatistics::default();
        stats.total_records = 100;
        stats.successful_records = 90;
        stats.failed_records = 10;
        stats.start_time = Some(SystemTime::now());
        stats.end_time = Some(SystemTime::now());

        monitor.record(stats).await;

        let metrics = monitor.get_metrics().await;
        assert_eq!(metrics.total_processed, 100);
        assert_eq!(metrics.total_success, 90);
        assert_eq!(metrics.total_failed, 10);
        assert!(metrics.error_rate > 0.0);
    }

    #[tokio::test]
    async fn test_anomaly_detection() {
        let monitor = PerformanceMonitor::new(100);

        let mut stats = SyncStatistics::default();
        stats.total_records = 100;
        stats.successful_records = 50;
        stats.failed_records = 50;

        monitor.record(stats).await;

        let anomalies = monitor.detect_anomalies().await;
        assert!(!anomalies.is_empty());
    }

    #[test]
    fn test_performance_report() {
        let report = PerformanceReport {
            uptime_seconds: 3600,
            total_processed: 10000,
            total_success: 9500,
            total_failed: 500,
            avg_processing_time_ms: 50.0,
            current_throughput: 100.0,
            peak_throughput: 150.0,
            error_rate: 0.05,
            history_count: 100,
            avg_success_rate: 95.0,
        };

        let formatted = report.format();
        assert!(formatted.contains("运行时长: 3600"));
        assert!(formatted.contains("总处理记录: 10000"));
    }
}
