use crate::transform::strategies::endatu_error::EndatuError;
use crate::{RefnoEnum, get_index_by_noun_in_parent};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;

/// ENDATU 索引缓存，符合 core.dll 的性能优化策略
static ENDATU_INDEX_CACHE: Lazy<Mutex<HashMap<(RefnoEnum, RefnoEnum), Option<u32>>>> =
    Lazy::new(|| {
        println!("🚀 初始化 ENDATU 索引缓存");
        Mutex::new(HashMap::new())
    });

/// 缓存统计信息
static CACHE_STATS: Lazy<Mutex<CacheStats>> = Lazy::new(|| Mutex::new(CacheStats::new()));

#[derive(Debug, Default, Clone)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub total_queries: u64,
}

impl CacheStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn hit_rate(&self) -> f64 {
        if self.total_queries == 0 {
            0.0
        } else {
            self.hits as f64 / self.total_queries as f64
        }
    }

    pub fn record_hit(&mut self) {
        self.hits += 1;
        self.total_queries += 1;
    }

    pub fn record_miss(&mut self) {
        self.misses += 1;
        self.total_queries += 1;
    }

    pub fn print_stats(&self) {
        println!("📊 ENDATU 缓存统计:");
        println!("   总查询: {}", self.total_queries);
        println!("   缓存命中: {}", self.hits);
        println!("   缓存未命中: {}", self.misses);
        println!("   命中率: {:.2}%", self.hit_rate() * 100.0);
    }
}

/// 获取缓存的 ENDATU 索引
///
/// 这个函数实现了与 core.dll 相同的缓存策略，
/// 减少重复的数据库查询，提升性能
pub async fn get_cached_endatu_index(
    parent: RefnoEnum,
    refno: RefnoEnum,
) -> Result<Option<u32>, EndatuError> {
    let cache_key = (parent, refno);

    // 尝试从缓存获取
    {
        let cache = ENDATU_INDEX_CACHE.lock().unwrap();
        if let Some(&cached) = cache.get(&cache_key) {
            // 记录缓存命中
            {
                let mut stats = CACHE_STATS.lock().unwrap();
                stats.record_hit();
            }
            return Ok(cached);
        }
    }

    // 缓存未命中，查询数据库
    {
        let mut stats = CACHE_STATS.lock().unwrap();
        stats.record_miss();
    }

    // 计算索引
    let result = get_index_by_noun_in_parent(parent, refno, Some("ENDATU"))
        .await
        .map_err(|e| EndatuError::GeometryCalculationError(format!("索引查询失败: {}", e)))?;

    // 存入缓存
    {
        let mut cache = ENDATU_INDEX_CACHE.lock().unwrap();

        // 防止缓存过大，超过 10000 条时清理一半
        if cache.len() > 10000 {
            let keys_to_remove: Vec<_> = cache.keys().take(5000).cloned().collect();
            for key in keys_to_remove {
                cache.remove(&key);
            }
            println!("🧹 清理 ENDATU 缓存，当前大小: {}", cache.len());
        }

        cache.insert(cache_key, result);
    }

    Ok(result)
}

/// 清空 ENDATU 缓存
pub fn clear_endatu_cache() {
    let mut cache = ENDATU_INDEX_CACHE.lock().unwrap();
    cache.clear();
    println!("🧹 已清空 ENDATU 缓存");
}

/// 获取缓存统计信息
pub fn get_cache_stats() -> CacheStats {
    CACHE_STATS.lock().unwrap().clone()
}

/// 打印缓存统计信息
pub fn print_cache_stats() {
    let stats = get_cache_stats();
    stats.print_stats();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RefnoEnum;

    #[tokio::test]
    async fn test_endatu_cache() {
        // 清空缓存
        clear_endatu_cache();

        let parent = RefnoEnum::from("test_parent");
        let refno = RefnoEnum::from("test_refno");

        // 第一次查询，应该缓存未命中
        let result1: Result<Option<u32>, EndatuError> =
            get_cached_endatu_index(parent, refno).await;
        assert!(result1.is_ok());

        // 第二次查询，应该缓存命中
        let result2: Result<Option<u32>, EndatuError> =
            get_cached_endatu_index(parent, refno).await;
        assert!(result2.is_ok());

        // 检查统计信息
        let stats = get_cache_stats();
        assert_eq!(stats.total_queries, 2);
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);

        print_cache_stats();
    }
}
