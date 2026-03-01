use crate::init_surreal;
use crate::options::{DbOption, ModelWriteMode};
use crate::rs_surreal::SUL_DB;
use anyhow::Result;
use std::sync::Mutex;
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

    // 打印配置信息
    let config_file_name =
        std::env::var("DB_OPTION_FILE").unwrap_or_else(|_| "db_options/DbOption".to_string());
    println!("📄 使用配置文件: {}.toml", config_file_name);
    println!("🌐 连接服务器: {}", db_option.get_version_db_conn_str());
    println!("🏷️  命名空间: {}", db_option.surreal_ns);
    println!("💾 数据库名: {}", db_option.project_name);
    println!("👤 用户名: {}", db_option.v_user);

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
/// 根据 `db_option.surreal_backend` 运行时选择连接方式：
/// - `"ws"` (默认): 使用 WebSocket 连接远程 SurrealDB
/// - `"rocksdb"`: 使用 RocksDB 嵌入式后端（本地文件，无需认证）
///
/// 此函数还会初始化 SurrealDB 通用函数定义
pub async fn initialize_databases(db_option: &DbOption) -> Result<()> {
    log::error!("[DEBUG] initialize_databases called");
    let backend = db_option.surreal_backend.as_str();
    log::error!("[DEBUG] backend = {}", backend);

    match backend {
        "rocksdb" => {
            let path = db_option
                .surreal_local_path
                .as_deref()
                .unwrap_or("data.rdb");
            println!("🗄️  初始化本地 RocksDB 嵌入式...");
            println!("📂 数据目录: {}", path);
            let conn_str = format!("rocksdb://{}", path);
            let config = surrealdb::opt::Config::default().ast_payload();
            SUL_DB
                .connect((&conn_str, config))
                .with_capacity(1000)
                .await
                .map_err(|e| anyhow::anyhow!("RocksDB 连接失败: {}", e))?;
            // 嵌入式无需 signin
            crate::use_ns_db_compat(&SUL_DB, &db_option.surreal_ns, &db_option.project_name)
                .await
                .map_err(|e| anyhow::anyhow!("use ns/db 失败: {}", e))?;
            println!(
                "✅ RocksDB 嵌入式连接成功: {} -> {}",
                path, db_option.project_name
            );
        }
        _ => {
            // WS 模式（默认）
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
                }
            }

            #[cfg(feature = "mem-kv-save")]
            {
                use crate::init_mem_db_with_retry;
                if let Err(e) = init_mem_db_with_retry(db_option).await {
                    eprintln!("❌ 内存KV数据库连接失败: {}", e);
                    eprintln!("   请检查内存KV数据库服务是否运行");
                }
            }
        }
    }

    // 4. 初始化 SurrealDB 通用函数定义 (使用 None 从配置文件自动读取路径)
    if let Err(e) = crate::function::define_common_functions(None).await {
        eprintln!("初始化通用函数失败: {} (忽略并继续)", e);
    }

    // 5. 初始化模型写入路由（SurrealOnly / Dual / KvOnly）与模型 KV(WS)
    let requested_mode = crate::rs_surreal::resolve_model_write_mode(db_option);
    let kv_conn_str = normalized_model_kv_conn_str(db_option);

    // DEBUG: 强制输出模式信息
    log::error!("[DEBUG] requested_mode = {:?}, kv_conn_str = {}", requested_mode, kv_conn_str);

    validate_model_write_requirements(requested_mode, &kv_conn_str)?;

    if matches!(requested_mode, ModelWriteMode::Dual | ModelWriteMode::KvOnly) {
        println!("🗄️ 初始化模型 KV（WebSocket）...");

        // 尝试连接 KV
        let connect_result = crate::rs_surreal::connect_model_kv(
            &kv_conn_str,
            &db_option.surreal_ns,
            &db_option.project_name,
            db_option.get_model_kv_user(),
            db_option.get_model_kv_password(),
        )
        .await;

        match connect_result {
            Ok(_) => println!("✅ 模型 KV 就绪: {}", kv_conn_str),
            Err(e) => {
                // 连接失败，尝试自动启动 SurrealKV
                eprintln!("⚠️  模型 KV 连接失败: {}", e);
                eprintln!("🚀 尝试自动启动 SurrealKV 服务...");

                if let Err(start_err) = start_surreal_kv_server(db_option) {
                    if requested_mode == ModelWriteMode::KvOnly {
                        return Err(anyhow::anyhow!(
                            "model_write_mode=kv_only 但模型 KV 启动失败: {}",
                            start_err
                        ));
                    }
                    eprintln!("❌ 模型 KV 启动失败: {}（退回 SurrealDB 单写）", start_err);
                } else {
                    // 启动成功，重新尝试连接
                    match crate::rs_surreal::connect_model_kv(
                        &kv_conn_str,
                        &db_option.surreal_ns,
                        &db_option.project_name,
                        db_option.get_model_kv_user(),
                        db_option.get_model_kv_password(),
                    )
                    .await
                    {
                        Ok(_) => println!("✅ 模型 KV 就绪: {}", kv_conn_str),
                        Err(e2) => {
                            if requested_mode == ModelWriteMode::KvOnly {
                                return Err(anyhow::anyhow!(
                                    "model_write_mode=kv_only 但模型 KV 连接失败: {}",
                                    e2
                                ));
                            }
                            eprintln!("❌ 模型 KV 连接失败: {}（退回 SurrealDB 单写）", e2);
                        }
                    }
                }
            }
        }
    }

    crate::rs_surreal::set_model_write_mode(requested_mode);
    println!(
        "🧭 模型写入模式: {} (kv_enabled={})",
        requested_mode.as_str(),
        crate::rs_surreal::is_model_kv_enabled()
    );

    Ok(())
}

/// 独立的模型 KV 连接初始化。
///
/// 根据当前运行时 `model_write_mode` 判断是否需要连接 KV_DB：
/// - `Dual` / `KvOnly` → 尝试连接，失败则自动启动 SurrealKV 服务后重试。
/// - `SurrealOnly` → 跳过。
///
/// 此函数可在 `init_surreal()` 之后单独调用，用于补充 KV 连接。
pub async fn ensure_model_kv_connected(db_option: &DbOption) -> Result<()> {
    let mode = crate::rs_surreal::current_model_write_mode();
    if !matches!(mode, ModelWriteMode::Dual | ModelWriteMode::KvOnly) {
        return Ok(());
    }

    // 如果 KV 已经连接成功，跳过
    if crate::rs_surreal::is_model_kv_enabled() {
        return Ok(());
    }

    let kv_conn_str = normalized_model_kv_conn_str(db_option);
    validate_model_write_requirements(mode, &kv_conn_str)?;

    println!("🗄️ 初始化模型 KV（WebSocket）...");

    let connect_result = crate::rs_surreal::connect_model_kv(
        &kv_conn_str,
        &db_option.surreal_ns,
        &db_option.project_name,
        db_option.get_model_kv_user(),
        db_option.get_model_kv_password(),
    )
    .await;

    match connect_result {
        Ok(_) => {
            println!("✅ 模型 KV 就绪: {}", kv_conn_str);
        }
        Err(e) => {
            eprintln!("⚠️  模型 KV 连接失败: {}", e);
            eprintln!("🚀 尝试自动启动 SurrealKV 服务...");

            if let Err(start_err) = start_surreal_kv_server(db_option) {
                if mode == ModelWriteMode::KvOnly {
                    return Err(anyhow::anyhow!(
                        "model_write_mode=kv_only 但模型 KV 启动失败: {}",
                        start_err
                    ));
                }
                eprintln!("❌ 模型 KV 启动失败: {}（退回 SurrealDB 单写）", start_err);
            } else {
                match crate::rs_surreal::connect_model_kv(
                    &kv_conn_str,
                    &db_option.surreal_ns,
                    &db_option.project_name,
                    db_option.get_model_kv_user(),
                    db_option.get_model_kv_password(),
                )
                .await
                {
                    Ok(_) => println!("✅ 模型 KV 就绪: {}", kv_conn_str),
                    Err(e2) => {
                        if mode == ModelWriteMode::KvOnly {
                            return Err(anyhow::anyhow!(
                                "model_write_mode=kv_only 但模型 KV 连接失败: {}",
                                e2
                            ));
                        }
                        eprintln!("❌ 模型 KV 连接失败: {}（退回 SurrealDB 单写）", e2);
                    }
                }
            }
        }
    }

    // KV 连接成功后，在 KV_DB 上也定义通用函数（fn::find_ancestor_type 等）
    if crate::rs_surreal::is_model_kv_enabled() {
        println!("📦 在 KV_DB 上定义通用函数...");
        if let Err(e) =
            crate::function::define_common_functions_on_db(&crate::rs_surreal::KV_DB, None).await
        {
            eprintln!("⚠️  KV_DB 通用函数定义失败: {}（写入可能受影响）", e);
        }

        // 模型写入 SQL 中 RELATION INSERT 引用了 pe 表（in: pe:...）,
        // KV_DB 上不存在 pe 表会导致 "The table 'pe' does not exist" 错误。
        // 这里预定义 pe 表（SCHEMALESS），使 RELATION 引用不报错。
        // pe 实际数据仍在 SUL_DB，此处仅为 schema 占位。
        println!("📦 在 KV_DB 上预定义模型写入依赖的基础表...");
        let schema_sql = "DEFINE TABLE IF NOT EXISTS pe SCHEMALESS PERMISSIONS FULL;";
        if let Err(e) = crate::rs_surreal::KV_DB.query(schema_sql).await {
            eprintln!("⚠️  KV_DB 基础表定义失败: {}", e);
        }
    }

    println!(
        "🧭 模型写入模式: {} (kv_enabled={})",
        mode.as_str(),
        crate::rs_surreal::is_model_kv_enabled()
    );
    Ok(())
}

#[inline]
fn normalized_model_kv_conn_str(db_option: &DbOption) -> String {
    db_option.get_model_kv_conn_str().trim().to_string()
}

#[inline]
fn validate_model_write_requirements(mode: ModelWriteMode, kv_conn_str: &str) -> Result<()> {
    let is_ws_endpoint = kv_conn_str.starts_with("ws://") || kv_conn_str.starts_with("wss://");
    if mode == ModelWriteMode::KvOnly && !is_ws_endpoint {
        return Err(anyhow::anyhow!(
            "model_write_mode=kv_only 但模型 KV 连接地址非法: {}",
            kv_conn_str
        ));
    }
    Ok(())
}

// ============================================================================
// SurrealDB 服务进程管理
// ============================================================================

static SURREAL_PROCESS: Mutex<Option<std::process::Child>> = Mutex::new(None);
static SURREAL_KV_PROCESS: Mutex<Option<std::process::Child>> = Mutex::new(None);

/// 根据 DbOption 配置启动 SurrealDB 服务进程。
///
/// 使用 `surreal_local_path` 作为 RocksDB 数据目录，
/// `v_port` 作为绑定端口，`v_user` / `v_password` 作为认证。
/// 启动前会自动清理占用目标端口的进程。
pub fn start_surreal_server(db_option: &DbOption) -> Result<()> {
    let port = db_option.v_port;
    let path = db_option
        .surreal_local_path
        .as_deref()
        .unwrap_or("data.rdb");
    let user = &db_option.v_user;
    let password = &db_option.v_password;

    // 先停掉旧进程（如果有）
    stop_surreal_server_inner();

    // 清理占用端口的进程（Windows）
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("powershell")
            .args([
                "-Command",
                &format!(
                    "Get-NetTCPConnection -LocalPort {} -ErrorAction SilentlyContinue | ForEach-Object {{ Stop-Process -Id $_.OwningProcess -Force -ErrorAction SilentlyContinue }}",
                    port
                ),
            ])
            .output();
    }

    // 清理占用端口的进程（Linux/macOS）
    #[cfg(not(target_os = "windows"))]
    {
        let _ = std::process::Command::new("sh")
            .args([
                "-c",
                &format!("lsof -ti:{} | xargs -r kill -9 2>/dev/null || true", port),
            ])
            .output();
    }

    let rocksdb_url = format!("rocksdb://{}", path);
    let bind_addr = format!("0.0.0.0:{}", port);

    println!("🚀 启动 SurrealDB 服务...");
    println!("   端口: {}", port);
    println!("   数据: {}", path);

    // 设置 RocksDB 性能优化环境变量
    let cpu = num_cpus::get();
    let envs = vec![
        ("SURREAL_SYNC_DATA", "false".to_string()),
        ("SURREAL_ROCKSDB_THREAD_COUNT", std::cmp::min(cpu, 16).to_string()),
        ("SURREAL_ROCKSDB_JOBS_COUNT", std::cmp::min(cpu * 2, 32).to_string()),
        ("SURREAL_ROCKSDB_MAX_CONCURRENT_SUBCOMPACTIONS",
            if cpu >= 16 { "8" } else { "4" }.to_string()),
        ("SURREAL_ROCKSDB_MAX_OPEN_FILES", "4096".to_string()),
        ("SURREAL_ROCKSDB_BLOCK_CACHE_SIZE", "16GB".to_string()),
        ("SURREAL_ROCKSDB_WRITE_BUFFER_SIZE", "256MB".to_string()),
        ("SURREAL_ROCKSDB_MAX_WRITE_BUFFER_NUMBER", "8".to_string()),
        ("SURREAL_ROCKSDB_MIN_WRITE_BUFFER_NUMBER_TO_MERGE", "2".to_string()),
        ("SURREAL_ROCKSDB_TARGET_FILE_SIZE_BASE", "256MB".to_string()),
        ("SURREAL_ROCKSDB_TARGET_FILE_SIZE_MULTIPLIER", "2".to_string()),
        ("SURREAL_ROCKSDB_FILE_COMPACTION_TRIGGER", "4".to_string()),
        ("SURREAL_ROCKSDB_STORAGE_LOG_LEVEL", "warn".to_string()),
        ("SURREAL_ROCKSDB_BLOB_COMPRESSION_TYPE", "lz4".to_string()),
    ];

    let mut cmd = std::process::Command::new("surreal");
    cmd.args(["start", "--user", user, "--pass", password, "--bind", &bind_addr, &rocksdb_url]);
    for (k, v) in &envs {
        cmd.env(k, v);
    }
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let child = cmd.spawn().map_err(|e| {
        anyhow::anyhow!("启动 surreal 进程失败（请确认 surreal 在 PATH 中）: {}", e)
    })?;

    let pid = child.id();
    *SURREAL_PROCESS.lock().unwrap() = Some(child);

    // 等待服务就绪
    std::thread::sleep(Duration::from_secs(2));
    println!("✅ SurrealDB 服务已启动 (PID: {})", pid);
    Ok(())
}

/// 停止由 `start_surreal_server` 启动的 SurrealDB 服务进程。
pub fn stop_surreal_server() {
    stop_surreal_server_inner();
}

fn stop_surreal_server_inner() {
    if let Ok(mut guard) = SURREAL_PROCESS.lock() {
        if let Some(ref mut child) = *guard {
            let pid = child.id();
            println!("🛑 停止 SurrealDB 服务 (PID: {})...", pid);
            let _ = child.kill();
            let _ = child.wait();
            println!("✅ SurrealDB 服务已停止");
        }
        *guard = None;
    }
}

/// 检查 SurrealDB 服务进程是否在运行。
pub fn is_surreal_server_running() -> bool {
    if let Ok(guard) = SURREAL_PROCESS.lock() {
        guard.is_some()
    } else {
        false
    }
}

// ============================================================================
// SurrealKV 服务进程管理
// ============================================================================

/// 自动启动 SurrealKV 服务进程。
///
/// 使用 `surrealkv://` 后端（本地嵌入式 KV），绑定到 `kv_port`（默认 8010）。
/// 数据存储在 `<surreal_local_path>.kv/` 目录中。
pub fn start_surreal_kv_server(db_option: &DbOption) -> Result<()> {
    let port = db_option.kv_port.trim();
    let port: u16 = if port.is_empty() {
        8010
    } else {
        port.parse().unwrap_or(8010)
    };

    let base_path = db_option
        .surreal_local_path
        .as_deref()
        .unwrap_or("data.rdb");
    let kv_data_path = format!("{}.kv", base_path);
    let kv_url = format!("surrealkv://{}", kv_data_path);
    let bind_addr = format!("0.0.0.0:{}", port);

    let user = db_option.get_model_kv_user();
    let password = db_option.get_model_kv_password();

    // 先停掉旧的 KV 进程（如果有）
    stop_surreal_kv_server_inner();

    // 清理占用端口的进程（Windows）
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("powershell")
            .args([
                "-Command",
                &format!(
                    "Get-NetTCPConnection -LocalPort {} -ErrorAction SilentlyContinue | ForEach-Object {{ Stop-Process -Id $_.OwningProcess -Force -ErrorAction SilentlyContinue }}",
                    port
                ),
            ])
            .output();
    }

    // 清理占用端口的进程（Linux/macOS）
    #[cfg(not(target_os = "windows"))]
    {
        let _ = std::process::Command::new("sh")
            .args([
                "-c",
                &format!("lsof -ti:{} | xargs -r kill -9 2>/dev/null || true", port),
            ])
            .output();
    }

    println!("🚀 自动启动 SurrealKV 服务...");
    println!("   端口: {}", port);
    println!("   数据: {}", kv_data_path);

    let mut cmd = std::process::Command::new("surreal");
    cmd.args(["start", "--user", user, "--pass", password, "--bind", &bind_addr, &kv_url]);
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let child = cmd.spawn().map_err(|e| {
        anyhow::anyhow!("启动 SurrealKV 进程失败（请确认 surreal 在 PATH 中）: {}", e)
    })?;

    let pid = child.id();
    *SURREAL_KV_PROCESS.lock().unwrap() = Some(child);

    // 等待服务就绪
    std::thread::sleep(Duration::from_secs(3));
    println!("✅ SurrealKV 服务已启动 (PID: {})", pid);
    Ok(())
}

/// 停止由 `start_surreal_kv_server` 启动的 SurrealKV 服务进程。
pub fn stop_surreal_kv_server() {
    stop_surreal_kv_server_inner();
}

fn stop_surreal_kv_server_inner() {
    if let Ok(mut guard) = SURREAL_KV_PROCESS.lock() {
        if let Some(ref mut child) = *guard {
            let pid = child.id();
            println!("🛑 停止 SurrealKV 服务 (PID: {})...", pid);
            let _ = child.kill();
            let _ = child.wait();
            println!("✅ SurrealKV 服务已停止");
        }
        *guard = None;
    }
}

#[cfg(test)]
mod tests {
    use super::{normalized_model_kv_conn_str, validate_model_write_requirements};
    use crate::options::{DbOption, ModelWriteMode};

    #[test]
    fn kv_only_requires_ws_endpoint() {
        let err = validate_model_write_requirements(ModelWriteMode::KvOnly, "http://127.0.0.1:8010")
            .expect_err("kv_only 在非 ws 地址时必须报错");
        assert!(err.to_string().contains("kv_only"));
    }

    #[test]
    fn dual_allows_non_ws_endpoint() {
        validate_model_write_requirements(ModelWriteMode::Dual, "http://127.0.0.1:8010")
            .expect("dual 在连接失败时可回退 SurrealDB 单写");
    }

    #[test]
    fn normalized_model_kv_conn_str_uses_kv_config() {
        let mut opt = DbOption::default();
        opt.kv_ip = "localhost".to_string();
        opt.kv_port = "8010".to_string();
        assert_eq!(normalized_model_kv_conn_str(&opt), "ws://127.0.0.1:8010");
    }
}
