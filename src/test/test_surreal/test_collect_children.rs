use crate::*;

#[tokio::test]
async fn test_collect_children_with_expr() -> anyhow::Result<()> {
    crate::init_test_surreal().await;

    // 使用已知的测试 refno
    let site_refno: RefnoEnum = "21491/10801".into();

    println!("\n=== 测试 1: 查询直接子节点 ID ===");
    let ids: Vec<RefnoEnum> =
        collect_children_with_expr(site_refno.clone(), &[], "VALUE id").await?;
    println!("找到 {} 个直接子节点", ids.len());
    assert!(!ids.is_empty(), "Should find children");

    println!("\n=== 测试 2: 查询直接子节点 ID（按类型过滤）===");
    let equi_ids: Vec<RefnoEnum> =
        collect_children_with_expr(site_refno.clone(), &["EQUI"], "VALUE id").await?;
    println!("找到 {} 个 EQUI 类型的直接子节点", equi_ids.len());

    println!("\n=== 测试 3: 查询直接子节点属性 ===");
    let attrs: Vec<NamedAttrMap> =
        collect_children_with_expr(site_refno.clone(), &["EQUI"], "VALUE id.refno.*").await?;
    println!("找到 {} 个 EQUI 属性", attrs.len());
    assert_eq!(equi_ids.len(), attrs.len(), "ID 数量应该与属性数量一致");

    println!("\n=== 测试 4: 使用便利函数 collect_children_filter_ids ===");
    let ids2 = collect_children_filter_ids(site_refno.clone(), &["EQUI"]).await?;
    assert_eq!(equi_ids.len(), ids2.len(), "便利函数应该返回相同数量的 ID");

    println!("\n=== 测试 5: 使用便利函数 collect_children_filter_attrs ===");
    let attrs2 = collect_children_filter_attrs(site_refno.clone(), &["EQUI"]).await?;
    assert_eq!(attrs.len(), attrs2.len(), "便利函数应该返回相同数量的属性");

    println!("\n=== 所有测试通过 ===");

    Ok(())
}
