use dashmap::DashMap;
use itertools::Itertools;
use once_cell::sync::Lazy;
use petgraph::Direction;
use petgraph::graph::DiGraph;
use petgraph::graph::NodeIndex;
use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

use crate::tool::db_tool::*;

/// noun 关系图，边的方向是 子类型 -> 可能的属主类型。
///
/// 数据来自 AVEVA 设计模板库 desvir.dat（core.dll 通过 dab 462 读取的同一份），
/// 977 个元素类型、3310 条边，已用真实模型 sam7200 对账：实际出现的 203 种
/// owner->member 组合全部命中。
pub static NOUN_GRAPH: Lazy<DiGraph<u32, u32>> =
    Lazy::new(|| serde_json::from_slice(include_bytes!("../noun_graph.json")).unwrap());

static NODE_BY_HASH: Lazy<HashMap<u32, NodeIndex<u32>>> = Lazy::new(|| {
    NOUN_GRAPH
        .node_indices()
        .map(|i| (NOUN_GRAPH[i], i))
        .collect()
});

/// 图里最长的最短路径是 9 跳。封顶是必需的：有 32 个类型可以拥有自己
/// （例如 ZONE 下面还能放 ZONE），不封顶会无限展开。
pub const MAX_NOUN_DEPTH: usize = 12;

/// 单次路径枚举返回的上限，避免在稠密子图上退化。
const MAX_PATHS: usize = 4096;

fn node_of(noun: &str) -> Option<NodeIndex<u32>> {
    NODE_BY_HASH.get(&db1_hash(noun)).copied()
}

fn name_of(node: NodeIndex<u32>) -> String {
    db1_dehash(NOUN_GRAPH[node])
}

/// `own_noun` 是否可以直接作为 `child_noun` 的属主。
pub fn is_owner_type(child_noun: &str, own_noun: &str) -> bool {
    let (Some(child), Some(own)) = (node_of(child_noun), node_of(own_noun)) else {
        return false;
    };
    NOUN_GRAPH
        .neighbors_directed(child, Direction::Outgoing)
        .any(|n| n == own)
}

/// 沿 owner 方向求传递闭包，等价于 core.dll 的 `DB_Noun::recurseEleTypes(ATT_OWNER)`。
///
/// 起点自身只有在能经由环回到自己时才会出现在结果里，这一点与原实现一致。
pub fn noun_ancestors(noun: &str) -> Vec<String> {
    let Some(start) = node_of(noun) else {
        return vec![];
    };
    let mut seen: HashSet<NodeIndex<u32>> = HashSet::new();
    let mut queue: VecDeque<NodeIndex<u32>> = VecDeque::from([start]);
    while let Some(cur) = queue.pop_front() {
        for next in NOUN_GRAPH.neighbors_directed(cur, Direction::Outgoing) {
            if seen.insert(next) {
                queue.push_back(next);
            }
        }
    }
    seen.into_iter().map(name_of).sorted().collect()
}

/// 遍历时的下探白名单：目标类型集合的祖先闭包并集。
///
/// 对应 `DB_IteratorCreator::getIterator()` 里的 pred2——只有类型落在这个集合里的
/// 节点才值得往下钻，其余子树整棵跳过。
pub fn descend_gate(target_nouns: &[&str]) -> Vec<String> {
    target_nouns
        .iter()
        .flat_map(|n| noun_ancestors(n))
        .unique()
        .sorted()
        .collect()
}

/// 枚举 `start_noun` 沿 owner 方向到 `end_noun` 的路径，深度与条数都有上限。
pub fn find_noun_path(start_noun: &str, end_noun: &str) -> Vec<Vec<String>> {
    let (Some(start), Some(end)) = (node_of(start_noun), node_of(end_noun)) else {
        return vec![];
    };
    let mut result = Vec::new();
    let mut stack = vec![start];
    let mut on_path: HashSet<NodeIndex<u32>> = HashSet::from([start]);
    walk_paths(start, end, &mut stack, &mut on_path, &mut result);
    result
}

fn walk_paths(
    cur: NodeIndex<u32>,
    end: NodeIndex<u32>,
    stack: &mut Vec<NodeIndex<u32>>,
    on_path: &mut HashSet<NodeIndex<u32>>,
    out: &mut Vec<Vec<String>>,
) {
    if out.len() >= MAX_PATHS || stack.len() > MAX_NOUN_DEPTH + 1 {
        return;
    }
    if cur == end && stack.len() > 1 {
        out.push(stack.iter().copied().map(name_of).collect());
        return;
    }
    for next in NOUN_GRAPH.neighbors_directed(cur, Direction::Outgoing) {
        if next != end && !on_path.insert(next) {
            continue;
        }
        stack.push(next);
        walk_paths(next, end, stack, on_path, out);
        stack.pop();
        if next != end {
            on_path.remove(&next);
        }
    }
}

/// 把多个目标类型的简单路径按层归并。
///
/// 层号必须沿简单路径统计：图里有 32 个自环（ZONE 下面还能放 ZONE），
/// 允许重复访问的话层数会一路顶到 `MAX_NOUN_DEPTH`，链路白白拉长一个数量级。
/// `index_from_end` 为真时层号从 `anchor` 侧开始计。
fn merge_levels(
    anchor: &str,
    others: &[&str],
    index_from_end: bool,
) -> Option<(Vec<BTreeSet<String>>, usize, usize)> {
    let mut levels: Vec<BTreeSet<String>> = Vec::new();
    let mut min_nodes = usize::MAX;
    let mut max_nodes = 0usize;

    for other in others {
        let (from, to) = if index_from_end {
            (*other, anchor)
        } else {
            (anchor, *other)
        };
        for mut path in find_noun_path(from, to) {
            if index_from_end {
                path.reverse();
            }
            min_nodes = min_nodes.min(path.len());
            max_nodes = max_nodes.max(path.len());
            if levels.len() < path.len() {
                levels.resize(path.len(), BTreeSet::new());
            }
            for (i, name) in path.into_iter().enumerate() {
                levels[i].insert(name);
            }
        }
    }

    if max_nodes == 0 {
        return None;
    }
    Some((levels, min_nodes, max_nodes))
}

fn render(
    levels: &[BTreeSet<String>],
    min_nodes: usize,
    max_nodes: usize,
    contains_self: bool,
    up: bool,
) -> String {
    let mut sql = String::new();
    if contains_self {
        sql.push_str("id as p0,");
    }
    for i in 1..max_nodes {
        let names = levels.get(i).filter(|n| !n.is_empty());
        match names {
            Some(names) if i + 1 >= min_nodes => {
                let filter = names.iter().map(|s| format!("'{s}'")).join(",");
                if up {
                    sql.push_str(&format!(
                        "->pe_owner[where out.noun in [{filter}]]->(? as p{i})"
                    ));
                } else {
                    sql.push_str(&format!(
                        "<-pe_owner[where in.noun in [{filter}]]<-(? as p{i})"
                    ));
                }
            }
            _ => sql.push_str(if up {
                "->pe_owner->(?)"
            } else {
                "<-pe_owner<-(?)"
            }),
        }
    }
    if sql.ends_with(',') {
        sql.pop();
    }
    sql
}

/// 路径枚举在结构专业那种稠密子图上要几十毫秒，而同一组参数会被反复问到。
static SQL_CACHE: Lazy<DashMap<(bool, String), Option<String>>> = Lazy::new(DashMap::new);

fn cached_sql(anchor: &str, filter_nouns: &[&str], up: bool) -> Option<String> {
    let key = (
        up,
        format!(
            "{anchor}\u{1}{}",
            filter_nouns.iter().sorted().join("\u{1}")
        ),
    );
    if let Some(hit) = SQL_CACHE.get(&key) {
        return hit.clone();
    }
    let contains_self = filter_nouns.contains(&anchor);
    let sql = match merge_levels(anchor, filter_nouns, !up) {
        Some((levels, min_nodes, max_nodes)) => {
            Some(render(&levels, min_nodes, max_nodes, contains_self, up))
        }
        None => contains_self.then(|| "id as p0".to_string()),
    };
    SQL_CACHE.insert(key, sql.clone());
    sql
}

/// 沿 owner 方向一直往上找到过滤类型为止，生成 SurrealDB 的关系遍历片段。
pub fn gen_noun_outcoming_relate_sql(start_noun: &str, filter_nouns: &[&str]) -> Option<String> {
    cached_sql(start_noun, filter_nouns, true)
}

/// 与 `gen_noun_outcoming_relate_sql` 相反，从 `end_noun` 往下钻到各过滤类型。
pub fn gen_noun_incoming_relate_sql(end_noun: &str, filter_nouns: &[&str]) -> Option<String> {
    cached_sql(end_noun, filter_nouns, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_is_owner() {
        assert_eq!(is_owner_type("ELBO", "BRAN"), true);
        assert_eq!(is_owner_type("BRAN", "PIPE"), true);
        assert_eq!(is_owner_type("PIPE", "ZONE"), true);
        assert_eq!(is_owner_type("ZONE", "SITE"), true);
        assert_eq!(is_owner_type("SITE", "WORL"), true);
        assert_eq!(is_owner_type("ELBO", "SITE"), false);
    }

    #[test]
    fn test_ancestors() {
        let a = noun_ancestors("ELBO");
        for expected in ["BRAN", "PIPE", "ZONE", "SITE", "WORL"] {
            assert!(
                a.contains(&expected.to_string()),
                "missing {expected} in {a:?}"
            );
        }
        assert!(
            !a.contains(&"EQUI".to_string()),
            "EQUI cannot contain an ELBO"
        );
    }

    #[test]
    fn test_descend_gate_covers_containers() {
        let gate = descend_gate(&["BOX", "CYLI", "ELBO"]);
        for expected in ["WORL", "SITE", "ZONE", "EQUI", "PIPE", "BRAN", "STRU"] {
            assert!(
                gate.contains(&expected.to_string()),
                "gate missing {expected}"
            );
        }
        // 纯叶子型的非几何元素不该出现在下探白名单里
        assert!(!gate.contains(&"TEXT".to_string()));
    }

    #[test]
    fn test_relate_sql_is_bounded() {
        let sql = gen_noun_incoming_relate_sql("SITE", &["ELBO"]).expect("path SITE -> ELBO");
        assert!(sql.contains("<-pe_owner"), "{sql}");
        assert!(sql.contains("'BRAN'"), "{sql}");

        for (anchor, targets) in [
            ("SITE", &["ELBO"][..]),
            ("ZONE", &["EQUI"][..]),
            ("SITE", &["ZONE", "EQUI"][..]),
            ("STRU", &["SCTN"][..]),
        ] {
            let started = std::time::Instant::now();
            let sql = gen_noun_incoming_relate_sql(anchor, targets).unwrap();
            let took = started.elapsed();
            println!(
                "{anchor} -> {targets:?}  hops={} in {took:?}\n  {sql}\n",
                sql.matches("pe_owner").count()
            );
            assert!(
                sql.matches("pe_owner").count() <= MAX_NOUN_DEPTH,
                "chain too long for {anchor} -> {targets:?}: {sql}"
            );
            assert!(
                took.as_millis() < 500,
                "too slow for {anchor} -> {targets:?}"
            );
        }
    }
}
