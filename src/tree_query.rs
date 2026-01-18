use crate::pdms_types::{
    BRAN_COMPONENT_NOUN_NAMES, GNERAL_LOOP_OWNER_NOUN_NAMES, GNERAL_PRIM_NOUN_NAMES,
    USE_CATE_NOUN_NAMES,
};
use crate::tool::db_tool::{db1_dehash, db1_hash};
use crate::{RefU64, RefnoEnum};
use async_trait::async_trait;
use indextree::{Arena, NodeId};
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use rkyv::{from_bytes, rancor::Error as RkyvError, to_bytes};
use std::collections::{HashMap, HashSet, VecDeque};
use std::io::Write;
use std::num::NonZeroUsize;
use std::path::Path;
use std::sync::Arc;
use flate2::write::{DeflateDecoder, DeflateEncoder};
use flate2::Compression;

const TREE_FLAG_HAS_GEO: u32 = 1 << 0;
const TREE_FLAG_IS_LEAF: u32 = 1 << 1;

#[derive(
    Debug,
    Clone,
    Copy,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub struct TreeNodeMeta {
    pub refno: RefU64,
    pub owner: RefU64,
    pub noun: u32,
    pub has_geo: bool,
    pub is_leaf: bool,
}

#[derive(Debug, Clone, Default)]
pub struct TreeQueryFilter {
    pub has_geo: Option<bool>,
    pub is_leaf: Option<bool>,
    pub noun_hashes: Option<Vec<u32>>,
}

impl TreeQueryFilter {
    fn matches(&self, node: &TreeNodeMeta) -> bool {
        if let Some(has_geo) = self.has_geo {
            if node.has_geo != has_geo {
                return false;
            }
        }
        if let Some(is_leaf) = self.is_leaf {
            if node.is_leaf != is_leaf {
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
}

impl Default for TreeQueryOptions {
    fn default() -> Self {
        Self {
            include_self: true,
            max_depth: None,
            filter: TreeQueryFilter::default(),
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

#[derive(Debug, Clone, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)]
pub struct TreeFile {
    pub dbnum: u32,
    pub root_refno: RefU64,
    pub arena: Arena<TreeNodeMeta>,
}

impl TreeFile {
    pub fn load(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let bytes = std::fs::read(path)?;
        Self::from_rkyv_compress_bytes(&bytes)
    }

    pub fn save(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let bytes = self.to_rkyv_compress_bytes()?;
        std::fs::write(path, bytes)?;
        Ok(())
    }

    pub fn to_rkyv_compress_bytes(&self) -> anyhow::Result<Vec<u8>> {
        let bytes = to_bytes::<RkyvError>(self)
            .map_err(|e| anyhow::anyhow!("rkyv serialize tree file failed: {e}"))?;
        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&bytes)?;
        Ok(encoder.finish()?)
    }

    pub fn from_rkyv_compress_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        let mut decoder = DeflateDecoder::new(Vec::new());
        decoder.write_all(bytes)?;
        let decoded = decoder.finish()?;
        let file = from_bytes::<TreeFile, RkyvError>(&decoded)
            .map_err(|e| anyhow::anyhow!("rkyv deserialize tree file failed: {e}"))?;
        Ok(file)
    }
}

#[derive(Debug)]
pub struct TreeIndex {
    dbnum: u32,
    root_refno: RefU64,
    roots: Vec<RefU64>,
    arena: Arena<TreeNodeMeta>,
    id_map: HashMap<RefU64, NodeId>,
}

impl TreeIndex {
    pub fn load_from_path(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let file = TreeFile::load(path)?;
        Ok(Self::from_tree_file(file))
    }

    pub fn from_tree_file(file: TreeFile) -> Self {
        let arena = file.arena;
        let mut id_map = HashMap::with_capacity(arena.count());
        let mut roots: Vec<RefU64> = Vec::new();
        for index1 in 1..=arena.count() {
            let Some(index1) = NonZeroUsize::new(index1) else {
                continue;
            };
            let Some(node_id) = arena.get_node_id_at(index1) else {
                continue;
            };
            let node = arena[node_id].get();
            id_map.insert(node.refno, node_id);
            if node_id.parent(&arena).is_none() {
                roots.push(node.refno);
            }
        }

        roots.sort_by_key(|refno| refno.0);
        if file.root_refno.0 != 0 {
            if let Some(pos) = roots.iter().position(|r| *r == file.root_refno) {
                roots.remove(pos);
            }
            roots.insert(0, file.root_refno);
        }

        Self {
            dbnum: file.dbnum,
            root_refno: file.root_refno,
            roots,
            arena,
            id_map,
        }
    }

    pub fn dbnum(&self) -> u32 {
        self.dbnum
    }

    pub fn root_refno(&self) -> RefU64 {
        self.root_refno
    }

    pub fn roots(&self) -> &[RefU64] {
        &self.roots
    }

    pub fn node_count(&self) -> usize {
        self.id_map.len()
    }

    pub fn contains_refno(&self, refno: RefU64) -> bool {
        self.id_map.contains_key(&refno)
    }

    pub fn all_refnos(&self) -> Vec<RefU64> {
        self.id_map.keys().copied().collect()
    }

    fn node_meta(&self, refno: RefU64) -> Option<TreeNodeMeta> {
        self.id_map.get(&refno).map(|id| *self.arena[*id].get())
    }

    fn collect_children(&self, parent: RefU64, filter: &TreeQueryFilter) -> Vec<RefU64> {
        let Some(&parent_id) = self.id_map.get(&parent) else {
            return Vec::new();
        };
        parent_id
            .children(&self.arena)
            .filter_map(|child_id| {
                let meta = self.arena[child_id].get();
                if filter.matches(meta) {
                    Some(meta.refno)
                } else {
                    None
                }
            })
            .collect()
    }

    fn collect_descendants_bfs(
        &self,
        root: RefU64,
        options: &TreeQueryOptions,
    ) -> Vec<RefU64> {
        let Some(&root_id) = self.id_map.get(&root) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        let mut queue: VecDeque<(NodeId, usize)> = VecDeque::new();
        queue.push_back((root_id, 0));
        while let Some((node_id, depth)) = queue.pop_front() {
            let node = self.arena[node_id].get();
            let is_root = depth == 0;
            if !(is_root && !options.include_self) && options.filter.matches(node) {
                out.push(node.refno);
            }
            if let Some(max_depth) = options.max_depth {
                if depth >= max_depth {
                    continue;
                }
            }
            for child_id in node_id.children(&self.arena) {
                queue.push_back((child_id, depth + 1));
            }
        }
        out
    }

    fn collect_ancestors_root_to_parent(
        &self,
        node: RefU64,
        options: &TreeQueryOptions,
    ) -> Vec<RefU64> {
        let mut chain: Vec<RefU64> = Vec::new();
        let mut current = node;
        let mut depth = 0usize;
        let mut visited = HashSet::new();
        loop {
            if !visited.insert(current) {
                break;
            }
            if !(current == node && !options.include_self) {
                if let Some(meta) = self.node_meta(current) {
                    if options.filter.matches(&meta) {
                        chain.push(current);
                    }
                    current = meta.owner;
                } else {
                    break;
                }
            } else if let Some(meta) = self.node_meta(current) {
                current = meta.owner;
            } else {
                break;
            }
            depth += 1;
            if let Some(max_depth) = options.max_depth {
                if depth >= max_depth {
                    break;
                }
            }
            if current.0 == 0 {
                break;
            }
        }
        chain.reverse();
        chain
    }
}

#[async_trait]
impl TreeQuery for TreeIndex {
    async fn get_node_meta(&self, refno: RefU64) -> anyhow::Result<Option<TreeNodeMeta>> {
        Ok(self.node_meta(refno))
    }

    async fn query_children(
        &self,
        parent: RefU64,
        filter: TreeQueryFilter,
    ) -> anyhow::Result<Vec<RefU64>> {
        Ok(self.collect_children(parent, &filter))
    }

    async fn query_descendants_bfs(
        &self,
        root: RefU64,
        options: TreeQueryOptions,
    ) -> anyhow::Result<Vec<RefU64>> {
        Ok(self.collect_descendants_bfs(root, &options))
    }

    async fn query_ancestors_root_to_parent(
        &self,
        node: RefU64,
        options: TreeQueryOptions,
    ) -> anyhow::Result<Vec<RefU64>> {
        Ok(self.collect_ancestors_root_to_parent(node, &options))
    }
}

#[derive(Debug, Default, Clone)]
pub struct SurrealTreeQuery;

#[async_trait]
impl TreeQuery for SurrealTreeQuery {
    async fn get_node_meta(&self, refno: RefU64) -> anyhow::Result<Option<TreeNodeMeta>> {
        let Some(pe) = crate::rs_surreal::get_pe(RefnoEnum::from(refno)).await? else {
            return Ok(None);
        };
        let noun_hash = db1_hash(pe.noun.as_str());
        let has_geo = is_geo_noun_hash(noun_hash);
        let children = crate::rs_surreal::get_children_refnos(RefnoEnum::from(refno)).await?;
        let is_leaf = children.is_empty();
        Ok(Some(TreeNodeMeta {
            refno,
            owner: pe.owner.refno(),
            noun: noun_hash,
            has_geo,
            is_leaf,
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

static TREE_INDEX_CACHE: Lazy<RwLock<HashMap<u32, Arc<TreeIndex>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

pub fn get_cached_tree_index(dbnum: u32) -> Option<Arc<TreeIndex>> {
    TREE_INDEX_CACHE.read().get(&dbnum).cloned()
}

pub fn load_tree_index_from_dir(dbnum: u32, dir: impl AsRef<Path>) -> anyhow::Result<Arc<TreeIndex>> {
    if let Some(index) = get_cached_tree_index(dbnum) {
        return Ok(index);
    }
    let path = dir.as_ref().join(format!("{}.tree", dbnum));
    let index = Arc::new(TreeIndex::load_from_path(&path)?);
    TREE_INDEX_CACHE.write().insert(dbnum, index.clone());
    Ok(index)
}

pub fn load_tree_index_from_path(path: impl AsRef<Path>) -> anyhow::Result<Arc<TreeIndex>> {
    let index = Arc::new(TreeIndex::load_from_path(path.as_ref())?);
    TREE_INDEX_CACHE.write().insert(index.dbnum(), index.clone());
    Ok(index)
}

pub fn remove_tree_index(dbnum: u32) {
    TREE_INDEX_CACHE.write().remove(&dbnum);
}

pub fn clear_tree_index_cache() {
    TREE_INDEX_CACHE.write().clear();
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

fn noun_hashes_to_names(hashes: &Option<Vec<u32>>) -> Vec<String> {
    let Some(hashes) = hashes else {
        return Vec::new();
    };
    hashes
        .iter()
        .filter_map(|hash| {
            let name = db1_dehash(*hash);
            if name.is_empty() {
                None
            } else {
                Some(name)
            }
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

/// 根据 refno 自动获取对应的 TreeIndex
pub fn get_tree_index_by_refno(refno: RefU64) -> Option<Arc<TreeIndex>> {
    let dbnum = get_dbnum_by_refno(refno)?;
    get_cached_tree_index(dbnum)
}
