use aios_core::tree_query::{
    load_tree_index_from_dir, load_tree_index_from_path, TreeQuery, TreeQueryFilter,
    TreeQueryOptions,
};
use aios_core::RefU64;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

const DEFAULT_DBNO: u32 = 1112;

fn resolve_tree_path() -> Result<(PathBuf, u32)> {
    let args: Vec<String> = std::env::args().collect();
    if let Some(pos) = args.iter().position(|x| x == "--path") {
        let path = args
            .get(pos + 1)
            .map(PathBuf::from)
            .context("缺少 --path 的路径参数")?;
        let dbnum = extract_dbno_from_path(&path).unwrap_or(DEFAULT_DBNO);
        return Ok((path, dbnum));
    }

    let dbnum = args
        .iter()
        .position(|x| x == "--dbnum")
        .and_then(|pos| args.get(pos + 1))
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(DEFAULT_DBNO);

    let dir = args
        .iter()
        .position(|x| x == "--dir")
        .and_then(|pos| args.get(pos + 1))
        .map(PathBuf::from)
        .unwrap_or_else(default_tree_dir);

    let path = dir.join(format!("{dbnum}.tree"));
    Ok((path, dbnum))
}

fn extract_dbno_from_path(path: &Path) -> Option<u32> {
    let stem = path.file_stem()?.to_string_lossy();
    stem.parse::<u32>().ok()
}

fn default_tree_dir() -> PathBuf {
    let prefer = PathBuf::from("../gen_model-dev/output/scene_tree");
    if prefer.exists() {
        prefer
    } else {
        PathBuf::from("output/scene_tree")
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let (path, dbnum) = resolve_tree_path()?;
    if !path.exists() {
        anyhow::bail!(
            "未找到 tree 文件: {} (dbnum={})",
            path.display(),
            dbnum
        );
    }

    let index = if let Some(parent) = path.parent() {
        load_tree_index_from_dir(dbnum, parent)?
    } else {
        load_tree_index_from_path(&path)?
    };

    println!("✅ tree 文件: {}", path.display());
    println!(
        "dbnum={}, root={}, nodes={}",
        index.dbnum(),
        index.root_refno(),
        index.node_count()
    );

    let root = index.root_refno();
    let children = index
        .query_children(root, TreeQueryFilter::default())
        .await?;
    print_samples("root children", &children, 10);

    let descendants = index
        .query_descendants_bfs(
            root,
            TreeQueryOptions {
                include_self: true,
                max_depth: Some(2),
                filter: TreeQueryFilter::default(),
            },
        )
        .await?;
    print_samples("root descendants (depth<=2)", &descendants, 10);

    let geo_leaf = index
        .query_descendants_bfs(
            root,
            TreeQueryOptions {
                include_self: false,
                max_depth: None,
                filter: TreeQueryFilter {
                    has_geo: Some(true),
                    is_leaf: Some(true),
                    noun_hashes: None,
                },
            },
        )
        .await?;
    print_samples("geo leaf descendants", &geo_leaf, 10);

    let sample = pick_sample_refno(&index, root);
    let ancestors = index
        .query_ancestors_root_to_parent(
            sample,
            TreeQueryOptions {
                include_self: true,
                max_depth: None,
                filter: TreeQueryFilter::default(),
            },
        )
        .await?;
    print_samples("ancestors (root->parent, include self)", &ancestors, 12);

    Ok(())
}

fn pick_sample_refno(index: &aios_core::tree_query::TreeIndex, root: RefU64) -> RefU64 {
    index
        .all_refnos()
        .into_iter()
        .find(|r| *r != root)
        .unwrap_or(root)
}

fn print_samples(label: &str, items: &[RefU64], limit: usize) {
    let show = items.len().min(limit);
    println!("{}: count={}, sample={:?}", label, items.len(), &items[..show]);
}
