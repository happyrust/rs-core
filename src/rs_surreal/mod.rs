pub mod adapter;
pub mod datacenter_query;
pub mod geom;
pub mod graph;
pub mod index;
pub mod mdb;
pub mod query;

// 新的重构模块
// pub mod query_builder;
// pub mod error_handler;
// pub mod cache_manager;
// pub mod queries;

pub mod cate;
pub mod resolve;
pub mod spatial;
mod table_const;
pub mod uda;

pub mod pbs;

pub mod inst;
pub mod inst_structs;

pub mod point;

pub mod function;

pub mod version;

pub mod e3d_db;
pub mod topology;

pub mod operation;

// XKT 生成相关查询
pub mod type_hierarchy;
pub mod xkt_query;

pub use cate::*;
pub use e3d_db::*;
pub use geom::*;
pub use graph::*;
pub use index::*;
pub use inst::*;
pub use inst_structs::*;
pub use mdb::*;
pub use pbs::*;
pub use point::*;
pub use query::*;
pub use resolve::*;
pub use spatial::*;
pub use topology::*;
pub use type_hierarchy::*;
pub use uda::*;
pub use xkt_query::*;

pub use adapter::create_surreal_adapter;

use once_cell::sync::Lazy;
use surrealdb::Surreal;
use surrealdb::engine::any::Any;
use surrealdb::opt::auth::Root;

// pub type SurlValue = surrealdb::Value;
pub type SurlValue = surrealdb::types::Value;
pub static SUL_DB: Lazy<Surreal<Any>> = Lazy::new(Surreal::init);
pub static SECOND_SUL_DB: Lazy<Surreal<Any>> = Lazy::new(Surreal::init);
pub static KV_DB: Lazy<Surreal<Any>> = Lazy::new(Surreal::init);

/// 内存KV数据库全局连接（用于PE数据额外备份）
#[cfg(feature = "mem-kv-save")]
pub static SUL_MEM_DB: Lazy<Surreal<Any>> = Lazy::new(Surreal::init);

///连接surreal
pub async fn connect_surdb(
    conn_str: &str,
    ns: &str,
    db: &str,
    username: &str,
    password: &str,
) -> Result<(), surrealdb::Error> {
    // 创建配置
    let config = surrealdb::opt::Config::default()
        .ast_payload()  // 启用AST格式
        ; // 设置容量
    SUL_DB
        .connect((conn_str, config))
        .with_capacity(1000)
        .await?;
    SUL_DB.use_ns(ns).use_db(db).await?;
    SUL_DB
        .signin(Root {
            username: username.to_owned(),
            password: password.to_owned(),
        })
        .await?;
    Ok(())
}

pub async fn connect_kvdb(
    conn_str: &str,
    ns: &str,
    db: &str,
    username: &str,
    password: &str,
) -> Result<(), surrealdb::Error> {
    SUL_DB.connect(conn_str).with_capacity(1000).await?;
    SUL_DB.use_ns(ns).use_db(db).await?;
    SUL_DB
        .signin(Root {
            username: username.to_owned(),
            password: password.to_owned(),
        })
        .await?;
    Ok(())
}

/// 带重试的内存KV数据库初始化（与 init_surreal_with_retry 风格一致）
#[cfg(feature = "mem-kv-save")]
pub async fn init_mem_db_with_retry(db_option: &crate::options::DbOption) -> anyhow::Result<()> {
    use std::time::Duration;

    let normalized_ip = if db_option.mem_kv_ip == "localhost" {
        "127.0.0.1".to_string()
    } else {
        db_option.mem_kv_ip.clone()
    };

    let addr = format!("{}:{}", normalized_ip, db_option.mem_kv_port);
    let conn_str = format!("ws://{}", addr);

    let max_retries: usize = 10;
    let mut attempt: usize = 0;
    loop {
        println!(
            "尝试连接内存KV: {} (NS={}, DB={})，第{}次",
            conn_str,
            db_option.project_code,
            db_option.project_name,
            attempt + 1
        );

        // 创建配置
        let config = surrealdb::opt::Config::default().ast_payload();

        let connect_result = async {
            SUL_MEM_DB
                .connect((&conn_str, config))
                .with_capacity(1000)
                .await?;
            SUL_MEM_DB
                .use_ns(&db_option.project_code)
                .use_db(&db_option.project_name)
                .await?;
            SUL_MEM_DB
                .signin(Root {
                    username: db_option.mem_kv_user.clone(),
                    password: db_option.mem_kv_password.clone(),
                })
                .await?;
            Ok::<(), surrealdb::Error>(())
        }
        .await;

        match connect_result {
            Ok(_) => {
                println!(
                    "✅ 内存KV数据库连接成功: {} -> NS: {}, DB: {}",
                    conn_str, db_option.project_code, db_option.project_name
                );
                return Ok(());
            }
            Err(e) => {
                attempt += 1;
                if attempt >= max_retries {
                    return Err(anyhow::anyhow!(e));
                }
                let backoff_ms = 200u64.saturating_mul(attempt as u64);
                eprintln!(
                    "⚠️ 内存KV连接失败(第{}次): {}，{}ms后重试...",
                    attempt, e, backoff_ms
                );
                tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
            }
        }
    }
}

pub fn convert_to_sql_str_array(nouns: &[&str]) -> String {
    let nouns_str = nouns
        .iter()
        .map(|s| format!("'{s}'"))
        .collect::<Vec<_>>()
        .join(",");
    nouns_str
}
