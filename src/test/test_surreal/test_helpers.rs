//! 测试辅助函数
//!
//! 提供内存数据库初始化等公共辅助功能

use crate::{SUL_DB, SurrealQueryExt};
use anyhow::Result;
use std::io::Read;
use surrealdb::Surreal;
use surrealdb::engine::local::{Db, Mem};

/// 为指定的数据库实例加载 SurrealDB 函数定义
///
/// 从指定目录加载所有 .surql 文件并在数据库中执行
async fn load_surreal_functions(db: &Surreal<Db>, script_dir: &str) -> Result<()> {
    let target_dir = std::fs::read_dir(script_dir)?
        .into_iter()
        .map(|entry| {
            let entry = entry.unwrap();
            entry.path()
        })
        .collect::<Vec<_>>();

    for file in target_dir {
        let file_name = file.file_name().unwrap().to_str().unwrap().to_string();
        println!("载入surreal {}", file_name);

        let mut file = std::fs::File::open(file)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        db.query(content).await?;
    }
    Ok(())
}

/// 创建新的内存数据库实例并初始化 SurrealDB 函数
///
/// 使用内存引擎（嵌入式模式），每次调用都会创建全新的独立数据库实例
///
/// # 特性
/// - 完全在内存中运行，不需要外部 SurrealDB 服务器
/// - 自动加载 resource/surreal 下的所有 SurrealDB 函数定义
/// - 测试结束后自动释放，无需手动 cleanup
/// - 支持完整的 SurrealQL 语法和函数
///
/// # 示例
/// ```no_run
/// let db = create_memory_db().await?;
/// db.query("CREATE person:test SET name = 'Alice'").await?;
/// ```
pub async fn create_memory_db() -> Result<Surreal<Db>> {
    // 创建嵌入式内存数据库连接
    // 参考：https://surrealdb.com/docs/surrealdb/embedding/rust
    let db = Surreal::new::<Mem>(())
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create embedded memory database: {}", e))?;

    // 设置命名空间和数据库
    db.use_ns("test")
        .use_db("test")
        .await
        .map_err(|e| anyhow::anyhow!("Failed to set namespace/database: {}", e))?;

    // 加载 SurrealDB 函数定义
    load_surreal_functions(&db, "resource/surreal")
        .await
        .map_err(|e| anyhow::anyhow!("Failed to load SurrealDB functions: {}", e))?;

    Ok(db)
}

/// 初始化全局 SUL_DB 为嵌入式内存数据库
///
/// 用于测试场景，将全局 SUL_DB 连接到嵌入式内存数据库而不是远程数据库服务器
///
/// # 特性
/// - 使用 SurrealDB 嵌入式模式（kv-mem 引擎）
/// - 自动加载 resource/surreal 下的所有 SurrealDB 函数定义
/// - 完全隔离，不影响真实数据库
/// - 性能更快，无网络延迟
///
/// # 示例
/// ```no_run
/// #[tokio::test]
/// async fn test_something() -> Result<()> {
///     init_sul_db_with_memory().await?;
///     // 现在可以使用全局 SUL_DB 进行测试
///     SUL_DB.query_response("SELECT * FROM pe").await?;
///     Ok(())
/// }
/// ```
pub async fn init_sul_db_with_memory() -> Result<()> {
    // 使用嵌入式内存引擎连接
    // SUL_DB 是 Surreal<Any> 类型，需要使用连接字符串
    // 参考：https://surrealdb.com/docs/surrealdb/embedding/rust#connect
    SUL_DB
        .connect("mem://")
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to embedded memory database: {}", e))?;

    // 设置命名空间和数据库
    SUL_DB
        .use_ns("test")
        .use_db("test")
        .await
        .map_err(|e| anyhow::anyhow!("Failed to set namespace/database: {}", e))?;

    // 加载 SurrealDB 函数定义（从 resource/surreal/*.surql 文件）
    crate::function::define_common_functions("resource/surreal")
        .await
        .map_err(|e| anyhow::anyhow!("Failed to define common functions: {}", e))?;

    Ok(())
}
