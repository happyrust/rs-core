//! Kuzu 查询提供者实现

use super::error::{QueryError, QueryResult};
use super::traits::*;
use crate::RefnoEnum;
use crate::types::{NamedAttrMap as NamedAttMap, SPdmsElement as PE};
use async_trait::async_trait;
use log::{debug, warn};

#[cfg(feature = "kuzu")]
use crate::rs_kuzu;

/// Kuzu 查询提供者
///
/// 实现了 `QueryProvider` trait，将查询委托给 Kuzu 查询函数
pub struct KuzuQueryProvider {
    /// 提供者名称
    name: String,
}

impl KuzuQueryProvider {
    /// 创建新的 Kuzu 查询提供者
    #[cfg(feature = "kuzu")]
    pub fn new() -> QueryResult<Self> {
        Ok(Self {
            name: "Kuzu".to_string(),
        })
    }

    /// 创建新的 Kuzu 查询提供者（无 kuzu feature 时的占位实现）
    #[cfg(not(feature = "kuzu"))]
    pub fn new() -> QueryResult<Self> {
        Err(QueryError::Other("Kuzu feature is not enabled".into()))
    }

    /// 使用自定义名称创建查询提供者
    pub fn with_name(name: impl Into<String>) -> QueryResult<Self> {
        #[cfg(feature = "kuzu")]
        {
            Ok(Self { name: name.into() })
        }

        #[cfg(not(feature = "kuzu"))]
        {
            let _ = name;
            Err(QueryError::Other("Kuzu feature is not enabled".into()))
        }
    }
}

// ============================================================================
// HierarchyQuery 实现
// ============================================================================

#[async_trait]
impl HierarchyQuery for KuzuQueryProvider {
    async fn get_children(&self, refno: RefnoEnum) -> QueryResult<Vec<RefnoEnum>> {
        debug!("[{}] get_children: {:?}", self.name, refno);

        #[cfg(feature = "kuzu")]
        {
            rs_kuzu::queries::hierarchy::kuzu_get_children_refnos(refno)
                .await
                .map_err(|e| QueryError::ExecutionError(e.to_string()))
        }

        #[cfg(not(feature = "kuzu"))]
        {
            let _ = refno;
            Err(QueryError::Other("Kuzu feature is not enabled".into()))
        }
    }

    async fn get_children_batch(&self, refnos: &[RefnoEnum]) -> QueryResult<Vec<RefnoEnum>> {
        debug!("[{}] get_children_batch: {} items", self.name, refnos.len());

        #[cfg(feature = "kuzu")]
        {
            rs_kuzu::queries::batch::kuzu_get_all_children_refnos(refnos)
                .await
                .map_err(|e| QueryError::ExecutionError(e.to_string()))
        }

        #[cfg(not(feature = "kuzu"))]
        {
            let _ = refnos;
            Err(QueryError::Other("Kuzu feature is not enabled".into()))
        }
    }

    async fn get_descendants(
        &self,
        refno: RefnoEnum,
        max_depth: Option<usize>,
    ) -> QueryResult<Vec<RefnoEnum>> {
        debug!(
            "[{}] get_descendants: {:?}, depth: {:?}",
            self.name, refno, max_depth
        );

        #[cfg(feature = "kuzu")]
        {
            let depth = max_depth.unwrap_or(12);
            rs_kuzu::queries::hierarchy::kuzu_query_deep_children_refnos_with_depth(refno, depth)
                .await
                .map_err(|e| QueryError::ExecutionError(e.to_string()))
        }

        #[cfg(not(feature = "kuzu"))]
        {
            let _ = (refno, max_depth);
            Err(QueryError::Other("Kuzu feature is not enabled".into()))
        }
    }

    async fn get_ancestors(&self, refno: RefnoEnum) -> QueryResult<Vec<RefnoEnum>> {
        debug!("[{}] get_ancestors: {:?}", self.name, refno);

        #[cfg(feature = "kuzu")]
        {
            rs_kuzu::queries::hierarchy::kuzu_query_ancestor_refnos(refno)
                .await
                .map_err(|e| QueryError::ExecutionError(e.to_string()))
        }

        #[cfg(not(feature = "kuzu"))]
        {
            let _ = refno;
            Err(QueryError::Other("Kuzu feature is not enabled".into()))
        }
    }

    async fn get_ancestors_of_type(
        &self,
        refno: RefnoEnum,
        nouns: &[&str],
    ) -> QueryResult<Vec<RefnoEnum>> {
        debug!(
            "[{}] get_ancestors_of_type: {:?}, nouns: {:?}",
            self.name, refno, nouns
        );

        #[cfg(feature = "kuzu")]
        {
            rs_kuzu::queries::hierarchy::kuzu_query_filter_ancestors(refno, nouns)
                .await
                .map_err(|e| QueryError::ExecutionError(e.to_string()))
        }

        #[cfg(not(feature = "kuzu"))]
        {
            let _ = (refno, nouns);
            Err(QueryError::Other("Kuzu feature is not enabled".into()))
        }
    }

    async fn get_descendants_filtered(
        &self,
        refno: RefnoEnum,
        nouns: &[&str],
        max_depth: Option<usize>,
    ) -> QueryResult<Vec<RefnoEnum>> {
        debug!(
            "[{}] get_descendants_filtered: {:?}, nouns: {:?}, depth: {:?}",
            self.name, refno, nouns, max_depth
        );

        #[cfg(feature = "kuzu")]
        {
            let depth = max_depth.unwrap_or(12);
            rs_kuzu::queries::hierarchy::kuzu_query_filter_deep_children_with_depth(
                refno, nouns, depth,
            )
            .await
            .map_err(|e| QueryError::ExecutionError(e.to_string()))
        }

        #[cfg(not(feature = "kuzu"))]
        {
            let _ = (refno, nouns, max_depth);
            Err(QueryError::Other("Kuzu feature is not enabled".into()))
        }
    }

    async fn get_children_pes(&self, refno: RefnoEnum) -> QueryResult<Vec<PE>> {
        debug!("[{}] get_children_pes: {:?}", self.name, refno);

        #[cfg(feature = "kuzu")]
        {
            rs_kuzu::queries::pe_query::kuzu_get_children_pes(refno)
                .await
                .map_err(|e| QueryError::ExecutionError(e.to_string()))
        }

        #[cfg(not(feature = "kuzu"))]
        {
            let _ = refno;
            Err(QueryError::Other("Kuzu feature is not enabled".into()))
        }
    }
}

// ============================================================================
// TypeQuery 实现
// ============================================================================

#[async_trait]
impl TypeQuery for KuzuQueryProvider {
    async fn query_by_type(
        &self,
        nouns: &[&str],
        dbnum: i32,
        has_children: Option<bool>,
    ) -> QueryResult<Vec<RefnoEnum>> {
        debug!(
            "[{}] query_by_type: nouns={:?}, dbnum={}, has_children={:?}",
            self.name, nouns, dbnum, has_children
        );

        #[cfg(feature = "kuzu")]
        {
            rs_kuzu::queries::type_filter::kuzu_query_type_refnos_by_dbnum(
                nouns,
                dbnum,
                has_children,
            )
            .await
            .map_err(|e| QueryError::ExecutionError(e.to_string()))
        }

        #[cfg(not(feature = "kuzu"))]
        {
            let _ = (nouns, dbnum, has_children);
            Err(QueryError::Other("Kuzu feature is not enabled".into()))
        }
    }

    async fn query_by_type_multi_db(
        &self,
        nouns: &[&str],
        dbnums: &[i32],
    ) -> QueryResult<Vec<RefnoEnum>> {
        debug!(
            "[{}] query_by_type_multi_db: nouns={:?}, dbnums={:?}",
            self.name, nouns, dbnums
        );

        #[cfg(feature = "kuzu")]
        {
            rs_kuzu::queries::type_filter::kuzu_query_type_refnos_by_dbnums(nouns, dbnums)
                .await
                .map_err(|e| QueryError::ExecutionError(e.to_string()))
        }

        #[cfg(not(feature = "kuzu"))]
        {
            let _ = (nouns, dbnums);
            Err(QueryError::Other("Kuzu feature is not enabled".into()))
        }
    }

    async fn get_world(&self, dbnum: i32) -> QueryResult<Option<RefnoEnum>> {
        debug!("[{}] get_world: dbnum={}", self.name, dbnum);

        #[cfg(feature = "kuzu")]
        {
            rs_kuzu::queries::type_filter::kuzu_get_world_by_dbnum(dbnum)
                .await
                .map_err(|e| QueryError::ExecutionError(e.to_string()))
        }

        #[cfg(not(feature = "kuzu"))]
        {
            let _ = dbnum;
            Err(QueryError::Other("Kuzu feature is not enabled".into()))
        }
    }

    async fn get_sites(&self, dbnum: i32) -> QueryResult<Vec<RefnoEnum>> {
        debug!("[{}] get_sites: dbnum={}", self.name, dbnum);

        #[cfg(feature = "kuzu")]
        {
            rs_kuzu::queries::type_filter::kuzu_get_sites_of_dbnum(dbnum)
                .await
                .map_err(|e| QueryError::ExecutionError(e.to_string()))
        }

        #[cfg(not(feature = "kuzu"))]
        {
            let _ = dbnum;
            Err(QueryError::Other("Kuzu feature is not enabled".into()))
        }
    }

    async fn count_by_type(&self, noun: &str, dbnum: i32) -> QueryResult<usize> {
        debug!(
            "[{}] count_by_type: noun={}, dbnum={}",
            self.name, noun, dbnum
        );

        #[cfg(feature = "kuzu")]
        {
            rs_kuzu::queries::type_filter::kuzu_count_by_type(noun, dbnum)
                .await
                .map_err(|e| QueryError::ExecutionError(e.to_string()))
        }

        #[cfg(not(feature = "kuzu"))]
        {
            let _ = (noun, dbnum);
            Err(QueryError::Other("Kuzu feature is not enabled".into()))
        }
    }
}

// ============================================================================
// BatchQuery 实现
// ============================================================================

#[async_trait]
impl BatchQuery for KuzuQueryProvider {
    async fn get_pes_batch(&self, refnos: &[RefnoEnum]) -> QueryResult<Vec<PE>> {
        debug!("[{}] get_pes_batch: {} items", self.name, refnos.len());

        #[cfg(feature = "kuzu")]
        {
            rs_kuzu::queries::batch::kuzu_get_pes_batch(refnos)
                .await
                .map_err(|e| QueryError::ExecutionError(e.to_string()))
        }

        #[cfg(not(feature = "kuzu"))]
        {
            let _ = refnos;
            Err(QueryError::Other("Kuzu feature is not enabled".into()))
        }
    }

    async fn get_attmaps_batch(&self, refnos: &[RefnoEnum]) -> QueryResult<Vec<NamedAttMap>> {
        debug!("[{}] get_attmaps_batch: {} items", self.name, refnos.len());

        #[cfg(feature = "kuzu")]
        {
            rs_kuzu::queries::attr_query::kuzu_get_attmaps_batch(refnos)
                .await
                .map_err(|e| QueryError::ExecutionError(e.to_string()))
        }

        #[cfg(not(feature = "kuzu"))]
        {
            let _ = refnos;
            Err(QueryError::Other("Kuzu feature is not enabled".into()))
        }
    }

    async fn get_full_names_batch(
        &self,
        refnos: &[RefnoEnum],
    ) -> QueryResult<Vec<(RefnoEnum, String)>> {
        debug!(
            "[{}] get_full_names_batch: {} items",
            self.name,
            refnos.len()
        );

        #[cfg(feature = "kuzu")]
        {
            rs_kuzu::queries::batch::kuzu_query_full_names_map(refnos)
                .await
                .map_err(|e| QueryError::ExecutionError(e.to_string()))
        }

        #[cfg(not(feature = "kuzu"))]
        {
            let _ = refnos;
            Err(QueryError::Other("Kuzu feature is not enabled".into()))
        }
    }
}

// ============================================================================
// GraphQuery 实现
// ============================================================================

#[async_trait]
impl GraphQuery for KuzuQueryProvider {
    async fn query_multi_descendants(
        &self,
        refnos: &[RefnoEnum],
        nouns: &[&str],
        max_depth: Option<usize>,
    ) -> QueryResult<Vec<RefnoEnum>> {
        debug!(
            "[{}] query_multi_descendants: {} refnos, {:?} nouns, depth: {:?}",
            self.name,
            refnos.len(),
            nouns,
            max_depth
        );

        #[cfg(feature = "kuzu")]
        {
            let depth = max_depth.unwrap_or(12);
            rs_kuzu::queries::multi_filter::kuzu_query_multi_filter_deep_children(
                refnos, nouns, depth,
            )
            .await
            .map_err(|e| QueryError::ExecutionError(e.to_string()))
        }

        #[cfg(not(feature = "kuzu"))]
        {
            let _ = (refnos, nouns, max_depth);
            Err(QueryError::Other("Kuzu feature is not enabled".into()))
        }
    }

    async fn find_shortest_path(
        &self,
        from: RefnoEnum,
        to: RefnoEnum,
    ) -> QueryResult<Vec<RefnoEnum>> {
        debug!("[{}] find_shortest_path: {:?} -> {:?}", self.name, from, to);

        #[cfg(feature = "kuzu")]
        {
            rs_kuzu::queries::graph_traverse::kuzu_find_shortest_path(from, to)
                .await
                .map_err(|e| QueryError::ExecutionError(e.to_string()))
        }

        #[cfg(not(feature = "kuzu"))]
        {
            let _ = (from, to);
            Err(QueryError::Other("Kuzu feature is not enabled".into()))
        }
    }

    async fn get_node_depth(&self, refno: RefnoEnum) -> QueryResult<usize> {
        debug!("[{}] get_node_depth: {:?}", self.name, refno);

        #[cfg(feature = "kuzu")]
        {
            rs_kuzu::queries::hierarchy::kuzu_get_node_depth(refno)
                .await
                .map_err(|e| QueryError::ExecutionError(e.to_string()))
        }

        #[cfg(not(feature = "kuzu"))]
        {
            let _ = refno;
            Err(QueryError::Other("Kuzu feature is not enabled".into()))
        }
    }
}

// ============================================================================
// QueryProvider 实现
// ============================================================================

#[async_trait]
impl QueryProvider for KuzuQueryProvider {
    async fn get_pe(&self, refno: RefnoEnum) -> QueryResult<Option<PE>> {
        debug!("[{}] get_pe: {:?}", self.name, refno);

        #[cfg(feature = "kuzu")]
        {
            rs_kuzu::queries::pe_query::kuzu_get_pe(refno)
                .await
                .map_err(|e| QueryError::ExecutionError(e.to_string()))
        }

        #[cfg(not(feature = "kuzu"))]
        {
            let _ = refno;
            Err(QueryError::Other("Kuzu feature is not enabled".into()))
        }
    }

    async fn get_attmap(&self, refno: RefnoEnum) -> QueryResult<Option<NamedAttMap>> {
        debug!("[{}] get_attmap: {:?}", self.name, refno);

        #[cfg(feature = "kuzu")]
        {
            rs_kuzu::queries::attr_query::kuzu_get_named_attmap(refno)
                .await
                .map_err(|e| QueryError::ExecutionError(e.to_string()))
        }

        #[cfg(not(feature = "kuzu"))]
        {
            let _ = refno;
            Err(QueryError::Other("Kuzu feature is not enabled".into()))
        }
    }

    async fn exists(&self, refno: RefnoEnum) -> QueryResult<bool> {
        debug!("[{}] exists: {:?}", self.name, refno);
        Ok(self.get_pe(refno).await?.is_some())
    }

    fn provider_name(&self) -> &str {
        &self.name
    }

    async fn health_check(&self) -> QueryResult<bool> {
        debug!("[{}] health_check", self.name);

        #[cfg(feature = "kuzu")]
        {
            // 尝试创建连接来检查健康状态
            match rs_kuzu::create_kuzu_connection() {
                Ok(_) => Ok(true),
                Err(e) => {
                    warn!("[{}] health check failed: {}", self.name, e);
                    Ok(false)
                }
            }
        }

        #[cfg(not(feature = "kuzu"))]
        {
            Ok(false)
        }
    }
}
