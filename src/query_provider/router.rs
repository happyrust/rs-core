//! 查询路由器
//!
//! 提供智能的查询路由和自动选择功能

use super::SurrealQueryProvider;
use super::error::{QueryError, QueryResult};
use super::traits::*;
use crate::RefnoEnum;
use crate::types::{NamedAttrMap as NamedAttMap, SPdmsElement as PE};
use async_trait::async_trait;
use log::{debug, info, warn};
use std::sync::{Arc, RwLock};

/// 查询引擎类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryEngine {
    /// 使用 SurrealDB
    SurrealDB,
    /// 自动选择（当前仅有 SurrealDB，实现等同于 SurrealDB）
    Auto,
}

impl Default for QueryEngine {
    fn default() -> Self {
        QueryEngine::Auto
    }
}

/// 查询策略配置
#[derive(Debug, Clone)]
pub struct QueryStrategy {
    /// 引擎选择
    pub engine: QueryEngine,
    /// 是否启用回退
    pub enable_fallback: bool,
    /// 查询超时时间（毫秒）
    pub timeout_ms: Option<u64>,
    /// 是否启用性能日志
    pub enable_performance_log: bool,
}

impl Default for QueryStrategy {
    fn default() -> Self {
        Self {
            engine: QueryEngine::Auto,
            enable_fallback: true,
            timeout_ms: Some(5000),
            enable_performance_log: true,
        }
    }
}

impl QueryStrategy {
    /// 创建只使用 SurrealDB 的策略
    pub fn surreal_only() -> Self {
        Self {
            engine: QueryEngine::SurrealDB,
            enable_fallback: false,
            ..Default::default()
        }
    }

    /// 创建自动选择策略（默认）
    pub fn auto() -> Self {
        Self::default()
    }

    /// 设置是否启用回退
    pub fn with_fallback(mut self, enable: bool) -> Self {
        self.enable_fallback = enable;
        self
    }

    /// 设置超时时间
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }

    /// 设置性能日志
    pub fn with_performance_log(mut self, enable: bool) -> Self {
        self.enable_performance_log = enable;
        self
    }
}

/// 查询路由器
///
/// 根据策略自动选择合适的查询提供者
pub struct QueryRouter {
    /// SurrealDB 查询提供者
    surreal_provider: Arc<SurrealQueryProvider>,
    /// 查询策略
    strategy: Arc<RwLock<QueryStrategy>>,
}

impl QueryRouter {
    /// 创建新的查询路由器
    pub fn new(strategy: QueryStrategy) -> QueryResult<Self> {
        let surreal_provider = Arc::new(SurrealQueryProvider::new()?);

        Ok(Self {
            surreal_provider,
            strategy: Arc::new(RwLock::new(strategy)),
        })
    }

    /// 创建使用自动选择策略的路由器
    pub fn auto() -> QueryResult<Self> {
        Self::new(QueryStrategy::auto())
    }

    /// 创建只使用 SurrealDB 的路由器
    pub fn surreal_only() -> QueryResult<Self> {
        Self::new(QueryStrategy::surreal_only())
    }

    /// 更新策略
    pub fn set_strategy(&self, strategy: QueryStrategy) {
        if let Ok(mut s) = self.strategy.write() {
            *s = strategy;
        }
    }

    /// 获取当前策略
    pub fn get_strategy(&self) -> QueryStrategy {
        self.strategy.read().unwrap().clone()
    }

    /// 选择查询提供者
    fn select_provider(&self) -> (QueryEngine, Arc<dyn QueryProvider>) {
        let strategy = self.get_strategy();

        match strategy.engine {
            QueryEngine::SurrealDB => {
                debug!("选择 SurrealDB 查询提供者");
                (QueryEngine::SurrealDB, self.surreal_provider.clone())
            }
            QueryEngine::Auto => {
                debug!("自动选择模式，使用 SurrealDB 查询提供者");
                (QueryEngine::SurrealDB, self.surreal_provider.clone())
            }
        }
    }

    /// 执行查询（带回退机制）
    async fn execute_with_fallback<F, T>(&self, query_name: &str, f: F) -> QueryResult<T>
    where
        F: Fn(
                Arc<dyn QueryProvider>,
            )
                -> std::pin::Pin<Box<dyn std::future::Future<Output = QueryResult<T>> + Send>>
            + Send,
        T: Send,
    {
        let strategy = self.get_strategy();
        let (engine, provider) = self.select_provider();

        let start_time = std::time::Instant::now();

        // 执行查询
        let result = f(provider.clone()).await;

        // 性能日志
        if strategy.enable_performance_log {
            let elapsed = start_time.elapsed();
            if elapsed.as_millis() > 100 {
                info!(
                    "[{}] {} 查询耗时: {:?}",
                    provider.provider_name(),
                    query_name,
                    elapsed
                );
            }
        }

        // 处理结果和回退
        match result {
            Ok(value) => Ok(value),
            Err(e) => {
                warn!(
                    "[{}] {} 查询失败: {}",
                    provider.provider_name(),
                    query_name,
                    e
                );

                // 如果启用了回退且当前不是 SurrealDB，则回退
                if strategy.enable_fallback && engine != QueryEngine::SurrealDB {
                    info!("回退到 SurrealDB 执行查询: {}", query_name);
                    let fallback_provider: Arc<dyn QueryProvider> = self.surreal_provider.clone();
                    f(fallback_provider).await
                } else {
                    Err(e)
                }
            }
        }
    }
}

// ============================================================================
// HierarchyQuery 实现
// ============================================================================

#[async_trait]
impl HierarchyQuery for QueryRouter {
    async fn get_children(&self, refno: RefnoEnum) -> QueryResult<Vec<RefnoEnum>> {
        self.execute_with_fallback("get_children", |provider| {
            Box::pin(async move { provider.get_children(refno).await })
        })
        .await
    }

    async fn get_descendants(
        &self,
        refno: RefnoEnum,
        max_depth: Option<usize>,
    ) -> QueryResult<Vec<RefnoEnum>> {
        self.execute_with_fallback("get_descendants", |provider| {
            Box::pin(async move { provider.get_descendants(refno, max_depth).await })
        })
        .await
    }

    async fn get_ancestors(&self, refno: RefnoEnum) -> QueryResult<Vec<RefnoEnum>> {
        self.execute_with_fallback("get_ancestors", |provider| {
            Box::pin(async move { provider.get_ancestors(refno).await })
        })
        .await
    }

    async fn get_ancestors_of_type(
        &self,
        refno: RefnoEnum,
        nouns: &[&str],
    ) -> QueryResult<Vec<RefnoEnum>> {
        self.execute_with_fallback("get_ancestors_of_type", |provider| {
            let nouns: Vec<String> = nouns.iter().map(|s| s.to_string()).collect();
            Box::pin(async move {
                let noun_refs: Vec<&str> = nouns.iter().map(|s| s.as_str()).collect();
                provider.get_ancestors_of_type(refno, &noun_refs).await
            })
        })
        .await
    }

    async fn get_descendants_filtered(
        &self,
        refno: RefnoEnum,
        nouns: &[&str],
        max_depth: Option<usize>,
    ) -> QueryResult<Vec<RefnoEnum>> {
        self.execute_with_fallback("get_descendants_filtered", |provider| {
            let nouns: Vec<String> = nouns.iter().map(|s| s.to_string()).collect();
            Box::pin(async move {
                let noun_refs: Vec<&str> = nouns.iter().map(|s| s.as_str()).collect();
                provider
                    .get_descendants_filtered(refno, &noun_refs, max_depth)
                    .await
            })
        })
        .await
    }

    async fn get_children_pes(&self, refno: RefnoEnum) -> QueryResult<Vec<PE>> {
        self.execute_with_fallback("get_children_pes", |provider| {
            Box::pin(async move { provider.get_children_pes(refno).await })
        })
        .await
    }
}

// ============================================================================
// TypeQuery 实现
// ============================================================================

#[async_trait]
impl TypeQuery for QueryRouter {
    async fn query_by_type(
        &self,
        nouns: &[&str],
        dbnum: i32,
        has_children: Option<bool>,
    ) -> QueryResult<Vec<RefnoEnum>> {
        self.execute_with_fallback("query_by_type", |provider| {
            let nouns: Vec<String> = nouns.iter().map(|s| s.to_string()).collect();
            Box::pin(async move {
                let noun_refs: Vec<&str> = nouns.iter().map(|s| s.as_str()).collect();
                provider
                    .query_by_type(&noun_refs, dbnum, has_children)
                    .await
            })
        })
        .await
    }

    async fn query_by_type_multi_db(
        &self,
        nouns: &[&str],
        dbnums: &[i32],
    ) -> QueryResult<Vec<RefnoEnum>> {
        self.execute_with_fallback("query_by_type_multi_db", |provider| {
            let nouns: Vec<String> = nouns.iter().map(|s| s.to_string()).collect();
            let dbnums = dbnums.to_vec();
            Box::pin(async move {
                let noun_refs: Vec<&str> = nouns.iter().map(|s| s.as_str()).collect();
                provider.query_by_type_multi_db(&noun_refs, &dbnums).await
            })
        })
        .await
    }

    async fn get_world(&self, dbnum: i32) -> QueryResult<Option<RefnoEnum>> {
        self.execute_with_fallback("get_world", |provider| {
            Box::pin(async move { provider.get_world(dbnum).await })
        })
        .await
    }

    async fn get_sites(&self, dbnum: i32) -> QueryResult<Vec<RefnoEnum>> {
        self.execute_with_fallback("get_sites", |provider| {
            Box::pin(async move { provider.get_sites(dbnum).await })
        })
        .await
    }

    async fn count_by_type(&self, noun: &str, dbnum: i32) -> QueryResult<usize> {
        self.execute_with_fallback("count_by_type", |provider| {
            let noun = noun.to_string();
            Box::pin(async move { provider.count_by_type(&noun, dbnum).await })
        })
        .await
    }
}

// ============================================================================
// BatchQuery 实现
// ============================================================================

#[async_trait]
impl BatchQuery for QueryRouter {
    async fn get_pes_batch(&self, refnos: &[RefnoEnum]) -> QueryResult<Vec<PE>> {
        self.execute_with_fallback("get_pes_batch", |provider| {
            let refnos = refnos.to_vec();
            Box::pin(async move { provider.get_pes_batch(&refnos).await })
        })
        .await
    }

    async fn get_attmaps_batch(&self, refnos: &[RefnoEnum]) -> QueryResult<Vec<NamedAttMap>> {
        self.execute_with_fallback("get_attmaps_batch", |provider| {
            let refnos = refnos.to_vec();
            Box::pin(async move { provider.get_attmaps_batch(&refnos).await })
        })
        .await
    }

    async fn get_full_names_batch(
        &self,
        refnos: &[RefnoEnum],
    ) -> QueryResult<Vec<(RefnoEnum, String)>> {
        self.execute_with_fallback("get_full_names_batch", |provider| {
            let refnos = refnos.to_vec();
            Box::pin(async move { provider.get_full_names_batch(&refnos).await })
        })
        .await
    }
}

// ============================================================================
// GraphQuery 实现
// ============================================================================

#[async_trait]
impl GraphQuery for QueryRouter {
    async fn query_multi_descendants(
        &self,
        refnos: &[RefnoEnum],
        nouns: &[&str],
    ) -> QueryResult<Vec<RefnoEnum>> {
        self.execute_with_fallback("query_multi_descendants", |provider| {
            let refnos = refnos.to_vec();
            let nouns: Vec<String> = nouns.iter().map(|s| s.to_string()).collect();
            Box::pin(async move {
                let noun_refs: Vec<&str> = nouns.iter().map(|s| s.as_str()).collect();
                provider.query_multi_descendants(&refnos, &noun_refs).await
            })
        })
        .await
    }

    async fn find_shortest_path(
        &self,
        from: RefnoEnum,
        to: RefnoEnum,
    ) -> QueryResult<Vec<RefnoEnum>> {
        self.execute_with_fallback("find_shortest_path", |provider| {
            Box::pin(async move { provider.find_shortest_path(from, to).await })
        })
        .await
    }

    async fn get_node_depth(&self, refno: RefnoEnum) -> QueryResult<usize> {
        self.execute_with_fallback("get_node_depth", |provider| {
            Box::pin(async move { provider.get_node_depth(refno).await })
        })
        .await
    }
}

// ============================================================================
// QueryProvider 实现
// ============================================================================

#[async_trait]
impl QueryProvider for QueryRouter {
    async fn get_pe(&self, refno: RefnoEnum) -> QueryResult<Option<PE>> {
        self.execute_with_fallback("get_pe", |provider| {
            Box::pin(async move { provider.get_pe(refno).await })
        })
        .await
    }

    async fn get_attmap(&self, refno: RefnoEnum) -> QueryResult<Option<NamedAttMap>> {
        self.execute_with_fallback("get_attmap", |provider| {
            Box::pin(async move { provider.get_attmap(refno).await })
        })
        .await
    }

    async fn exists(&self, refno: RefnoEnum) -> QueryResult<bool> {
        self.execute_with_fallback("exists", |provider| {
            Box::pin(async move { provider.exists(refno).await })
        })
        .await
    }

    fn provider_name(&self) -> &str {
        "QueryRouter"
    }

    async fn health_check(&self) -> QueryResult<bool> {
        // 检查 SurrealDB 提供者的健康状态
        let surreal_health = self.surreal_provider.health_check().await.unwrap_or(false);
        Ok(surreal_health)
    }
}
