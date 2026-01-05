//! 测试 ELBO 24383_73932 的模型生成和 geo_type
use aios_core::rs_surreal::inst::query_insts_with_batch;
use aios_core::{RefnoEnum, SUL_DB, init_surreal};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_surreal().await?;

    // 先检查数据库是否有数据
    println!("=== 检查数据库状态 ===\n");

    let sql = r#"SELECT count() as cnt FROM inst_relate"#;
    let resp = SUL_DB.query(sql).await?;
    println!("inst_relate 记录数: {:?}\n", resp);

    let sql = r#"SELECT count() as cnt FROM geo_relate"#;
    let resp = SUL_DB.query(sql).await?;
    println!("geo_relate 记录数: {:?}\n", resp);

    // 如果有数据，查询 geo_type 分布
    println!("--- geo_type 分布 ---");
    let sql = r#"SELECT geo_type, count() as cnt FROM geo_relate GROUP BY geo_type"#;
    let resp = SUL_DB.query(sql).await?;
    println!("{:?}\n", resp);

    // 测试特定 ELBO
    let refno = RefnoEnum::from("24383_73932");
    println!("=== 测试 ELBO {} ===\n", refno.to_pe_key());

    // query_insts_with_batch
    println!("--- query_insts_with_batch (enable_holes=true) ---");
    let results = query_insts_with_batch([&refno].into_iter(), true, Some(10)).await?;
    if results.is_empty() {
        println!("  ⚠️ 没有数据，请先生成模型");
    } else {
        for inst in &results {
            println!(
                "  {} | has_neg={} | insts.len={}",
                inst.refno.to_pe_key(),
                inst.has_neg,
                inst.insts.len(),
            );
        }
    }

    println!("\n--- query_insts_with_batch (enable_holes=false) ---");
    let results = query_insts_with_batch([&refno].into_iter(), false, Some(10)).await?;
    if results.is_empty() {
        println!("  ⚠️ 没有数据，请先生成模型");
    } else {
        for inst in &results {
            println!(
                "  {} | has_neg={} | insts.len={}",
                inst.refno.to_pe_key(),
                inst.has_neg,
                inst.insts.len(),
            );
        }
    }

    Ok(())
}
