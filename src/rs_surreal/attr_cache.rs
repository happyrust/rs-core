use once_cell::sync::Lazy;
use parking_lot::RwLock;
use serde::Deserialize;
use std::collections::HashMap;
use surrealdb::types as surrealdb_types;
use surrealdb::types::SurrealValue;

use crate::rs_surreal::SUL_DB;

/// 全局属性中文名缓存
/// key: 属性名（如 "NAME", "REFNO", "OWNER"）
/// value: 中文名（如 "名称", "参考号", "所有者"）
pub static ATTR_CN_NAME_CACHE: Lazy<RwLock<HashMap<String, String>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

/// 从数据库加载所有属性的中文名称到缓存
///
/// 此函数应在数据库初始化后调用，一次性加载所有属性元数据
pub async fn load_attr_cn_names() -> anyhow::Result<()> {
    // 使用 record::id(id) 将 RecordId 转换为字符串
    // 直接查询所有记录，包括 meta_cn_name 为空的
    let sql = r#"
        SELECT record::id(id) as id, meta_cn_name 
        FROM att_meta;
    "#;

    #[derive(Debug, Deserialize, SurrealValue)]
    struct AttrMeta {
        id: String,
        meta_cn_name: Option<String>,
    }

    let mut response = SUL_DB.query(sql).await?;
    let records: Vec<AttrMeta> = response.take(0)?;
    
    tracing::info!("📊 从数据库查询到 {} 条 att_meta 记录", records.len());

    let mut cache = ATTR_CN_NAME_CACHE.write();
    cache.clear();
    
    let mut none_count = 0;
    let mut empty_count = 0;

    for record in records {
        // 只存储有中文名的属性
        match record.meta_cn_name {
            Some(cn_name) if !cn_name.is_empty() => {
                cache.insert(record.id, cn_name);
            }
            Some(_) => {
                empty_count += 1;
            }
            None => {
                none_count += 1;
            }
        }
    }

    let count = cache.len();
    tracing::info!("已加载 {} 个属性中文名称到缓存 (跳过 {} 个空值, {} 个 NONE)", count, empty_count, none_count);
    
    // 输出前5个样例用于验证
    if count > 0 {
        let samples: Vec<String> = cache.iter()
            .take(5)
            .map(|(k, v)| format!("{} -> {}", k, v))
            .collect();
        tracing::info!("样例属性: {}", samples.join(", "));
    }

    Ok(())
}

/// 获取属性的中文名称（从缓存中快速查询）
///
/// # 参数
/// * `attr_name` - 属性名（如 "NAME", "REFNO"）
///
/// # 返回值
/// * `Some(String)` - 如果找到对应的中文名
/// * `None` - 如果缓存中没有该属性的中文名
#[inline]
pub fn get_attr_cn_name(attr_name: &str) -> Option<String> {
    ATTR_CN_NAME_CACHE.read().get(attr_name).cloned()
}

/// 检查缓存是否已加载
#[inline]
pub fn is_cache_loaded() -> bool {
    !ATTR_CN_NAME_CACHE.read().is_empty()
}

/// 获取缓存中的属性数量
#[inline]
pub fn cache_size() -> usize {
    ATTR_CN_NAME_CACHE.read().len()
}


