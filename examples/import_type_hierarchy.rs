use std::path::PathBuf;

use aios_core::rs_surreal::type_hierarchy::{generate_and_apply, load_report_from_path};
use aios_core::rs_surreal::{SUL_DB, connect_surdb};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about = "将层级报告导入 SurrealDB")]
struct Args {
    /// SurrealDB 连接字符串，例如 ws://127.0.0.1:8000
    #[arg(long, default_value = "ws://127.0.0.1:8000")]
    endpoint: String,
    /// SurrealDB 命名空间
    #[arg(long, default_value = "app")]
    namespace: String,
    /// SurrealDB 数据库名
    #[arg(long, default_value = "app")]
    database: String,
    /// 登录用户名
    #[arg(long, default_value = "root")]
    username: String,
    /// 登录密码
    #[arg(long, default_value = "root")]
    password: String,
    /// 层级 JSON 文件路径
    #[arg(long)]
    path: PathBuf,
    /// 数据来源标签，用于后续清理
    #[arg(long, default_value = "hierarchy_report")]
    source: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    connect_surdb(
        &args.endpoint,
        &args.namespace,
        &args.database,
        &args.username,
        &args.password,
    )
    .await?;

    let report = load_report_from_path(&args.path)?;
    let scripts = generate_and_apply(&*SUL_DB, &report, &args.source).await?;

    println!(
        "导入完成: 节点 {} 条，边 {} 条，闭包 {} 条",
        scripts.node_statements.len(),
        scripts.edge_statements.len(),
        scripts.path_statements.len()
    );

    Ok(())
}
