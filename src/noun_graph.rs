use itertools::Itertools;
//使用once_cell宏来实现单例模式, 读取 noun_graph.json 得到一个 DiGraph
use once_cell::sync::Lazy;
use petgraph::graph::DiGraph;
use petgraph::graph::NodeIndex;
use std::fs::File;
use std::io::Read;

use crate::tool::db_tool::*;

//使用once_cell宏来实现单例模式, 读取 noun_graph.json 得到一个 DiGraph
pub static NOUN_GRAPH: Lazy<DiGraph<u32, u32>> = Lazy::new(|| {
    let mut file = File::open("noun_graph.json").unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();
    let graph: DiGraph<u32, u32> = serde_json::from_str(&contents).unwrap();
    graph
});

//指定起始的和目标noun node, 传入的参数是noun name, 通过 db1_hash 找到对应的node，找到经过的路径的noun， 使用all_simple_paths
pub fn find_noun_path(start_noun: &str, end_noun: &str) -> Vec<Vec<String>> {
    let start_node = NOUN_GRAPH
        .node_indices()
        .find(|i| NOUN_GRAPH[*i] == db1_hash(start_noun))
        .unwrap();
    let end_node = NOUN_GRAPH
        .node_indices()
        .find(|i| NOUN_GRAPH[*i] == db1_hash(end_noun))
        .unwrap();
    let mut paths = petgraph::algo::all_simple_paths::<Vec<NodeIndex<u32>>, &DiGraph<u32, u32>>(
        &NOUN_GRAPH,
        start_node,
        end_node,
        0,
        None,
    );
    let mut result = Vec::new();
    while let Some(path) = paths.next() {
        let mut path_str = Vec::new();
        for node in path {
            path_str.push(db1_dehash(NOUN_GRAPH[node]));
        }
        result.push(path_str);
    }
    result
}

pub fn gen_noun_outcoming_relate_path(start_noun: &str, filter_nouns: &[&str]) -> Option<String> {
    let paths = filter_nouns
        .into_iter()
        .map(|n| {
            find_noun_path(n, start_noun)
                .into_iter()
                .map(|x: Vec<String>| x.into_iter().rev().collect::<Vec<_>>())
                // .unique()
                .collect::<Vec<_>>()
        })
        .flatten()
        .collect::<Vec<_>>();
    // dbg!(&paths);
    if paths.is_empty() {
        return None;
    }
    let min_len = paths.iter().map(|x| x.len()).min().unwrap();
    let max_len = paths.iter().map(|x| x.len()).max().unwrap();
    let mut sql = "".to_string();
    for i in 1..max_len {
        if i >= min_len - 1 {
            let filter = paths
                .iter()
                .filter_map(|x| x.get(i).map(|s| format!("'{s}'")))
                .unique()
                .collect::<Vec<_>>();
            // dbg!(&filter);
            sql.push_str(&format!(
                "<-pe_owner[where in.noun in [{}]]<-(? as p{})",
                filter.join(","),
                i,
            ));
        } else {
            sql.push_str(&format!("<-pe_owner<-(?)"));
        }
    }
    // println!("Sql is {}", &sql);
    Some(sql)
}

//指定起始的和目标noun node, 传入的参数是noun name, 通过 db1_hash 找到对应的node，找到经过的路径的noun， 使用all_simple_paths
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_path() {
        // Test case 1: Start and end nouns are the same
        let start_noun = "ELBO";
        let end_noun = "SITE";
        // let result = gen_noun_outcoming_relate_path(&[start_noun], end_noun, "");
        // dbg!(&result);
        // assert_eq!(result, vec![vec![start_noun.to_string()]]);

        // Test case 2: Start and end nouns are connected directly
        // let start_noun = "noun1";
        // let end_noun = "noun2";
        // let result = find_path(start_noun, end_noun);
        // assert_eq!(result, vec![vec![start_noun.to_string(), end_noun.to_string()]]);

        // // Test case 3: Start and end nouns are connected through multiple paths
        // let start_noun = "noun1";
        // let end_noun = "noun4";
        // let result = find_path(start_noun, end_noun);
        // assert_eq!(
        //     result,
        //     vec![
        //         vec![
        //             start_noun.to_string(),
        //             "noun2".to_string(),
        //             "noun3".to_string(),
        //             end_noun.to_string()
        //         ],
        //         vec![
        //             start_noun.to_string(),
        //             "noun2".to_string(),
        //             "noun4".to_string()
        //         ]
        //     ]
        // );

        // // Test case 4: Start and end nouns are not connected
        // let start_noun = "noun1";
        // let end_noun = "noun5";
        // let result = find_path(start_noun, end_noun);
        // assert_eq!(result, Vec::<Vec<String>>::new());
    }
}
