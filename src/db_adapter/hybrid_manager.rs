//! 混合数据库管理器
//!
//! 协调多个数据库适配器，实现智能路由和回退

use super::config::*;
use super::traits::*;
use crate::types::*;
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;

/// 混合数据库管理器
#[derive(Clone)]
pub struct HybridDatabaseManager {
    /// 主数据库适配器
    primary: Arc<dyn DatabaseAdapter>,
    /// 次要数据库适配器（可选）
    secondary: Option<Arc<dyn DatabaseAdapter>>,
    /// 配置
    config: HybridConfig,
    /// 名称
    name: String,
}

impl std::fmt::Debug for HybridDatabaseManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HybridDatabaseManager")
            .field("primary", &self.primary.name())
            .field("secondary", &self.secondary.as_ref().map(|s| s.name()))
            .field("mode", &self.config.mode)
            .finish()
    }
}

impl HybridDatabaseManager {
    /// 创建新的混合数据库管理器
    pub fn new(
        primary: Arc<dyn DatabaseAdapter>,
        secondary: Option<Arc<dyn DatabaseAdapter>>,
        config: HybridConfig,
    ) -> Self {
        let name = format!(
            "Hybrid<{},{}>",
            primary.name(),
            secondary.as_ref().map(|s| s.name()).unwrap_or("None")
        );

        Self {
            primary,
            secondary,
            config,
            name,
        }
    }

    /// 智能路由：根据查询特征选择最优数据库
    async fn route_query<T, F1, F2>(
        &self,
        prefer_graph: bool,
        primary_fn: F1,
        secondary_fn: F2,
    ) -> anyhow::Result<T>
    where
        F1: std::future::Future<Output = anyhow::Result<T>>,
        F2: std::future::Future<Output = anyhow::Result<T>>,
    {
        match self.config.mode {
            HybridMode::SurrealPrimary => {
                self.execute_with_fallback(primary_fn, secondary_fn).await
            }
            HybridMode::KuzuPrimary => self.execute_with_fallback(secondary_fn, primary_fn).await,
            HybridMode::DualSurrealPreferred => {
                if prefer_graph {
                    // 图查询优先次要数据库（Kuzu）
                    self.execute_with_fallback(secondary_fn, primary_fn).await
                } else {
                    self.execute_with_fallback(primary_fn, secondary_fn).await
                }
            }
            HybridMode::DualKuzuPreferred => {
                if prefer_graph {
                    self.execute_with_fallback(secondary_fn, primary_fn).await
                } else {
                    // 非图查询也优先 Kuzu（因为性能好）
                    self.execute_with_fallback(secondary_fn, primary_fn).await
                }
            }
            HybridMode::WriteToSurrealReadFromKuzu => {
                // 读操作从次要数据库
                secondary_fn.await
            }
        }
    }

    /// 执行查询并在失败时回退
    async fn execute_with_fallback<T, F1, F2>(&self, primary: F1, fallback: F2) -> anyhow::Result<T>
    where
        F1: std::future::Future<Output = anyhow::Result<T>>,
        F2: std::future::Future<Output = anyhow::Result<T>>,
    {
        // 添加超时
        let timeout = Duration::from_millis(self.config.query_timeout_ms);

        match tokio::time::timeout(timeout, primary).await {
            Ok(Ok(result)) => Ok(result),
            Ok(Err(e)) => {
                if self.config.fallback_on_error && self.secondary.is_some() {
                    log::warn!("主数据库查询失败，回退到备用数据库: {}", e);
                    fallback.await
                } else {
                    Err(e)
                }
            }
            Err(_) => {
                // 超时
                if self.config.fallback_on_error && self.secondary.is_some() {
                    log::warn!("主数据库查询超时，回退到备用数据库");
                    fallback.await
                } else {
                    Err(anyhow::anyhow!("查询超时"))
                }
            }
        }
    }

    /// 双写：同时写入两个数据库
    async fn dual_write<F1, F2>(&self, primary_write: F1, secondary_write: F2) -> anyhow::Result<()>
    where
        F1: std::future::Future<Output = anyhow::Result<()>>,
        F2: std::future::Future<Output = anyhow::Result<()>>,
    {
        // 并行写入
        let (r1, r2) = tokio::join!(primary_write, secondary_write);

        // 记录错误
        if let Err(e) = &r1 {
            log::error!("主数据库写入失败: {}", e);
        }
        if let Err(e) = &r2 {
            log::error!("次要数据库写入失败: {}", e);
        }

        // 只要有一个成功就认为写入成功
        if r1.is_ok() || r2.is_ok() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("双数据库写入都失败"))
        }
    }

    /// 选择写入策略
    async fn execute_write<F1, F2>(
        &self,
        primary_write: F1,
        secondary_write: F2,
    ) -> anyhow::Result<()>
    where
        F1: std::future::Future<Output = anyhow::Result<()>>,
        F2: std::future::Future<Output = anyhow::Result<()>>,
    {
        match self.config.mode {
            HybridMode::WriteToSurrealReadFromKuzu | HybridMode::SurrealPrimary => {
                primary_write.await
            }
            HybridMode::KuzuPrimary => secondary_write.await,
            HybridMode::DualSurrealPreferred | HybridMode::DualKuzuPreferred => {
                // 双写模式
                self.dual_write(primary_write, secondary_write).await
            }
        }
    }
}

#[async_trait]
impl DatabaseAdapter for HybridDatabaseManager {
    fn name(&self) -> &str {
        &self.name
    }

    fn capabilities(&self) -> DatabaseCapabilities {
        // 合并两个数据库的能力
        let primary_caps = self.primary.capabilities();
        let secondary_caps = self
            .secondary
            .as_ref()
            .map(|s| s.capabilities())
            .unwrap_or_default();

        DatabaseCapabilities {
            supports_graph_traversal: primary_caps.supports_graph_traversal
                || secondary_caps.supports_graph_traversal,
            supports_transactions: primary_caps.supports_transactions
                || secondary_caps.supports_transactions,
            supports_versioning: primary_caps.supports_versioning
                || secondary_caps.supports_versioning,
            supports_live_queries: primary_caps.supports_live_queries
                || secondary_caps.supports_live_queries,
            supports_full_text_search: primary_caps.supports_full_text_search
                || secondary_caps.supports_full_text_search,
            supports_vector_index: primary_caps.supports_vector_index
                || secondary_caps.supports_vector_index,
        }
    }

    async fn health_check(&self) -> anyhow::Result<bool> {
        let primary_ok = self.primary.health_check().await.unwrap_or(false);
        let secondary_ok = if let Some(secondary) = &self.secondary {
            secondary.health_check().await.unwrap_or(false)
        } else {
            true
        };

        Ok(primary_ok || secondary_ok)
    }

    // ==================== PE 操作 ====================

    async fn get_pe(
        &self,
        refno: RefnoEnum,
        ctx: Option<QueryContext>,
    ) -> anyhow::Result<Option<SPdmsElement>> {
        let prefer_graph = ctx
            .as_ref()
            .map(|c| c.requires_graph_traversal)
            .unwrap_or(false);

        if let Some(secondary) = &self.secondary {
            self.route_query(
                prefer_graph,
                self.primary.get_pe(refno, ctx.clone()),
                secondary.get_pe(refno, ctx),
            )
            .await
        } else {
            self.primary.get_pe(refno, ctx).await
        }
    }

    async fn query_children(
        &self,
        refno: RefnoEnum,
        ctx: Option<QueryContext>,
    ) -> anyhow::Result<Vec<RefnoEnum>> {
        // 层次查询，优先图数据库
        if let Some(secondary) = &self.secondary {
            self.route_query(
                true, // 这是图查询
                self.primary.query_children(refno, ctx.clone()),
                secondary.query_children(refno, ctx),
            )
            .await
        } else {
            self.primary.query_children(refno, ctx).await
        }
    }

    async fn query_ancestors(
        &self,
        refno: RefnoEnum,
        ctx: Option<QueryContext>,
    ) -> anyhow::Result<Vec<RefnoEnum>> {
        // 祖先查询，优先图数据库
        if let Some(secondary) = &self.secondary {
            self.route_query(
                true,
                self.primary.query_ancestors(refno, ctx.clone()),
                secondary.query_ancestors(refno, ctx),
            )
            .await
        } else {
            self.primary.query_ancestors(refno, ctx).await
        }
    }

    async fn save_pe(&self, pe: &SPdmsElement) -> anyhow::Result<()> {
        if let Some(secondary) = &self.secondary {
            self.execute_write(self.primary.save_pe(pe), secondary.save_pe(pe))
                .await
        } else {
            self.primary.save_pe(pe).await
        }
    }

    async fn delete_pe(&self, refno: RefnoEnum) -> anyhow::Result<()> {
        if let Some(secondary) = &self.secondary {
            self.execute_write(self.primary.delete_pe(refno), secondary.delete_pe(refno))
                .await
        } else {
            self.primary.delete_pe(refno).await
        }
    }

    // ==================== 属性操作 ====================

    async fn get_attmap(
        &self,
        refno: RefnoEnum,
        ctx: Option<QueryContext>,
    ) -> anyhow::Result<NamedAttrMap> {
        if let Some(secondary) = &self.secondary {
            self.route_query(
                false, // 普通查询
                self.primary.get_attmap(refno, ctx.clone()),
                secondary.get_attmap(refno, ctx),
            )
            .await
        } else {
            self.primary.get_attmap(refno, ctx).await
        }
    }

    async fn save_attmap(&self, refno: RefnoEnum, attmap: &NamedAttrMap) -> anyhow::Result<()> {
        if let Some(secondary) = &self.secondary {
            self.execute_write(
                self.primary.save_attmap(refno, attmap),
                secondary.save_attmap(refno, attmap),
            )
            .await
        } else {
            self.primary.save_attmap(refno, attmap).await
        }
    }

    // ==================== 关系操作 ====================

    async fn create_relation(
        &self,
        from: RefnoEnum,
        to: RefnoEnum,
        rel_type: &str,
    ) -> anyhow::Result<()> {
        if let Some(secondary) = &self.secondary {
            self.execute_write(
                self.primary.create_relation(from, to, rel_type),
                secondary.create_relation(from, to, rel_type),
            )
            .await
        } else {
            self.primary.create_relation(from, to, rel_type).await
        }
    }

    async fn query_related(
        &self,
        refno: RefnoEnum,
        rel_type: &str,
        ctx: Option<QueryContext>,
    ) -> anyhow::Result<Vec<RefnoEnum>> {
        if let Some(secondary) = &self.secondary {
            self.route_query(
                true, // 关系查询
                self.primary.query_related(refno, rel_type, ctx.clone()),
                secondary.query_related(refno, rel_type, ctx),
            )
            .await
        } else {
            self.primary.query_related(refno, rel_type, ctx).await
        }
    }

    async fn delete_relation(
        &self,
        from: RefnoEnum,
        to: RefnoEnum,
        rel_type: &str,
    ) -> anyhow::Result<()> {
        if let Some(secondary) = &self.secondary {
            self.execute_write(
                self.primary.delete_relation(from, to, rel_type),
                secondary.delete_relation(from, to, rel_type),
            )
            .await
        } else {
            self.primary.delete_relation(from, to, rel_type).await
        }
    }

    // ==================== 图遍历操作 ====================

    async fn shortest_path(
        &self,
        from: RefnoEnum,
        to: RefnoEnum,
        ctx: Option<QueryContext>,
    ) -> anyhow::Result<Vec<RefnoEnum>> {
        // 最短路径必须用图数据库
        if let Some(secondary) = &self.secondary {
            if secondary.capabilities().supports_graph_traversal {
                return secondary.shortest_path(from, to, ctx).await;
            }
        }
        self.primary.shortest_path(from, to, ctx).await
    }

    async fn query_subtree(
        &self,
        refno: RefnoEnum,
        max_depth: usize,
        ctx: Option<QueryContext>,
    ) -> anyhow::Result<Vec<RefnoEnum>> {
        // 子树查询，优先图数据库
        if let Some(secondary) = &self.secondary {
            self.route_query(
                true,
                self.primary.query_subtree(refno, max_depth, ctx.clone()),
                secondary.query_subtree(refno, max_depth, ctx),
            )
            .await
        } else {
            self.primary.query_subtree(refno, max_depth, ctx).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hybrid_manager_creation() {
        use crate::db_adapter::surreal_adapter::SurrealAdapter;

        let primary = Arc::new(SurrealAdapter::new());
        let config = HybridConfig::default();

        let manager = HybridDatabaseManager::new(primary, None, config);
        assert!(manager.name().contains("Hybrid"));
    }
}
