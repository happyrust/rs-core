use crate::graph::*;
use crate::noun_graph::gen_noun_incoming_relate_sql;
use crate::noun_graph::gen_noun_outcoming_relate_sql;
use crate::pdms_types::CATA_WITHOUT_REUSE_GEO_NAMES;
use crate::petgraph::PetRefnoGraph;
use crate::tool::db_tool::db1_hash;
use crate::{RefnoEnum, SUL_DB};
use petgraph::algo::all_simple_paths;
use petgraph::graph::Graph;
use petgraph::graph::NodeIndex;
use petgraph::graphmap::DiGraphMap;
use petgraph::graphmap::GraphMap;
use std::collections::HashSet;

use super::test_helpers::init_sul_db_with_memory;

#[tokio::test]
async fn test_query_multi_filter_deep_children_with_layer_data() -> anyhow::Result<()> {
    // 使用内存数据库初始化全局 SUL_DB（加载 resource/surreal 下的函数定义）
    init_sul_db_with_memory().await?;

    // 读取测试数据文件并插入
    let test_data_path = "src/test/json/layers/layer_01.txt";
    let test_data = std::fs::read_to_string(test_data_path).expect("Failed to read test data file");

    // 直接插入文件中的 JSON 数组
    let insert_sql = format!("INSERT INTO pe {};", test_data);
    SUL_DB.query(&insert_sql).await?;

    // 测试 1: 从 WORL 节点查询所有 SITE 类型的子孙
    let worl_refno: RefnoEnum = "9304/0".into();
    let result = query_multi_filter_deep_children(&[worl_refno], &["SITE"]).await?;

    assert!(!result.is_empty(), "Should find SITE descendants");
    assert!(
        result.contains(&RefnoEnum::from("17496/169982")),
        "Should contain SITE:17496_169982"
    );

    // 测试 2: 从 SITE 节点查询所有 ZONE 和 EQUI 类型的子孙
    let site_refno: RefnoEnum = "17496/169982".into();
    let result = query_multi_filter_deep_children(&[site_refno.clone()], &["ZONE", "EQUI"]).await?;

    assert!(!result.is_empty(), "Should find ZONE and EQUI descendants");
    assert!(
        result.contains(&RefnoEnum::from("17496/171099")),
        "Should contain ZONE:17496_171099"
    );
    assert!(
        result.contains(&RefnoEnum::from("17496/171100")),
        "Should contain EQUI:17496_171100"
    );

    // 测试 3: 从 ZONE 节点查询所有 EQUI 类型的子孙
    let zone_refno: RefnoEnum = "17496/171099".into();
    let result = query_multi_filter_deep_children(&[zone_refno.clone()], &["EQUI"]).await?;

    assert_eq!(result.len(), 1, "Should find exactly 1 EQUI");
    assert!(
        result.contains(&RefnoEnum::from("17496/171100")),
        "Should contain EQUI:17496_171100"
    );

    // 测试 4: 多个起点查询（SITE 和 ZONE 一起查询 EQUI）
    let result =
        query_multi_filter_deep_children(&[site_refno.clone(), zone_refno.clone()], &["EQUI"])
            .await?;

    assert_eq!(
        result.len(),
        1,
        "Should find 1 unique EQUI from multiple starting points"
    );
    assert!(
        result.contains(&RefnoEnum::from("17496/171100")),
        "Should contain EQUI:17496_171100"
    );

    // 测试 5: 查询所有类型（nouns 为空）
    let result = query_multi_filter_deep_children(&[worl_refno.clone()], &[]).await?;

    assert!(
        !result.is_empty(),
        "Should find all descendants when nouns is empty"
    );
    // 应该包含 SITE、ZONE、EQUI 等所有子孙

    // 测试 6: 空的 refnos 输入
    let result = query_multi_filter_deep_children(&[], &["EQUI"]).await?;

    assert!(result.is_empty(), "Empty refnos should return empty result");

    // 测试 7: 查询不存在的类型
    let result = query_multi_filter_deep_children(&[worl_refno], &["NONEXISTENT_TYPE"]).await?;

    assert!(
        result.is_empty(),
        "Should return empty for non-existent type"
    );

    // 清理测试数据
    let cleanup_sql = r#"
        DELETE pe:17496_171101;
        DELETE pe:17496_171100;
        DELETE pe:17496_171099;
        DELETE pe:17496_169983;
        DELETE pe:25688_4135;
        DELETE pe:17496_169982;
        DELETE pe:9304_0;

        DELETE pe_owner WHERE in IN [
            pe:17496_171101, pe:17496_171100, pe:17496_171099,
            pe:17496_169983, pe:25688_4135, pe:17496_169982
        ];
    "#;

    SUL_DB.query(cleanup_sql).await?;

    Ok(())
}
