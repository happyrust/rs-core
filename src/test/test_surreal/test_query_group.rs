use crate::{RefU64, RefnoEnum, rs_surreal};
use glam::Vec3;
use std::sync::Arc;

#[tokio::test]
async fn test_group_cata_hash() -> anyhow::Result<()> {
    crate::init_test_surreal().await;
    let refnos: Vec<RefnoEnum> = vec!["21895_68780".into()];

    // let r = crate::query_deep_children_filter_inst(&refnos, &["BRAN", "HANG"], false)
    //     .await
    //     .unwrap();

    // dbg!(&r);
    // let branch_refnos: Vec<RefnoEnum> = r.into_iter().map(|x| x.into()).collect();

    let map = crate::query_group_by_cata_hash(&refnos)
        .await
        .unwrap_or_default();
    dbg!(&map);

    // let group = rs_surreal::query_group_by_cata_hash(&refnos)
    //     .await
    //     .unwrap();
    // dbg!(&group);
    Ok(())
}

#[tokio::test]
async fn test_query_deep_children_spre() -> anyhow::Result<()> {
    crate::init_test_surreal().await;

    // 测试单个节点 pe:21895_68780
    let refno: RefU64 = "21895/68780".into();

    // 不过滤 inst_relate 和 tubi_relate
    let result_no_filter = crate::query_deep_children_refnos_filter_spre(&[refno.into()], false)
        .await
        .unwrap();
    println!("\n=== pe:21895_68780 测试 (filter=false) ===");
    println!(
        "子孙节点中具有 CATR/SPRE 属性的节点数量: {}",
        result_no_filter.len()
    );
    if !result_no_filter.is_empty() {
        println!("前 10 个节点:");
        for (i, node) in result_no_filter.iter().take(10).enumerate() {
            println!("  {}. {}", i + 1, node);
        }
    }

    // 过滤掉有 inst_relate 和 tubi_relate 的节点
    let result_with_filter = crate::query_deep_children_refnos_filter_spre(&[refno.into()], true)
        .await
        .unwrap();
    println!("\n=== pe:21895_68780 测试 (filter=true) ===");
    println!("过滤后的节点数量: {}", result_with_filter.len());
    println!(
        "过滤掉的节点数量: {}",
        result_no_filter.len() - result_with_filter.len()
    );
    if !result_with_filter.is_empty() {
        println!("前 10 个节点:");
        for (i, node) in result_with_filter.iter().take(10).enumerate() {
            println!("  {}. {}", i + 1, node);
        }
    }

    // 测试多个节点
    let refno1: RefU64 = "21895/68780".into();
    let refno2: RefU64 = "16507/4480".into();
    let result_multi =
        crate::query_deep_children_refnos_filter_spre(&[refno1.into(), refno2.into()], false)
            .await
            .unwrap();
    println!("\n=== 多节点测试 (pe:21895_68780 + pe:16507/4480) ===");
    println!("总节点数量（去重后）: {}", result_multi.len());

    Ok(())
}

#[tokio::test]
async fn test_query_deep_children_filter_inst() -> anyhow::Result<()> {
    crate::init_test_surreal().await;

    // 测试单个节点
    let refno: RefU64 = "24383/73928".into();
    let result_single = crate::query_deep_children_filter_inst(&[refno], &["BRAN", "HANG"], false)
        .await
        .unwrap();
    println!("\n=== 单节点测试 (pe:24383/73928) ===");
    println!("找到 {} 个 BRAN/HANG 类型的子孙节点", result_single.len());

    // 测试过滤功能
    let result_filtered = crate::query_deep_children_filter_inst(&[refno], &["BRAN", "HANG"], true)
        .await
        .unwrap();
    println!("\n=== 单节点测试 (filter=true) ===");
    println!("过滤后的节点数量: {}", result_filtered.len());
    println!(
        "过滤掉的节点数量: {}",
        result_single.len() - result_filtered.len()
    );

    // 测试多个节点
    let refno1: RefU64 = "24383/73928".into();
    let refno2: RefU64 = "17496/274056".into();
    let result_multi = crate::query_deep_children_filter_inst(&[refno1, refno2], &["BRAN"], false)
        .await
        .unwrap();
    println!("\n=== 多节点测试 (两个节点) ===");
    println!("总节点数量（去重后）: {}", result_multi.len());

    // 测试空类型（返回所有类型）
    let result_all_types = crate::query_deep_children_filter_inst(&[refno], &[], false)
        .await
        .unwrap();
    println!("\n=== 所有类型测试 ===");
    println!("所有类型的子孙节点数量: {}", result_all_types.len());

    Ok(())
}
