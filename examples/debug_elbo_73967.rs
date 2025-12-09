//! 调试 ELBO 24383_73967 为什么显示不出来
use aios_core::rs_surreal::inst::query_insts_with_batch;
use aios_core::{init_surreal, RefnoEnum, SUL_DB};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_surreal().await?;

    let refno = RefnoEnum::from("24383_73967");
    let pe_key = refno.to_pe_key();
    
    println!("=== 调试 ELBO {} ===\n", pe_key);
    
    // 5. 检查 booled_id 指向的几何体
    println!("--- 5. 检查 booled_id 指向的几何体 ---");
    let sql = r#"SELECT * FROM inst_geo:⟨16751683998175714625⟩"#;
    let resp = SUL_DB.query(sql).await?;
    println!("booled_id 指向的 inst_geo: {:?}", resp);
    
    // 6. 检查是否有 Compound 类型的 geo_relate
    println!("\n--- 6. Compound geo_relate 检查 ---");
    let sql = format!(r#"
        SELECT 
            (SELECT geo_type, record::id(out) as geo_id
             FROM out->geo_relate 
             WHERE geo_type = 'Compound') AS compounds
        FROM inst_relate WHERE in = {}
    "#, pe_key);
    let resp = SUL_DB.query(&sql).await?;
    println!("{:?}", resp);

    // 7. query_insts_with_batch 结果
    println!("\n--- 7. query_insts_with_batch 结果 ---");
    let with_holes = query_insts_with_batch([&refno], true, Some(10)).await?;
    println!("enable_holes=true: 返回 {} 条", with_holes.len());
    for inst in &with_holes {
        println!(
            "  refno={} has_neg={} insts.len={}",
            inst.refno.to_pe_key(),
            inst.has_neg,
            inst.insts.len()
        );
    }
    
    let without_holes = query_insts_with_batch([&refno], false, Some(10)).await?;
    println!("\nenable_holes=false: 返回 {} 条", without_holes.len());
    for inst in &without_holes {
        println!(
            "  refno={} has_neg={} insts.len={}",
            inst.refno.to_pe_key(),
            inst.has_neg,
            inst.insts.len()
        );
    }

    Ok(())
}
