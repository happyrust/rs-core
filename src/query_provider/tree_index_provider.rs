//! TreeIndex 查询提供者实现（层级查询使用 indextree）

use super::error::{QueryError, QueryResult};
use super::surreal_provider::SurrealQueryProvider;
use super::traits::*;
use crate::tool::db_tool::db1_hash;
use crate::tree_query::{
    get_cached_tree_index, get_dbnum_by_refno, load_db_meta_info, TreeIndex, TreeQuery,
    TreeQueryFilter, TreeQueryOptions,
};
use crate::types::{NamedAttrMap as NamedAttMap, SPdmsElement as PE};
use crate::{RefU64, RefnoEnum};
use async_trait::async_trait;
use log::{debug, info, warn};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// TreeIndex 查询提供者（层级查询走 TreeIndex，其它查询委托 SurrealDB）
pub struct TreeIndexQueryProvider {
    name: String,
    /// dbnum -> TreeIndex 映射（用于快速查找）
    index_map: HashMap<u32, Arc<TreeIndex>>,
    surreal_provider: SurrealQueryProvider,
}

impl TreeIndexQueryProvider {
    /// 从目录中加载所有 .tree 文件和 db_meta_info.json
    pub fn from_tree_dir(tree_dir: impl Into<PathBuf>) -> QueryResult<Self> {
        let tree_dir = tree_dir.into();
        
        // 加载 db_meta_info.json（如果存在）
        let meta_path = tree_dir.join("db_meta_info.json");
        if meta_path.exists() {
            if let Err(e) = load_db_meta_info(&meta_path) {
                warn!("加载 db_meta_info.json 失败: {}", e);
            } else {
                info!("已加载 db_meta_info.json: {}", meta_path.display());
            }
        }
        
        // 加载所有 .tree 文件
        let indexes = load_tree_indexes_from_dir(&tree_dir)?;
        if indexes.is_empty() {
            return Err(QueryError::NotFound(format!(
                "未在目录 {} 中找到 .tree 文件",
                tree_dir.display()
            )));
        }
        
        // 构建 dbnum -> TreeIndex 映射
        let mut index_map = HashMap::new();
        for index in &indexes {
            index_map.insert(index.dbnum(), index.clone());
        }
        
        Ok(Self {
            name: "TreeIndex".to_string(),
            index_map,
            surreal_provider: SurrealQueryProvider::new()?,
        })
    }

    /// 根据 refno 查找对应的 TreeIndex（优先使用 ref0 -> dbnum 映射）
    fn find_index(&self, refno: RefU64) -> Option<Arc<TreeIndex>> {
        // 优先使用 ref0 -> dbnum 缓存（O(1) 查找）
        if let Some(dbnum) = get_dbnum_by_refno(refno) {
            if let Some(index) = self.index_map.get(&dbnum) {
                return Some(index.clone());
            }
        }
        
        // 回退：遍历所有 index 查找（兼容未加载 db_meta_info 的情况）
        for index in self.index_map.values() {
            if index.contains_refno(refno) {
                return Some(index.clone());
            }
        }
        None
    }

    fn build_filter(nouns: &[&str]) -> TreeQueryFilter {
        let noun_hashes = if nouns.is_empty() {
            None
        } else {
            Some(nouns.iter().map(|n| db1_hash(n)).collect())
        };
        TreeQueryFilter {
            noun_hashes,
            ..Default::default()
        }
    }

    fn build_descendants_options(
        nouns: &[&str],
        max_depth: Option<usize>,
        include_self: bool,
    ) -> TreeQueryOptions {
        TreeQueryOptions {
            include_self,
            max_depth,
            filter: Self::build_filter(nouns),
        }
    }
}

fn load_tree_indexes_from_dir(dir: &Path) -> QueryResult<Vec<Arc<TreeIndex>>> {
    let mut indexes = Vec::new();
    let entries = std::fs::read_dir(dir)
        .map_err(|e| QueryError::ExecutionError(format!("读取 tree 目录失败: {e}")))?;
    for entry in entries {
        let entry = entry
            .map_err(|e| QueryError::ExecutionError(format!("读取目录条目失败: {e}")))?;
        let path = entry.path();
        let is_tree = path
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.eq_ignore_ascii_case("tree"))
            .unwrap_or(false);
        if !is_tree {
            continue;
        }
        match crate::tree_query::load_tree_index_from_path(&path) {
            Ok(index) => indexes.push(index),
            Err(e) => {
                warn!("加载 tree 文件失败: {} -> {}", path.display(), e);
            }
        }
    }
    Ok(indexes)
}

// ============================================================================
// HierarchyQuery 实现（走 TreeIndex）
// ============================================================================

#[async_trait]
impl HierarchyQuery for TreeIndexQueryProvider {
    async fn get_children(&self, refno: RefnoEnum) -> QueryResult<Vec<RefnoEnum>> {
        debug!("[{}] get_children: {:?}", self.name, refno);
        let Some(index) = self.find_index(refno.refno()) else {
            return Ok(Vec::new());
        };
        let children = index
            .query_children(refno.refno(), TreeQueryFilter::default())
            .await
            .map_err(|e| QueryError::ExecutionError(e.to_string()))?;
        Ok(children.into_iter().map(RefnoEnum::from).collect())
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
        let Some(index) = self.find_index(refno.refno()) else {
            return Ok(Vec::new());
        };
        let options = Self::build_descendants_options(&[], max_depth, false);
        let descendants = index
            .query_descendants_bfs(refno.refno(), options)
            .await
            .map_err(|e| QueryError::ExecutionError(e.to_string()))?;
        Ok(descendants.into_iter().map(RefnoEnum::from).collect())
    }

    async fn get_ancestors(&self, refno: RefnoEnum) -> QueryResult<Vec<RefnoEnum>> {
        debug!("[{}] get_ancestors: {:?}", self.name, refno);
        let Some(index) = self.find_index(refno.refno()) else {
            return Ok(Vec::new());
        };
        let options = TreeQueryOptions {
            include_self: false,
            max_depth: None,
            filter: TreeQueryFilter::default(),
        };
        let ancestors = index
            .query_ancestors_root_to_parent(refno.refno(), options)
            .await
            .map_err(|e| QueryError::ExecutionError(e.to_string()))?;
        Ok(ancestors.into_iter().map(RefnoEnum::from).collect())
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
        let Some(index) = self.find_index(refno.refno()) else {
            return Ok(Vec::new());
        };
        let options = TreeQueryOptions {
            include_self: false,
            max_depth: None,
            filter: Self::build_filter(nouns),
        };
        let ancestors = index
            .query_ancestors_root_to_parent(refno.refno(), options)
            .await
            .map_err(|e| QueryError::ExecutionError(e.to_string()))?;
        Ok(ancestors.into_iter().map(RefnoEnum::from).collect())
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
        let Some(index) = self.find_index(refno.refno()) else {
            return Ok(Vec::new());
        };
        let options = Self::build_descendants_options(nouns, max_depth, false);
        let descendants = index
            .query_descendants_bfs(refno.refno(), options)
            .await
            .map_err(|e| QueryError::ExecutionError(e.to_string()))?;
        Ok(descendants.into_iter().map(RefnoEnum::from).collect())
    }

    async fn get_children_pes(&self, refno: RefnoEnum) -> QueryResult<Vec<PE>> {
        self.surreal_provider.get_children_pes(refno).await
    }
}

// ============================================================================
// TypeQuery/BatchQuery/GraphQuery/QueryProvider：委托 SurrealDB
// ============================================================================

#[async_trait]
impl TypeQuery for TreeIndexQueryProvider {
    async fn query_by_type(
        &self,
        nouns: &[&str],
        dbnum: i32,
        has_children: Option<bool>,
    ) -> QueryResult<Vec<RefnoEnum>> {
        self.surreal_provider
            .query_by_type(nouns, dbnum, has_children)
            .await
    }

    async fn query_by_type_name_contains(
        &self,
        nouns: &[&str],
        dbnum: i32,
        keyword: &str,
        case_sensitive: bool,
    ) -> QueryResult<Vec<RefnoEnum>> {
        self.surreal_provider
            .query_by_type_name_contains(nouns, dbnum, keyword, case_sensitive)
            .await
    }

    async fn query_by_type_multi_db(
        &self,
        nouns: &[&str],
        dbnums: &[i32],
    ) -> QueryResult<Vec<RefnoEnum>> {
        self.surreal_provider
            .query_by_type_multi_db(nouns, dbnums)
            .await
    }

    async fn get_world(&self, dbnum: i32) -> QueryResult<Option<RefnoEnum>> {
        self.surreal_provider.get_world(dbnum).await
    }

    async fn get_sites(&self, dbnum: i32) -> QueryResult<Vec<RefnoEnum>> {
        self.surreal_provider.get_sites(dbnum).await
    }

    async fn count_by_type(&self, noun: &str, dbnum: i32) -> QueryResult<usize> {
        self.surreal_provider.count_by_type(noun, dbnum).await
    }
}

#[async_trait]
impl BatchQuery for TreeIndexQueryProvider {
    async fn get_pes_batch(&self, refnos: &[RefnoEnum]) -> QueryResult<Vec<PE>> {
        self.surreal_provider.get_pes_batch(refnos).await
    }

    async fn get_attmaps_batch(&self, refnos: &[RefnoEnum]) -> QueryResult<Vec<NamedAttMap>> {
        self.surreal_provider.get_attmaps_batch(refnos).await
    }

    async fn get_full_names_batch(
        &self,
        refnos: &[RefnoEnum],
    ) -> QueryResult<Vec<(RefnoEnum, String)>> {
        self.surreal_provider.get_full_names_batch(refnos).await
    }
}

#[async_trait]
impl GraphQuery for TreeIndexQueryProvider {
    async fn query_multi_descendants(
        &self,
        refnos: &[RefnoEnum],
        nouns: &[&str],
    ) -> QueryResult<Vec<RefnoEnum>> {
        if refnos.is_empty() {
            return Ok(Vec::new());
        }
        // 模型生成路径约定：refnos 不会互相重叠（不会产生重复节点），因此这里不做去重，
        // 直接按 refnos 输入顺序拼接每个 root 的 BFS 结果，保证顺序稳定。
        let mut out: Vec<RefU64> = Vec::new();
        let options = Self::build_descendants_options(nouns, None, false);
        for refno in refnos {
            let Some(index) = self.find_index(refno.refno()) else {
                continue;
            };
            let descendants = index
                .query_descendants_bfs(refno.refno(), options.clone())
                .await
                .map_err(|e| QueryError::ExecutionError(e.to_string()))?;
            out.extend(descendants);
        }
        Ok(out.into_iter().map(RefnoEnum::from).collect())
    }

    async fn find_shortest_path(
        &self,
        from: RefnoEnum,
        to: RefnoEnum,
    ) -> QueryResult<Vec<RefnoEnum>> {
        self.surreal_provider.find_shortest_path(from, to).await
    }

    async fn get_node_depth(&self, refno: RefnoEnum) -> QueryResult<usize> {
        let Some(index) = self.find_index(refno.refno()) else {
            return Ok(0);
        };
        let options = TreeQueryOptions {
            include_self: false,
            max_depth: None,
            filter: TreeQueryFilter::default(),
        };
        let ancestors = index
            .query_ancestors_root_to_parent(refno.refno(), options)
            .await
            .map_err(|e| QueryError::ExecutionError(e.to_string()))?;
        Ok(ancestors.len())
    }
}

#[async_trait]
impl QueryProvider for TreeIndexQueryProvider {
    async fn get_pe(&self, refno: RefnoEnum) -> QueryResult<Option<PE>> {
        self.surreal_provider.get_pe(refno).await
    }

    async fn get_attmap(&self, refno: RefnoEnum) -> QueryResult<Option<NamedAttMap>> {
        self.surreal_provider.get_attmap(refno).await
    }

    async fn exists(&self, refno: RefnoEnum) -> QueryResult<bool> {
        self.surreal_provider.exists(refno).await
    }

    fn provider_name(&self) -> &str {
        &self.name
    }

    async fn health_check(&self) -> QueryResult<bool> {
        self.surreal_provider.health_check().await
    }
}
