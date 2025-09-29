//! 同步管理器
//!
//! 协调和管理数据同步过程

use super::{SyncDirection, SyncFilter, SyncMode, SyncStatistics, SyncStrategy, SyncTask, SyncTaskStatus, SyncTaskType};
use crate::db_adapter::{DatabaseAdapter, QueryContext};
use crate::types::*;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 同步管理器
pub struct SyncManager {
    /// 源数据库适配器
    source_adapter: Arc<dyn DatabaseAdapter>,
    /// 目标数据库适配器
    target_adapter: Arc<dyn DatabaseAdapter>,
    /// 同步策略
    strategy: SyncStrategy,
    /// 同步过滤器
    filter: SyncFilter,
    /// 活动任务
    active_tasks: Arc<RwLock<HashMap<String, SyncTask>>>,
    /// 统计信息
    statistics: Arc<RwLock<SyncStatistics>>,
}

impl SyncManager {
    /// 创建新的同步管理器
    pub fn new(
        source: Arc<dyn DatabaseAdapter>,
        target: Arc<dyn DatabaseAdapter>,
        strategy: SyncStrategy,
        filter: SyncFilter,
    ) -> Self {
        Self {
            source_adapter: source,
            target_adapter: target,
            strategy,
            filter,
            active_tasks: Arc::new(RwLock::new(HashMap::new())),
            statistics: Arc::new(RwLock::new(SyncStatistics::default())),
        }
    }

    /// 执行同步
    pub async fn sync(&self) -> Result<SyncStatistics> {
        match self.strategy.mode {
            SyncMode::Full => self.sync_full().await,
            SyncMode::Incremental => self.sync_incremental().await,
            SyncMode::Realtime => self.sync_realtime().await,
            SyncMode::OnDemand => Ok(SyncStatistics::default()),
        }
    }

    /// 全量同步
    async fn sync_full(&self) -> Result<SyncStatistics> {
        let mut stats = SyncStatistics::default();
        stats.start_time = Some(std::time::SystemTime::now());

        // 创建全量同步任务
        let mut task = SyncTask::new(SyncTaskType::SyncAll);
        task.start();

        // 获取所有需要同步的 PE
        let all_pes = self.get_all_pes_to_sync().await?;
        task.total_count = all_pes.len();

        // 批量同步
        for batch in all_pes.chunks(self.strategy.batch_size) {
            match self.sync_batch_pes(batch).await {
                Ok(batch_stats) => {
                    task.success_count += batch_stats.successful_records;
                    stats.merge(&batch_stats);
                }
                Err(e) => {
                    task.record_failure(format!("批量同步失败: {}", e));
                    if !self.strategy.continue_on_error {
                        task.fail(e.to_string());
                        break;
                    }
                }
            }
            task.update_progress(task.processed_count, task.total_count);
        }

        task.complete();
        stats.end_time = Some(std::time::SystemTime::now());

        Ok(stats)
    }

    /// 增量同步
    async fn sync_incremental(&self) -> Result<SyncStatistics> {
        let mut stats = SyncStatistics::default();
        stats.start_time = Some(std::time::SystemTime::now());

        // 创建增量同步任务
        let mut task = SyncTask::new(SyncTaskType::SyncAll);
        task.start();

        // 获取上次同步时间戳（从统计信息中获取）
        let last_sync = self.statistics.read().await.end_time;

        if let Some(last_sync_time) = last_sync {
            // 查询自上次同步以来有变更的 PE
            let changed_pes = self.get_changed_pes_since(last_sync_time).await?;
            task.total_count = changed_pes.len();

            log::info!("发现 {} 个变更的 PE 需要同步", changed_pes.len());

            // 批量同步变更的 PE
            for batch in changed_pes.chunks(self.strategy.batch_size) {
                match self.sync_batch_pes(batch).await {
                    Ok(batch_stats) => {
                        task.success_count += batch_stats.successful_records;
                        stats.merge(&batch_stats);
                    }
                    Err(e) => {
                        task.record_failure(format!("批量增量同步失败: {}", e));
                        if !self.strategy.continue_on_error {
                            task.fail(e.to_string());
                            break;
                        }
                    }
                }
                task.update_progress(task.processed_count, task.total_count);
            }
        } else {
            // 首次同步，执行全量同步
            log::info!("首次同步，执行全量同步");
            return self.sync_full().await;
        }

        task.complete();
        stats.end_time = Some(std::time::SystemTime::now());

        // 更新统计信息
        let mut global_stats = self.statistics.write().await;
        global_stats.merge(&stats);

        Ok(stats)
    }

    /// 实时同步
    async fn sync_realtime(&self) -> Result<SyncStatistics> {
        // TODO: 实现基于监听器的实时同步
        // 需要监听源数据库的变更事件

        log::info!("实时同步暂未实现");
        Ok(SyncStatistics::default())
    }

    /// 同步单个 PE
    pub async fn sync_pe(&self, refno: RefnoEnum) -> Result<()> {
        let ctx = QueryContext::default();

        // 从源数据库获取 PE
        let pe = self.source_adapter.get_pe(refno, Some(ctx.clone())).await?;

        if let Some(pe_data) = pe {
            // 保存到目标数据库
            self.target_adapter.save_pe(&pe_data).await?;

            // 同步属性
            self.sync_attributes(refno).await?;

            // 同步关系
            self.sync_relations(refno).await?;
        }

        Ok(())
    }

    /// 同步属性
    async fn sync_attributes(&self, refno: RefnoEnum) -> Result<()> {
        let ctx = QueryContext::default();

        // 从源数据库获取属性
        let attmap = self.source_adapter.get_attmap(refno, Some(ctx)).await?;

        // 过滤属性
        let filtered_attmap = self.filter_attributes(attmap);

        // 保存到目标数据库
        if !filtered_attmap.is_empty() {
            self.target_adapter.save_attmap(refno, &filtered_attmap).await?;
        }

        Ok(())
    }

    /// 同步关系
    async fn sync_relations(&self, refno: RefnoEnum) -> Result<()> {
        let ctx = QueryContext::default();

        // 同步子元素关系
        let children = self.source_adapter.query_children(refno, Some(ctx.clone())).await?;
        for child in children {
            self.target_adapter.create_relation(refno, child, "OWNS").await?;
        }

        // TODO: 同步其他类型的关系（REFERS_TO, USES_CATA等）

        Ok(())
    }

    /// 批量同步 PE
    async fn sync_batch_pes(&self, refnos: &[RefnoEnum]) -> Result<SyncStatistics> {
        let mut stats = SyncStatistics::default();
        stats.total_records = refnos.len();

        for refno in refnos {
            match self.sync_pe(*refno).await {
                Ok(_) => {
                    stats.successful_records += 1;
                }
                Err(e) => {
                    stats.failed_records += 1;
                    log::error!("同步 PE {} 失败: {}", refno.refno().0, e);

                    if !self.strategy.continue_on_error {
                        return Err(e);
                    }
                }
            }
        }

        Ok(stats)
    }

    /// 获取自指定时间以来变更的 PE
    async fn get_changed_pes_since(&self, since: std::time::SystemTime) -> Result<Vec<RefnoEnum>> {
        // TODO: 实际实现需要数据库支持变更追踪
        // 这里暂时返回所有 PE 的一个子集进行演示

        log::info!("查询自 {:?} 以来的变更", since);

        // 获取所有 PE 并过滤
        let all_pes = self.get_all_pes_to_sync().await?;

        // 实际应该基于变更时间戳过滤
        // 这里作为示例只返回前100个
        let changed_pes = all_pes.into_iter()
            .take(100)
            .collect();

        Ok(changed_pes)
    }

    /// 获取所有需要同步的 PE
    async fn get_all_pes_to_sync(&self) -> Result<Vec<RefnoEnum>> {
        let ctx = QueryContext::default();

        // 从源数据库获取所有 PE
        // TODO: 使用 query_all_pes 方法（当实现后）
        // 暂时使用查询根节点和遍历的方式获取所有 PE
        let root_pe = RefnoEnum::from(RefU64(1)); // 假设从根节点开始
        let all_pes = self.source_adapter.query_subtree(root_pe, 999, Some(ctx)).await
            .unwrap_or_else(|_| vec![]);

        // 应用过滤器
        let filtered_pes: Vec<RefnoEnum> = all_pes
            .into_iter()
            .filter(|pe| {
                // 检查 refno 范围
                if !self.filter.matches_refno(pe.refno()) {
                    return false;
                }

                // TODO: 检查元素类型（需要先获取 PE 的类型信息）
                // TODO: 检查修改时间（需要从属性中获取）

                true
            })
            .collect();

        log::info!("查询到 {} 个需要同步的 PE", filtered_pes.len());
        Ok(filtered_pes)
    }

    /// 过滤属性
    fn filter_attributes(&self, mut attmap: NamedAttrMap) -> NamedAttrMap {
        // 根据过滤器过滤属性
        attmap.retain(|name, _| self.filter.matches_attribute(name));
        attmap
    }

    /// 获取同步进度
    pub async fn get_progress(&self, task_id: &str) -> Option<SyncTask> {
        let tasks = self.active_tasks.read().await;
        tasks.get(task_id).cloned()
    }

    /// 取消同步任务
    pub async fn cancel_task(&self, task_id: &str) -> Result<()> {
        let mut tasks = self.active_tasks.write().await;
        if let Some(task) = tasks.get_mut(task_id) {
            task.cancel();
            Ok(())
        } else {
            Err(anyhow::anyhow!("任务不存在: {}", task_id))
        }
    }

    /// 获取统计信息
    pub async fn get_statistics(&self) -> SyncStatistics {
        let stats = self.statistics.read().await;
        stats.clone()
    }
}

/// 同步构建器
pub struct SyncManagerBuilder {
    source: Option<Arc<dyn DatabaseAdapter>>,
    target: Option<Arc<dyn DatabaseAdapter>>,
    strategy: SyncStrategy,
    filter: SyncFilter,
}

impl Default for SyncManagerBuilder {
    fn default() -> Self {
        Self {
            source: None,
            target: None,
            strategy: SyncStrategy::default(),
            filter: SyncFilter::default(),
        }
    }
}

impl SyncManagerBuilder {
    /// 创建新的构建器
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置源数据库
    pub fn source(mut self, adapter: Arc<dyn DatabaseAdapter>) -> Self {
        self.source = Some(adapter);
        self
    }

    /// 设置目标数据库
    pub fn target(mut self, adapter: Arc<dyn DatabaseAdapter>) -> Self {
        self.target = Some(adapter);
        self
    }

    /// 设置同步策略
    pub fn strategy(mut self, strategy: SyncStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// 设置过滤器
    pub fn filter(mut self, filter: SyncFilter) -> Self {
        self.filter = filter;
        self
    }

    /// 构建同步管理器
    pub fn build(self) -> Result<SyncManager> {
        let source = self.source.ok_or_else(|| anyhow::anyhow!("源数据库未设置"))?;
        let target = self.target.ok_or_else(|| anyhow::anyhow!("目标数据库未设置"))?;

        Ok(SyncManager::new(source, target, self.strategy, self.filter))
    }
}