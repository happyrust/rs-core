//! 分析 geo_relate 复用问题
use aios_core::{SUL_DB, init_surreal};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_surreal().await?;

    // 1. 查看 ELBO 24383_73967 的 geo_relate 详情
    println!("=== 1. ELBO 24383_73967 的 geo_relate ===");
    let sql = r#"
        SELECT 
            id,
            in as inst_info,
            out as inst_geo,
            geo_type,
            booled_id
        FROM geo_relate WHERE in.in = pe:⟨24383_73967⟩
    "#;
    let resp = SUL_DB.query(sql).await?;
    println!("{:?}\n", resp);

    // 2. 查看同一个 inst_geo 被多少 geo_relate 引用
    println!("=== 2. inst_geo:2 被哪些 geo_relate 引用 ===");
    let sql = r#"
        SELECT 
            id,
            in as inst_info,
            in.in as pe_refno,
            geo_type,
            booled_id
        FROM geo_relate WHERE out = inst_geo:⟨2⟩ LIMIT 10
    "#;
    let resp = SUL_DB.query(sql).await?;
    println!("{:?}\n", resp);

    // 3. 查看这些元素的 inst_relate 状态
    println!("=== 3. 引用 inst_geo:2 的元素的 inst_relate 状态 ===");
    let sql = r#"
        SELECT 
            in.id as refno,
            bool_status,
            has_cata_neg
        FROM inst_relate 
        WHERE out IN (SELECT VALUE in FROM geo_relate WHERE out = inst_geo:⟨2⟩)
        LIMIT 10
    "#;
    let resp = SUL_DB.query(sql).await?;
    println!("{:?}\n", resp);

    Ok(())
}
