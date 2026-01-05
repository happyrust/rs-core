use aios_core::rs_surreal::inst::query_insts_with_batch;
use aios_core::{RefnoEnum, SUL_DB, init_surreal};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_surreal().await?;

    let refno = RefnoEnum::from("24383_73967");
    println!("=== 测试 ELBO {} ===\n", refno);

    // 1. 查看 geo_relate 的 geo_type
    println!("--- geo_relate geo_type ---");
    let sql = format!(
        r#"
        SELECT geo_type, visible, record::id(out) as geo_id 
        FROM geo_relate WHERE in.in = {}
    "#,
        refno.to_pe_key()
    );
    let resp = SUL_DB.query(&sql).await?;
    println!("{:?}\n", resp);

    // 2. enable_holes=true
    println!("--- query_insts_with_batch (enable_holes=true) ---");
    let results = query_insts_with_batch([&refno].into_iter(), true, None).await?;
    for inst in &results {
        println!(
            "  {} | has_neg={} | insts.len={}",
            inst.refno,
            inst.has_neg,
            inst.insts.len()
        );
    }

    // 3. enable_holes=false
    println!("\n--- query_insts_with_batch (enable_holes=false) ---");
    let results = query_insts_with_batch([&refno].into_iter(), false, None).await?;
    for inst in &results {
        println!(
            "  {} | has_neg={} | insts.len={}",
            inst.refno,
            inst.has_neg,
            inst.insts.len()
        );
    }

    Ok(())
}
