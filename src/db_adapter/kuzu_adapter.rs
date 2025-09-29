//! Kuzu 适配器实现

#[cfg(feature = "kuzu")]
use super::traits::*;
#[cfg(feature = "kuzu")]
use crate::rs_kuzu;
#[cfg(feature = "kuzu")]
use crate::types::*;
#[cfg(feature = "kuzu")]
use async_trait::async_trait;

#[cfg(feature = "kuzu")]
/// Kuzu 适配器
#[derive(Debug, Clone)]
pub struct KuzuAdapter {
    name: String,
}

#[cfg(feature = "kuzu")]
impl KuzuAdapter {
    /// 创建新的 Kuzu 适配器
    pub fn new() -> Self {
        Self {
            name: "Kuzu".to_string(),
        }
    }
}

#[cfg(feature = "kuzu")]
impl Default for KuzuAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "kuzu")]
#[async_trait]
impl DatabaseAdapter for KuzuAdapter {
    fn name(&self) -> &str {
        &self.name
    }

    fn capabilities(&self) -> DatabaseCapabilities {
        DatabaseCapabilities {
            supports_graph_traversal: true, // Kuzu 强项
            supports_transactions: true,
            supports_versioning: false,
            supports_live_queries: false,
            supports_full_text_search: true,
            supports_vector_index: true,
        }
    }

    async fn health_check(&self) -> anyhow::Result<bool> {
        // 检查 Kuzu 是否已初始化
        Ok(rs_kuzu::is_kuzu_initialized())
    }

    // ==================== PE 操作 ====================

    async fn get_pe(
        &self,
        refno: RefnoEnum,
        _ctx: Option<QueryContext>,
    ) -> anyhow::Result<Option<SPdmsElement>> {
        rs_kuzu::queries::get_pe_from_kuzu(refno).await
    }

    async fn query_children(
        &self,
        refno: RefnoEnum,
        _ctx: Option<QueryContext>,
    ) -> anyhow::Result<Vec<RefnoEnum>> {
        rs_kuzu::queries::query_children_refnos_kuzu(refno).await
    }

    async fn query_ancestors(
        &self,
        refno: RefnoEnum,
        _ctx: Option<QueryContext>,
    ) -> anyhow::Result<Vec<RefnoEnum>> {
        // TODO: 实现 Kuzu 祖先查询
        // 使用 Cypher: MATCH (child:PE)-[:OWNS*]->(ancestor:PE)
        log::debug!("KuzuAdapter: 查询祖先 {:?}", refno);
        Ok(vec![])
    }

    async fn save_pe(&self, pe: &SPdmsElement) -> anyhow::Result<()> {
        rs_kuzu::operations::save_pe_kuzu(pe).await
    }

    async fn save_pe_batch(&self, pes: Vec<SPdmsElement>) -> anyhow::Result<()> {
        rs_kuzu::operations::save_pe_batch_kuzu(pes).await
    }

    async fn delete_pe(&self, refno: RefnoEnum) -> anyhow::Result<()> {
        // TODO: 实现 PE 删除逻辑
        log::debug!("KuzuAdapter: 删除 PE {:?}", refno);
        Ok(())
    }

    // ==================== 属性操作 ====================

    async fn get_attmap(
        &self,
        refno: RefnoEnum,
        _ctx: Option<QueryContext>,
    ) -> anyhow::Result<NamedAttrMap> {
        rs_kuzu::queries::get_named_attmap_kuzu(refno).await
    }

    async fn save_attmap(
        &self,
        refno: RefnoEnum,
        attmap: &NamedAttrMap,
    ) -> anyhow::Result<()> {
        rs_kuzu::operations::save_attmap_kuzu(refno, attmap).await
    }

    // ==================== 关系操作 ====================

    async fn create_relation(
        &self,
        from: RefnoEnum,
        to: RefnoEnum,
        rel_type: &str,
    ) -> anyhow::Result<()> {
        rs_kuzu::operations::create_relation_kuzu(from, to, rel_type).await
    }

    async fn query_related(
        &self,
        refno: RefnoEnum,
        rel_type: &str,
        _ctx: Option<QueryContext>,
    ) -> anyhow::Result<Vec<RefnoEnum>> {
        rs_kuzu::queries::query_related_kuzu(refno, rel_type).await
    }

    async fn delete_relation(
        &self,
        from: RefnoEnum,
        to: RefnoEnum,
        rel_type: &str,
    ) -> anyhow::Result<()> {
        // TODO: 实现关系删除逻辑
        log::debug!(
            "KuzuAdapter: 删除关系 {:?} -[{}]-> {:?}",
            from,
            rel_type,
            to
        );
        Ok(())
    }

    // ==================== 图遍历操作 ====================

    async fn shortest_path(
        &self,
        from: RefnoEnum,
        to: RefnoEnum,
        _ctx: Option<QueryContext>,
    ) -> anyhow::Result<Vec<RefnoEnum>> {
        rs_kuzu::queries::shortest_path_kuzu(from, to).await
    }

    async fn query_path(
        &self,
        from: RefnoEnum,
        pattern: &str,
        _ctx: Option<QueryContext>,
    ) -> anyhow::Result<Vec<Vec<RefnoEnum>>> {
        // TODO: 实现路径查询
        // 使用 Cypher 模式匹配
        log::debug!("KuzuAdapter: 查询路径 {:?} 模式: {}", from, pattern);
        Ok(vec![])
    }

    async fn query_subtree(
        &self,
        refno: RefnoEnum,
        max_depth: usize,
        _ctx: Option<QueryContext>,
    ) -> anyhow::Result<Vec<RefnoEnum>> {
        // 使用 Kuzu 的强大图遍历能力
        // MATCH (root:PE)-[:OWNS*1..max_depth]->(descendant:PE)
        // WHERE root.refno = $refno
        // RETURN DISTINCT descendant.refno
        log::debug!(
            "KuzuAdapter: 查询子树 {:?} 深度: {}",
            refno,
            max_depth
        );

        // 临时实现：使用递归
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

#[cfg(feature = "kuzu")]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kuzu_adapter_creation() {
        let adapter = KuzuAdapter::new();
        assert_eq!(adapter.name(), "Kuzu");
    }

    #[test]
    fn test_kuzu_adapter_capabilities() {
        let adapter = KuzuAdapter::new();
        let caps = adapter.capabilities();

        assert!(caps.supports_graph_traversal);
        assert!(caps.supports_transactions);
        assert!(!caps.supports_versioning);
        assert!(caps.supports_full_text_search);
        assert!(caps.supports_vector_index);
    }
}