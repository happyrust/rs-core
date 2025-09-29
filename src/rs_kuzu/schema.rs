//! Kuzu 图模式定义
//!
//! 定义工厂设计数据的图模式结构

#[cfg(feature = "kuzu")]
use super::create_kuzu_connection;
#[cfg(feature = "kuzu")]
use anyhow::Result;

#[cfg(feature = "kuzu")]
/// 初始化 Kuzu 图模式
///
/// 创建所有必要的节点表和关系表
pub async fn init_kuzu_schema() -> Result<()> {
    log::info!("正在初始化 Kuzu 图模式...");

    let conn = create_kuzu_connection()?;

    // 1. 创建 PE (Plant Element) 节点表
    log::debug!("创建 PE 节点表");
    conn.query(
        r#"
        CREATE NODE TABLE IF NOT EXISTS PE(
            refno INT64,
            name STRING,
            noun STRING,
            dbnum INT32,
            sesno INT32,
            cata_hash STRING,
            deleted BOOLEAN,
            status_code STRING,
            lock BOOLEAN,
            PRIMARY KEY(refno)
        );
        "#,
    )?;

    // 2. 创建属性节点表
    log::debug!("创建 Attribute 节点表");
    conn.query(
        r#"
        CREATE NODE TABLE IF NOT EXISTS Attribute(
            id INT64,
            refno INT64,
            attr_name STRING,
            attr_value STRING,
            attr_type STRING,
            PRIMARY KEY(id)
        );
        "#,
    )?;

    // 3. 创建 UDA (User Defined Attribute) 节点表
    log::debug!("创建 UDA 节点表");
    conn.query(
        r#"
        CREATE NODE TABLE IF NOT EXISTS UDA(
            id INT64,
            refno INT64,
            uda_name STRING,
            uda_value STRING,
            uda_type STRING,
            PRIMARY KEY(id)
        );
        "#,
    )?;

    // 4. 创建层次关系表（owner-child）
    log::debug!("创建 OWNS 关系表");
    conn.query(
        r#"
        CREATE REL TABLE IF NOT EXISTS OWNS(
            FROM PE TO PE,
            relation_type STRING DEFAULT 'owner'
        );
        "#,
    )?;

    // 5. 创建属性关系表
    log::debug!("创建 HAS_ATTR 关系表");
    conn.query(
        r#"
        CREATE REL TABLE IF NOT EXISTS HAS_ATTR(
            FROM PE TO Attribute
        );
        "#,
    )?;

    // 6. 创建 UDA 关系表
    log::debug!("创建 HAS_UDA 关系表");
    conn.query(
        r#"
        CREATE REL TABLE IF NOT EXISTS HAS_UDA(
            FROM PE TO UDA
        );
        "#,
    )?;

    // 7. 创建引用关系表（用于 REFNO 类型的属性）
    log::debug!("创建 REFERS_TO 关系表");
    conn.query(
        r#"
        CREATE REL TABLE IF NOT EXISTS REFERS_TO(
            FROM PE TO PE,
            attr_name STRING,
            ref_type STRING DEFAULT 'attribute'
        );
        "#,
    )?;

    // 8. 创建设计关系表（Design-使用-Catalogue）
    log::debug!("创建 USES_CATA 关系表");
    conn.query(
        r#"
        CREATE REL TABLE IF NOT EXISTS USES_CATA(
            FROM PE TO PE,
            cata_hash STRING
        );
        "#,
    )?;

    // 9. 创建索引（如果支持）
    log::debug!("创建索引");
    // 注意：Kuzu 0.8 版本可能不完全支持二级索引，主键自动索引
    // 如果需要，可以在这里添加索引创建语句

    log::info!("Kuzu 图模式初始化完成");
    Ok(())
}

#[cfg(feature = "kuzu")]
/// 删除所有表（慎用！）
pub async fn drop_all_tables() -> Result<()> {
    log::warn!("正在删除所有 Kuzu 表...");

    let conn = create_kuzu_connection()?;

    // 先删除关系表
    let rel_tables = vec!["OWNS", "HAS_ATTR", "HAS_UDA", "REFERS_TO", "USES_CATA"];
    for table in rel_tables {
        match conn.query(&format!("DROP TABLE IF EXISTS {};", table)) {
            Ok(_) => log::debug!("删除关系表: {}", table),
            Err(e) => log::warn!("删除关系表 {} 失败: {}", table, e),
        }
    }

    // 再删除节点表
    let node_tables = vec!["PE", "Attribute", "UDA"];
    for table in node_tables {
        match conn.query(&format!("DROP TABLE IF EXISTS {};", table)) {
            Ok(_) => log::debug!("删除节点表: {}", table),
            Err(e) => log::warn!("删除节点表 {} 失败: {}", table, e),
        }
    }

    log::info!("所有表已删除");
    Ok(())
}

#[cfg(feature = "kuzu")]
/// 检查模式是否已初始化
pub async fn is_schema_initialized() -> Result<bool> {
    let conn = create_kuzu_connection()?;

    // 尝试查询 PE 表，如果表不存在会报错
    match conn.query("MATCH (p:PE) RETURN count(p) LIMIT 1;") {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

#[cfg(feature = "kuzu")]
/// 获取模式统计信息
#[derive(Debug, Clone)]
pub struct SchemaStats {
    /// PE 节点数量
    pub pe_count: u64,
    /// 属性节点数量
    pub attribute_count: u64,
    /// UDA 节点数量
    pub uda_count: u64,
    /// OWNS 关系数量
    pub owns_count: u64,
    /// REFERS_TO 关系数量
    pub refers_to_count: u64,
}

#[cfg(feature = "kuzu")]
impl SchemaStats {
    /// 查询模式统计信息
    pub async fn query() -> Result<Self> {
        // 查询各个表的记录数
        let pe_count = Self::count_nodes("PE").await?;
        let attribute_count = Self::count_nodes("Attribute").await?;
        let uda_count = Self::count_nodes("UDA").await?;
        let owns_count = Self::count_rels("OWNS").await?;
        let refers_to_count = Self::count_rels("REFERS_TO").await?;

        Ok(Self {
            pe_count,
            attribute_count,
            uda_count,
            owns_count,
            refers_to_count,
        })
    }

    async fn count_nodes(table_name: &str) -> Result<u64> {
        let conn = create_kuzu_connection()?;
        let query = format!("MATCH (n:{}) RETURN count(n) AS cnt;", table_name);
        let mut result = conn.query(&query)?;

        if let Some(record) = result.next() {
            // Kuzu 使用索引号获取值
            let value = record.get(0)
                .ok_or_else(|| anyhow::anyhow!("无法获取count值"))?;

            // 尝试从 Value 中提取 i64
            if let kuzu::Value::Int64(count) = value {
                Ok(*count as u64)
            } else {
                Err(anyhow::anyhow!("count值类型不匹配"))
            }
        } else {
            Ok(0)
        }
    }

    async fn count_rels(table_name: &str) -> Result<u64> {
        let conn = create_kuzu_connection()?;
        let query = format!("MATCH ()-[r:{}]->() RETURN count(r) AS cnt;", table_name);
        let mut result = conn.query(&query)?;

        if let Some(record) = result.next() {
            // Kuzu 使用索引号获取值
            let value = record.get(0)
                .ok_or_else(|| anyhow::anyhow!("无法获取count值"))?;

            // 尝试从 Value 中提取 i64
            if let kuzu::Value::Int64(count) = value {
                Ok(*count as u64)
            } else {
                Err(anyhow::anyhow!("count值类型不匹配"))
            }
        } else {
            Ok(0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // 需要实际的 Kuzu 数据库
    async fn test_schema_initialization() {
        // 这个测试需要实际的数据库环境
        // 仅用于开发时手动测试
    }
}