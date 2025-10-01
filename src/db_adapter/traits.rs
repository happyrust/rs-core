//! 数据库适配器接口定义
//!
//! 提供统一的数据库访问接口

use crate::types::*;
use async_trait::async_trait;
use std::fmt::Debug;

/// 数据库能力标识
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DatabaseCapabilities {
    /// 是否支持图遍历
    pub supports_graph_traversal: bool,
    /// 是否支持事务
    pub supports_transactions: bool,
    /// 是否支持版本控制
    pub supports_versioning: bool,
    /// 是否支持实时查询
    pub supports_live_queries: bool,
    /// 是否支持全文搜索
    pub supports_full_text_search: bool,
    /// 是否支持向量索引
    pub supports_vector_index: bool,
}

impl Default for DatabaseCapabilities {
    fn default() -> Self {
        Self {
            supports_graph_traversal: false,
            supports_transactions: false,
            supports_versioning: false,
            supports_live_queries: false,
            supports_full_text_search: false,
            supports_vector_index: false,
        }
    }
}

/// 查询上下文
#[derive(Debug, Clone)]
pub struct QueryContext {
    /// 查询超时时间（毫秒）
    pub timeout_ms: Option<u64>,
    /// 是否需要图遍历能力
    pub requires_graph_traversal: bool,
    /// 是否需要事务
    pub requires_transaction: bool,
    /// 优先级（0-10，10 最高）
    pub priority: u8,
}

impl Default for QueryContext {
    fn default() -> Self {
        Self {
            timeout_ms: Some(5000),
            requires_graph_traversal: false,
            requires_transaction: false,
            priority: 5,
        }
    }
}

/// 统一数据库适配器接口
#[async_trait]
pub trait DatabaseAdapter: Send + Sync + Debug {
    /// 获取适配器名称
    fn name(&self) -> &str;

    /// 获取数据库能力
    fn capabilities(&self) -> DatabaseCapabilities;

    /// 检查是否健康
    async fn health_check(&self) -> anyhow::Result<bool>;

    // ==================== PE 操作 ====================

    /// 查询单个 PE
    async fn get_pe(
        &self,
        refno: RefnoEnum,
        ctx: Option<QueryContext>,
    ) -> anyhow::Result<Option<SPdmsElement>>;

    /// 批量查询 PE
    async fn get_pe_batch(
        &self,
        refnos: &[RefnoEnum],
        ctx: Option<QueryContext>,
    ) -> anyhow::Result<Vec<SPdmsElement>> {
        let mut results = Vec::new();
        for refno in refnos {
            if let Some(pe) = self.get_pe(*refno, ctx.clone()).await? {
                results.push(pe);
            }
        }
        Ok(results)
    }

    /// 查询子元素
    async fn query_children(
        &self,
        refno: RefnoEnum,
        ctx: Option<QueryContext>,
    ) -> anyhow::Result<Vec<RefnoEnum>>;

    /// 查询祖先元素
    async fn query_ancestors(
        &self,
        refno: RefnoEnum,
        ctx: Option<QueryContext>,
    ) -> anyhow::Result<Vec<RefnoEnum>>;

    /// 保存 PE
    async fn save_pe(&self, pe: &SPdmsElement) -> anyhow::Result<()>;

    /// 批量保存 PE
    async fn save_pe_batch(&self, pes: Vec<SPdmsElement>) -> anyhow::Result<()> {
        for pe in pes {
            self.save_pe(&pe).await?;
        }
        Ok(())
    }

    /// 删除 PE
    async fn delete_pe(&self, refno: RefnoEnum) -> anyhow::Result<()>;

    // ==================== 属性操作 ====================

    /// 查询属性
    async fn get_attmap(
        &self,
        refno: RefnoEnum,
        ctx: Option<QueryContext>,
    ) -> anyhow::Result<NamedAttrMap>;

    /// 查询属性（包含 UDA）
    async fn get_attmap_with_uda(
        &self,
        refno: RefnoEnum,
        ctx: Option<QueryContext>,
    ) -> anyhow::Result<NamedAttrMap> {
        // 默认实现：调用普通查询
        self.get_attmap(refno, ctx).await
    }

    /// 保存属性
    async fn save_attmap(&self, refno: RefnoEnum, attmap: &NamedAttrMap) -> anyhow::Result<()>;

    // ==================== 关系操作 ====================

    /// 创建关系
    async fn create_relation(
        &self,
        from: RefnoEnum,
        to: RefnoEnum,
        rel_type: &str,
    ) -> anyhow::Result<()>;

    /// 查询相关元素
    async fn query_related(
        &self,
        refno: RefnoEnum,
        rel_type: &str,
        ctx: Option<QueryContext>,
    ) -> anyhow::Result<Vec<RefnoEnum>>;

    /// 删除关系
    async fn delete_relation(
        &self,
        from: RefnoEnum,
        to: RefnoEnum,
        rel_type: &str,
    ) -> anyhow::Result<()>;

    // ==================== 图遍历操作（可选）====================

    /// 最短路径查询
    async fn shortest_path(
        &self,
        from: RefnoEnum,
        to: RefnoEnum,
        ctx: Option<QueryContext>,
    ) -> anyhow::Result<Vec<RefnoEnum>> {
        if !self.capabilities().supports_graph_traversal {
            return Err(anyhow::anyhow!("数据库不支持图遍历功能"));
        }
        Err(anyhow::anyhow!("未实现最短路径查询"))
    }

    /// 路径查询（支持模式匹配）
    async fn query_path(
        &self,
        from: RefnoEnum,
        pattern: &str,
        ctx: Option<QueryContext>,
    ) -> anyhow::Result<Vec<Vec<RefnoEnum>>> {
        if !self.capabilities().supports_graph_traversal {
            return Err(anyhow::anyhow!("数据库不支持图遍历功能"));
        }
        Err(anyhow::anyhow!("未实现路径查询"))
    }

    /// 查询指定深度的子树
    async fn query_subtree(
        &self,
        refno: RefnoEnum,
        max_depth: usize,
        ctx: Option<QueryContext>,
    ) -> anyhow::Result<Vec<RefnoEnum>> {
        // 默认实现：递归查询
        let mut result = vec![refno];
        let mut current_level = vec![refno];

        for _ in 0..max_depth {
            let mut next_level = Vec::new();
            for parent in &current_level {
                let children = self.query_children(*parent, ctx.clone()).await?;
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

    // ==================== 批量操作 ====================

    /// 批量查询子元素
    async fn query_children_batch(
        &self,
        refnos: &[RefnoEnum],
        ctx: Option<QueryContext>,
    ) -> anyhow::Result<Vec<Vec<RefnoEnum>>> {
        let mut results = Vec::new();
        for refno in refnos {
            let children = self.query_children(*refno, ctx.clone()).await?;
            results.push(children);
        }
        Ok(results)
    }

    // ==================== 统计和元数据 ====================

    /// 查询元素数量
    async fn count_elements(&self, filter: Option<&str>) -> anyhow::Result<u64> {
        Err(anyhow::anyhow!("未实现元素计数"))
    }

    /// 查询关系数量
    async fn count_relations(&self, rel_type: Option<&str>) -> anyhow::Result<u64> {
        Err(anyhow::anyhow!("未实现关系计数"))
    }
}

/// 数据库适配器错误类型
#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    #[error("数据库连接错误: {0}")]
    ConnectionError(String),

    #[error("查询错误: {0}")]
    QueryError(String),

    #[error("不支持的操作: {0}")]
    UnsupportedOperation(String),

    #[error("超时: {0}")]
    Timeout(String),

    #[error("数据不存在: {0}")]
    NotFound(String),

    #[error("数据冲突: {0}")]
    Conflict(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capabilities_default() {
        let caps = DatabaseCapabilities::default();
        assert!(!caps.supports_graph_traversal);
        assert!(!caps.supports_transactions);
    }

    #[test]
    fn test_query_context_default() {
        let ctx = QueryContext::default();
        assert_eq!(ctx.timeout_ms, Some(5000));
        assert!(!ctx.requires_graph_traversal);
        assert_eq!(ctx.priority, 5);
    }
}
