//! Kuzu 图数据库集成模块
//!
//! 提供 Kuzu 嵌入式图数据库的集成支持，用于高效的图遍历和关系查询。

#[cfg(feature = "kuzu")]
pub mod connection;
#[cfg(feature = "kuzu")]
pub mod schema;
#[cfg(feature = "kuzu")]
pub mod types;
#[cfg(feature = "kuzu")]
pub mod queries;
#[cfg(feature = "kuzu")]
pub mod operations;
#[cfg(feature = "kuzu")]
pub mod adapter;

#[cfg(feature = "kuzu")]
pub use connection::*;
#[cfg(feature = "kuzu")]
pub use schema::*;
#[cfg(feature = "kuzu")]
pub use types::*;
#[cfg(feature = "kuzu")]
pub use adapter::create_kuzu_adapter;

#[cfg(feature = "kuzu")]
use kuzu::{Database, Connection, SystemConfig};
#[cfg(feature = "kuzu")]
use once_cell::sync::Lazy;
#[cfg(feature = "kuzu")]
use parking_lot::RwLock;
#[cfg(feature = "kuzu")]
use std::sync::Arc;
#[cfg(feature = "kuzu")]
use std::cell::RefCell;

#[cfg(feature = "kuzu")]
/// 全局 Kuzu 数据库实例
pub static KUZU_DB: Lazy<Arc<RwLock<Option<Database>>>> =
    Lazy::new(|| Arc::new(RwLock::new(None)));

#[cfg(feature = "kuzu")]
/// 初始化 Kuzu 数据库
///
/// # 参数
/// * `path` - 数据库文件路径
/// * `config` - 系统配置
///
/// # 示例
/// ```no_run
/// # use aios_core::rs_kuzu::init_kuzu;
/// # use kuzu::SystemConfig;
/// # tokio_test::block_on(async {
/// init_kuzu("./data/kuzu_db", SystemConfig::default()).await.unwrap();
/// # });
/// ```
pub async fn init_kuzu(path: &str, config: SystemConfig) -> anyhow::Result<()> {
    log::info!("正在初始化 Kuzu 数据库: {}", path);

    let db = Database::new(path, config)?;
    *KUZU_DB.write() = Some(db);

    log::info!("Kuzu 数据库初始化成功");
    Ok(())
}

#[cfg(feature = "kuzu")]
/// Kuzu 连接包装器
///
/// 持有数据库读锁和连接，确保生命周期正确
pub struct KuzuConnectionGuard {
    _guard: parking_lot::RwLockReadGuard<'static, Option<Database>>,
    conn: Connection<'static>,
}

#[cfg(feature = "kuzu")]
impl std::ops::Deref for KuzuConnectionGuard {
    type Target = Connection<'static>;

    fn deref(&self) -> &Self::Target {
        &self.conn
    }
}

#[cfg(feature = "kuzu")]
/// 创建新的 Kuzu 连接
///
/// 返回一个包装器，持有必要的锁和连接
pub fn create_kuzu_connection() -> anyhow::Result<KuzuConnectionGuard> {
    // SAFETY: 我们将 guard 的生命周期转换为 'static
    // 这是安全的，因为 KUZU_DB 是全局静态变量
    let guard: parking_lot::RwLockReadGuard<'static, Option<Database>> = unsafe {
        std::mem::transmute(KUZU_DB.read())
    };

    let db = guard
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Kuzu 数据库未初始化"))?;

    // SAFETY: 数据库引用的生命周期被扩展为 'static
    // 这是安全的，因为我们持有 guard，确保数据库不会被释放
    let db_static: &'static Database = unsafe {
        &*(db as *const Database)
    };

    let conn = Connection::new(db_static)?;

    Ok(KuzuConnectionGuard {
        _guard: guard,
        conn,
    })
}

#[cfg(feature = "kuzu")]
/// 检查 Kuzu 是否已初始化
pub fn is_kuzu_initialized() -> bool {
    KUZU_DB.read().is_some()
}

#[cfg(not(feature = "kuzu"))]
/// Kuzu 功能未启用时的占位函数
pub fn kuzu_feature_disabled() {
    panic!("Kuzu 功能未启用。请在 Cargo.toml 中添加 'kuzu' feature");
}