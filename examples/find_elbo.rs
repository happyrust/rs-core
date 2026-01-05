use aios_core::{SUL_DB, init_surreal};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 使用环境变量设置配置文件
    std::env::set_var("DB_CONFIG", "DbOption-zsy");
    init_surreal().await?;

    println!("--- 查找 ELBO ---");
    let sql = r#"SELECT id, noun FROM pe WHERE noun = 'ELBO' LIMIT 10"#;
    let resp = SUL_DB.query(sql).await?;
    println!("{:?}", resp);

    Ok(())
}
