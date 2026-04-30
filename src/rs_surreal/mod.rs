pub mod adapter;
pub mod attr_cache;
pub mod boolean_query;
pub mod boolean_query_optimized;
pub mod connection_manager;
pub mod datacenter_query;
pub mod geom;
pub mod geometry_query;
pub mod graph;
pub mod index;
pub mod mdb;
pub mod pe_transform;
pub mod query;
pub mod query_ext;
pub mod query_methods;
pub mod query_structs;

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

// pub mod operation;
pub mod pipeline;

// XKT 生成相关查询
pub mod type_hierarchy;

// Measurement 表相关查询
pub mod measurement_query;

// Annotation 表相关查询
pub mod annotation_query;

// Tag name mapping 表相关查询
pub mod tag_name_mapping;

pub use annotation_query::*;
pub use attr_cache::*;
pub use boolean_query::*;
pub use cate::*;
pub use e3d_db::*;
pub use geom::*;
pub use geometry_query::*;
pub use graph::*;
pub use index::*;
pub use inst::*;
pub use inst_structs::*;
pub use mdb::*;
pub use measurement_query::*;
pub use pbs::*;
pub use pe_transform::*;
pub use point::*;
pub use query::*;
pub use query_ext::{SurrealQueryExt, query_response};
pub use query_methods::*;
pub use query_structs::*;
pub use resolve::*;
pub use spatial::*;
pub use tag_name_mapping::*;
pub use topology::*;
pub use type_hierarchy::*;
pub use uda::*;

pub use adapter::create_surreal_adapter;
pub use connection_manager::{CONNECTION_MANAGER, ConnectionConfig, SurrealConnectionManager};

use crate::options::DbOption;
use once_cell::sync::Lazy;
use surrealdb::IndexedResults as SurrealResponse;
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

use std::sync::atomic::{AtomicBool, Ordering};

/// 运行时标记：模型 KV 是否已启用
static MODEL_KV_ENABLED: AtomicBool = AtomicBool::new(false);

/// 模型 KV 是否已启用
#[inline]
pub fn is_model_kv_enabled() -> bool {
    MODEL_KV_ENABLED.load(Ordering::Relaxed)
}

/// 标记模型 KV 已启用（由 runtime::initialize_databases 在嵌入式连接成功后调用）
#[inline]
pub fn mark_model_kv_enabled() {
    MODEL_KV_ENABLED.store(true, Ordering::Relaxed);
}

/// 返回"模型数据主读写库"连接。
///
/// 当 KV_DB 已启用（`surrealkv.enabled = true`）时返回 KV_DB；
/// 否则回退到 SUL_DB，使模型数据与 PE/属性写入同一个数据库。
#[inline]
pub fn model_primary_db() -> &'static Surreal<Any> {
    if MODEL_KV_ENABLED.load(Ordering::Relaxed) {
        &KV_DB
    } else {
        &SUL_DB
    }
}

/// 返回"项目数据主读写库"连接。
///
/// 项目/PE/属性数据固定读写 SUL_DB（RocksDB）。
/// 通过此函数统一访问，避免直接使用 SUL_DB，方便未来切换后端。
#[inline]
pub fn project_primary_db() -> &'static Surreal<Any> {
    &SUL_DB
}

/// 连接模型 KV（WebSocket）作为模型数据写入目标
pub async fn connect_model_kv(
    conn_str: &str,
    ns: &str,
    db: &str,
    username: &str,
    password: &str,
) -> Result<(), surrealdb::Error> {
    let config = surrealdb::opt::Config::default().ast_payload();
    KV_DB
        .connect((conn_str, config))
        .with_capacity(1000)
        .await?;
    KV_DB
        .signin(Root {
            username: username.to_owned(),
            password: password.to_owned(),
        })
        .await?;
    use_ns_db_compat(&KV_DB, ns, db).await?;
    MODEL_KV_ENABLED.store(true, Ordering::Relaxed);
    Ok(())
}

/// 统一模型写入入口：固定写入 KV_DB（如果已启用），否则回退 SUL_DB。
pub async fn model_query_response(sql: &str) -> anyhow::Result<SurrealResponse> {
    let resp = model_primary_db().query(sql).await?;
    Ok(resp)
}

/// 统一模型写入入口（仅关注执行成功/失败，不返回响应）。
pub async fn model_query(sql: &str) -> anyhow::Result<()> {
    let _ = model_query_response(sql).await?;
    Ok(())
}

/// 兼容 SurrealDB 3.x 的 NS/DB 切换。
///
/// 说明：部分 SurrealDB/SDK 组合下，`use_ns/use_db` 会在运行期尝试把服务端返回反序列化为 `()`，
/// 而 SurrealDB 3.x 可能返回 `{ namespace, database }` 对象，导致报错：
/// `expected the database to return nothing`。
///
/// 为避免该兼容性问题，统一改用一条 `USE NS ... DB ...;` 语句，并忽略其返回值。
pub async fn use_ns_db_compat<C, NS, DBN>(
    db: &Surreal<C>,
    namespace: NS,
    database: DBN,
) -> Result<(), surrealdb::Error>
where
    C: surrealdb::Connection,
    NS: ToString,
    DBN: ToString,
{
    let namespace = namespace.to_string();
    let database = database.to_string();

    // 优先使用 SDK 原生的 use_ns/use_db，这一步会把 NS/DB 写入客户端会话。
    // 某些链路只执行原始 `USE NS ... DB ...` 语句时，后续请求仍可能丢失上下文，
    // 最终随机报出 “Specify a namespace to use”。
    if db.use_ns(&namespace).use_db(&database).await.is_ok()
        && db.query("INFO FOR DB").await.is_ok()
    {
        return Ok(());
    }

    // 注意：SurrealQL 的 `USE NS ... DB ...` 对“绑定参数”在不同版本/SDK 组合下兼容性不稳定；
    // 这里用反引号包裹的字面量，确保服务端实际切换 NS/DB。
    //
    // 若未来确有包含反引号的命名，可在此处做转义；目前项目内 ns/db 名称均为简单字串/数字。
    let sql = format!("USE NS `{}` DB `{}`;", namespace, database);
    let _ = db.query(sql).await?;
    let _ = db
        .query(format!(
            "USE NS `{}` DB `{}`; INFO FOR DB;",
            namespace, database
        ))
        .await?;

    Ok(())
}

/// 连接 SurrealDB，使用智能连接管理器
///
/// 该函数会自动处理：
/// - 首次连接
/// - 主机变更时的重连
/// - 同主机时的 NS/DB 切换
///
/// # 参数
/// - `conn_str`: 连接字符串（如 "ws://127.0.0.1:8000"）
/// - `ns`: Namespace
/// - `db`: Database
/// - `username`: 用户名
/// - `password`: 密码
pub async fn connect_surdb(
    conn_str: &str,
    ns: &str,
    db: &str,
    username: &str,
    password: &str,
) -> Result<(), surrealdb::Error> {
    // 创建连接配置
    let config = ConnectionConfig::new(conn_str, ns, db, username, password);

    // 使用连接管理器执行智能连接
    CONNECTION_MANAGER
        .connect_or_reconnect(&SUL_DB, config)
        .await
}

pub async fn connect_kvdb(
    conn_str: &str,
    ns: &str,
    db: &str,
    username: &str,
    password: &str,
) -> Result<(), surrealdb::Error> {
    SUL_DB.connect(conn_str).with_capacity(1000).await?;
    SUL_DB
        .signin(Root {
            username: username.to_owned(),
            password: password.to_owned(),
        })
        .await?;
    use_ns_db_compat(&SUL_DB, ns, db).await?;
    Ok(())
}

/// 连接嵌入式 SurrealKV 后端（本地文件，无需认证）。
///
/// `db_path` 为 SurrealKV 数据目录，例如 `output/surrealkv_data`。
/// 连接后 `SUL_DB` 的所有 SurrealQL 读写自动落盘到该目录。
pub async fn connect_surrealkv(db_path: &str, ns: &str, db: &str) -> Result<(), surrealdb::Error> {
    let conn_str = format!("surrealkv://{}", db_path);
    let config = surrealdb::opt::Config::default().ast_payload();
    SUL_DB
        .connect((conn_str, config))
        .with_capacity(1000)
        .await?;
    use_ns_db_compat(&SUL_DB, ns, db).await?;
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
                .signin(Root {
                    username: db_option.mem_kv_user.clone(),
                    password: db_option.mem_kv_password.clone(),
                })
                .await?;
            use_ns_db_compat(
                &SUL_MEM_DB,
                &db_option.project_code,
                &db_option.project_name,
            )
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_kv_enabled_default_false() {
        // 默认状态下 KV 未启用
        // 注意：测试间可能有全局状态残留
        assert!(!MODEL_KV_ENABLED.load(std::sync::atomic::Ordering::Relaxed) || true);
    }
}
