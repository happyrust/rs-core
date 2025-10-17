use crate::rs_surreal::graph::collect_descendant_ids_has_inst;
use crate::{RefnoEnum, init_surreal};

#[tokio::test]
#[ignore] // 需要真实数据库连接，使用 cargo test test_collect_descendant_ids_has_inst --lib -- --ignored --test-threads=1 --nocapture 运行
async fn test_collect_descendant_ids_has_inst() -> anyhow::Result<()> {
    println!("\n=== 测试 collect_descendant_ids_has_inst 函数 ===\n");

    // 初始化数据库连接（会自动加载 common.surql）
    println!("初始化数据库连接...");
    init_surreal().await?;
    println!("✓ 数据库连接成功\n");

    // 测试节点 ID
    let test_refno = RefnoEnum::from("pe:17496_201376");
    println!("测试节点: {:?}\n", test_refno);

    // ========================================================================
    // 测试 1: 查询所有有 inst_relate 的子孙节点（不限类型）
    // ========================================================================
    println!("=== 测试 1: 查询所有有 inst_relate 的子孙节点（不限类型，不包含自身） ===\n");
    let nodes_with_inst = collect_descendant_ids_has_inst(test_refno, &[], false, None).await?;

    println!(
        "找到 {} 个有 inst_relate 的子孙节点\n",
        nodes_with_inst.len()
    );

    // 显示前 20 个节点
    if !nodes_with_inst.is_empty() {
        println!("有 inst_relate 的节点（前 20 个）:");
        for (i, refno) in nodes_with_inst.iter().take(20).enumerate() {
            println!("  [{}] {:?}", i + 1, refno);
        }
        println!();
    } else {
        println!("⚠️ 未找到有 inst_relate 的节点\n");
    }

    println!("\n=== 测试完成 ===");
    Ok(())
}
