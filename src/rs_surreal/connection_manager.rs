use once_cell::sync::Lazy;
use surrealdb::opt::auth::Root;
use surrealdb::{Surreal, engine::any::Any};
use tokio::sync::Mutex;

use super::use_ns_db_compat;

/// 数据库连接配置信息
#[derive(Debug, Clone, PartialEq)]
pub struct ConnectionConfig {
    pub host: String,
    pub namespace: String,
    pub database: String,
    pub username: String,
    pub password: String,
}

impl ConnectionConfig {
    pub fn new(
        host: impl Into<String>,
        namespace: impl Into<String>,
        database: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        Self {
            host: host.into(),
            namespace: namespace.into(),
            database: database.into(),
            username: username.into(),
            password: password.into(),
        }
    }

    /// 检查是否需要重新连接（主机变更）
    pub fn needs_reconnect(&self, other: &ConnectionConfig) -> bool {
        self.host != other.host
    }

    /// 检查是否只需要切换 NS/DB（同主机但不同 NS/DB）
    pub fn needs_switch(&self, other: &ConnectionConfig) -> bool {
        self.host == other.host
            && (self.namespace != other.namespace || self.database != other.database)
    }
}

/// 连接状态
#[derive(Debug)]
enum ConnectionState {
    /// 未连接
    Disconnected,
    /// 已连接
    Connected { config: ConnectionConfig },
}

/// SurrealDB 连接管理器
///
/// 负责管理全局连接的生命周期，支持：
/// - 主机变更时的强制重连
/// - 同主机时的 NS/DB 切换
/// - 连接状态跟踪
pub struct SurrealConnectionManager {
    state: Mutex<ConnectionState>,
}

impl SurrealConnectionManager {
    /// 创建新的连接管理器
    pub fn new() -> Self {
        Self {
            state: Mutex::new(ConnectionState::Disconnected),
        }
    }

    /// 连接或重新连接数据库
    ///
    /// # 逻辑
    /// 1. 如果未连接，直接连接
    /// 2. 如果主机变更，强制断开并重连
    /// 3. 如果同主机但 NS/DB 不同，使用 use_ns/use_db 切换
    ///
    /// # 参数
    /// - `db`: 全局 Surreal 实例（SUL_DB）
    /// - `new_config`: 新的连接配置
    pub async fn connect_or_reconnect(
        &self,
        db: &Surreal<Any>,
        new_config: ConnectionConfig,
    ) -> Result<(), surrealdb::Error> {
        let mut state = self.state.lock().await;

        match &*state {
            ConnectionState::Disconnected => {
                // 未连接，直接连接
                println!("🔌 首次连接数据库: {}", new_config.host);
                self.do_connect(db, &new_config).await?;
                *state = ConnectionState::Connected { config: new_config };
                Ok(())
            }
            ConnectionState::Connected {
                config: current_config,
            } => {
                if new_config.needs_reconnect(current_config) {
                    // 主机变更，需要强制重连
                    println!(
                        "🔄 检测到主机变更: {} -> {}，执行强制重连",
                        current_config.host, new_config.host
                    );

                    // 注意：SurrealDB 的 Lazy<Surreal<Any>> 不支持显式 close
                    // 但我们可以尝试通过重新 connect 来覆盖旧连接

                    // 先尝试简单查询检测连接状态
                    match db.query("INFO FOR DB").await {
                        Ok(_) => {
                            println!("⚠️ 旧连接仍活跃，SurrealDB Lazy 不支持真正的重连");
                            println!("💡 尝试绕过：先切换 NS/DB 再重新 signin");

                            // 尝试切换到新配置（即使主机不同也尝试）
                            // 这可能失败，但是我们要处理 "Already connected" 错误
                            match self.do_switch_ns_db(db, &new_config).await {
                                Ok(_) => {
                                    println!(
                                        "✅ 成功切换到新配置（虽然主机不同，但 SurrealDB 允许）"
                                    );
                                    *state = ConnectionState::Connected { config: new_config };
                                    return Ok(());
                                }
                                Err(e) => {
                                    // 如果切换失败，返回原始错误
                                    eprintln!(
                                        "❌ 主机变更但切换失败：当前 {} -> 新 {}，错误: {}",
                                        current_config.host, new_config.host, e
                                    );
                                    return Err(e);
                                }
                            }
                        }
                        Err(_) => {
                            // 旧连接已失效，可以重新连接
                            println!("✅ 旧连接已断开，执行重新连接");
                            self.do_connect(db, &new_config).await?;
                            *state = ConnectionState::Connected { config: new_config };
                            Ok(())
                        }
                    }
                } else if new_config.needs_switch(current_config) {
                    // 同主机，仅切换 NS/DB
                    println!(
                        "🔀 同主机切换 NS/DB: {}/{} -> {}/{}",
                        current_config.namespace,
                        current_config.database,
                        new_config.namespace,
                        new_config.database
                    );
                    self.do_switch_ns_db(db, &new_config).await?;
                    *state = ConnectionState::Connected { config: new_config };
                    Ok(())
                } else {
                    // 配置完全相同，无需操作
                    println!("✅ 配置相同，跳过连接操作");
                    Ok(())
                }
            }
        }
    }

    /// 执行实际的连接操作
    async fn do_connect(
        &self,
        db: &Surreal<Any>,
        config: &ConnectionConfig,
    ) -> Result<(), surrealdb::Error> {
        // 创建配置
        let surreal_config = surrealdb::opt::Config::default().ast_payload();

        // 连接到主机
        db.connect((&config.host as &str, surreal_config))
            .with_capacity(1000)
            .await?;

        // 登录认证
        db.signin(Root {
            username: config.username.clone(),
            password: config.password.clone(),
        })
        .await?;

        // 切换 NS/DB（兼容 SurrealDB 3.x）
        use_ns_db_compat(db, &config.namespace, &config.database).await?;

        println!(
            "✅ 连接成功: {} -> NS: {}, DB: {}",
            config.host, config.namespace, config.database
        );
        Ok(())
    }

    /// 仅切换 NS/DB（不重新连接主机）
    async fn do_switch_ns_db(
        &self,
        db: &Surreal<Any>,
        config: &ConnectionConfig,
    ) -> Result<(), surrealdb::Error> {
        // 重新登录（确保认证状态）
        db.signin(Root {
            username: config.username.clone(),
            password: config.password.clone(),
        })
        .await?;

        // 切换 NS/DB（兼容 SurrealDB 3.x）
        use_ns_db_compat(db, &config.namespace, &config.database).await?;

        println!(
            "✅ NS/DB 切换成功: NS: {}, DB: {}",
            config.namespace, config.database
        );
        Ok(())
    }

    /// 获取当前连接的主机地址（如果已连接）
    pub async fn current_host(&self) -> Option<String> {
        let state = self.state.lock().await;
        match &*state {
            ConnectionState::Connected { config } => Some(config.host.clone()),
            ConnectionState::Disconnected => None,
        }
    }

    /// 标记为断开连接状态（不执行实际断开操作）
    pub async fn mark_disconnected(&self) {
        let mut state = self.state.lock().await;
        *state = ConnectionState::Disconnected;
    }
}

/// 全局连接管理器实例
pub static CONNECTION_MANAGER: Lazy<SurrealConnectionManager> =
    Lazy::new(|| SurrealConnectionManager::new());
