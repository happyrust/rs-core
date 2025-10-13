//! Kuzu 图模式定义
//!
//! 定义工厂设计数据的图模式结构，直接从 all_attr_info.json 生成

#[cfg(feature = "kuzu")]
use super::create_kuzu_connection;
#[cfg(feature = "kuzu")]
use super::json_schema::{generate_all_table_sqls, load_attr_info_json};
#[cfg(feature = "kuzu")]
use anyhow::{Context, Result};
#[cfg(feature = "kuzu")]
use kuzu::Connection;
#[cfg(feature = "kuzu")]
use std::collections::HashSet;

#[cfg(feature = "kuzu")]
/// 初始化 Kuzu 图模式
///
/// 直接从 all_attr_info.json 创建所有必要的节点表和关系表
pub async fn init_kuzu_schema() -> Result<()> {
    log::info!("正在初始化 Kuzu 图模式...");

    let conn = create_kuzu_connection()?;

    // 生成所有表的 SQL 语句
    let sqls = generate_all_table_sqls().context("生成表 SQL 失败")?;

    log::info!("准备创建 {} 个表", sqls.len());

    // 执行所有 SQL 语句
    let mut success_count = 0;
    let mut failed_tables = Vec::new();

    for sql in sqls {
        // 提取表名用于日志
        let table_name = extract_table_name(&sql);

        match conn.query(&sql) {
            Ok(_) => {
                log::debug!("创建表成功: {}", table_name);
                success_count += 1;
            }
            Err(e) => {
                // 对于已存在的表，这不是错误
                if e.to_string().contains("already exists") {
                    log::debug!("表已存在，跳过: {}", table_name);
                    success_count += 1;
                } else {
                    log::warn!("创建表失败 {}: {}", table_name, e);
                    failed_tables.push((table_name, e.to_string()));
                }
            }
        }
    }

    if !failed_tables.is_empty() {
        log::error!("以下表创建失败:");
        for (table, error) in &failed_tables {
            log::error!("  - {}: {}", table, error);
        }
    }

    log::info!(
        "Kuzu 图模式初始化完成: 成功创建/验证 {} 个表",
        success_count
    );

    // 确保没有遗留的事务 - 尝试回滚任何可能存在的事务
    // 如果没有活跃事务，这个操作会失败但不影响功能
    let _ = conn.query("ROLLBACK");

    Ok(())
}

#[cfg(feature = "kuzu")]
fn extract_table_name(sql: &str) -> String {
    // 从 SQL 语句中提取表名
    if let Some(pos) = sql.find("TABLE IF NOT EXISTS ") {
        let start = pos + "TABLE IF NOT EXISTS ".len();
        if let Some(end_pos) = sql[start..].find('(') {
            return sql[start..start + end_pos].trim().to_string();
        }
    } else if let Some(pos) = sql.find("TABLE ") {
        let start = pos + "TABLE ".len();
        if let Some(end_pos) = sql[start..].find('(') {
            return sql[start..start + end_pos].trim().to_string();
        }
    }

    "UNKNOWN".to_string()
}

#[cfg(feature = "kuzu")]
/// 获取当前数据库中的所有表
pub async fn list_tables() -> Result<Vec<String>> {
    let conn = create_kuzu_connection()?;

    // Kuzu 的系统表查询
    let mut result = conn.query("CALL table_info() RETURN name;")?;

    let mut tables = Vec::new();
    while let Some(row) = result.next() {
        if let Some(name_val) = row.get(0) {
            tables.push(name_val.to_string());
        }
    }

    Ok(tables)
}

#[cfg(feature = "kuzu")]
/// 验证模式是否正确创建
pub async fn validate_schema() -> Result<()> {
    log::info!("验证 Kuzu 模式...");

    let conn = create_kuzu_connection()?;
    let attr_info = load_attr_info_json()?;

    let mut missing_tables = Vec::new();
    let mut checked_tables = HashSet::new();

    // 检查 PE 主表
    if !table_exists(&conn, "PE")? {
        missing_tables.push("PE".to_string());
    }
    checked_tables.insert("PE".to_string());

    // 检查每个 noun 的属性表
    for noun in attr_info.named_attr_info_map.keys() {
        let table_name = format!("Attr_{}", noun.to_uppercase());

        if checked_tables.contains(&table_name) {
            continue;
        }

        if !table_exists(&conn, &table_name)? {
            missing_tables.push(table_name.clone());
        }
        checked_tables.insert(table_name);

        // 检查对应的关系表
        let rel_name = format!("TO_{}", noun.to_uppercase());
        if !table_exists(&conn, &rel_name)? {
            missing_tables.push(rel_name);
        }
    }

    if !missing_tables.is_empty() {
        log::warn!("缺少以下表: {:?}", missing_tables);
        return Err(anyhow::anyhow!(
            "模式验证失败，缺少 {} 个表",
            missing_tables.len()
        ));
    }

    log::info!("模式验证成功，所有必要的表都已创建");
    Ok(())
}

#[cfg(feature = "kuzu")]
fn table_exists(conn: &Connection, table_name: &str) -> Result<bool> {
    // 尝试查询表，如果表不存在会报错
    match conn.query(&format!(
        "MATCH (n:{}) RETURN COUNT(*) LIMIT 1;",
        table_name
    )) {
        Ok(_) => Ok(true),
        Err(e) => {
            if e.to_string().contains("does not exist") {
                Ok(false)
            } else {
                // 其他错误则传递
                Err(e.into())
            }
        }
    }
}

#[cfg(feature = "kuzu")]
/// 删除所有表（慎用！）
pub async fn drop_all_tables() -> Result<()> {
    log::warn!("正在删除所有 Kuzu 表...");

    let conn = create_kuzu_connection()?;
    let attr_info = load_attr_info_json()?;

    let mut dropped_count = 0;
    let mut failed_drops = Vec::new();

    // 先删除关系表（避免外键约束）
    let mut rel_tables = vec!["OWNS".to_string()];

    // 添加所有 TO_<NOUN> 关系表
    for noun in attr_info.named_attr_info_map.keys() {
        rel_tables.push(format!("TO_{}", noun.to_uppercase()));

        // 可能存在的引用边表
        // 这些是动态生成的，需要通过查询系统表获取
        // 暂时跳过，或者通过 list_tables 获取
    }

    for table in &rel_tables {
        match conn.query(&format!("DROP TABLE IF EXISTS {};", table)) {
            Ok(_) => {
                log::debug!("删除关系表: {}", table);
                dropped_count += 1;
            }
            Err(e) => {
                log::warn!("删除关系表 {} 失败: {}", table, e);
                failed_drops.push((table.clone(), e.to_string()));
            }
        }
    }

    // 删除属性节点表
    for noun in attr_info.named_attr_info_map.keys() {
        let table_name = format!("Attr_{}", noun.to_uppercase());
        match conn.query(&format!("DROP TABLE IF EXISTS {};", table_name)) {
            Ok(_) => {
                log::debug!("删除节点表: {}", table_name);
                dropped_count += 1;
            }
            Err(e) => {
                log::warn!("删除节点表 {} 失败: {}", table_name, e);
                failed_drops.push((table_name, e.to_string()));
            }
        }
    }

    // 最后删除 PE 主表
    match conn.query("DROP TABLE IF EXISTS PE;") {
        Ok(_) => {
            log::debug!("删除 PE 主表");
            dropped_count += 1;
        }
        Err(e) => {
            log::warn!("删除 PE 表失败: {}", e);
            failed_drops.push(("PE".to_string(), e.to_string()));
        }
    }

    if !failed_drops.is_empty() {
        log::error!("以下表删除失败:");
        for (table, error) in &failed_drops {
            log::error!("  - {}: {}", table, error);
        }
    }

    log::info!("已删除 {} 个表", dropped_count);
    Ok(())
}

#[cfg(feature = "kuzu")]
/// 重新初始化模式（删除并重建）
pub async fn reinit_schema() -> Result<()> {
    log::info!("开始重新初始化 Kuzu 模式...");

    // 1. 删除所有表
    drop_all_tables().await?;

    // 2. 重新创建所有表
    init_kuzu_schema().await?;

    // 3. 验证模式
    validate_schema().await?;

    log::info!("Kuzu 模式重新初始化完成");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[cfg(feature = "kuzu")]
    async fn test_init_schema() {
        let result = init_kuzu_schema().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[cfg(feature = "kuzu")]
    async fn test_validate_schema() {
        init_kuzu_schema().await.unwrap();
        let result = validate_schema().await;
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(feature = "kuzu")]
    fn test_extract_table_name() {
        let sql1 = "CREATE NODE TABLE IF NOT EXISTS Attr_ELBO(refno INT64 PRIMARY KEY)";
        assert_eq!(extract_table_name(sql1), "Attr_ELBO");

        let sql2 = "CREATE REL TABLE IF NOT EXISTS TO_ELBO(FROM PE TO Attr_ELBO)";
        assert_eq!(extract_table_name(sql2), "TO_ELBO");

        let sql3 = "DROP TABLE PE;";
        assert_eq!(extract_table_name(sql3), "UNKNOWN");
    }
}
