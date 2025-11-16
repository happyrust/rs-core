//! MySQL 连接池管理模块
//!
//! 提供独立的 MySQL 连接池管理功能，从 AiosDBMgr 迁移而来

use crate::options::DbOption;
use crate::table_const::{GLOBAL_DATABASE, PUHUA_MATERIAL_DATABASE};
use std::collections::HashMap;
use std::time::Duration;

#[cfg(feature = "sql")]
use sqlx::pool::PoolOptions;
#[cfg(feature = "sql")]
use sqlx::{MySql, Pool};

/// 获取默认的 MySQL 连接字符串
pub fn default_mysql_conn_str(db_option: &DbOption) -> String {
    let user = db_option.user.as_str();
    let pwd = urlencoding::encode(&db_option.password);
    let ip = db_option.ip.as_str();
    let port = db_option.port.as_str();
    format!("mysql://{user}:{pwd}@{ip}:{port}")
}

/// 获取浦华数据库连接字符串
pub fn puhua_conn_str(db_option: &DbOption) -> String {
    let user = db_option.puhua_database_user.as_str();
    let pwd = db_option.puhua_database_password.as_str();
    let ip = db_option.puhua_database_ip.as_str();
    format!("mysql://{user}:{pwd}@{ip}")
}

#[cfg(feature = "sql")]
/// 获取全局配置数据库连接池
///
/// # 返回
/// MySQL 连接池，连接到 GLOBAL_DATABASE
pub async fn get_global_pool(db_option: &DbOption) -> anyhow::Result<Pool<MySql>> {
    let connection_str = default_mysql_conn_str(db_option);
    let url = &format!("{connection_str}/{}", GLOBAL_DATABASE);
    PoolOptions::new()
        .max_connections(500)
        .acquire_timeout(Duration::from_secs(10 * 60))
        .connect(url)
        .await
        .map_err(|x| anyhow::anyhow!(x.to_string()))
}

#[cfg(feature = "sql")]
/// 获取项目数据库连接池
///
/// # 返回
/// MySQL 连接池，连接到项目数据库
pub async fn get_project_pool(db_option: &DbOption) -> anyhow::Result<Pool<MySql>> {
    let connection_str = default_mysql_conn_str(db_option);
    let url = &format!("{connection_str}/{}", db_option.project_name);
    PoolOptions::new()
        .max_connections(500)
        .acquire_timeout(Duration::from_secs(10 * 60))
        .connect(url)
        .await
        .map_err(|x| anyhow::anyhow!(x.to_string()))
}

#[cfg(feature = "sql")]
/// 获取多个项目的数据库连接池
///
/// # 参数
/// - `db_option`: 数据库配置（包含 included_projects）
///
/// # 返回
/// 项目名到连接池的映射
pub async fn get_project_pools(
    db_option: &DbOption,
) -> anyhow::Result<HashMap<String, Pool<MySql>>> {
    let connection_str = default_mysql_conn_str(db_option);
    let mut map = HashMap::new();
    for project in &db_option.included_projects {
        let url = format!("{connection_str}/{}", project);
        let pool: Pool<MySql> = PoolOptions::new()
            .max_connections(500)
            .acquire_timeout(Duration::from_secs(10 * 60))
            .connect(&url)
            .await
            .map_err(|x| anyhow::anyhow!(x.to_string()))?;
        map.entry(project.to_string()).or_insert(pool);
    }
    Ok(map)
}

#[cfg(feature = "sql")]
/// 获取浦华外部数据库连接池
///
/// # 返回
/// MySQL 连接池，连接到浦华材料数据库
pub async fn get_puhua_pool(db_option: &DbOption) -> anyhow::Result<Pool<MySql>> {
    let conn = puhua_conn_str(db_option);
    let url = &format!("{conn}/{}", PUHUA_MATERIAL_DATABASE);
    PoolOptions::new()
        .max_connections(500)
        .acquire_timeout(Duration::from_secs(10 * 60))
        .connect(url)
        .await
        .map_err(|x| anyhow::anyhow!(x.to_string()))
}
