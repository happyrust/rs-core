use crate::init_surreal;
use crate::options::DbOption;
use crate::rs_surreal::SUL_DB;
use anyhow::Result;
use std::time::Duration;
use tokio::time::sleep;

/// 为 `DbOption` 提供针对 SurrealDB 连接的校验与摘要功能。
pub trait DbOptionSurrealExt {
    fn validate_connection_config(&self) -> Result<(), String>;
    fn connection_summary(&self) -> String;
}

impl DbOptionSurrealExt for DbOption {
    fn validate_connection_config(&self) -> Result<(), String> {
        if self.v_ip.is_empty() {
            return Err("数据库IP不能为空".to_string());
        }

        if self.v_port == 0 {
            return Err("数据库端口不能为0".to_string());
        }

        if self.v_user.is_empty() {
            return Err("数据库用户名不能为空".to_string());
        }

        if self.project_name.is_empty() {
            return Err("项目名称不能为空".to_string());
        }

        Ok(())
    }

    fn connection_summary(&self) -> String {
        format!(
            "host: {}:{} | user: {} | ns: {} | db: {}",
            self.v_ip, self.v_port, self.v_user, self.surreal_ns, self.project_name
        )
    }
}

/// 在启用 `local` 特性时，使用 RocksDB 后端连接本地 SurrealDB。
pub async fn connect_local_rocksdb(project_name: &str) -> Result<()> {
    let config = surrealdb::opt::Config::default().ast_payload();
    SUL_DB
        .connect((format!("rocksdb://{}.rdb", project_name), config))
        .with_capacity(1000)
        .await?;
    Ok(())
}

/// 改进的 SurrealDB 连接初始化流程，包含自动重试与错误诊断。
pub async fn init_surreal_with_retry(db_option: &DbOption) -> Result<()> {
    db_option
        .validate_connection_config()
        .map_err(|e| anyhow::anyhow!("配置验证失败: {}", e))?;

    let max_retries = 3;
    let mut last_error = None;

    for attempt in 1..=max_retries {
        println!("🔄 数据库连接尝试 {}/{}", attempt, max_retries);

        match try_connect_database().await {
            Ok(_) => {
                println!("✅ 数据库连接成功");
                return Ok(());
            }
            Err(e) => {
                let error_msg = e.to_string();
                last_error = Some(anyhow::anyhow!("{}", error_msg));
                eprintln!("❌ 连接尝试 {} 失败: {}", attempt, error_msg);

                if attempt < max_retries {
                    let wait_time = attempt * 2;
                    println!("⏳ {}秒后重试...", wait_time);
                    sleep(Duration::from_secs(wait_time as u64)).await;
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("连接失败")))
}

/// 尝试进行一次完整的 SurrealDB 初始化与可用性校验。
pub async fn try_connect_database() -> Result<()> {
    println!("使用 aios_core::init_surreal 初始化数据库...");
    match init_surreal().await {
        Ok(_) => {
            println!("✓ 数据库初始化完成");
        }
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("Already connected") {
                println!("⚠️ 已经连接，跳过重复初始化");
            } else {
                return Err(anyhow::anyhow!("数据库初始化失败: {}", msg));
            }
        }
    }

    SUL_DB
        .query("RETURN 1;")
        .await
        .map_err(|e| anyhow::anyhow!("测试查询失败: {}", e))?;

    println!("✓ 功能测试通过");
    Ok(())
}

/// 统一的数据库初始化入口，包含所有数据库连接和函数定义
///
/// 根据编译特性自动选择合适的初始化方式：
/// - `local` 特性: 使用 RocksDB 后端
/// - `ws` 特性: 使用 WebSocket 连接远程 SurrealDB
/// - `mem-kv-save` 特性: 额外初始化内存 KV 数据库
///
/// 此函数还会初始化 SurrealDB 通用函数定义
pub async fn initialize_databases(db_option: &DbOption) -> Result<()> {
    // 1. 初始化本地 RocksDB（如果启用 local 特性）
    #[cfg(feature = "local")]
    {
        println!("初始化本地 RocksDB...");
        connect_local_rocksdb(&db_option.project_name).await?;
    }

    // 2. 初始化远程 SurrealDB（如果启用 ws 特性）
    #[cfg(not(feature = "local"))]
    {
        println!("数据库连接中...");
        match init_surreal_with_retry(db_option).await {
            Ok(_) => {
                println!(
                    "✅ 数据库连接成功: {} -> {}",
                    db_option.get_version_db_conn_str(),
                    db_option.project_name
                );
            }
            Err(e) => {
                eprintln!("❌ 数据库连接失败: {}", e);
                eprintln!("   配置信息: {}", db_option.connection_summary());
                eprintln!("   请检查 SurrealDB 服务是否运行，配置是否正确");
                // 不直接返回错误，让应用继续运行但标记数据库不可用
            }
        }

        // 3. 初始化内存 KV 数据库（如果启用 mem-kv-save 特性）
        #[cfg(feature = "mem-kv-save")]
        {
            use crate::init_mem_db_with_retry;
            if let Err(e) = init_mem_db_with_retry(db_option).await {
                eprintln!("❌ 内存KV数据库连接失败: {}", e);
                eprintln!("   请检查内存KV数据库服务是否运行");
            }
        }
    }

    // 4. 初始化 SurrealDB 通用函数定义 (使用 None 从配置文件自动读取路径)
    if let Err(e) =
        crate::function::define_common_functions(None).await
    {
        eprintln!("初始化通用函数失败: {} (忽略并继续)", e);
    }

    Ok(())
}
