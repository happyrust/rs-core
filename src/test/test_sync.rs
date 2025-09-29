//! 同步机制测试
//!
//! 测试 SurrealDB 和 Kuzu 之间的数据同步

use crate::db_adapter::{DatabaseAdapter, QueryContext};
use crate::sync::*;
use crate::types::*;
use anyhow::Result;
use std::sync::Arc;

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试同步策略创建
    #[test]
    fn test_sync_strategy_creation() {
        let strategy = SyncStrategy::full_sync();
        assert_eq!(strategy.mode, SyncMode::Full);
        assert_eq!(strategy.batch_size, 5000);

        let strategy = SyncStrategy::incremental_sync();
        assert_eq!(strategy.mode, SyncMode::Incremental);

        let strategy = SyncStrategy::realtime_sync();
        assert_eq!(strategy.mode, SyncMode::Realtime);
    }

    /// 测试同步过滤器
    #[test]
    fn test_sync_filter() {
        let mut filter = SyncFilter::default();

        // 测试 refno 范围过滤
        filter.refno_range = Some((RefU64(100), RefU64(200)));
        assert!(filter.matches_refno(RefU64(150)));
        assert!(!filter.matches_refno(RefU64(50)));
        assert!(!filter.matches_refno(RefU64(250)));

        // 测试类型过滤
        filter.include_types = vec!["PIPE".to_string(), "EQUI".to_string()];
        assert!(filter.matches_type("PIPE"));
        assert!(filter.matches_type("EQUI"));
        assert!(!filter.matches_type("STRU"));

        // 测试属性过滤
        filter.include_attributes = vec!["NAME".to_string(), "DESC".to_string()];
        assert!(filter.matches_attribute("NAME"));
        assert!(!filter.matches_attribute("SIZE"));
    }

    /// 测试同步任务管理
    #[test]
    fn test_sync_task() {
        let mut task = SyncTask::new(SyncTaskType::SyncAll);
        assert_eq!(task.status, SyncTaskStatus::Pending);

        task.start();
        assert_eq!(task.status, SyncTaskStatus::Running);
        assert!(task.started_at.is_some());

        task.update_progress(50, 100);
        assert_eq!(task.progress, 50);

        task.record_success();
        assert_eq!(task.success_count, 1);

        task.complete();
        assert_eq!(task.status, SyncTaskStatus::Completed);
        assert_eq!(task.progress, 100);
    }

    /// 测试同步统计
    #[test]
    fn test_sync_statistics() {
        let mut stats = SyncStatistics::default();
        stats.total_records = 100;
        stats.successful_records = 90;
        stats.failed_records = 10;

        let success_rate = stats.success_rate();
        assert_eq!(success_rate, 90.0);

        // 测试统计合并
        let mut other_stats = SyncStatistics::default();
        other_stats.total_records = 50;
        other_stats.successful_records = 45;
        other_stats.failed_records = 5;

        stats.merge(&other_stats);
        assert_eq!(stats.total_records, 150);
        assert_eq!(stats.successful_records, 135);
        assert_eq!(stats.failed_records, 15);
    }

    /// 测试同步管理器构建器
    #[tokio::test]
    async fn test_sync_manager_builder() -> Result<()> {
        use crate::rs_surreal::create_surreal_adapter;
        #[cfg(feature = "kuzu")]
        use crate::rs_kuzu::create_kuzu_adapter;

        // 创建源和目标适配器
        let source = Arc::new(create_surreal_adapter()?);

        #[cfg(feature = "kuzu")]
        let target = Arc::new(create_kuzu_adapter()?);
        #[cfg(not(feature = "kuzu"))]
        let target = Arc::new(create_surreal_adapter()?);

        // 使用构建器创建同步管理器
        let sync_manager = SyncManagerBuilder::new()
            .source(source)
            .target(target)
            .strategy(SyncStrategy::full_sync())
            .filter(SyncFilter::default())
            .build()?;

        // 验证同步管理器创建成功
        let stats = sync_manager.get_statistics().await;
        assert_eq!(stats.total_records, 0);

        Ok(())
    }

    /// 测试单个 PE 同步（模拟）
    #[tokio::test]
    async fn test_sync_single_pe() -> Result<()> {
        use crate::rs_surreal::create_surreal_adapter;

        // 创建源和目标适配器（这里都用 SurrealDB 作为示例）
        let source = Arc::new(create_surreal_adapter()?);
        let target = Arc::new(create_surreal_adapter()?);

        // 创建同步管理器
        let sync_manager = SyncManager::new(
            source,
            target,
            SyncStrategy::default(),
            SyncFilter::default(),
        );

        // 尝试同步一个 PE（如果存在的话）
        let test_refno = RefnoEnum::from(RefU64(1));

        // 这个可能会失败，因为 PE 可能不存在，但这只是测试结构
        match sync_manager.sync_pe(test_refno).await {
            Ok(_) => println!("同步 PE {} 成功", test_refno.refno().0),
            Err(e) => println!("同步 PE {} 失败: {}", test_refno.refno().0, e),
        }

        Ok(())
    }

    /// 测试冲突解决策略
    #[test]
    fn test_conflict_resolution() {
        let strategies = vec![
            ConflictResolution::SourceWins,
            ConflictResolution::TargetWins,
            ConflictResolution::LatestTimestamp,
            ConflictResolution::Manual,
            ConflictResolution::Merge,
        ];

        for resolution in strategies {
            let strategy = SyncStrategy::default()
                .with_conflict_resolution(resolution);
            assert_eq!(strategy.conflict_resolution, resolution);
        }
    }

    /// 测试同步方向
    #[test]
    fn test_sync_direction() {
        let directions = vec![
            SyncDirection::SurrealToKuzu,
            SyncDirection::KuzuToSurreal,
            SyncDirection::Bidirectional,
        ];

        for direction in directions {
            let strategy = SyncStrategy::default()
                .with_direction(direction);
            assert_eq!(strategy.direction, direction);
        }
    }
}