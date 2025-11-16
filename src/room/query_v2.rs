use crate::shape::pdms_shape::PlantMesh;
use crate::{RefU64, RefnoEnum, SUL_DB, query_insts};
use glam::Vec3;
use nalgebra::Point3;
use parry3d::bounding_volume::Aabb;
use parry3d::math::Isometry;
use parry3d::query::PointQuery;
use parry3d::shape::TriMeshFlags;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

#[cfg(all(not(target_arch = "wasm32"), feature = "sqlite"))]
use crate::spatial::hybrid_index::{QueryOptions, get_hybrid_index};
#[cfg(all(not(target_arch = "wasm32"), feature = "sqlite"))]
use anyhow::Context;

/// 几何网格缓存，使用 DashMap 提升并发性能
use dashmap::DashMap;
use std::sync::OnceLock;

static GEOMETRY_CACHE: OnceLock<DashMap<String, Arc<crate::shape::pdms_shape::PlantMesh>>> =
    OnceLock::new();

fn get_geometry_cache() -> &'static DashMap<String, Arc<crate::shape::pdms_shape::PlantMesh>> {
    GEOMETRY_CACHE.get_or_init(|| DashMap::new())
}

/// 房间查询性能统计
#[derive(Debug, Clone)]
pub struct RoomQueryStats {
    pub total_queries: u64,
    pub cache_hits: u64,
    pub avg_query_time_ms: f32,
    pub geometry_cache_size: usize,
}

static QUERY_STATS: OnceLock<Arc<RwLock<RoomQueryStats>>> = OnceLock::new();

fn get_query_stats() -> &'static Arc<RwLock<RoomQueryStats>> {
    QUERY_STATS.get_or_init(|| {
        Arc::new(RwLock::new(RoomQueryStats {
            total_queries: 0,
            cache_hits: 0,
            avg_query_time_ms: 0.0,
            geometry_cache_size: 0,
        }))
    })
}

/// 改进版本的房间号查询函数
///
/// 使用混合空间索引和优化的缓存机制
#[cfg(all(not(target_arch = "wasm32"), feature = "sqlite"))]
pub async fn query_room_number_by_point_v2(point: Vec3) -> anyhow::Result<Option<String>> {
    let start_time = Instant::now();

    let Some(refno) = query_room_panel_by_point_v2(point).await? else {
        return Ok(None);
    };

    // 使用 SurrealDB 查询房间号
    let mut response = SUL_DB
        .query(format!(
            r#"
            select value room_num from only {}<-room_panel_relate limit 1;
        "#,
            refno.to_pe_key()
        ))
        .await?;

    let room_number: Option<String> = response.take(0)?;

    // 更新统计信息
    update_query_stats(start_time, false).await;

    debug!(
        "房间号查询完成: point={:?}, refno={:?}, room={:?}, 耗时={:?}",
        point,
        refno,
        room_number,
        start_time.elapsed()
    );

    Ok(room_number)
}

/// 改进版本的房间面板查询函数
///
/// 主要改进：
/// 1. 使用混合空间索引替代纯 SQLite 查询
/// 2. 优化几何网格缓存机制
/// 3. 添加性能监控和统计
#[cfg(all(not(target_arch = "wasm32"), feature = "sqlite"))]
pub async fn query_room_panel_by_point_v2(point: Vec3) -> anyhow::Result<Option<RefnoEnum>> {
    let start_time = Instant::now();

    // 1. 使用混合空间索引进行候选查询
    let hybrid_index = get_hybrid_index().await;
    let query_options = QueryOptions {
        tolerance: 0.001,
        max_results: 256,
        use_cache: true,
        ..Default::default()
    };

    let candidates = hybrid_index
        .query_containing_point(point, &query_options)
        .await
        .context("混合空间索引查询失败")?;

    if candidates.is_empty() {
        debug!("未找到候选房间面板: point={:?}", point);
        return Ok(None);
    }

    info!("找到 {} 个候选房间面板", candidates.len());

    // 2. 转换为 RefnoEnum 并查询几何实例
    let refnos: Vec<RefnoEnum> = candidates
        .iter()
        .map(|result| RefnoEnum::Refno(RefU64(result.refno.0)))
        .collect();

    let insts = query_insts(&refnos, true).await?;
    let pt: Point3<f32> = point.into();
    let parry_pt = parry3d::math::Point::new(pt.x, pt.y, pt.z);

    // 3. 精确几何检测
    for result in &candidates {
        let refno = RefU64(result.refno.0);

        // 跳过明显错误的包围盒
        if result.aabb.mins.x > 1_000_000.0 {
            continue;
        }

        let Some(geom_inst) = insts.iter().find(|x| x.refno.refno() == refno) else {
            continue;
        };

        // 4. 检查每个几何实例
        for inst in &geom_inst.insts {
            // 使用优化的几何缓存
            let mesh = match load_geometry_cached(&inst.geo_hash).await {
                Ok(mesh) => mesh,
                Err(_) => {
                    warn!("无法加载几何文件: {}", inst.geo_hash);
                    continue;
                }
            };

            let Some(tri_mesh) = mesh.get_tri_mesh_with_flag(
                (geom_inst.world_trans * &inst.transform).to_matrix(),
                TriMeshFlags::ORIENTED,
            ) else {
                continue;
            };

            // 5. 精确点包含检测
            if tri_mesh.contains_point(&Isometry::identity(), &parry_pt) {
                info!(
                    "找到包含点的房间面板: refno={}, 耗时={:?}",
                    refno.0,
                    start_time.elapsed()
                );
                return Ok(Some(RefnoEnum::Refno(refno)));
            }
        }
    }

    debug!("未找到包含点的房间面板: point={:?}", point);
    Ok(None)
}

/// 优化的几何缓存加载函数
async fn load_geometry_cached(
    geo_hash: &str,
) -> anyhow::Result<Arc<crate::shape::pdms_shape::PlantMesh>> {
    let cache = get_geometry_cache();

    // 检查缓存
    if let Some(cached_mesh) = cache.get(geo_hash) {
        update_query_stats(Instant::now(), true).await;
        return Ok(cached_mesh.clone());
    }

    // 加载几何文件
    let file_path = format!("assets/meshes/{}.mesh", geo_hash);
    let mesh = tokio::task::spawn_blocking(move || {
        crate::shape::pdms_shape::PlantMesh::des_mesh_file(&file_path)
    })
    .await
    .context("几何文件加载任务失败")??;

    let mesh_arc = Arc::new(mesh);

    // 缓存管理：如果缓存过大，清理一些条目
    if cache.len() > 1000 {
        // 简单的 LRU 策略：随机清理一些条目
        let keys_to_remove: Vec<String> = cache
            .iter()
            .take(100)
            .map(|entry| entry.key().clone())
            .collect();

        for key in keys_to_remove {
            cache.remove(&key);
        }
    }

    cache.insert(geo_hash.to_string(), mesh_arc.clone());

    Ok(mesh_arc)
}

/// 更新查询统计信息
async fn update_query_stats(start_time: Instant, cache_hit: bool) {
    let stats_lock = get_query_stats();
    let mut stats = stats_lock.write().await;

    stats.total_queries += 1;

    if cache_hit {
        stats.cache_hits += 1;
    }

    let query_time_ms = start_time.elapsed().as_millis() as f32;
    stats.avg_query_time_ms = (stats.avg_query_time_ms * (stats.total_queries - 1) as f32
        + query_time_ms)
        / stats.total_queries as f32;

    stats.geometry_cache_size = get_geometry_cache().len();
}

/// 获取房间查询统计信息
pub async fn get_room_query_stats() -> RoomQueryStats {
    let stats_lock = get_query_stats();
    let stats = stats_lock.read().await;
    stats.clone()
}

/// 清理几何缓存
pub fn clear_geometry_cache() {
    let cache = get_geometry_cache();
    cache.clear();
    info!("几何缓存已清理");
}

/// 批量房间查询函数
///
/// 对多个点进行并发查询，提升批量查询性能
#[cfg(all(not(target_arch = "wasm32"), feature = "sqlite"))]
pub async fn batch_query_room_numbers(
    points: Vec<Vec3>,
    max_concurrent: usize,
) -> anyhow::Result<Vec<Option<String>>> {
    use futures::stream::{self, StreamExt};

    let start_time = Instant::now();

    let results = stream::iter(points)
        .map(|point| async move { query_room_number_by_point_v2(point).await })
        .buffer_unordered(max_concurrent)
        .collect::<Vec<_>>()
        .await;

    let mut room_numbers = Vec::new();
    for result in results {
        room_numbers.push(result?);
    }

    info!(
        "批量房间查询完成: {} 个点, 耗时={:?}",
        room_numbers.len(),
        start_time.elapsed()
    );

    Ok(room_numbers)
}

/// 预热几何缓存
///
/// 预加载常用的几何文件到缓存中
pub async fn preheat_geometry_cache(geo_hashes: Vec<String>) -> anyhow::Result<()> {
    let start_time = Instant::now();

    for geo_hash in geo_hashes {
        if let Err(e) = load_geometry_cached(&geo_hash).await {
            warn!("预热几何缓存失败: geo_hash={}, error={}", geo_hash, e);
        }
    }

    info!(
        "几何缓存预热完成: 缓存大小={}, 耗时={:?}",
        get_geometry_cache().len(),
        start_time.elapsed()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_room_query_stats() {
        let stats = get_room_query_stats().await;
        assert_eq!(stats.total_queries, 0);
        assert_eq!(stats.cache_hits, 0);
    }

    #[tokio::test]
    async fn test_geometry_cache() {
        clear_geometry_cache();
        assert_eq!(get_geometry_cache().len(), 0);
    }

    #[tokio::test]
    async fn test_batch_query_empty() {
        let points = vec![];
        let results = batch_query_room_numbers(points, 10).await.unwrap();
        assert!(results.is_empty());
    }
}
