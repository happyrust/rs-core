use anyhow::{Result, anyhow};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Arc;
use surrealdb::Surreal;
use surrealdb::engine::remote::ws::{Client, Ws, Wss};
use surrealdb::opt::auth::Root;
use tokio::sync::RwLock;
use tokio::time::{Duration, timeout};

/// SurrealDB WebSocket 连接配置。
#[derive(Clone, Debug)]
pub struct ConnectionConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub namespace: Option<String>,
    pub database: Option<String>,
    pub secure: bool,
}

impl ConnectionConfig {
    pub fn address(&self) -> String {
        format!("{}:{}", self.host.trim(), self.port)
    }
}

/// SurrealDB 连接的轻量封装，避免上层直接接触 SurrealDB 泛型类型。
#[derive(Clone)]
pub struct ConnectionHandle {
    inner: Arc<Surreal<Client>>,
}

impl ConnectionHandle {
    fn new(db: Surreal<Client>) -> Self {
        Self {
            inner: Arc::new(db),
        }
    }

    /// 在需要内部操作时获取连接实例。
    pub fn inner(&self) -> Arc<Surreal<Client>> {
        self.inner.clone()
    }
}

/// 针对部署站点的全局连接池。
#[derive(Clone)]
pub struct DeploymentConnectionPool {
    inner: Arc<RwLock<HashMap<String, Arc<ConnectionHandle>>>>,
}

impl DeploymentConnectionPool {
    fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn global() -> &'static DeploymentConnectionPool {
        static POOL: Lazy<DeploymentConnectionPool> = Lazy::new(DeploymentConnectionPool::new);
        &POOL
    }

    pub async fn get_or_create(
        &self,
        deployment_id: &str,
        config: &ConnectionConfig,
    ) -> Result<Arc<ConnectionHandle>> {
        let mut connections = self.inner.write().await;
        if let Some(existing) = connections.get(deployment_id) {
            return Ok(existing.clone());
        }

        let handle = Arc::new(connect_with_config(config).await?);
        connections.insert(deployment_id.to_string(), handle.clone());
        Ok(handle)
    }

    pub async fn insert(&self, deployment_id: String, handle: Arc<ConnectionHandle>) {
        self.inner.write().await.insert(deployment_id, handle);
    }

    pub async fn remove(&self, deployment_id: &str) -> Option<Arc<ConnectionHandle>> {
        self.inner.write().await.remove(deployment_id)
    }

    pub async fn clear(&self) {
        self.inner.write().await.clear();
    }
}

/// 使用配置初始化 SurrealDB 连接。
pub async fn connect_with_config(config: &ConnectionConfig) -> Result<ConnectionHandle> {
    let address = config.address();
    let db = if config.secure {
        Surreal::new::<Wss>(address.as_str()).await?
    } else {
        Surreal::new::<Ws>(address.as_str()).await?
    };

    if !config.username.is_empty() || !config.password.is_empty() {
        db.signin(Root {
            username: config.username.clone(),
            password: config.password.clone(),
        })
        .await?;
    }

    if let (Some(ns), Some(dbname)) = (config.namespace.as_ref(), config.database.as_ref()) {
        db.use_ns(ns).use_db(dbname).await?;
    }

    Ok(ConnectionHandle::new(db))
}

/// 验证 SurrealDB 连接是否可用（假设 TCP 端口可达）。
pub async fn verify_connection(config: &ConnectionConfig) -> Result<()> {
    let address = config.address();
    let transport = if config.secure { "wss" } else { "ws" };

    let db = match timeout(
        Duration::from_secs(3),
        if config.secure {
            Surreal::new::<Wss>(address.as_str())
        } else {
            Surreal::new::<Ws>(address.as_str())
        },
    )
    .await
    {
        Ok(Ok(db)) => db,
        Ok(Err(e)) => return Err(anyhow!("建立 {}://{} 连接失败: {}", transport, address, e)),
        Err(_) => return Err(anyhow!("建立 {}://{} 连接超时", transport, address)),
    };

    if !config.username.is_empty() || !config.password.is_empty() {
        match timeout(
            Duration::from_secs(3),
            db.signin(Root {
                username: config.username.clone(),
                password: config.password.clone(),
            }),
        )
        .await
        {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => return Err(anyhow!("认证失败: {}", e)),
            Err(_) => return Err(anyhow!("认证超时")),
        }
    }

    if let (Some(ns), Some(dbname)) = (config.namespace.as_ref(), config.database.as_ref()) {
        if let Err(e) = db.use_ns(ns).use_db(dbname).await {
            return Err(anyhow!(
                "切换命名空间/数据库失败 (ns={}, db={}): {}",
                ns,
                dbname,
                e
            ));
        }
    }

    if let Err(e) = db.query("RETURN 'ok'").await {
        return Err(anyhow!("测试查询失败: {}", e));
    }

    Ok(())
}

/// 创建 Web UI 运行所需的基本表结构。
pub async fn create_required_tables(
    host: &str,
    port: u16,
    username: &str,
    password: &str,
    namespace: Option<&str>,
    database: Option<&str>,
) -> Result<(), String> {
    let config = ConnectionConfig {
        host: host.to_string(),
        port,
        username: username.to_string(),
        password: password.to_string(),
        namespace: namespace
            .map(|s| s.to_string())
            .or_else(|| Some("1516".into())),
        database: database
            .map(|s| s.to_string())
            .or_else(|| Some("AvevaMarineSample".into())),
        secure: false,
    };

    let handle = connect_with_config(&config)
        .await
        .map_err(|e| format!("连接数据库失败: {}", e))?;

    let sql = r#"
        DEFINE TABLE IF NOT EXISTS dbnum_info_table SCHEMAFULL;
        DEFINE FIELD IF NOT EXISTS dbnum ON dbnum_info_table TYPE int;
        DEFINE FIELD IF NOT EXISTS db_type ON dbnum_info_table TYPE string;
        DEFINE FIELD IF NOT EXISTS file_name ON dbnum_info_table TYPE string;
        DEFINE FIELD IF NOT EXISTS count ON dbnum_info_table TYPE int DEFAULT 0;

        DEFINE TABLE IF NOT EXISTS sync_account_table SCHEMAFULL;
        DEFINE FIELD IF NOT EXISTS username ON sync_account_table TYPE string;
        DEFINE FIELD IF NOT EXISTS password ON sync_account_table TYPE string;
        DEFINE FIELD IF NOT EXISTS role ON sync_account_table TYPE string DEFAULT 'user';
        DEFINE FIELD IF NOT EXISTS sync_strategy ON sync_account_table TYPE string DEFAULT 'incremental';
        DEFINE FIELD IF NOT EXISTS last_sync_time ON sync_account_table TYPE datetime DEFAULT time::now();
    "#;

    handle
        .inner()
        .query(sql)
        .await
        .map_err(|e| format!("初始化表结构失败: {}", e))?;

    Ok(())
}

/// 测试数据库连接（用于 Web UI 配置测试）。
pub async fn test_database_connection(config: &ConnectionConfig) -> Result<()> {
    verify_connection(config).await
}
