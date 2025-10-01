//! SurrealDB 适配器实现

use super::traits::*;
use crate::rs_surreal;
use crate::types::*;
use async_trait::async_trait;

/// SurrealDB 适配器
#[derive(Debug, Clone)]
pub struct SurrealAdapter {
    name: String,
}

impl SurrealAdapter {
    /// 创建新的 SurrealDB 适配器
    pub fn new() -> Self {
        Self {
            name: "SurrealDB".to_string(),
        }
    }
}

impl Default for SurrealAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DatabaseAdapter for SurrealAdapter {
    fn name(&self) -> &str {
        &self.name
    }

    fn capabilities(&self) -> DatabaseCapabilities {
        DatabaseCapabilities {
            supports_graph_traversal: true, // SurrealDB 支持图查询
            supports_transactions: true,
            supports_versioning: true,
            supports_live_queries: true,
            supports_full_text_search: false,
            supports_vector_index: false,
        }
    }

    async fn health_check(&self) -> anyhow::Result<bool> {
        // 简单的健康检查：尝试执行一个查询
        match rs_surreal::SUL_DB.query("SELECT 1").await {
            Ok(_) => Ok(true),
            Err(e) => {
                log::warn!("SurrealDB 健康检查失败: {}", e);
                Ok(false)
            }
        }
    }

    // ==================== PE 操作 ====================

    async fn get_pe(
        &self,
        refno: RefnoEnum,
        _ctx: Option<QueryContext>,
    ) -> anyhow::Result<Option<SPdmsElement>> {
        rs_surreal::query::get_pe(refno).await
    }

    async fn get_pe_batch(
        &self,
        refnos: &[RefnoEnum],
        ctx: Option<QueryContext>,
    ) -> anyhow::Result<Vec<SPdmsElement>> {
        let mut results = Vec::with_capacity(refnos.len());
        for refno in refnos {
            if let Some(pe) = self.get_pe(*refno, ctx.clone()).await? {
                results.push(pe);
            }
        }
        Ok(results)
    }

    async fn query_children(
        &self,
        refno: RefnoEnum,
        _ctx: Option<QueryContext>,
    ) -> anyhow::Result<Vec<RefnoEnum>> {
        rs_surreal::query::get_children_refnos(refno).await
    }

    async fn query_ancestors(
        &self,
        refno: RefnoEnum,
        _ctx: Option<QueryContext>,
    ) -> anyhow::Result<Vec<RefnoEnum>> {
        rs_surreal::query::query_ancestor_refnos(refno).await
    }

    async fn save_pe(&self, pe: &SPdmsElement) -> anyhow::Result<()> {
        // TODO: 实现 PE 保存逻辑
        log::debug!("SurrealAdapter: 保存 PE {:?}", pe.refno);
        Ok(())
    }

    async fn delete_pe(&self, refno: RefnoEnum) -> anyhow::Result<()> {
        // TODO: 实现 PE 删除逻辑
        log::debug!("SurrealAdapter: 删除 PE {:?}", refno);
        Ok(())
    }

    // ==================== 属性操作 ====================

    async fn get_attmap(
        &self,
        refno: RefnoEnum,
        _ctx: Option<QueryContext>,
    ) -> anyhow::Result<NamedAttrMap> {
        rs_surreal::query::get_named_attmap(refno).await
    }

    async fn get_attmap_with_uda(
        &self,
        refno: RefnoEnum,
        _ctx: Option<QueryContext>,
    ) -> anyhow::Result<NamedAttrMap> {
        rs_surreal::query::get_named_attmap_with_uda(refno).await
    }

    async fn save_attmap(&self, refno: RefnoEnum, _attmap: &NamedAttrMap) -> anyhow::Result<()> {
        // TODO: 实现属性保存逻辑
        log::debug!("SurrealAdapter: 保存属性 {:?}", refno);
        Ok(())
    }

    // ==================== 关系操作 ====================

    async fn create_relation(
        &self,
        from: RefnoEnum,
        to: RefnoEnum,
        rel_type: &str,
    ) -> anyhow::Result<()> {
        // TODO: 实现关系创建逻辑
        log::debug!(
            "SurrealAdapter: 创建关系 {:?} -[{}]-> {:?}",
            from,
            rel_type,
            to
        );
        Ok(())
    }

    async fn query_related(
        &self,
        refno: RefnoEnum,
        rel_type: &str,
        _ctx: Option<QueryContext>,
    ) -> anyhow::Result<Vec<RefnoEnum>> {
        // TODO: 实现相关元素查询
        log::debug!("SurrealAdapter: 查询相关元素 {:?} -[{}]->", refno, rel_type);
        Ok(vec![])
    }

    async fn delete_relation(
        &self,
        from: RefnoEnum,
        to: RefnoEnum,
        rel_type: &str,
    ) -> anyhow::Result<()> {
        // TODO: 实现关系删除逻辑
        log::debug!(
            "SurrealAdapter: 删除关系 {:?} -[{}]-> {:?}",
            from,
            rel_type,
            to
        );
        Ok(())
    }

    // ==================== 图遍历操作 ====================

    async fn query_subtree(
        &self,
        refno: RefnoEnum,
        max_depth: usize,
        _ctx: Option<QueryContext>,
    ) -> anyhow::Result<Vec<RefnoEnum>> {
        // 使用 SurrealDB 的递归查询
        // TODO: 优化为单次查询
        let mut result = vec![refno];
        let mut current_level = vec![refno];

        for _ in 0..max_depth {
            let mut next_level = Vec::new();
            for parent in &current_level {
                let children = self.query_children(*parent, None).await?;
                next_level.extend(children.clone());
                result.extend(children);
            }
            if next_level.is_empty() {
                break;
            }
            current_level = next_level;
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_surreal_adapter_creation() {
        let adapter = SurrealAdapter::new();
        assert_eq!(adapter.name(), "SurrealDB");
    }

    #[test]
    fn test_surreal_adapter_capabilities() {
        let adapter = SurrealAdapter::new();
        let caps = adapter.capabilities();

        assert!(caps.supports_graph_traversal);
        assert!(caps.supports_transactions);
        assert!(caps.supports_versioning);
        assert!(caps.supports_live_queries);
    }
}
