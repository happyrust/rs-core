use aios_core::{init_surreal, SUL_DB};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_surreal().await?;
    
    // 检查 pe 表中是否有这个 refno
    println!("--- 检查 pe:24383_73932 是否存在 ---");
    let sql = r#"SELECT id, noun FROM pe WHERE id = pe:⟨24383_73932⟩"#;
    let resp = SUL_DB.query(sql).await?;
    println!("{:?}\n", resp);
    
    // 查找 24383 开头的 ELBO
    println!("--- 查找 24383 开头的 ELBO ---");
    let sql = r#"SELECT id, noun FROM pe WHERE noun = 'ELBO' AND string::starts_with(meta::id(id), '24383') LIMIT 5"#;
    let resp = SUL_DB.query(sql).await?;
    println!("{:?}\n", resp);
    
    Ok(())
}
