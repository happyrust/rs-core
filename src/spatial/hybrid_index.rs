use anyhow::{Context, Result};
use dashmap::DashMap;
use glam::Vec3;
use nalgebra::Point3;
use parry3d::bounding_volume::Aabb;
use rstar::{AABB, RTree, RTreeObject};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::{RefU64, get_db_option};

/// 空间元素，用于混合索引
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialElement {
    pub refno: RefU64,
    pub aabb: Aabb,
    pub element_type: String,
    pub last_updated: SystemTime,
    pub confidence: f32, // 空间位置的置信度
}

impl RTreeObject for SpatialElement {
    type Envelope = AABB<[f32; 3]>;

    fn envelope(&self) -> Self::Envelope {
        AABB::from_corners(
            [self.aabb.mins.x, self.aabb.mins.y, self.aabb.mins.z],
            [self.aabb.maxs.x, self.aabb.maxs.y, self.aabb.maxs.z],
        )
    }
}

/// 查询选项
#[derive(Debug, Clone)]
pub struct QueryOptions {
    pub tolerance: f32,
    pub max_results: usize,
    pub element_types: Vec<String>,
    pub min_confidence: f32,
    pub use_cache: bool,
}

impl Default for QueryOptions {
    fn default() -> Self {
        Self {
            tolerance: 0.001,
            max_results: 1000,
            element_types: vec![],
            min_confidence: 0.0,
            use_cache: true,
        }
    }
}

/// 查询结果
#[derive(Debug, Clone)]
pub struct QueryResult {
    pub refno: RefU64,
    pub aabb: Aabb,
    pub distance: f32,
    pub confidence: f32,
}

/// 索引统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    pub total_elements: usize,
    pub memory_elements: usize,
    pub sqlite_elements: usize,
    pub cache_hit_rate: f32,
    pub last_rebuild_time: SystemTime,
    pub query_count: u64,
    pub avg_query_time_ms: f32,
}

/// 混合空间索引
///
/// 结合内存 R*-tree 和 SQLite R-tree 的优势：
/// - 内存索引：快速查询，支持复杂几何操作
/// - SQLite 索引：持久化存储，支持大规模数据
/// - 智能缓存：热点数据优先加载到内存
pub struct HybridSpatialIndex {
    // 内存 R*-tree 索引
    memory_index: Arc<RwLock<RTree<SpatialElement>>>,

    // 查询缓存 (point -> results)
    query_cache: DashMap<String, (Vec<QueryResult>, Instant)>,

    // 统计信息
    stats: Arc<RwLock<IndexStats>>,

    // 配置
    cache_ttl: Duration,
    max_cache_size: usize,
    preload_threshold: f32, // 置信度阈值，高于此值的元素预加载到内存
}

impl HybridSpatialIndex {
    /// 创建新的混合空间索引
    pub async fn new() -> Result<Self> {
        let db_option = get_db_option();

        let index = Self {
            memory_index: Arc::new(RwLock::new(RTree::new())),
            query_cache: DashMap::new(),
            stats: Arc::new(RwLock::new(IndexStats {
                total_elements: 0,
                memory_elements: 0,
                sqlite_elements: 0,
                cache_hit_rate: 0.0,
                last_rebuild_time: SystemTime::now(),
                query_count: 0,
                avg_query_time_ms: 0.0,
            })),
            cache_ttl: Duration::from_secs(300), // 5分钟缓存
            max_cache_size: 10000,
            preload_threshold: 0.8, // 置信度 > 0.8 的元素预加载
        };

        // 初始化时预加载高置信度数据
        index.preload_high_confidence_data().await?;

        Ok(index)
    }

    /// 预加载高置信度数据到内存索引
    async fn preload_high_confidence_data(&self) -> Result<()> {
        let start_time = Instant::now();

        // 从 SQLite 加载高置信度数据
        let elements = self.load_high_confidence_from_sqlite().await?;

        let mut memory_index = self.memory_index.write().await;
        *memory_index = RTree::bulk_load(elements.clone());
        drop(memory_index);

        // 更新统计信息
        let mut stats = self.stats.write().await;
        stats.memory_elements = elements.len();
        stats.last_rebuild_time = SystemTime::now();

        info!(
            "预加载 {} 个高置信度空间元素到内存，耗时 {:?}",
            elements.len(),
            start_time.elapsed()
        );

        Ok(())
    }

    /// 从 SQLite 加载高置信度数据
    async fn load_high_confidence_from_sqlite(&self) -> Result<Vec<SpatialElement>> {
        use crate::spatial::sqlite;

        let conn = sqlite::open_connection()?;
        let mut stmt = conn.prepare(
            "SELECT i.refno, i.element_type, i.confidence, 
                    r.min_x, r.max_x, r.min_y, r.max_y, r.min_z, r.max_z
             FROM items i 
             JOIN aabb_index r ON i.refno = r.id 
             WHERE i.confidence >= ?1
             ORDER BY i.confidence DESC",
        )?;

        let rows = stmt.query_map([self.preload_threshold], |row| {
            let refno: i64 = row.get(0)?;
            let element_type: String = row.get(1)?;
            let confidence: f32 = row.get(2)?;
            let min_x: f32 = row.get(3)?;
            let max_x: f32 = row.get(4)?;
            let min_y: f32 = row.get(5)?;
            let max_y: f32 = row.get(6)?;
            let min_z: f32 = row.get(7)?;
            let max_z: f32 = row.get(8)?;

            Ok(SpatialElement {
                refno: RefU64(refno as u64),
                aabb: Aabb::new(
                    Point3::new(min_x, min_y, min_z),
                    Point3::new(max_x, max_y, max_z),
                ),
                element_type,
                last_updated: SystemTime::now(),
                confidence,
            })
        })?;

        let mut elements = Vec::new();
        for row in rows {
            elements.push(row?);
        }

        Ok(elements)
    }

    /// 查询包含指定点的空间元素
    pub async fn query_containing_point(
        &self,
        point: Vec3,
        options: &QueryOptions,
    ) -> Result<Vec<QueryResult>> {
        let start_time = Instant::now();

        // 生成缓存键
        let cache_key = if options.use_cache {
            Some(format!(
                "point_{:.3}_{:.3}_{:.3}_tol_{:.3}",
                point.x, point.y, point.z, options.tolerance
            ))
        } else {
            None
        };

        // 检查缓存
        if let Some(ref key) = cache_key {
            if let Some(cached_entry) = self.query_cache.get(key) {
                let (results, timestamp) = cached_entry.value();
                if timestamp.elapsed() < self.cache_ttl {
                    self.update_stats(start_time, true).await;
                    return Ok(results.clone());
                }
            }
        }

        // 执行查询
        let results = self.execute_point_query(point, options).await?;

        // 更新缓存
        if let Some(key) = cache_key {
            self.update_cache(key, results.clone());
        }

        self.update_stats(start_time, false).await;
        Ok(results)
    }

    /// 执行点查询的核心逻辑
    async fn execute_point_query(
        &self,
        point: Vec3,
        options: &QueryOptions,
    ) -> Result<Vec<QueryResult>> {
        let query_point = Point3::new(point.x, point.y, point.z);
        let mut results = Vec::new();

        // 1. 首先查询内存索引
        let memory_results = self.query_memory_index(query_point, options).await;
        results.extend(memory_results);

        // 2. 如果内存索引结果不足，查询 SQLite 索引
        if results.len() < options.max_results {
            let sqlite_results = self.query_sqlite_index(query_point, options).await?;

            // 去重并合并结果
            for sqlite_result in sqlite_results {
                if !results.iter().any(|r| r.refno == sqlite_result.refno) {
                    results.push(sqlite_result);
                }
            }
        }

        // 3. 过滤和排序
        results.retain(|r| r.confidence >= options.min_confidence);
        if !options.element_types.is_empty() {
            // 注意：这里需要额外查询元素类型信息
            // 为简化实现，暂时跳过类型过滤
        }

        results.sort_by(|a, b| {
            a.distance
                .partial_cmp(&b.distance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(options.max_results);

        Ok(results)
    }

    /// 查询内存索引
    async fn query_memory_index(
        &self,
        point: Point3<f32>,
        options: &QueryOptions,
    ) -> Vec<QueryResult> {
        let memory_index = self.memory_index.read().await;
        let query_envelope = AABB::from_corners(
            [
                point.x - options.tolerance,
                point.y - options.tolerance,
                point.z - options.tolerance,
            ],
            [
                point.x + options.tolerance,
                point.y + options.tolerance,
                point.z + options.tolerance,
            ],
        );

        memory_index
            .locate_in_envelope_intersecting(&query_envelope)
            .filter_map(|element| {
                if element.aabb.contains_local_point(&point) {
                    let center = element.aabb.center();
                    let distance = (point - center).norm();
                    Some(QueryResult {
                        refno: element.refno,
                        aabb: element.aabb,
                        distance,
                        confidence: element.confidence,
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    /// 查询 SQLite 索引
    async fn query_sqlite_index(
        &self,
        point: Point3<f32>,
        options: &QueryOptions,
    ) -> Result<Vec<QueryResult>> {
        use crate::spatial::sqlite;

        let conn = sqlite::open_connection()?;
        let mut stmt = conn.prepare(
            "SELECT i.refno, i.confidence,
                    r.min_x, r.max_x, r.min_y, r.max_y, r.min_z, r.max_z
             FROM items i 
             JOIN aabb_index r ON i.refno = r.id 
             WHERE r.min_x <= ?1 AND r.max_x >= ?1
               AND r.min_y <= ?2 AND r.max_y >= ?2  
               AND r.min_z <= ?3 AND r.max_z >= ?3
             ORDER BY i.confidence DESC
             LIMIT ?4",
        )?;

        let rows = stmt.query_map(
            [point.x, point.y, point.z, options.max_results as f32],
            |row| {
                let refno: i64 = row.get(0)?;
                let confidence: f32 = row.get(1)?;
                let min_x: f32 = row.get(2)?;
                let max_x: f32 = row.get(3)?;
                let min_y: f32 = row.get(4)?;
                let max_y: f32 = row.get(5)?;
                let min_z: f32 = row.get(6)?;
                let max_z: f32 = row.get(7)?;

                let aabb = Aabb::new(
                    Point3::new(min_x, min_y, min_z),
                    Point3::new(max_x, max_y, max_z),
                );
                let center = aabb.center();
                let distance = (point - center).norm();

                Ok(QueryResult {
                    refno: RefU64(refno as u64),
                    aabb,
                    distance,
                    confidence,
                })
            },
        )?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }

        Ok(results)
    }

    /// 更新查询缓存
    fn update_cache(&self, key: String, results: Vec<QueryResult>) {
        // 如果缓存已满，清理过期条目
        if self.query_cache.len() >= self.max_cache_size {
            self.cleanup_expired_cache();
        }

        self.query_cache.insert(key, (results, Instant::now()));
    }

    /// 清理过期缓存
    fn cleanup_expired_cache(&self) {
        let now = Instant::now();
        self.query_cache
            .retain(|_, (_, timestamp)| now.duration_since(*timestamp) < self.cache_ttl);
    }

    /// 更新统计信息
    async fn update_stats(&self, start_time: Instant, cache_hit: bool) {
        let mut stats = self.stats.write().await;
        stats.query_count += 1;

        let query_time_ms = start_time.elapsed().as_millis() as f32;
        stats.avg_query_time_ms = (stats.avg_query_time_ms * (stats.query_count - 1) as f32
            + query_time_ms)
            / stats.query_count as f32;

        if cache_hit {
            let total_queries = stats.query_count as f32;
            let cache_hits = stats.cache_hit_rate * (total_queries - 1.0) + 1.0;
            stats.cache_hit_rate = cache_hits / total_queries;
        }
    }

    /// 获取索引统计信息
    pub async fn get_stats(&self) -> IndexStats {
        let stats = self.stats.read().await;
        let mut result = stats.clone();
        result.total_elements = result.memory_elements + result.sqlite_elements;
        result
    }

    /// 重建内存索引
    pub async fn rebuild_memory_index(&self) -> Result<()> {
        info!("开始重建内存空间索引");
        let start_time = Instant::now();

        let elements = self.load_high_confidence_from_sqlite().await?;

        let mut memory_index = self.memory_index.write().await;
        *memory_index = RTree::bulk_load(elements.clone());
        drop(memory_index);

        // 清理缓存
        self.query_cache.clear();

        // 更新统计信息
        let mut stats = self.stats.write().await;
        stats.memory_elements = elements.len();
        stats.last_rebuild_time = SystemTime::now();

        info!(
            "内存空间索引重建完成，加载 {} 个元素，耗时 {:?}",
            elements.len(),
            start_time.elapsed()
        );

        Ok(())
    }
}

/// 全局混合空间索引实例
static HYBRID_INDEX: tokio::sync::OnceCell<HybridSpatialIndex> = tokio::sync::OnceCell::const_new();

/// 获取全局混合空间索引实例
pub async fn get_hybrid_index() -> &'static HybridSpatialIndex {
    HYBRID_INDEX
        .get_or_init(|| async {
            HybridSpatialIndex::new()
                .await
                .expect("Failed to initialize hybrid spatial index")
        })
        .await
}

/// 便捷函数：查询包含指定点的空间元素
pub async fn query_containing_point(
    point: Vec3,
    max_results: usize,
) -> Result<Vec<(RefU64, Aabb)>> {
    let index = get_hybrid_index().await;
    let options = QueryOptions {
        max_results,
        ..Default::default()
    };

    let results = index.query_containing_point(point, &options).await?;
    Ok(results.into_iter().map(|r| (r.refno, r.aabb)).collect())
}
