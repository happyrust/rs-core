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

/// 根据房间关键词查询房间和面板关系
///
/// 从数据库中查询包含指定关键词的房间，并获取每个房间关联的面板列表。
/// 支持多关键词查询（OR条件）和项目特定的查询逻辑。
///
/// # 参数
/// * `room_keywords` - 房间关键词列表，例如 `vec!["-R-".to_string()]`
///
/// # 返回
/// * `Vec<(RefnoEnum, String, Vec<RefnoEnum>)>` - 房间-面板映射列表
///   - 元组第一项：房间 RefnoEnum
///   - 元组第二项：房间号（从 NAME 字段提取）
///   - 元组第三项：该房间包含的面板 RefnoEnum 列表
///
/// # 错误
/// 当数据库查询失败或数据格式错误时返回错误
///
/// # 示例
/// ```no_run
/// use aios_core::room::query_room_panels_by_keywords;
///
/// # async fn example() -> anyhow::Result<()> {
/// let keywords = vec!["-R-".to_string()];
/// let rooms = query_room_panels_by_keywords(&keywords).await?;
///
/// for (room_refno, room_num, panels) in rooms {
///     println!("房间 {}: {} 个面板", room_num, panels.len());
/// }
/// # Ok(())
/// # }
/// ```
///
/// # 性能提示
/// - 会自动过滤无效的 RefnoEnum
/// - 会自动跳过没有面板的房间
/// - 支持并发查询，性能较好
pub async fn query_room_panels_by_keywords(
    room_keywords: &Vec<String>,
) -> anyhow::Result<Vec<(RefnoEnum, String, Vec<RefnoEnum>)>> {
    use crate::types::RecordId;
    use itertools::Itertools;

    let start_time = Instant::now();

    // 构建 OR 条件过滤器
    if room_keywords.is_empty() {
        return Ok(vec![]);
    }
    let filter = room_keywords
        .iter()
        .map(|x| format!("'{}' in NAME", x))
        .join(" or ");

    info!(
        "开始查询房间面板: 关键词={:?}, 过滤条件={}",
        room_keywords, filter
    );

    // 根据项目类型选择查询语句
    #[cfg(feature = "project_hd")]
    let sql = format!(
        r#"
        select value [  id,
                        array::last(string::split(NAME, '-')),
                        array::flatten([REFNO<-pe_owner<-pe, REFNO<-pe_owner<-pe<-pe_owner<-pe])[?noun='PANE']
                    ] from FRMW where {filter}
    "#
    );

    #[cfg(not(feature = "project_hd"))]
    let sql = format!(
        r#"
        select value [  id,
                        array::last(string::split(NAME, '-')),
                        array::flatten([REFNO<-pe_owner<-pe])[?noun='PANE']
                    ] from SBFR where {filter}
    "#
    );

    debug!("执行 SQL 查询: {}", sql);

    // 执行查询
    let mut response = SUL_DB.query(sql).await.context("房间面板查询失败")?;
    let raw_result: Vec<(RecordId, String, Vec<RecordId>)> =
        response.take(0).context("解析查询结果失败")?;

    debug!("原始查询结果数: {}", raw_result.len());

    // 转换和过滤数据
    let room_groups: Vec<(RefnoEnum, String, Vec<RefnoEnum>)> = raw_result
        .into_iter()
        .filter_map(|(room_id, room_num, panel_ids)| {
            let room_refno = RefnoEnum::from(room_id);
            if !room_refno.is_valid() {
                warn!("跳过无效的房间 RefnoEnum: room_num={}", room_num);
                return None;
            }

            let panel_refnos: Vec<RefnoEnum> = panel_ids
                .into_iter()
                .filter_map(|id| {
                    let refno = RefnoEnum::from(id);
                    if refno.is_valid() { Some(refno) } else { None }
                })
                .collect();

            if panel_refnos.is_empty() {
                debug!("跳过没有面板的房间: room_num={}", room_num);
                return None;
            }

            Some((room_refno, room_num, panel_refnos))
        })
        .collect();

    let total_panels: usize = room_groups.iter().map(|(_, _, panels)| panels.len()).sum();

    info!(
        "房间面板查询完成: 找到 {} 个房间, {} 个面板, 耗时={:?}",
        room_groups.len(),
        total_panels,
        start_time.elapsed()
    );

    Ok(room_groups)
}

/// 查询与房间面板相交的元素（使用 SQLite 空间索引）
///
/// 通过空间索引查找与房间所有面板的 AABB 相交的元素。
///
/// # 参数
/// * `room_refno` - 房间 RefnoEnum
/// * `panel_refnos` - 房间的面板 RefnoEnum 列表
/// * `exclude_nouns` - 需要排除的元素类型（如 ["PANE", "FRMW", "SBFR"]）
///
/// # 返回
/// * `Vec<(RefU64, Aabb, Option<String>)>` - 元素列表（RefU64, AABB, noun）
///
/// # 示例
/// ```no_run
/// use aios_core::room::query_elements_in_room_by_spatial_index;
///
/// # async fn example() -> anyhow::Result<()> {
/// let room_refno = RefnoEnum::from_str("24381_34850")?;
/// let panel_refnos = vec![RefnoEnum::from_str("24381_34851")?];
/// let exclude = vec!["PANE".to_string(), "FRMW".to_string()];
///
/// let elements = query_elements_in_room_by_spatial_index(
///     &room_refno,
///     &panel_refnos,
///     &exclude
/// ).await?;
///
/// println!("找到 {} 个元素", elements.len());
/// # Ok(())
/// # }
/// ```
#[cfg(all(not(target_arch = "wasm32"), feature = "sqlite"))]
pub async fn query_elements_in_room_by_spatial_index(
    room_refno: &RefnoEnum,
    panel_refnos: &[RefnoEnum],
    exclude_nouns: &[String],
) -> anyhow::Result<Vec<(RefU64, Aabb, Option<String>)>> {
    use crate::spatial::sqlite;
    use std::collections::HashSet;

    let start_time = Instant::now();

    info!(
        "开始空间索引查询: 房间={}, 面板数={}, 排除类型={:?}",
        room_refno,
        panel_refnos.len(),
        exclude_nouns
    );

    if panel_refnos.is_empty() {
        warn!("面板列表为空，无法查询");
        return Ok(vec![]);
    }

    // 1. 查询所有面板的几何信息（AABB）
    let panel_keys: Vec<String> = panel_refnos.iter().map(|r| r.to_pe_key()).collect();

    let sql = format!("SELECT value [id, PXYZ] FROM [{}]", panel_keys.join(","));

    let mut response = SUL_DB.query(sql).await.context("查询面板几何信息失败")?;

    // 解析面板数据并计算 AABB
    let panel_aabbs = parse_panel_aabbs(&mut response).await?;

    debug!("成功获取 {} 个面板的 AABB", panel_aabbs.len());

    // 2. 使用 SQLite 空间索引查询与每个面板相交的元素
    let mut all_elements: HashSet<RefU64> = HashSet::new();
    let mut element_data: Vec<(RefU64, Aabb, Option<String>)> = Vec::new();

    for (panel_refno, panel_aabb) in &panel_aabbs {
        debug!("查询与面板 {} 相交的元素", panel_refno);

        // 稍微扩展一点 AABB 以包含边界上的元素
        let expanded_aabb = expand_aabb(panel_aabb, 0.1);

        // 查询与面板相交的元素
        match sqlite::query_overlap(&expanded_aabb, None, None, &[]) {
            Ok(elements) => {
                debug!("  找到 {} 个候选元素", elements.len());
                for (refno, aabb, noun) in elements {
                    // 排除指定类型
                    if let Some(ref noun_str) = noun {
                        if exclude_nouns.contains(noun_str) {
                            continue;
                        }
                    }

                    // 去重
                    if all_elements.insert(refno) {
                        element_data.push((refno, aabb, noun));
                    }
                }
            }
            Err(e) => {
                warn!("查询空间索引失败: {}", e);
            }
        }
    }

    info!(
        "空间索引查询完成: 房间={}, 找到 {} 个元素, 耗时={:?}",
        room_refno,
        element_data.len(),
        start_time.elapsed()
    );

    Ok(element_data)
}

/// 解析面板的 AABB（从查询结果）
#[cfg(all(not(target_arch = "wasm32"), feature = "sqlite"))]
async fn parse_panel_aabbs(
    response: &mut surrealdb::IndexedResults,
) -> anyhow::Result<Vec<(RefU64, Aabb)>> {
    use crate::types::RecordId;
    use nalgebra::Point3;

    // 使用元组类型反序列化 (id, PXYZ)
    let panels: Vec<(RecordId, Option<Vec<f64>>)> = response.take(0)?;
    let mut aabbs = Vec::new();

    for (record_id, pxyz_opt) in panels {
        let refno = RefU64::from(record_id);

        // 提取位置
        if let Some(pxyz_vec) = pxyz_opt {
            if pxyz_vec.len() >= 3 {
                let pxyz = Vec3::new(pxyz_vec[0] as f32, pxyz_vec[1] as f32, pxyz_vec[2] as f32);

                // 简单处理：使用位置创建一个小的 AABB
                // TODO: 实际应该根据面板的完整几何信息计算
                let min_point = Point3::new(pxyz.x - 0.1, pxyz.y - 0.1, pxyz.z - 0.1);
                let max_point = Point3::new(pxyz.x + 0.1, pxyz.y + 0.1, pxyz.z + 0.1);
                let aabb = Aabb::new(min_point, max_point);

                aabbs.push((refno, aabb));
            }
        }
    }

    Ok(aabbs)
}

/// 从 SurrealDB 数组中提取 Vec3
#[cfg(all(not(target_arch = "wasm32"), feature = "sqlite"))]
fn extract_vec3_from_array(arr: &[surrealdb::types::Value]) -> anyhow::Result<Vec3> {
    use surrealdb::types as surrealdb_types;

    if arr.len() < 3 {
        anyhow::bail!("数组长度不足 3");
    }

    let x = match &arr[0] {
        surrealdb_types::Value::Number(surrealdb_types::Number::Float(f)) => *f as f32,
        surrealdb_types::Value::Number(surrealdb_types::Number::Int(i)) => *i as f32,
        _ => anyhow::bail!("无法解析 x 坐标"),
    };

    let y = match &arr[1] {
        surrealdb_types::Value::Number(surrealdb_types::Number::Float(f)) => *f as f32,
        surrealdb_types::Value::Number(surrealdb_types::Number::Int(i)) => *i as f32,
        _ => anyhow::bail!("无法解析 y 坐标"),
    };

    let z = match &arr[2] {
        surrealdb_types::Value::Number(surrealdb_types::Number::Float(f)) => *f as f32,
        surrealdb_types::Value::Number(surrealdb_types::Number::Int(i)) => *i as f32,
        _ => anyhow::bail!("无法解析 z 坐标"),
    };

    Ok(Vec3::new(x, y, z))
}

/// 扩展 AABB
#[cfg(all(not(target_arch = "wasm32"), feature = "sqlite"))]
fn expand_aabb(aabb: &Aabb, margin: f32) -> Aabb {
    let mins = Point3::new(
        aabb.mins.x - margin,
        aabb.mins.y - margin,
        aabb.mins.z - margin,
    );
    let maxs = Point3::new(
        aabb.maxs.x + margin,
        aabb.maxs.y + margin,
        aabb.maxs.z + margin,
    );
    Aabb::new(mins, maxs)
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

    #[tokio::test]
    #[ignore = "需要真实数据库连接"]
    async fn test_query_room_panels_by_keywords_basic() {
        // 基本功能测试
        use crate::{get_db_option, init_surreal};

        init_surreal().await.expect("初始化数据库失败");
        let db_option = get_db_option();
        let keywords = db_option.get_room_key_word();

        let result = query_room_panels_by_keywords(&keywords).await;
        assert!(result.is_ok(), "查询应该成功");

        let rooms = result.unwrap();
        println!("找到 {} 个房间", rooms.len());

        for (room_refno, room_num, panels) in &rooms {
            println!(
                "  房间 {}: RefNo={}, 面板数={}",
                room_num,
                room_refno,
                panels.len()
            );
            assert!(room_refno.is_valid(), "房间 RefnoEnum 应该有效");
            assert!(!panels.is_empty(), "房间应该至少有一个面板");
        }
    }

    #[tokio::test]
    #[ignore = "需要真实数据库连接"]
    async fn test_query_room_panels_by_keywords_empty() {
        // 测试空关键词
        use crate::init_surreal;

        init_surreal().await.expect("初始化数据库失败");

        let keywords = vec![];
        let result = query_room_panels_by_keywords(&keywords).await;

        // 空关键词应该返回空结果或所有房间
        assert!(result.is_ok(), "查询应该成功");
    }

    #[tokio::test]
    #[ignore = "需要真实数据库连接"]
    async fn test_query_room_panels_by_keywords_multiple() {
        // 测试多个关键词
        use crate::init_surreal;

        init_surreal().await.expect("初始化数据库失败");

        let keywords = vec!["-R-".to_string(), "-RM-".to_string()];
        let result = query_room_panels_by_keywords(&keywords).await;

        assert!(result.is_ok(), "查询应该成功");

        let rooms = result.unwrap();
        println!("使用多个关键词找到 {} 个房间", rooms.len());

        // 验证数据完整性
        for (room_refno, room_num, panels) in &rooms {
            assert!(room_refno.is_valid(), "房间 RefnoEnum 应该有效");
            assert!(!room_num.is_empty(), "房间号不应为空");
            assert!(!panels.is_empty(), "面板列表不应为空");

            // 验证所有面板 RefnoEnum 都有效
            for panel in panels {
                assert!(panel.is_valid(), "面板 RefnoEnum 应该有效");
            }
        }
    }
}
