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
pub use uda::*;

pub use adapter::create_surreal_adapter;

use once_cell::sync::Lazy;
use surrealdb::Surreal;
use surrealdb::engine::any::Any;
use surrealdb::opt::auth::Root;

// pub type SurlValue = surrealdb::Value;
pub type SurlValue = surrealdb::sql::Value;
pub type SurlStrand = surrealdb::sql::Strand;
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
    SUL_DB.signin(Root { username, password }).await?;
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
    SUL_DB.signin(Root { username, password }).await?;
    Ok(())
}

/// 初始化内存KV数据库连接
///
/// 连接到配置的内存KV数据库实例，用于PE数据的额外备份。
///
/// # 配置
///
/// 需要在 DbOption.toml 中配置：
/// ```toml
/// mem_kv_ip = "localhost"
/// mem_kv_port = "8011"
/// mem_kv_user = "root"
/// mem_kv_password = "root"
/// ```
///
/// # 错误处理
///
/// 如果连接失败，返回错误。调用者可以决定是否继续运行应用。
///
/// # 示例
///
/// ```rust,no_run
/// use aios_core::init_mem_db;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     match init_mem_db().await {
///         Ok(_) => println!("备份数据库连接成功"),
///         Err(e) => eprintln!("备份数据库连接失败: {}", e),
///     }
///     Ok(())
/// }
/// ```
#[cfg(feature = "mem-kv-save")]
pub async fn init_mem_db() -> anyhow::Result<()> {
    use crate::get_db_option;

    let db_option = get_db_option();

    // 构建连接字符串
    let address = format!("{}:{}", db_option.mem_kv_ip, db_option.mem_kv_port);
    let conn_str = format!("ws://{}", address);

    println!("正在连接到内存KV数据库: {}", conn_str);

    // 创建配置
    let config = surrealdb::opt::Config::default()
        .ast_payload();  // 启用AST格式

    // 连接到数据库
    SUL_MEM_DB
        .connect((&conn_str, config))
        .with_capacity(1000)
        .await?;

    // 使用命名空间和数据库
    SUL_MEM_DB
        .use_ns(&db_option.project_code)
        .use_db(&db_option.project_name)
        .await?;

    // 认证
    SUL_MEM_DB.signin(Root {
        username: &db_option.mem_kv_user,
        password: &db_option.mem_kv_password,
    }).await?;

    println!("✅ 内存KV数据库连接成功: {} -> NS: {}, DB: {}",
        conn_str, db_option.project_code, db_option.project_name);

    Ok(())
}

pub fn convert_to_sql_str_array(nouns: &[&str]) -> String {
    let nouns_str = nouns
        .iter()
        .map(|s| format!("'{s}'"))
        .collect::<Vec<_>>()
        .join(",");
    nouns_str
}
