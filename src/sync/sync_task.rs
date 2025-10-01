//! 同步任务定义
//!
//! 定义同步任务和状态

use crate::types::*;
use std::time::{Duration, SystemTime};

/// 同步任务状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncTaskStatus {
    /// 待处理
    Pending,
    /// 运行中
    Running,
    /// 已完成
    Completed,
    /// 失败
    Failed,
    /// 已取消
    Cancelled,
    /// 暂停
    Paused,
}

/// 同步任务类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncTaskType {
    /// 同步单个PE
    SyncPE(RefnoEnum),
    /// 同步多个PE
    SyncBatchPE(Vec<RefnoEnum>),
    /// 同步属性
    SyncAttributes(RefnoEnum),
    /// 同步关系
    SyncRelations(RefnoEnum),
    /// 同步子树
    SyncSubtree(RefnoEnum, u32),
    /// 全量同步
    SyncAll,
}

/// 同步任务
#[derive(Debug, Clone)]
pub struct SyncTask {
    /// 任务ID
    pub id: String,
    /// 任务类型
    pub task_type: SyncTaskType,
    /// 任务状态
    pub status: SyncTaskStatus,
    /// 创建时间
    pub created_at: SystemTime,
    /// 开始时间
    pub started_at: Option<SystemTime>,
    /// 完成时间
    pub completed_at: Option<SystemTime>,
    /// 进度 (0-100)
    pub progress: u8,
    /// 已处理项目数
    pub processed_count: usize,
    /// 总项目数
    pub total_count: usize,
    /// 成功项目数
    pub success_count: usize,
    /// 失败项目数
    pub failure_count: usize,
    /// 错误信息
    pub error_message: Option<String>,
    /// 重试次数
    pub retry_count: u32,
}

impl SyncTask {
    /// 创建新的同步任务
    pub fn new(task_type: SyncTaskType) -> Self {
        use uuid::Uuid;

        Self {
            id: Uuid::new_v4().to_string(),
            task_type,
            status: SyncTaskStatus::Pending,
            created_at: SystemTime::now(),
            started_at: None,
            completed_at: None,
            progress: 0,
            processed_count: 0,
            total_count: 0,
            success_count: 0,
            failure_count: 0,
            error_message: None,
            retry_count: 0,
        }
    }

    /// 开始任务
    pub fn start(&mut self) {
        self.status = SyncTaskStatus::Running;
        self.started_at = Some(SystemTime::now());
    }

    /// 更新进度
    pub fn update_progress(&mut self, processed: usize, total: usize) {
        self.processed_count = processed;
        self.total_count = total;
        if total > 0 {
            self.progress = ((processed as f64 / total as f64) * 100.0) as u8;
        }
    }

    /// 记录成功
    pub fn record_success(&mut self) {
        self.success_count += 1;
        self.processed_count += 1;
        self.update_progress(self.processed_count, self.total_count);
    }

    /// 记录失败
    pub fn record_failure(&mut self, error: String) {
        self.failure_count += 1;
        self.processed_count += 1;
        self.error_message = Some(error);
        self.update_progress(self.processed_count, self.total_count);
    }

    /// 完成任务
    pub fn complete(&mut self) {
        self.status = SyncTaskStatus::Completed;
        self.completed_at = Some(SystemTime::now());
        self.progress = 100;
    }

    /// 失败任务
    pub fn fail(&mut self, error: String) {
        self.status = SyncTaskStatus::Failed;
        self.completed_at = Some(SystemTime::now());
        self.error_message = Some(error);
    }

    /// 取消任务
    pub fn cancel(&mut self) {
        self.status = SyncTaskStatus::Cancelled;
        self.completed_at = Some(SystemTime::now());
    }

    /// 暂停任务
    pub fn pause(&mut self) {
        self.status = SyncTaskStatus::Paused;
    }

    /// 恢复任务
    pub fn resume(&mut self) {
        self.status = SyncTaskStatus::Running;
    }

    /// 获取运行时长
    pub fn duration(&self) -> Option<Duration> {
        if let Some(start) = self.started_at {
            let end = self.completed_at.unwrap_or_else(SystemTime::now);
            end.duration_since(start).ok()
        } else {
            None
        }
    }

    /// 是否可以重试
    pub fn can_retry(&self, max_retries: u32) -> bool {
        self.status == SyncTaskStatus::Failed && self.retry_count < max_retries
    }

    /// 重试任务
    pub fn retry(&mut self) {
        self.retry_count += 1;
        self.status = SyncTaskStatus::Pending;
        self.error_message = None;
    }
}

/// 同步结果统计
#[derive(Debug, Clone, Default)]
pub struct SyncStatistics {
    /// 总同步任务数
    pub total_tasks: usize,
    /// 成功任务数
    pub successful_tasks: usize,
    /// 失败任务数
    pub failed_tasks: usize,
    /// 总处理记录数
    pub total_records: usize,
    /// 成功记录数
    pub successful_records: usize,
    /// 失败记录数
    pub failed_records: usize,
    /// 跳过记录数
    pub skipped_records: usize,
    /// 总耗时
    pub total_duration: Duration,
    /// 开始时间
    pub start_time: Option<SystemTime>,
    /// 结束时间
    pub end_time: Option<SystemTime>,
}

impl SyncStatistics {
    /// 合并统计信息
    pub fn merge(&mut self, other: &SyncStatistics) {
        self.total_tasks += other.total_tasks;
        self.successful_tasks += other.successful_tasks;
        self.failed_tasks += other.failed_tasks;
        self.total_records += other.total_records;
        self.successful_records += other.successful_records;
        self.failed_records += other.failed_records;
        self.skipped_records += other.skipped_records;
        self.total_duration += other.total_duration;

        // 更新开始和结束时间
        if let Some(other_start) = other.start_time {
            self.start_time = Some(match self.start_time {
                Some(current) => current.min(other_start),
                None => other_start,
            });
        }

        if let Some(other_end) = other.end_time {
            self.end_time = Some(match self.end_time {
                Some(current) => current.max(other_end),
                None => other_end,
            });
        }
    }

    /// 计算成功率
    pub fn success_rate(&self) -> f64 {
        if self.total_records > 0 {
            (self.successful_records as f64 / self.total_records as f64) * 100.0
        } else {
            0.0
        }
    }
}
