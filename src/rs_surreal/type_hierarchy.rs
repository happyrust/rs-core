use std::collections::{BTreeMap, HashSet, VecDeque};
use std::path::Path;

use anyhow::{Result, anyhow};
use serde::Deserialize;
use surrealdb::{Connection, Surreal};

/// 层级报告结构，与 hierarchy_report_correct.json 对应
#[derive(Debug, Deserialize)]
pub struct HierarchyReport {
    pub total_types: usize,
    pub known_types: usize,
    pub unknown_types: usize,
    pub description: Option<String>,
    pub types: BTreeMap<String, TypeEntry>,
}

/// 单个类型的详细信息
#[derive(Debug, Deserialize, Clone)]
pub struct TypeEntry {
    pub hash: Option<String>,
    #[serde(default)]
    pub parents: Vec<String>,
    #[serde(default)]
    pub children: Vec<String>,
}

/// 生成的 SurrealQL 脚本集合
#[derive(Debug, Default, Clone)]
pub struct HierarchyImportScripts {
    pub node_statements: Vec<String>,
    pub edge_statements: Vec<String>,
    pub path_statements: Vec<String>,
}

impl HierarchyImportScripts {
    pub fn all_statements(&self) -> impl Iterator<Item = &String> {
        self.node_statements
            .iter()
            .chain(self.edge_statements.iter())
            .chain(self.path_statements.iter())
    }
}

/// 从磁盘读取层级报告
pub fn load_report_from_path<P: AsRef<Path>>(path: P) -> Result<HierarchyReport> {
    let path = path.as_ref();
    let raw = std::fs::read_to_string(path)
        .map_err(|e| anyhow!("读取层级文件失败 {}: {}", path.display(), e))?;
    let report: HierarchyReport = serde_json::from_str(&raw)
        .map_err(|e| anyhow!("解析层级文件失败 {}: {}", path.display(), e))?;
    Ok(report)
}

/// 基于层级报告生成 SurrealQL 导入脚本
pub fn generate_import_scripts(report: &HierarchyReport, source: &str) -> HierarchyImportScripts {
    let mut scripts = HierarchyImportScripts::default();
    let mut valid_nodes: HashSet<String> = HashSet::new();

    for (code, entry) in &report.types {
        scripts
            .node_statements
            .push(build_node_statement(code, entry, source));
        valid_nodes.insert(code.clone());
    }

    let mut edge_keys: HashSet<(String, String)> = HashSet::new();
    for (child_code, entry) in &report.types {
        for parent_code in entry.parents.iter().filter(|p| valid_nodes.contains(*p)) {
            if edge_keys.insert((parent_code.clone(), child_code.clone())) {
                scripts
                    .edge_statements
                    .push(build_edge_statement(parent_code, child_code, source));
            }
        }
    }

    let mut adjacency: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (code, entry) in &report.types {
        let children: Vec<&str> = entry
            .children
            .iter()
            .filter(|child| valid_nodes.contains(child.as_str()))
            .map(|s| s.as_str())
            .collect();
        adjacency.insert(code.as_str(), children);
    }

    let mut path_keys: HashSet<(String, String)> = HashSet::new();
    for ancestor in report.types.keys() {
        let mut queue: VecDeque<(String, Vec<String>, usize)> = VecDeque::new();
        let mut visited: BTreeMap<String, usize> = BTreeMap::new();

        if let Some(children) = adjacency.get(ancestor.as_str()) {
            for child in children {
                queue.push_back((
                    (*child).to_owned(),
                    vec![ancestor.clone(), (*child).to_owned()],
                    1,
                ));
            }
        }

        while let Some((descendant, path, depth)) = queue.pop_front() {
            if visited.get(&descendant).map_or(false, |d| *d <= depth) {
                continue;
            }
            visited.insert(descendant.clone(), depth);

            if path_keys.insert((ancestor.clone(), descendant.clone())) {
                scripts.path_statements.push(build_path_statement(
                    ancestor,
                    &descendant,
                    depth,
                    &path,
                    source,
                ));
            }

            if let Some(children) = adjacency.get(descendant.as_str()) {
                for child in children {
                    let mut next_path = path.clone();
                    next_path.push((*child).to_owned());
                    queue.push_back(((*child).to_owned(), next_path, depth + 1));
                }
            }
        }
    }

    scripts
}

/// 将脚本执行到 SurrealDB
pub async fn apply_scripts<C>(db: &Surreal<C>, scripts: &HierarchyImportScripts) -> Result<()>
where
    C: Connection,
{
    for stmt in scripts.all_statements() {
        db.query(stmt.clone()).await?;
    }
    Ok(())
}

pub async fn generate_and_apply<C, S>(
    db: &Surreal<C>,
    report: &HierarchyReport,
    source: S,
) -> Result<HierarchyImportScripts>
where
    C: Connection,
    S: AsRef<str>,
{
    let scripts = generate_import_scripts(report, source.as_ref());
    apply_scripts(db, &scripts).await?;
    Ok(scripts)
}

fn build_node_statement(code: &str, entry: &TypeEntry, source: &str) -> String {
    let code_lit = format!("{}:{}", TYPE_NODE_TABLE, sanitize(code));
    let hash_literal = entry
        .hash
        .as_ref()
        .map(|h| format!("'{}'", sanitize(h)))
        .unwrap_or_else(|| "NONE".to_string());
    format!(
        "UPSERT {code_lit} CONTENT {{ code: '{code}', hash: {hash_literal}, parent_count: {parent_count}, child_count: {child_count}, status: 'imported', source: '{source}' }};",
        code_lit = code_lit,
        code = sanitize(code),
        hash_literal = hash_literal,
        parent_count = entry.parents.len(),
        child_count = entry.children.len(),
        source = sanitize(source),
    )
}

fn build_edge_statement(parent: &str, child: &str, source: &str) -> String {
    format!(
        "RELATE {parent}->{edge_table}->{child} SET depth_hint = 1, source = '{source}';",
        parent = format_record(TYPE_NODE_TABLE, parent),
        child = format_record(TYPE_NODE_TABLE, child),
        edge_table = TYPE_EDGE_TABLE,
        source = sanitize(source),
    )
}

fn build_path_statement(
    ancestor: &str,
    descendant: &str,
    depth: usize,
    path: &[String],
    source: &str,
) -> String {
    let path_id = format!("{}__{}", ancestor, descendant);
    format!(
        "UPSERT {table}:{path_id} CONTENT {{ ancestor_code: '{ancestor}', descendant_code: '{descendant}', depth: {depth}, path: {path_array}, source: '{source}' }};",
        table = TYPE_PATH_TABLE,
        path_id = sanitize(&path_id),
        ancestor = sanitize(ancestor),
        descendant = sanitize(descendant),
        depth = depth,
        path_array = format_array(path),
        source = sanitize(source),
    )
}

fn sanitize(value: &str) -> String {
    value.replace("'", "''")
}

fn format_record(table: &str, key: &str) -> String {
    format!("{table}:{id}", table = table, id = sanitize(key))
}

fn format_array(values: &[String]) -> String {
    let inner = values
        .iter()
        .map(|v| format!("'{}'", sanitize(v)))
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{inner}]")
}

const TYPE_NODE_TABLE: &str = "type_node";
const TYPE_EDGE_TABLE: &str = "type_edge";
const TYPE_PATH_TABLE: &str = "type_path";
