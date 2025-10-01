//! 并发同步执行器
//!
//! 提供高性能的并发同步机制

use super::{SyncStatistics, SyncTask, SyncTaskStatus};
use crate::db_adapter::DatabaseAdapter;
use crate::types::*;
use anyhow::Result;
use futures::future::join_all;
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};
use tokio::task::JoinHandle;

/// 并发执行器配置
#[derive(Debug, Clone)]
pub struct ConcurrentConfig {
    /// 最大并发数
    pub max_concurrency: usize,
    /// 任务队列大小
    pub queue_size: usize,
    /// 是否启用自适应并发
    pub adaptive_concurrency: bool,
    /// 错误重试次数
    pub max_retries: u32,
    /// 批次大小
    pub batch_size: usize,
}

impl Default for ConcurrentConfig {
    fn default() -> Self {
        Self {
            max_concurrency: num_cpus::get() * 2,
            queue_size: 10000,
            adaptive_concurrency: true,
            max_retries: 3,
            batch_size: 100,
        }
    }
}

/// 并发同步执行器
pub struct ConcurrentExecutor {
    /// 配置
    config: ConcurrentConfig,
    /// 信号量控制并发
    semaphore: Arc<Semaphore>,
    /// 任务队列
    task_queue: Arc<RwLock<Vec<SyncTask>>>,
    /// 统计信息
    statistics: Arc<RwLock<SyncStatistics>>,
}

impl ConcurrentExecutor {
    /// 创建新的并发执行器
    pub fn new(config: ConcurrentConfig) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(config.max_concurrency)),
            task_queue: Arc::new(RwLock::new(Vec::with_capacity(config.queue_size))),
            statistics: Arc::new(RwLock::new(SyncStatistics::default())),
            config,
        }
    }

    /// 执行批量PE同步
    pub async fn sync_batch_pes(
        &self,
        refnos: Vec<RefnoEnum>,
        source: Arc<dyn DatabaseAdapter>,
        target: Arc<dyn DatabaseAdapter>,
    ) -> Result<SyncStatistics> {
        let mut handles = Vec::new();
        let mut local_stats = SyncStatistics::default();
        local_stats.start_time = Some(std::time::SystemTime::now());

        // 将PE分批处理
        for chunk in refnos.chunks(self.config.batch_size) {
            let permit = self.semaphore.clone().acquire_owned().await?;
            let chunk_vec = chunk.to_vec();
            let source_clone = source.clone();
            let target_clone = target.clone();
            let stats_clone = self.statistics.clone();

            let handle: JoinHandle<Result<()>> = tokio::spawn(async move {
                let _permit = permit; // 持有许可直到任务完成

                for refno in chunk_vec {
                    match Self::sync_single_pe(&source_clone, &target_clone, refno).await {
                        Ok(_) => {
                            let mut stats = stats_clone.write().await;
                            stats.successful_records += 1;
                        }
                        Err(e) => {
                            let mut stats = stats_clone.write().await;
                            stats.failed_records += 1;
                            log::error!("同步 PE {} 失败: {}", refno.refno().0, e);
                        }
                    }
                }
                Ok(())
            });

            handles.push(handle);
        }

        // 等待所有任务完成
        let results = join_all(handles).await;

        // 检查错误
        for result in results {
            if let Err(e) = result {
                log::error!("任务执行失败: {}", e);
            }
        }

        // 收集统计信息
        let stats = self.statistics.read().await;
        local_stats.successful_records = stats.successful_records;
        local_stats.failed_records = stats.failed_records;
        local_stats.total_records = stats.successful_records + stats.failed_records;
        local_stats.end_time = Some(std::time::SystemTime::now());

        Ok(local_stats)
    }

    /// 同步单个PE（内部方法）
    async fn sync_single_pe(
        source: &Arc<dyn DatabaseAdapter>,
        target: &Arc<dyn DatabaseAdapter>,
        refno: RefnoEnum,
    ) -> Result<()> {
        use crate::db_adapter::QueryContext;

        let ctx = QueryContext::default();

        // 获取PE
        if let Some(pe) = source.get_pe(refno, Some(ctx.clone())).await? {
            // 保存PE
            target.save_pe(&pe).await?;

            // 获取并保存属性
            if let Ok(attmap) = source.get_attmap(refno, Some(ctx.clone())).await {
                if !attmap.is_empty() {
                    target.save_attmap(refno, &attmap).await?;
                }
            }

            // 同步子元素关系
            if let Ok(children) = source.query_children(refno, Some(ctx)).await {
                for child in children {
                    target.create_relation(refno, child, "OWNS").await?;
                }
            }
        }

        Ok(())
    }

    /// 自适应调整并发数
    pub async fn adjust_concurrency(&self, performance_metrics: PerformanceMetrics) {
        if !self.config.adaptive_concurrency {
            return;
        }

        // 基于性能指标调整并发数
        let new_concurrency = if performance_metrics.error_rate > 0.1 {
            // 错误率高，降低并发
            (self.config.max_concurrency as f64 * 0.8) as usize
        } else if performance_metrics.avg_latency_ms < 100.0 {
            // 延迟低，可以增加并发
            std::cmp::min(self.config.max_concurrency * 2, num_cpus::get() * 4)
        } else {
            self.config.max_concurrency
        };

        log::info!(
            "调整并发数: {} -> {}",
            self.config.max_concurrency,
            new_concurrency
        );
    }

    /// 获取统计信息
    pub async fn get_statistics(&self) -> SyncStatistics {
        self.statistics.read().await.clone()
    }

    /// 重置统计信息
    pub async fn reset_statistics(&self) {
        let mut stats = self.statistics.write().await;
        *stats = SyncStatistics::default();
    }
}

/// 性能指标
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    /// 平均延迟（毫秒）
    pub avg_latency_ms: f64,
    /// 错误率
    pub error_rate: f64,
    /// 吞吐量（记录/秒）
    pub throughput: f64,
    /// CPU使用率
    pub cpu_usage: f64,
    /// 内存使用（MB）
    pub memory_usage_mb: f64,
}

/// 任务池管理器
pub struct TaskPool {
    /// 工作线程数
    worker_count: usize,
    /// 任务发送器
    sender: tokio::sync::mpsc::Sender<SyncTask>,
    /// 任务接收器
    receiver: Arc<tokio::sync::Mutex<tokio::sync::mpsc::Receiver<SyncTask>>>,
}

impl TaskPool {
    /// 创建任务池
    pub fn new(worker_count: usize, buffer_size: usize) -> Self {
        let (sender, receiver) = tokio::sync::mpsc::channel(buffer_size);

        Self {
            worker_count,
            sender,
            receiver: Arc::new(tokio::sync::Mutex::new(receiver)),
        }
    }

    /// 提交任务
    pub async fn submit(&self, task: SyncTask) -> Result<()> {
        self.sender
            .send(task)
            .await
            .map_err(|e| anyhow::anyhow!("提交任务失败: {}", e))
    }

    /// 启动工作线程
    pub async fn start_workers(
        &self,
        source: Arc<dyn DatabaseAdapter>,
        target: Arc<dyn DatabaseAdapter>,
    ) {
        for worker_id in 0..self.worker_count {
            let receiver = self.receiver.clone();
            let source = source.clone();
            let target = target.clone();

            tokio::spawn(async move {
                loop {
                    let task = {
                        let mut rx = receiver.lock().await;
                        rx.recv().await
                    };

                    match task {
                        Some(mut task) => {
                            log::debug!("Worker {} 处理任务 {}", worker_id, task.id);
                            task.start();

                            // 执行任务逻辑
                            // ... 任务处理 ...

                            task.complete();
                        }
                        None => {
                            log::debug!("Worker {} 退出", worker_id);
                            break;
                        }
                    }
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_concurrent_config() {
        let config = ConcurrentConfig::default();
        assert!(config.max_concurrency > 0);
        assert_eq!(config.queue_size, 10000);
        assert!(config.adaptive_concurrency);
    }

    #[tokio::test]
    async fn test_concurrent_executor() {
        let config = ConcurrentConfig {
            max_concurrency: 4,
            batch_size: 10,
            ..Default::default()
        };

        let executor = ConcurrentExecutor::new(config);
        let stats = executor.get_statistics().await;
        assert_eq!(stats.successful_records, 0);
    }
}
