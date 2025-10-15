//! SurrealDB 查询提供者实现

use super::error::{QueryError, QueryResult};
use super::traits::*;
use crate::RefnoEnum;
use crate::rs_surreal;
use crate::types::{NamedAttrMap as NamedAttMap, SPdmsElement as PE};
use async_trait::async_trait;
use log::{debug, warn};

/// SurrealDB 查询提供者
///
/// 实现了 `QueryProvider` trait，将查询委托给现有的 SurrealDB 查询函数
pub struct SurrealQueryProvider {
    /// 提供者名称
    name: String,
}

impl SurrealQueryProvider {
    /// 创建新的 SurrealDB 查询提供者
    pub fn new() -> QueryResult<Self> {
        Ok(Self {
            name: "SurrealDB".to_string(),
        })
    }

    /// 使用自定义名称创建查询提供者
    pub fn with_name(name: impl Into<String>) -> QueryResult<Self> {
        Ok(Self { name: name.into() })
    }
}

impl Default for SurrealQueryProvider {
    fn default() -> Self {
        Self::new().expect("Failed to create SurrealQueryProvider")
    }
}

// ============================================================================
// HierarchyQuery 实现
// ============================================================================

#[async_trait]
impl HierarchyQuery for SurrealQueryProvider {
    async fn get_children(&self, refno: RefnoEnum) -> QueryResult<Vec<RefnoEnum>> {
        debug!("[{}] get_children: {:?}", self.name, refno);
        rs_surreal::get_children_refnos(refno)
            .await
            .map_err(|e| QueryError::ExecutionError(e.to_string()))
    }

    async fn get_children_batch(&self, refnos: &[RefnoEnum]) -> QueryResult<Vec<RefnoEnum>> {
        debug!("[{}] get_children_batch: {} items", self.name, refnos.len());
        rs_surreal::query::get_all_children_refnos(refnos)
            .await
            .map_err(|e| QueryError::ExecutionError(e.to_string()))
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

        // SurrealDB 的深层查询默认是 12 层
        if let Some(depth) = max_depth {
            if depth > 12 {
                warn!("SurrealDB 最大支持 12 层递归，实际深度: {}", depth);
            }
        }

        rs_surreal::graph::query_deep_children_refnos(refno)
            .await
            .map_err(|e| QueryError::ExecutionError(e.to_string()))
    }

    async fn get_ancestors(&self, refno: RefnoEnum) -> QueryResult<Vec<RefnoEnum>> {
        debug!("[{}] get_ancestors: {:?}", self.name, refno);
        crate::query_ancestor_refnos(refno)
            .await
            .map_err(|e| QueryError::ExecutionError(e.to_string()))
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
        rs_surreal::graph::query_filter_ancestors(refno, nouns)
            .await
            .map_err(|e| QueryError::ExecutionError(e.to_string()))
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

        if max_depth.is_some() && max_depth.unwrap() > 12 {
            warn!("SurrealDB 最大支持 12 层递归");
        }

        rs_surreal::graph::query_filter_deep_children(refno, nouns)
            .await
            .map_err(|e| QueryError::ExecutionError(e.to_string()))
    }

    async fn get_children_pes(&self, refno: RefnoEnum) -> QueryResult<Vec<PE>> {
        debug!("[{}] get_children_pes: {:?}", self.name, refno);
        rs_surreal::query::get_children_pes(refno)
            .await
            .map_err(|e| QueryError::ExecutionError(e.to_string()))
    }
}

// ============================================================================
// TypeQuery 实现
// ============================================================================

#[async_trait]
impl TypeQuery for SurrealQueryProvider {
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

        rs_surreal::mdb::query_type_refnos_by_dbnum(nouns, dbnum as u32, has_children, false)
            .await
            .map_err(|e| QueryError::ExecutionError(e.to_string()))
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

        // 转换 i32 到 u32
        let dbnums_u32: Vec<u32> = dbnums.iter().map(|&d| d as u32).collect();
        rs_surreal::mdb::query_type_refnos_by_dbnums(nouns, &dbnums_u32)
            .await
            .map_err(|e| QueryError::ExecutionError(e.to_string()))
    }

    async fn get_world(&self, dbnum: i32) -> QueryResult<Option<RefnoEnum>> {
        debug!("[{}] get_world: dbnum={}", self.name, dbnum);
        // 查询 WORLD 类型
        let worlds = self.query_by_type(&["WORLD"], dbnum, None).await?;
        Ok(worlds.first().copied())
    }

    async fn get_sites(&self, dbnum: i32) -> QueryResult<Vec<RefnoEnum>> {
        debug!("[{}] get_sites: dbnum={}", self.name, dbnum);
        // 查询 SITE 类型
        self.query_by_type(&["SITE"], dbnum, None).await
    }

    async fn count_by_type(&self, noun: &str, dbnum: i32) -> QueryResult<usize> {
        debug!(
            "[{}] count_by_type: noun={}, dbnum={}",
            self.name, noun, dbnum
        );

        // 通过查询所有元素并统计数量
        let refnos = self.query_by_type(&[noun], dbnum, None).await?;
        Ok(refnos.len())
    }
}

// ============================================================================
// BatchQuery 实现
// ============================================================================

#[async_trait]
impl BatchQuery for SurrealQueryProvider {
    async fn get_pes_batch(&self, refnos: &[RefnoEnum]) -> QueryResult<Vec<PE>> {
        debug!("[{}] get_pes_batch: {} items", self.name, refnos.len());

        // 逐个查询（SurrealDB 没有原生的批量 PE 查询）
        let mut results = Vec::new();
        for &refno in refnos {
            if let Ok(Some(pe)) = rs_surreal::get_pe(refno).await {
                results.push(pe);
            }
        }
        Ok(results)
    }

    async fn get_attmaps_batch(&self, refnos: &[RefnoEnum]) -> QueryResult<Vec<NamedAttMap>> {
        debug!("[{}] get_attmaps_batch: {} items", self.name, refnos.len());

        // 逐个查询
        let mut results = Vec::new();
        for &refno in refnos {
            if let Ok(attmap) = rs_surreal::get_named_attmap(refno).await {
                results.push(attmap);
            }
        }
        Ok(results)
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

        rs_surreal::query_full_names_map(refnos)
            .await
            .map(|map| map.into_iter().collect())
            .map_err(|e| QueryError::ExecutionError(e.to_string()))
    }
}

// ============================================================================
// GraphQuery 实现
// ============================================================================

#[async_trait]
impl GraphQuery for SurrealQueryProvider {
    async fn query_multi_descendants(
        &self,
        refnos: &[RefnoEnum],
        nouns: &[&str],
    ) -> QueryResult<Vec<RefnoEnum>> {
        debug!(
            "[{}] query_multi_descendants: {} refnos, {:?} nouns",
            self.name,
            refnos.len(),
            nouns
        );

        rs_surreal::graph::query_multi_filter_deep_children(refnos, nouns)
            .await
            .map(|set| set.into_iter().collect())
            .map_err(|e| QueryError::ExecutionError(e.to_string()))
    }

    async fn find_shortest_path(
        &self,
        from: RefnoEnum,
        to: RefnoEnum,
    ) -> QueryResult<Vec<RefnoEnum>> {
        debug!("[{}] find_shortest_path: {:?} -> {:?}", self.name, from, to);

        // SurrealDB 没有直接的最短路径查询，需要自己实现
        // 简单实现：先查询 from 的所有祖先，看 to 是否在其中
        let ancestors = self.get_ancestors(from).await?;

        if ancestors.contains(&to) {
            // to 是 from 的祖先
            let mut path = vec![from];
            let mut current = from;

            while current != to {
                // 获取父节点
                if let Ok(Some(pe)) = rs_surreal::get_pe(current).await {
                    current = pe.owner;
                    path.push(current);
                } else {
                    break;
                }
            }

            Ok(path)
        } else {
            // 检查 from 是否是 to 的祖先
            let to_ancestors = self.get_ancestors(to).await?;
            if to_ancestors.contains(&from) {
                let mut path = self.find_shortest_path(to, from).await?;
                path.reverse();
                Ok(path)
            } else {
                // 没有直接路径
                Ok(Vec::new())
            }
        }
    }

    async fn get_node_depth(&self, refno: RefnoEnum) -> QueryResult<usize> {
        debug!("[{}] get_node_depth: {:?}", self.name, refno);

        let ancestors = self.get_ancestors(refno).await?;
        Ok(ancestors.len())
    }
}

// ============================================================================
// QueryProvider 实现
// ============================================================================

#[async_trait]
impl QueryProvider for SurrealQueryProvider {
    async fn get_pe(&self, refno: RefnoEnum) -> QueryResult<Option<PE>> {
        debug!("[{}] get_pe: {:?}", self.name, refno);
        rs_surreal::get_pe(refno)
            .await
            .map_err(|e| QueryError::ExecutionError(e.to_string()))
    }

    async fn get_attmap(&self, refno: RefnoEnum) -> QueryResult<Option<NamedAttMap>> {
        debug!("[{}] get_attmap: {:?}", self.name, refno);
        rs_surreal::get_named_attmap(refno)
            .await
            .map(Some)
            .map_err(|e| QueryError::ExecutionError(e.to_string()))
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

        // 尝试执行一个简单的查询来检查连接
        match rs_surreal::SUL_DB.health().await {
            Ok(_) => Ok(true),
            Err(e) => {
                warn!("[{}] health check failed: {}", self.name, e);
                Ok(false)
            }
        }
    }
}
