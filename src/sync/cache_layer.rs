//! 缓存层优化
//!
//! 提供高性能的缓存机制以减少数据库查询

use crate::types::*;
use anyhow::Result;
use lru::LruCache;
use std::hash::Hash;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// 缓存项
#[derive(Debug, Clone)]
struct CacheEntry<T> {
    /// 缓存的数据
    data: T,
    /// 插入时间
    inserted_at: Instant,
    /// 访问次数
    access_count: u64,
}

/// 通用缓存层
pub struct CacheLayer<K: Hash + Eq, V: Clone> {
    /// LRU缓存
    cache: Arc<RwLock<LruCache<K, CacheEntry<V>>>>,
    /// 缓存统计
    stats: Arc<RwLock<CacheStats>>,
    /// 缓存配置
    config: CacheConfig,
}

/// 缓存配置
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// 最大缓存项数
    pub max_entries: usize,
    /// 过期时间
    pub ttl: Duration,
    /// 是否启用统计
    pub enable_stats: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 10000,
            ttl: Duration::from_secs(300), // 5分钟
            enable_stats: true,
        }
    }
}

/// 缓存统计
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// 命中次数
    pub hits: u64,
    /// 未命中次数
    pub misses: u64,
    /// 插入次数
    pub inserts: u64,
    /// 更新次数
    pub updates: u64,
    /// 驱逐次数
    pub evictions: u64,
}

impl CacheStats {
    /// 计算命中率
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

impl<K: Hash + Eq, V: Clone> CacheLayer<K, V> {
    /// 创建新的缓存层
    pub fn new(config: CacheConfig) -> Self {
        let cache = LruCache::new(config.max_entries.try_into().unwrap());
        Self {
            cache: Arc::new(RwLock::new(cache)),
            stats: Arc::new(RwLock::new(CacheStats::default())),
            config,
        }
    }

    /// 获取缓存项
    pub async fn get(&self, key: &K) -> Option<V> {
        let mut cache = self.cache.write().await;

        if let Some(entry) = cache.get_mut(key) {
            // 检查是否过期
            if entry.inserted_at.elapsed() > self.config.ttl {
                cache.pop(key);
                if self.config.enable_stats {
                    let mut stats = self.stats.write().await;
                    stats.misses += 1;
                }
                return None;
            }

            entry.access_count += 1;
            if self.config.enable_stats {
                let mut stats = self.stats.write().await;
                stats.hits += 1;
            }
            Some(entry.data.clone())
        } else {
            if self.config.enable_stats {
                let mut stats = self.stats.write().await;
                stats.misses += 1;
            }
            None
        }
    }

    /// 插入缓存项
    pub async fn insert(&self, key: K, value: V) {
        let mut cache = self.cache.write().await;

        let entry = CacheEntry {
            data: value,
            inserted_at: Instant::now(),
            access_count: 0,
        };

        let evicted = cache.push(key, entry);

        if self.config.enable_stats {
            let mut stats = self.stats.write().await;
            if evicted.is_some() {
                stats.evictions += 1;
            }
            stats.inserts += 1;
        }
    }

    /// 更新缓存项
    pub async fn update(&self, key: K, value: V) {
        let mut cache = self.cache.write().await;

        if let Some(entry) = cache.get_mut(&key) {
            entry.data = value;
            entry.inserted_at = Instant::now();
            if self.config.enable_stats {
                let mut stats = self.stats.write().await;
                stats.updates += 1;
            }
        } else {
            let entry = CacheEntry {
                data: value,
                inserted_at: Instant::now(),
                access_count: 0,
            };
            cache.put(key, entry);
            if self.config.enable_stats {
                let mut stats = self.stats.write().await;
                stats.inserts += 1;
            }
        }
    }

    /// 清除缓存
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// 获取统计信息
    pub async fn get_stats(&self) -> CacheStats {
        self.stats.read().await.clone()
    }

    /// 重置统计信息
    pub async fn reset_stats(&self) {
        let mut stats = self.stats.write().await;
        *stats = CacheStats::default();
    }
}

/// PE缓存层
pub struct PECache {
    cache: CacheLayer<RefnoEnum, SPdmsElement>,
}

impl PECache {
    /// 创建PE缓存
    pub fn new(max_entries: usize) -> Self {
        let config = CacheConfig {
            max_entries,
            ttl: Duration::from_secs(600), // PE缓存10分钟
            enable_stats: true,
        };
        Self {
            cache: CacheLayer::new(config),
        }
    }

    /// 获取PE
    pub async fn get(&self, refno: RefnoEnum) -> Option<SPdmsElement> {
        self.cache.get(&refno).await
    }

    /// 缓存PE
    pub async fn put(&self, refno: RefnoEnum, pe: SPdmsElement) {
        self.cache.insert(refno, pe).await;
    }

    /// 批量缓存PE
    pub async fn put_batch(&self, pes: Vec<(RefnoEnum, SPdmsElement)>) {
        for (refno, pe) in pes {
            self.cache.insert(refno, pe).await;
        }
    }

    /// 获取统计信息
    pub async fn stats(&self) -> CacheStats {
        self.cache.get_stats().await
    }
}

/// 属性缓存层
pub struct AttributeCache {
    cache: CacheLayer<RefnoEnum, NamedAttrMap>,
}

impl AttributeCache {
    /// 创建属性缓存
    pub fn new(max_entries: usize) -> Self {
        let config = CacheConfig {
            max_entries,
            ttl: Duration::from_secs(300), // 属性缓存5分钟
            enable_stats: true,
        };
        Self {
            cache: CacheLayer::new(config),
        }
    }

    /// 获取属性
    pub async fn get(&self, refno: RefnoEnum) -> Option<NamedAttrMap> {
        self.cache.get(&refno).await
    }

    /// 缓存属性
    pub async fn put(&self, refno: RefnoEnum, attmap: NamedAttrMap) {
        self.cache.insert(refno, attmap).await;
    }

    /// 获取统计信息
    pub async fn stats(&self) -> CacheStats {
        self.cache.get_stats().await
    }
}

/// 关系缓存层
pub struct RelationCache {
    /// 子元素缓存
    children_cache: CacheLayer<RefnoEnum, Vec<RefnoEnum>>,
    /// 父元素缓存
    parent_cache: CacheLayer<RefnoEnum, Option<RefnoEnum>>,
}

impl RelationCache {
    /// 创建关系缓存
    pub fn new(max_entries: usize) -> Self {
        let config = CacheConfig {
            max_entries,
            ttl: Duration::from_secs(300),
            enable_stats: true,
        };
        Self {
            children_cache: CacheLayer::new(config.clone()),
            parent_cache: CacheLayer::new(config),
        }
    }

    /// 获取子元素
    pub async fn get_children(&self, refno: RefnoEnum) -> Option<Vec<RefnoEnum>> {
        self.children_cache.get(&refno).await
    }

    /// 缓存子元素
    pub async fn put_children(&self, refno: RefnoEnum, children: Vec<RefnoEnum>) {
        self.children_cache.insert(refno, children).await;
    }

    /// 获取父元素
    pub async fn get_parent(&self, refno: RefnoEnum) -> Option<Option<RefnoEnum>> {
        self.parent_cache.get(&refno).await
    }

    /// 缓存父元素
    pub async fn put_parent(&self, refno: RefnoEnum, parent: Option<RefnoEnum>) {
        self.parent_cache.insert(refno, parent).await;
    }
}

/// 统一缓存管理器
pub struct CacheManager {
    /// PE缓存
    pub pe_cache: Arc<PECache>,
    /// 属性缓存
    pub attr_cache: Arc<AttributeCache>,
    /// 关系缓存
    pub relation_cache: Arc<RelationCache>,
}

impl CacheManager {
    /// 创建缓存管理器
    pub fn new() -> Self {
        Self {
            pe_cache: Arc::new(PECache::new(10000)),
            attr_cache: Arc::new(AttributeCache::new(20000)),
            relation_cache: Arc::new(RelationCache::new(15000)),
        }
    }

    /// 清除所有缓存
    pub async fn clear_all(&self) {
        self.pe_cache.cache.clear().await;
        self.attr_cache.cache.clear().await;
        self.relation_cache.children_cache.clear().await;
        self.relation_cache.parent_cache.clear().await;
    }

    /// 获取所有缓存统计
    pub async fn get_all_stats(&self) -> CacheManagerStats {
        CacheManagerStats {
            pe_stats: self.pe_cache.stats().await,
            attr_stats: self.attr_cache.stats().await,
            children_stats: self.relation_cache.children_cache.get_stats().await,
            parent_stats: self.relation_cache.parent_cache.get_stats().await,
        }
    }
}

/// 缓存管理器统计
#[derive(Debug, Clone)]
pub struct CacheManagerStats {
    pub pe_stats: CacheStats,
    pub attr_stats: CacheStats,
    pub children_stats: CacheStats,
    pub parent_stats: CacheStats,
}

impl CacheManagerStats {
    /// 计算总体命中率
    pub fn overall_hit_rate(&self) -> f64 {
        let total_hits = self.pe_stats.hits
            + self.attr_stats.hits
            + self.children_stats.hits
            + self.parent_stats.hits;

        let total_misses = self.pe_stats.misses
            + self.attr_stats.misses
            + self.children_stats.misses
            + self.parent_stats.misses;

        if total_hits + total_misses == 0 {
            0.0
        } else {
            total_hits as f64 / (total_hits + total_misses) as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_layer() {
        let config = CacheConfig {
            max_entries: 100,
            ttl: Duration::from_secs(60),
            enable_stats: true,
        };

        let cache: CacheLayer<i32, String> = CacheLayer::new(config);

        // 测试插入和获取
        cache.insert(1, "value1".to_string()).await;
        let value = cache.get(&1).await;
        assert_eq!(value, Some("value1".to_string()));

        // 测试未命中
        let value = cache.get(&2).await;
        assert_eq!(value, None);

        // 检查统计
        let stats = cache.get_stats().await;
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.inserts, 1);
    }

    #[tokio::test]
    async fn test_pe_cache() {
        let cache = PECache::new(100);

        let refno = RefnoEnum::from(RefU64(123));
        let pe = PElement::default();

        cache.put(refno, pe.clone()).await;
        let cached = cache.get(refno).await;
        assert!(cached.is_some());

        let stats = cache.stats().await;
        assert_eq!(stats.hits, 1);
    }

    #[test]
    fn test_cache_stats_hit_rate() {
        let mut stats = CacheStats::default();
        stats.hits = 80;
        stats.misses = 20;

        assert_eq!(stats.hit_rate(), 0.8);
    }
}
