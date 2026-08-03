use crate::pdms_types::{
    BRAN_COMPONENT_NOUN_NAMES, GNERAL_LOOP_OWNER_NOUN_NAMES, GNERAL_PRIM_NOUN_NAMES,
    USE_CATE_NOUN_NAMES,
};
use crate::tool::db_tool::{db1_dehash, db1_hash};
use crate::{RefU64, RefnoEnum, SUL_DB, SurrealQueryExt};
use async_trait::async_trait;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
use std::path::Path;

#[derive(Debug, Clone, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct TreeNodeMeta {
    pub refno: RefU64,
    pub owner: RefU64,
    pub noun: u32,
    pub cata_hash: Option<u64>,
}

#[derive(Debug, Clone, Default)]
pub struct TreeQueryFilter {
    pub has_geo: Option<bool>,
    pub is_leaf: Option<bool>,
    pub noun_hashes: Option<HashSet<u32>>,
}

impl TreeQueryFilter {
    /// 供各 `TreeQuery` 实现复用的节点匹配语义，保证不同层级数据源（SurrealDB 图查询、
    /// 调用方自建的内存快照）对 has_geo / is_leaf / noun 三个维度的判定完全一致。
    pub fn matches(&self, node: &TreeNodeMeta, node_has_geo: bool, node_is_leaf: bool) -> bool {
        if let Some(filter_has_geo) = self.has_geo {
            if node_has_geo != filter_has_geo {
                return false;
            }
        }
        if let Some(filter_is_leaf) = self.is_leaf {
            if node_is_leaf != filter_is_leaf {
                return false;
            }
        }
        if let Some(hashes) = &self.noun_hashes {
            if !hashes.contains(&node.noun) {
                return false;
            }
        }
        true
    }
}

#[derive(Debug, Clone)]
pub struct TreeQueryOptions {
    pub include_self: bool,
    pub max_depth: Option<usize>,
    pub filter: TreeQueryFilter,
    /// 匹配到目标节点后不再递归其子节点
    pub prune_on_match: bool,
}

impl Default for TreeQueryOptions {
    fn default() -> Self {
        Self {
            include_self: true,
            max_depth: None,
            filter: TreeQueryFilter::default(),
            prune_on_match: false,
        }
    }
}

#[async_trait]
pub trait TreeQuery: Send + Sync {
    async fn get_node_meta(&self, refno: RefU64) -> anyhow::Result<Option<TreeNodeMeta>>;

    async fn query_children(
        &self,
        parent: RefU64,
        filter: TreeQueryFilter,
    ) -> anyhow::Result<Vec<RefU64>>;

    async fn query_descendants_bfs(
        &self,
        root: RefU64,
        options: TreeQueryOptions,
    ) -> anyhow::Result<Vec<RefU64>>;

    async fn query_ancestors_root_to_parent(
        &self,
        node: RefU64,
        options: TreeQueryOptions,
    ) -> anyhow::Result<Vec<RefU64>>;
}

#[derive(Debug, Default, Clone)]
pub struct SurrealTreeQuery;

#[async_trait]
impl TreeQuery for SurrealTreeQuery {
    //todo 使用 IndexTree里已有的信息
    async fn get_node_meta(&self, refno: RefU64) -> anyhow::Result<Option<TreeNodeMeta>> {
        let Some(pe) = crate::rs_surreal::get_pe(RefnoEnum::from(refno)).await? else {
            return Ok(None);
        };
        let noun_hash = db1_hash(pe.noun.as_str());
        let inst_info_id = {
            let sql = format!(
                "select value record::id(out) from {}->inst_relate limit 1;",
                RefnoEnum::from(refno).to_pe_key()
            );
            SUL_DB
                .query_take::<Option<String>>(&sql, 0)
                .await
                .unwrap_or(None)
        };
        let cata_hash = inst_info_id.as_deref().and_then(|s| s.parse::<u64>().ok());
        Ok(Some(TreeNodeMeta {
            refno,
            owner: pe.owner.refno(),
            noun: noun_hash,
            cata_hash,
        }))
    }

    async fn query_children(
        &self,
        parent: RefU64,
        filter: TreeQueryFilter,
    ) -> anyhow::Result<Vec<RefU64>> {
        let nouns = noun_hashes_to_names(&filter.noun_hashes);
        let children = crate::rs_surreal::collect_children_filter_ids(
            RefnoEnum::from(parent),
            &nouns.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
        )
        .await?;
        Ok(children.into_iter().map(|r| r.refno()).collect())
    }

    async fn query_descendants_bfs(
        &self,
        root: RefU64,
        options: TreeQueryOptions,
    ) -> anyhow::Result<Vec<RefU64>> {
        let nouns = noun_hashes_to_names(&options.filter.noun_hashes);
        if matches!(options.max_depth, Some(0)) {
            if options.include_self {
                return Ok(vec![root]);
            }
            return Ok(Vec::new());
        }
        let range = options.max_depth.map(|d| format!("1..{}", d));
        let mut descendants = crate::rs_surreal::collect_descendant_filter_ids(
            &[RefnoEnum::from(root)],
            &nouns.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
            range.as_deref(),
        )
        .await?;
        let mut out = Vec::new();
        if options.include_self {
            out.push(root);
        }
        out.extend(descendants.drain(..).map(|r| r.refno()));
        Ok(out)
    }

    async fn query_ancestors_root_to_parent(
        &self,
        node: RefU64,
        options: TreeQueryOptions,
    ) -> anyhow::Result<Vec<RefU64>> {
        let ancestors = if options.filter.noun_hashes.is_some() {
            let nouns = noun_hashes_to_names(&options.filter.noun_hashes);
            crate::rs_surreal::query_filter_ancestors(
                RefnoEnum::from(node),
                &nouns.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
            )
            .await?
        } else {
            crate::rs_surreal::query_ancestor_refnos(RefnoEnum::from(node)).await?
        };
        let mut out: Vec<RefU64> = ancestors.iter().map(|r| r.refno()).collect();
        if options.include_self {
            out.push(node);
        }
        if let Some(max_depth) = options.max_depth {
            if out.len() > max_depth {
                out = out[out.len().saturating_sub(max_depth)..].to_vec();
            }
        }
        Ok(out)
    }
}

static GEO_NOUN_HASHES: Lazy<HashSet<u32>> = Lazy::new(|| {
    let mut set = HashSet::new();
    let iter = USE_CATE_NOUN_NAMES
        .iter()
        .chain(GNERAL_LOOP_OWNER_NOUN_NAMES.iter())
        .chain(GNERAL_PRIM_NOUN_NAMES.iter())
        .chain(BRAN_COMPONENT_NOUN_NAMES.iter());
    for noun in iter {
        set.insert(db1_hash(noun));
    }
    set.insert(db1_hash("BRAN"));
    set.insert(db1_hash("HANG"));
    set
});

pub fn is_geo_noun_hash(noun: u32) -> bool {
    GEO_NOUN_HASHES.contains(&noun)
}

fn noun_hashes_to_names(hashes: &Option<HashSet<u32>>) -> Vec<String> {
    let Some(hashes) = hashes else {
        return Vec::new();
    };
    hashes
        .iter()
        .filter_map(|hash| {
            let name = db1_dehash(*hash);
            if name.is_empty() { None } else { Some(name) }
        })
        .collect()
}

// ============================================================================
// DbMetaInfo: ref0 -> dbnum 映射
// ============================================================================

use serde::{Deserialize, Serialize};

/// 数据库元信息（从 db_meta_info.json 加载）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbMetaInfo {
    pub version: u32,
    pub updated_at: String,
    pub ref0_to_dbnum: HashMap<String, u32>,
    #[serde(default)]
    pub db_files: HashMap<String, serde_json::Value>,
}

impl DbMetaInfo {
    /// 从文件加载
    pub fn load(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let meta: DbMetaInfo = serde_json::from_str(&content)?;
        Ok(meta)
    }

    /// 根据 ref0 获取 dbnum
    pub fn get_dbnum(&self, ref0: u32) -> Option<u32> {
        self.ref0_to_dbnum.get(&ref0.to_string()).copied()
    }

    /// 根据 RefU64 获取 dbnum
    pub fn get_dbnum_by_refno(&self, refno: RefU64) -> Option<u32> {
        self.get_dbnum(refno.get_0())
    }
}

/// 全局 ref0 -> dbnum 映射缓存
static REF0_TO_DBNUM_CACHE: Lazy<RwLock<HashMap<u32, u32>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

/// 加载 db_meta_info.json 并缓存 ref0 -> dbnum 映射
pub fn load_db_meta_info(path: impl AsRef<Path>) -> anyhow::Result<DbMetaInfo> {
    let meta = DbMetaInfo::load(path)?;
    let mut cache = REF0_TO_DBNUM_CACHE.write();
    for (ref0_str, dbnum) in &meta.ref0_to_dbnum {
        if let Ok(ref0) = ref0_str.parse::<u32>() {
            cache.insert(ref0, *dbnum);
        }
    }
    Ok(meta)
}

/// 根据 ref0 获取 dbnum（从缓存）
pub fn get_dbnum_by_ref0(ref0: u32) -> Option<u32> {
    REF0_TO_DBNUM_CACHE.read().get(&ref0).copied()
}

/// 根据 RefU64 获取 dbnum（从缓存）
pub fn get_dbnum_by_refno(refno: RefU64) -> Option<u32> {
    get_dbnum_by_ref0(refno.get_0())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta(refno: u64, owner: u64, noun: &str) -> TreeNodeMeta {
        TreeNodeMeta {
            refno: RefU64(refno),
            owner: RefU64(owner),
            noun: db1_hash(noun),
            cata_hash: None,
        }
    }

    #[test]
    fn test_filter_matches_dynamic_flags() {
        let node = meta(2, 1, "BRAN");
        let filter = TreeQueryFilter {
            has_geo: Some(true),
            is_leaf: Some(true),
            noun_hashes: None,
        };
        assert!(filter.matches(&node, true, true));
        assert!(!filter.matches(&node, false, true));
        assert!(!filter.matches(&node, true, false));
    }

    #[test]
    fn test_filter_matches_noun_hashes() {
        let node = meta(2, 1, "BRAN");
        let filter = TreeQueryFilter {
            has_geo: None,
            is_leaf: None,
            noun_hashes: Some(HashSet::from([db1_hash("BRAN")])),
        };
        assert!(filter.matches(&node, false, false));

        let other = TreeQueryFilter {
            has_geo: None,
            is_leaf: None,
            noun_hashes: Some(HashSet::from([db1_hash("SITE")])),
        };
        assert!(!other.matches(&node, false, false));
    }
}
