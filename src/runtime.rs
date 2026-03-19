use crate::init_surreal;
use crate::options::{DbConnMode, DbOption};
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
        if self.surreal_ip.is_empty() {
            return Err("数据库IP不能为空".to_string());
        }

        if self.surreal_port == 0 {
            return Err("数据库端口不能为0".to_string());
        }

        if self.surreal_user.is_empty() {
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
            self.surreal_ip,
            self.surreal_port,
            self.surreal_user,
            self.surreal_ns,
            self.project_name
        )
    }
}

/// 检测是否为 RocksDB LOCK 相关错误（可自动重试）。
#[cfg(feature = "kv-rocksdb")]
fn is_rocksdb_lock_error(err: &impl std::fmt::Display) -> bool {
    let s = err.to_string();
    s.contains("LOCK") || s.contains("lock file") || s.contains("Resource temporarily unavailable")
}

/// 清理 RocksDB 残留 LOCK 文件。
///
/// - `force=true`：通过 `lsof` 找到持有 LOCK 的进程并 kill，然后删除 LOCK 文件。
/// - `force=false`：仅在没有 surreal 进程运行时才清理（原有逻辑）。
#[cfg(feature = "kv-rocksdb")]
pub fn cleanup_stale_rocksdb_lock(data_path: &str, force: bool) {
    let lock_path = std::path::Path::new(data_path).join("LOCK");
    if !lock_path.exists() {
        return;
    }

    if force {
        // --force 模式：用 lsof 精确查找持有 LOCK 文件的进程并 kill
        println!("   🔧 --force 模式：强制清理 LOCK 文件");
        #[cfg(unix)]
        {
            if let Ok(output) = std::process::Command::new("lsof")
                .arg(lock_path.to_str().unwrap_or_default())
                .output()
            {
                let stdout = String::from_utf8_lossy(&output.stdout);
                // lsof 输出格式：第二列为 PID（跳过首行标题）
                for line in stdout.lines().skip(1) {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if let Some(pid_str) = parts.get(1) {
                        if let Ok(pid) = pid_str.parse::<u32>() {
                            // 不要 kill 自己
                            if pid == std::process::id() {
                                continue;
                            }
                            println!("   🛑 终止占用 LOCK 的进程 PID={}", pid);
                            let _ = std::process::Command::new("kill")
                                .args(["-9", pid_str])
                                .output();
                        }
                    }
                }
            }
            // kill 后等待进程退出
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
        #[cfg(windows)]
        {
            // Windows 暂不支持 force kill，仅提示
            println!("   ⚠️  Windows 暂不支持 --force 自动终止占用进程，请手动关闭后重试");
        }
    } else {
        // 非 force 模式：检查是否有 surreal 相关进程
        #[cfg(unix)]
        let has_surreal_process = std::process::Command::new("pgrep")
            .arg("surreal")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        #[cfg(windows)]
        let has_surreal_process = std::process::Command::new("tasklist")
            .args(["/FI", "IMAGENAME eq surreal.exe", "/NH"])
            .output()
            .map(|o| {
                let out = String::from_utf8_lossy(&o.stdout);
                out.contains("surreal.exe")
            })
            .unwrap_or(false);

        if has_surreal_process {
            println!("   ⚠️  LOCK 文件存在且有 surreal 进程在运行，跳过清理");
            return;
        }
    }

    match std::fs::remove_file(&lock_path) {
        Ok(()) => println!("   🧹 已清理残留 LOCK 文件: {}", lock_path.display()),
        Err(e) => println!("   ⚠️  无法删除 LOCK 文件: {} ({})", lock_path.display(), e),
    }
}

/// 在启用 `kv-rocksdb` 特性时，使用 RocksDB 后端连接本地 SurrealDB。
#[cfg(feature = "kv-rocksdb")]
pub async fn connect_local_rocksdb(project_name: &str) -> Result<()> {
    let config = surrealdb::opt::Config::default().ast_payload();
    SUL_DB
        .connect((format!("rocksdb://db-data/{}.rdb", project_name), config))
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
    println!("👤 用户名: {}", db_option.surreal_user);

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
/// 根据 `DbOption` 中的 `[surrealdb]` 和 `[surrealkv]` 配置初始化两个数据库连接：
/// - SurrealDB（PE/属性/输入数据）：file 或 ws
/// - SurrealKV（模型数据写入）：file 或 ws，固定启用
pub async fn initialize_databases(db_option: &DbOption) -> Result<()> {
    // 修复 SurrealDB 3.x 图遍历在默认 planner 下可能返回空的问题
    unsafe { std::env::set_var("SURREAL_PLANNER_STRATEGY", "compute-only") };

    // 1. 初始化 SurrealDB（输入数据源）
    let sdb_cfg = db_option.effective_surrealdb();
    let sdb_conn_str = db_option.surrealdb_conn_str();

    match sdb_cfg.mode {
        DbConnMode::File => {
            #[cfg(feature = "kv-rocksdb")]
            {
                let path = db_option.surrealdb_data_path();
                println!("🗄️  初始化本地 RocksDB 嵌入式...");
                println!("📂 数据目录: {}", path);
                let config = surrealdb::opt::Config::default().ast_payload();
                let mut last_err: Option<String> = None;
                for attempt in 1..=2 {
                    let config = surrealdb::opt::Config::default().ast_payload();
                    match SUL_DB
                        .connect((&sdb_conn_str, config))
                        .with_capacity(1000)
                        .await
                    {
                        Ok(_) => {
                            last_err = None;
                            break;
                        }
                        Err(e) => {
                            let err_msg = e.to_string();
                            last_err = Some(err_msg.clone());
                            if err_msg.contains("Already connected") {
                                println!("⚠️  SUL_DB 已连接，跳过重复初始化");
                                last_err = None;
                                break;
                            } else if attempt == 1 && is_rocksdb_lock_error(&err_msg) {
                                let force = std::env::var("AIOS_FORCE_LOCK")
                                    .map(|v| v == "1")
                                    .unwrap_or(false);
                                println!("⚠️  检测到 RocksDB LOCK 冲突，清理残留锁后重试...");
                                cleanup_stale_rocksdb_lock(&path, force);
                                sleep(Duration::from_millis(500)).await;
                            } else {
                                return Err(anyhow::anyhow!("RocksDB 连接失败: {}", err_msg));
                            }
                        }
                    }
                }
                crate::use_ns_db_compat(&SUL_DB, &db_option.surreal_ns, &db_option.project_name)
                    .await
                    .map_err(|e| anyhow::anyhow!("use ns/db 失败: {}", e))?;
                println!(
                    "✅ RocksDB 嵌入式连接成功: {} -> {}",
                    path, db_option.project_name
                );
            }
            #[cfg(not(feature = "kv-rocksdb"))]
            {
                return Err(anyhow::anyhow!(
                    "DbConnMode::File (RocksDB 嵌入式) 需要启用 kv-rocksdb 特性。\
                    请使用 cargo build --features kv-rocksdb 重新构建。"
                ));
            }
        }
        DbConnMode::Ws => {
            println!("🗄️  初始化 SurrealDB（WebSocket）...");
            match init_surreal_with_retry(db_option).await {
                Ok(_) => {
                    println!(
                        "✅ SurrealDB 连接成功: {} -> {}",
                        sdb_conn_str, db_option.project_name
                    );
                }
                Err(e) => {
                    eprintln!("❌ SurrealDB 连接失败: {}", e);
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

    // 2. 初始化 SurrealDB 通用函数定义
    if let Err(e) = crate::function::define_common_functions(None).await {
        eprintln!("初始化通用函数失败: {} (忽略并继续)", e);
    }

    // 3. 初始化 SurrealKV（模型数据写入）
    let kv_cfg = db_option.effective_surrealkv();

    if !kv_cfg.enabled {
        // KV 未启用：模型数据写回主 SurrealDB（SUL_DB）
        println!("🗄️  SurrealKV 已禁用 (surrealkv.enabled=false)，模型数据写回主 SurrealDB");
    } else {
        let kv_conn_str = db_option.surrealkv_conn_str();
        println!("🗄️  初始化 SurrealKV（{}）...", kv_cfg.mode.as_str());

        match kv_cfg.mode {
            DbConnMode::File => {
                #[cfg(not(any(feature = "kv-rocksdb", feature = "kv-surrealkv")))]
                {
                    return Err(anyhow::anyhow!(
                        "SurrealKV DbConnMode::File 需要启用 kv-rocksdb 或 kv-surrealkv 特性。\
                        请使用 cargo build --features kv-rocksdb 重新构建。"
                    ));
                }
                #[cfg(any(feature = "kv-rocksdb", feature = "kv-surrealkv"))]
                {
                    let path = db_option.surrealkv_data_path();
                    println!("📂 KV 数据目录: {}", path);
                    let config = surrealdb::opt::Config::default().ast_payload();
                    for attempt in 1..=2 {
                        let config = surrealdb::opt::Config::default().ast_payload();
                        match crate::rs_surreal::KV_DB
                            .connect((&kv_conn_str, config))
                            .with_capacity(1000)
                            .await
                        {
                            Ok(_) => break,
                            Err(e) => {
                                let err_msg = e.to_string();
                                if attempt == 1 && is_rocksdb_lock_error(&err_msg) {
                                    let force = std::env::var("AIOS_FORCE_LOCK")
                                        .map(|v| v == "1")
                                        .unwrap_or(false);
                                    println!("⚠️  检测到 SurrealKV LOCK 冲突，清理残留锁后重试...");
                                    cleanup_stale_rocksdb_lock(&path, force);
                                    sleep(Duration::from_millis(500)).await;
                                } else {
                                    return Err(anyhow::anyhow!(
                                        "SurrealKV 嵌入式连接失败: {}",
                                        err_msg
                                    ));
                                }
                            }
                        }
                    }
                    crate::use_ns_db_compat(
                        &crate::rs_surreal::KV_DB,
                        &db_option.surreal_ns,
                        &db_option.project_name,
                    )
                    .await
                    .map_err(|e| anyhow::anyhow!("KV use ns/db 失败: {}", e))?;
                    crate::rs_surreal::mark_model_kv_enabled();
                    println!(
                        "✅ SurrealKV 嵌入式连接成功: {} -> {}",
                        path, db_option.project_name
                    );
                }
            }
            DbConnMode::Ws => {
                match crate::rs_surreal::connect_model_kv(
                    &kv_conn_str,
                    &db_option.surreal_ns,
                    &db_option.project_name,
                    &kv_cfg.user,
                    &kv_cfg.password,
                )
                .await
                {
                    Ok(_) => println!("✅ SurrealKV 就绪: {}", kv_conn_str),
                    Err(e) => {
                        return Err(anyhow::anyhow!(
                            "SurrealKV 连接失败: {}（请检查服务是否运行）",
                            e
                        ));
                    }
                }
            }
        }

        // 4. KV 连接成功后定义通用函数和基础表
        if crate::rs_surreal::is_model_kv_enabled() {
            println!("📦 在 KV_DB 上定义通用函数...");
            if let Err(e) =
                crate::function::define_common_functions_on_db(&crate::rs_surreal::KV_DB, None)
                    .await
            {
                eprintln!("⚠️  KV_DB 通用函数定义失败: {}（写入可能受影响）", e);
            }
            let schema_sql = "DEFINE TABLE IF NOT EXISTS pe SCHEMALESS PERMISSIONS FULL;";
            if let Err(e) = crate::rs_surreal::KV_DB.query(schema_sql).await {
                eprintln!("⚠️  KV_DB 基础表定义失败: {}", e);
            }
        }
    }

    println!(
        "🧭 数据库初始化完成 (surrealdb={}, surrealkv_enabled={}, kv_active={})",
        sdb_cfg.mode.as_str(),
        kv_cfg.enabled,
        crate::rs_surreal::is_model_kv_enabled()
    );

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
/// `surreal_port` 作为绑定端口，`surreal_user` / `surreal_password` 作为认证。
/// 启动前会自动清理占用目标端口的进程。
pub fn start_surreal_server(db_option: &DbOption) -> Result<()> {
    let port = db_option.surreal_port;
    let path_owned = db_option.surrealdb_data_path();
    let path = path_owned.as_str();
    let user = &db_option.surreal_user;
    let password = &db_option.surreal_password;

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
        (
            "SURREAL_ROCKSDB_THREAD_COUNT",
            std::cmp::min(cpu, 16).to_string(),
        ),
        (
            "SURREAL_ROCKSDB_JOBS_COUNT",
            std::cmp::min(cpu * 2, 32).to_string(),
        ),
        (
            "SURREAL_ROCKSDB_MAX_CONCURRENT_SUBCOMPACTIONS",
            if cpu >= 16 { "8" } else { "4" }.to_string(),
        ),
        ("SURREAL_ROCKSDB_MAX_OPEN_FILES", "4096".to_string()),
        ("SURREAL_ROCKSDB_BLOCK_CACHE_SIZE", "16GB".to_string()),
        ("SURREAL_ROCKSDB_WRITE_BUFFER_SIZE", "256MB".to_string()),
        ("SURREAL_ROCKSDB_MAX_WRITE_BUFFER_NUMBER", "8".to_string()),
        (
            "SURREAL_ROCKSDB_MIN_WRITE_BUFFER_NUMBER_TO_MERGE",
            "2".to_string(),
        ),
        ("SURREAL_ROCKSDB_TARGET_FILE_SIZE_BASE", "256MB".to_string()),
        (
            "SURREAL_ROCKSDB_TARGET_FILE_SIZE_MULTIPLIER",
            "2".to_string(),
        ),
        ("SURREAL_ROCKSDB_FILE_COMPACTION_TRIGGER", "4".to_string()),
        ("SURREAL_ROCKSDB_STORAGE_LOG_LEVEL", "warn".to_string()),
        ("SURREAL_ROCKSDB_BLOB_COMPRESSION_TYPE", "lz4".to_string()),
        ("SURREAL_PLANNER_STRATEGY", "compute-only".to_string()),
    ];

    let mut cmd = std::process::Command::new("surreal");
    cmd.args([
        "start",
        "--user",
        user,
        "--pass",
        password,
        "--bind",
        &bind_addr,
        &rocksdb_url,
    ]);
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
/// 使用 `surrealkv://` 后端（本地嵌入式 KV），绑定到 surrealkv.port。
pub fn start_surreal_kv_server(db_option: &DbOption) -> Result<()> {
    let port = db_option.surrealkv.port;

    let kv_data_path = db_option.surrealkv_data_path();
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
    cmd.args([
        "start", "--user", user, "--pass", password, "--bind", &bind_addr, &kv_url,
    ]);
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let child = cmd.spawn().map_err(|e| {
        anyhow::anyhow!(
            "启动 SurrealKV 进程失败（请确认 surreal 在 PATH 中）: {}",
            e
        )
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
    use crate::options::{DbConnMode, DbOption, SurrealKvConfig};

    #[test]
    fn effective_surrealkv_ws_conn_str() {
        let mut opt = DbOption::default();
        opt.surrealkv = SurrealKvConfig {
            mode: DbConnMode::Ws,
            ip: "localhost".to_string(),
            port: 8010,
            ..Default::default()
        };
        let kv_cfg = opt.effective_surrealkv();
        assert_eq!(kv_cfg.conn_str(), "ws://127.0.0.1:8010");
    }
}
