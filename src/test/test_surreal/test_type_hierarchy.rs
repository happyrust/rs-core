use std::collections::BTreeMap;

use crate::rs_surreal::type_hierarchy::{HierarchyReport, TypeEntry, generate_import_scripts};
use anyhow::Result;

#[test]
fn test_generate_import_scripts_basic() -> Result<()> {
    let mut types: BTreeMap<String, TypeEntry> = BTreeMap::new();
    types.insert(
        "ROOT".to_string(),
        TypeEntry {
            hash: Some("0x1".to_string()),
            parents: vec![],
            children: vec!["MID".to_string()],
        },
    );
    types.insert(
        "MID".to_string(),
        TypeEntry {
            hash: Some("0x2".to_string()),
            parents: vec!["ROOT".to_string()],
            children: vec!["LEAF".to_string()],
        },
    );
    types.insert(
        "LEAF".to_string(),
        TypeEntry {
            hash: Some("0x3".to_string()),
            parents: vec!["MID".to_string()],
            children: vec![],
        },
    );

    let report = HierarchyReport {
        total_types: types.len(),
        known_types: types.len(),
        unknown_types: 0,
        description: None,
        types,
    };

    let scripts = generate_import_scripts(&report, "test_source");

    assert_eq!(scripts.node_statements.len(), 3);
    assert!(
        scripts
            .node_statements
            .iter()
            .any(|stmt| stmt.contains("UPSERT type_node:ROOT"))
    );
    assert_eq!(scripts.edge_statements.len(), 2);
    assert!(
        scripts
            .edge_statements
            .iter()
            .any(|stmt| stmt.contains("RELATE type_node:ROOT->type_edge->type_node:MID"))
    );
    assert_eq!(scripts.path_statements.len(), 3);
    assert!(
        scripts
            .path_statements
            .iter()
            .any(|stmt| stmt.contains("UPSERT type_path:ROOT__LEAF"))
    );

    Ok(())
}
