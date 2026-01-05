//! 验证 query_insts_with_batch 在 enable_holes 切换时返回的 insts/has_neg 差异。
use aios_core::rs_surreal::inst::query_insts_with_batch;
use aios_core::{RefnoEnum, SUL_DB, SurrealQueryExt, init_surreal};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 连接数据库（使用默认 DbOption.toml）
    init_surreal().await?;

    let refno = RefnoEnum::from("pe:17496_106028");

    // 检查 ngmr_relate 数据
    let sql_ngmr = r#"SELECT * FROM ngmr_relate WHERE out = pe:17496_106028 LIMIT 3"#;
    let resp_ngmr = SUL_DB.query(sql_ngmr).await?;
    println!("ngmr_relate for 17496_106028:\n{:?}\n", resp_ngmr);

    // 检查 neg_relate 数据
    let sql_neg = r#"SELECT * FROM neg_relate WHERE out = pe:17496_106028 LIMIT 3"#;
    let resp_neg = SUL_DB.query(sql_neg).await?;
    println!("neg_relate for 17496_106028:\n{:?}\n", resp_neg);

    // 检查新查询是否工作
    let sql_new_query = r#"
        SELECT 
            in.out AS id,
            in.geo_type AS geo_type,
            in.trans.d AS trans
        FROM pe:17496_106028<-ngmr_relate
        WHERE in.trans.d != NONE
    "#;
    let resp_new = SUL_DB.query(sql_new_query).await?;
    println!("new ngmr_relate query:\n{:?}\n", resp_new);

    // 检查负实体 pe:17496_106029 的 geo_relate
    let sql1 = r#"
        SELECT 
            (SELECT geo_type, record::id(out) as id, trans.d as trans, out.aabb.d as aabb 
             FROM out->geo_relate 
             WHERE trans.d != NONE) AS geos
        FROM inst_relate:17496_106029
    "#;
    let resp1 = SUL_DB.query(sql1).await?;
    println!("geo_relate for 17496_106029 (negative entity):");
    println!("{:?}", resp1);

    // 检查负实体有没有 geo_type="Neg"
    let sql2 = r#"
        SELECT 
            (SELECT geo_type, record::id(out) as id 
             FROM out->geo_relate 
             WHERE geo_type == "Neg") AS neg_geos
        FROM inst_relate:17496_106029
    "#;
    let resp2 = SUL_DB.query(sql2).await?;
    println!("\nneg_geos (geo_type=Neg) for 17496_106029: {:?}", resp2);

    // enable_holes = true
    let with_holes = query_insts_with_batch([&refno], true, Some(10)).await?;
    println!("enable_holes=true: 返回 {} 条", with_holes.len());
    for inst in &with_holes {
        println!(
            "[holes] refno={} has_neg={} insts={:?}",
            inst.refno.to_pe_key(),
            inst.has_neg,
            inst.insts
        );
    }

    // enable_holes = false
    let without_holes = query_insts_with_batch([&refno], false, Some(10)).await?;
    println!("enable_holes=false: 返回 {} 条", without_holes.len());
    for inst in &without_holes {
        println!(
            "[plain] refno={} has_neg={} insts={:?}",
            inst.refno.to_pe_key(),
            inst.has_neg,
            inst.insts
        );
    }

    Ok(())
}
