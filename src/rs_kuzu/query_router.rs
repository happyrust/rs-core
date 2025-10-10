//! Kuzu 查询路由器
//!
//! 提供统一的查询接口，支持在 SurrealDB、Kuzu 和自动选择模式之间切换

use crate::rs_surreal;
use crate::rs_surreal::graph as surreal_graph;
use crate::rs_surreal::mdb as surreal_mdb;
use crate::rs_kuzu::queries::{hierarchy as kuzu_hierarchy, type_filter as kuzu_type_filter};
use crate::types::RefnoEnum;
use anyhow::Result;
use std::sync::Arc;
use parking_lot::RwLock;
use once_cell::sync::Lazy;

/// 查询引擎选择策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryEngine {
    /// 使用 SurrealDB
    SurrealDB,
    /// 使用 Kuzu
    Kuzu,
    /// 自动选择 (优先 Kuzu，失败则回退到 SurrealDB)
    Auto,
}

impl Default for QueryEngine {
    fn default() -> Self {
        Self::Auto
    }
}

impl QueryEngine {
    /// 从字符串解析
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "surrealdb" | "surreal" => Ok(Self::SurrealDB),
            "kuzu" => Ok(Self::Kuzu),
            "auto" => Ok(Self::Auto),
            _ => Err(anyhow::anyhow!("未知的查询引擎: {}", s)),
        }
    }
}

/// 全局查询引擎配置
pub static QUERY_ENGINE: Lazy<Arc<RwLock<QueryEngine>>> =
    Lazy::new(|| Arc::new(RwLock::new(QueryEngine::Auto)));

/// 设置全局查询引擎
pub fn set_query_engine(engine: QueryEngine) {
    *QUERY_ENGINE.write() = engine;
}

/// 获取当前查询引擎
pub fn get_query_engine() -> QueryEngine {
    *QUERY_ENGINE.read()
}

/// 统一查询路由器
///
/// 提供统一的查询接口，根据配置的策略自动选择 SurrealDB 或 Kuzu
#[derive(Debug, Clone)]
pub struct QueryRouter {
    strategy: QueryEngine,
}

impl QueryRouter {
    /// 创建新的路由器，使用指定策略
    pub fn new(strategy: QueryEngine) -> Self {
        Self { strategy }
    }

    /// 创建使用全局配置的路由器
    pub fn from_global() -> Self {
        Self {
            strategy: get_query_engine(),
        }
    }

    /// 创建使用 Auto 模式的路由器
    pub fn auto() -> Self {
        Self {
            strategy: QueryEngine::Auto,
        }
    }

    /// 获取直接子节点
    ///
    /// # 参数
    /// * `refno` - 父节点 refno
    ///
    /// # 返回
    /// * `Result<Vec<RefnoEnum>>` - 子节点列表
    pub async fn get_children_refnos(&self, refno: RefnoEnum) -> Result<Vec<RefnoEnum>> {
        match self.strategy {
            QueryEngine::SurrealDB => {
                rs_surreal::get_children_refnos(refno).await
            }
            QueryEngine::Kuzu => {
                kuzu_hierarchy::kuzu_get_children_refnos(refno).await
            }
            QueryEngine::Auto => {
                self.auto_get_children_refnos(refno).await
            }
        }
    }

    /// Auto 模式: 获取子节点
    async fn auto_get_children_refnos(&self, refno: RefnoEnum) -> Result<Vec<RefnoEnum>> {
        // 优先尝试 Kuzu
        match kuzu_hierarchy::kuzu_get_children_refnos(refno).await {
            Ok(result) => {
                log::debug!("✓ Kuzu query succeeded for get_children_refnos");
                Ok(result)
            }
            Err(e) => {
                log::warn!("Kuzu query failed, fallback to SurrealDB: {}", e);
                rs_surreal::get_children_refnos(refno).await
            }
        }
    }

    /// 查询所有祖先
    ///
    /// # 参数
    /// * `refno` - 子节点 refno
    ///
    /// # 返回
    /// * `Result<Vec<RefnoEnum>>` - 祖先列表
    pub async fn query_ancestor_refnos(&self, refno: RefnoEnum) -> Result<Vec<RefnoEnum>> {
        match self.strategy {
            QueryEngine::SurrealDB => {
                crate::query_ancestor_refnos(refno).await
            }
            QueryEngine::Kuzu => {
                kuzu_hierarchy::kuzu_query_ancestor_refnos(refno).await
            }
            QueryEngine::Auto => {
                self.auto_query_ancestor_refnos(refno).await
            }
        }
    }

    /// Auto 模式: 查询祖先
    async fn auto_query_ancestor_refnos(&self, refno: RefnoEnum) -> Result<Vec<RefnoEnum>> {
        match kuzu_hierarchy::kuzu_query_ancestor_refnos(refno).await {
            Ok(result) => {
                log::debug!("✓ Kuzu query succeeded for query_ancestor_refnos");
                Ok(result)
            }
            Err(e) => {
                log::warn!("Kuzu query failed, fallback to SurrealDB: {}", e);
                crate::query_ancestor_refnos(refno).await
            }
        }
    }

    /// 查询深层子孙 (12层递归)
    ///
    /// # 参数
    /// * `refno` - 父节点 refno
    ///
    /// # 返回
    /// * `Result<Vec<RefnoEnum>>` - 所有子孙列表
    pub async fn query_deep_children_refnos(&self, refno: RefnoEnum) -> Result<Vec<RefnoEnum>> {
        match self.strategy {
            QueryEngine::SurrealDB => {
                surreal_graph::query_deep_children_refnos(refno).await
            }
            QueryEngine::Kuzu => {
                kuzu_hierarchy::kuzu_query_deep_children_refnos(refno).await
            }
            QueryEngine::Auto => {
                self.auto_query_deep_children_refnos(refno).await
            }
        }
    }

    /// Auto 模式: 查询深层子孙
    async fn auto_query_deep_children_refnos(&self, refno: RefnoEnum) -> Result<Vec<RefnoEnum>> {
        match kuzu_hierarchy::kuzu_query_deep_children_refnos(refno).await {
            Ok(result) => {
                log::debug!("✓ Kuzu query succeeded for query_deep_children_refnos");
                Ok(result)
            }
            Err(e) => {
                log::warn!("Kuzu query failed, fallback to SurrealDB: {}", e);
                surreal_graph::query_deep_children_refnos(refno).await
            }
        }
    }

    /// 按类型过滤深层子孙
    ///
    /// # 参数
    /// * `refno` - 父节点 refno
    /// * `nouns` - 类型过滤列表
    ///
    /// # 返回
    /// * `Result<Vec<RefnoEnum>>` - 匹配的子孙列表
    pub async fn query_filter_deep_children(
        &self,
        refno: RefnoEnum,
        nouns: &[&str],
    ) -> Result<Vec<RefnoEnum>> {
        match self.strategy {
            QueryEngine::SurrealDB => {
                surreal_graph::query_filter_deep_children(refno, nouns).await
            }
            QueryEngine::Kuzu => {
                kuzu_hierarchy::kuzu_query_filter_deep_children(refno, nouns).await
            }
            QueryEngine::Auto => {
                self.auto_query_filter_deep_children(refno, nouns).await
            }
        }
    }

    /// Auto 模式: 按类型过滤深层子孙
    async fn auto_query_filter_deep_children(
        &self,
        refno: RefnoEnum,
        nouns: &[&str],
    ) -> Result<Vec<RefnoEnum>> {
        match kuzu_hierarchy::kuzu_query_filter_deep_children(refno, nouns).await {
            Ok(result) => {
                log::debug!("✓ Kuzu query succeeded for query_filter_deep_children");
                Ok(result)
            }
            Err(e) => {
                log::warn!("Kuzu query failed, fallback to SurrealDB: {}", e);
                surreal_graph::query_filter_deep_children(refno, nouns).await
            }
        }
    }

    /// 按 dbnum 和 noun 查询
    ///
    /// # 参数
    /// * `nouns` - noun 类型列表
    /// * `dbnum` - 数据库编号
    /// * `has_children` - 是否有子节点过滤
    ///
    /// # 返回
    /// * `Result<Vec<RefnoEnum>>` - 匹配的元素列表
    pub async fn query_type_refnos_by_dbnum(
        &self,
        nouns: &[&str],
        dbnum: u32,
        has_children: Option<bool>,
    ) -> Result<Vec<RefnoEnum>> {
        match self.strategy {
            QueryEngine::SurrealDB => {
                surreal_mdb::query_type_refnos_by_dbnum(nouns, dbnum, has_children, false).await
            }
            QueryEngine::Kuzu => {
                kuzu_type_filter::kuzu_query_type_refnos_by_dbnum(nouns, dbnum as i32, has_children).await
            }
            QueryEngine::Auto => {
                self.auto_query_type_refnos_by_dbnum(nouns, dbnum, has_children).await
            }
        }
    }

    /// Auto 模式: 按 dbnum 和 noun 查询
    async fn auto_query_type_refnos_by_dbnum(
        &self,
        nouns: &[&str],
        dbnum: u32,
        has_children: Option<bool>,
    ) -> Result<Vec<RefnoEnum>> {
        match kuzu_type_filter::kuzu_query_type_refnos_by_dbnum(nouns, dbnum as i32, has_children).await {
            Ok(result) => {
                log::debug!("✓ Kuzu query succeeded for query_type_refnos_by_dbnum");
                Ok(result)
            }
            Err(e) => {
                log::warn!("Kuzu query failed, fallback to SurrealDB: {}", e);
                surreal_mdb::query_type_refnos_by_dbnum(nouns, dbnum, has_children, false).await
            }
        }
    }

    /// 获取 WORLD 节点
    ///
    /// # 参数
    /// * `dbnum` - 数据库编号
    ///
    /// # 返回
    /// * `Result<Option<RefnoEnum>>` - WORLD 节点 refno
    pub async fn get_world_by_dbnum(&self, dbnum: u32) -> Result<Option<RefnoEnum>> {
        match self.strategy {
            QueryEngine::SurrealDB => {
                // SurrealDB: 查询 WORLD 类型
                surreal_mdb::query_type_refnos_by_dbnum(&["WORLD"], dbnum, None, false)
                    .await
                    .map(|v| v.into_iter().next())
            }
            QueryEngine::Kuzu => {
                kuzu_type_filter::kuzu_get_world_by_dbnum(dbnum as i32).await
            }
            QueryEngine::Auto => {
                self.auto_get_world_by_dbnum(dbnum).await
            }
        }
    }

    /// Auto 模式: 获取 WORLD 节点
    async fn auto_get_world_by_dbnum(&self, dbnum: u32) -> Result<Option<RefnoEnum>> {
        match kuzu_type_filter::kuzu_get_world_by_dbnum(dbnum as i32).await {
            Ok(result) => {
                log::debug!("✓ Kuzu query succeeded for get_world_by_dbnum");
                Ok(result)
            }
            Err(e) => {
                log::warn!("Kuzu query failed, fallback to SurrealDB: {}", e);
                surreal_mdb::query_type_refnos_by_dbnum(&["WORLD"], dbnum, None, false)
                    .await
                    .map(|v| v.into_iter().next())
            }
        }
    }
}

impl Default for QueryRouter {
    fn default() -> Self {
        Self::from_global()
    }
}

/// 便捷函数: 使用全局配置的路由器获取子节点
pub async fn routed_get_children_refnos(refno: RefnoEnum) -> Result<Vec<RefnoEnum>> {
    QueryRouter::from_global().get_children_refnos(refno).await
}

/// 便捷函数: 使用全局配置的路由器查询祖先
pub async fn routed_query_ancestor_refnos(refno: RefnoEnum) -> Result<Vec<RefnoEnum>> {
    QueryRouter::from_global().query_ancestor_refnos(refno).await
}

/// 便捷函数: 使用全局配置的路由器查询深层子孙
pub async fn routed_query_deep_children_refnos(refno: RefnoEnum) -> Result<Vec<RefnoEnum>> {
    QueryRouter::from_global().query_deep_children_refnos(refno).await
}

/// 便捷函数: 使用全局配置的路由器按类型过滤
pub async fn routed_query_type_refnos_by_dbnum(
    nouns: &[&str],
    dbnum: u32,
    has_children: Option<bool>,
) -> Result<Vec<RefnoEnum>> {
    QueryRouter::from_global().query_type_refnos_by_dbnum(nouns, dbnum, has_children).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::RefU64;

    #[test]
    fn test_query_engine_from_str() {
        assert_eq!(QueryEngine::from_str("surrealdb").unwrap(), QueryEngine::SurrealDB);
        assert_eq!(QueryEngine::from_str("kuzu").unwrap(), QueryEngine::Kuzu);
        assert_eq!(QueryEngine::from_str("auto").unwrap(), QueryEngine::Auto);
        assert!(QueryEngine::from_str("invalid").is_err());
    }

    #[test]
    fn test_set_get_query_engine() {
        set_query_engine(QueryEngine::Kuzu);
        assert_eq!(get_query_engine(), QueryEngine::Kuzu);

        set_query_engine(QueryEngine::SurrealDB);
        assert_eq!(get_query_engine(), QueryEngine::SurrealDB);

        set_query_engine(QueryEngine::Auto);
        assert_eq!(get_query_engine(), QueryEngine::Auto);
    }

    #[tokio::test]
    #[ignore] // 需要数据库环境
    async fn test_query_router_children() {
        let router = QueryRouter::new(QueryEngine::Auto);
        let refno = RefnoEnum::from(RefU64(123));
        let result = router.get_children_refnos(refno).await;
        // 应该返回结果（无论是从 Kuzu 还是 SurrealDB）
        assert!(result.is_ok());
    }
}
