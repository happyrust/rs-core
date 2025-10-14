//! 同步策略定义
//!
//! 定义不同的数据同步策略

use crate::types::*;
use std::time::Duration;

/// 同步方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncDirection {
    /// 从源数据库同步到目标数据库
    SourceToTarget,
    /// 从目标数据库同步回源数据库
    TargetToSource,
    /// 双向同步
    Bidirectional,
}

/// 同步模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncMode {
    /// 全量同步
    Full,
    /// 增量同步
    Incremental,
    /// 实时同步
    Realtime,
    /// 按需同步
    OnDemand,
}

/// 冲突解决策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictResolution {
    /// 源数据库优先
    SourceWins,
    /// 目标数据库优先
    TargetWins,
    /// 最新时间戳优先
    LatestTimestamp,
    /// 手动解决
    Manual,
    /// 合并
    Merge,
}

/// 同步策略配置
#[derive(Debug, Clone)]
pub struct SyncStrategy {
    /// 同步方向
    pub direction: SyncDirection,
    /// 同步模式
    pub mode: SyncMode,
    /// 冲突解决策略
    pub conflict_resolution: ConflictResolution,
    /// 批次大小
    pub batch_size: usize,
    /// 同步间隔（定时同步时使用）
    pub sync_interval: Duration,
    /// 是否在错误时继续
    pub continue_on_error: bool,
    /// 重试次数
    pub retry_count: u32,
    /// 重试延迟
    pub retry_delay: Duration,
}

impl Default for SyncStrategy {
    fn default() -> Self {
        Self {
            direction: SyncDirection::SourceToTarget,
            mode: SyncMode::Incremental,
            conflict_resolution: ConflictResolution::SourceWins,
            batch_size: 1000,
            sync_interval: Duration::from_secs(60),
            continue_on_error: true,
            retry_count: 3,
            retry_delay: Duration::from_secs(1),
        }
    }
}

impl SyncStrategy {
    /// 创建全量同步策略
    pub fn full_sync() -> Self {
        Self {
            mode: SyncMode::Full,
            batch_size: 5000,
            ..Default::default()
        }
    }

    /// 创建增量同步策略
    pub fn incremental_sync() -> Self {
        Self {
            mode: SyncMode::Incremental,
            ..Default::default()
        }
    }

    /// 创建实时同步策略
    pub fn realtime_sync() -> Self {
        Self {
            mode: SyncMode::Realtime,
            sync_interval: Duration::from_millis(100),
            batch_size: 100,
            ..Default::default()
        }
    }

    /// 设置同步方向
    pub fn with_direction(mut self, direction: SyncDirection) -> Self {
        self.direction = direction;
        self
    }

    /// 设置冲突解决策略
    pub fn with_conflict_resolution(mut self, resolution: ConflictResolution) -> Self {
        self.conflict_resolution = resolution;
        self
    }
}

/// 同步过滤器
#[derive(Debug, Clone)]
pub struct SyncFilter {
    /// 要包含的元素类型
    pub include_types: Vec<String>,
    /// 要排除的元素类型
    pub exclude_types: Vec<String>,
    /// 要包含的 refno 范围
    pub refno_range: Option<(RefU64, RefU64)>,
    /// 要包含的属性名
    pub include_attributes: Vec<String>,
    /// 要排除的属性名
    pub exclude_attributes: Vec<String>,
    /// 修改时间范围
    pub modified_after: Option<std::time::SystemTime>,
    pub modified_before: Option<std::time::SystemTime>,
}

impl Default for SyncFilter {
    fn default() -> Self {
        Self {
            include_types: vec![],
            exclude_types: vec![],
            refno_range: None,
            include_attributes: vec![],
            exclude_attributes: vec![],
            modified_after: None,
            modified_before: None,
        }
    }
}

impl SyncFilter {
    /// 检查 refno 是否符合过滤条件
    pub fn matches_refno(&self, refno: RefU64) -> bool {
        if let Some((min, max)) = self.refno_range {
            refno.0 >= min.0 && refno.0 <= max.0
        } else {
            true
        }
    }

    /// 检查类型是否符合过滤条件
    pub fn matches_type(&self, element_type: &str) -> bool {
        // 如果有排除列表，检查是否在其中
        if !self.exclude_types.is_empty() && self.exclude_types.contains(&element_type.to_string())
        {
            return false;
        }

        // 如果有包含列表，检查是否在其中
        if !self.include_types.is_empty() {
            return self.include_types.contains(&element_type.to_string());
        }

        true
    }

    /// 检查属性是否符合过滤条件
    pub fn matches_attribute(&self, attr_name: &str) -> bool {
        // 如果有排除列表，检查是否在其中
        if !self.exclude_attributes.is_empty()
            && self.exclude_attributes.contains(&attr_name.to_string())
        {
            return false;
        }

        // 如果有包含列表，检查是否在其中
        if !self.include_attributes.is_empty() {
            return self.include_attributes.contains(&attr_name.to_string());
        }

        true
    }
}
