//! Transform module for calculating local and world transforms for entities
//!
//! This module provides functions to calculate transforms (local positions and orientations)
//! for different entity types. It extracts functionality from the `get_world_transform` method
//! to calculate only the local transform of each node relative to its parent, which can then
//! be combined to get the world transform without recalculating from the root each time.

use crate::rs_surreal::pe_transform::{
    PeTransformEntry, clear_pe_transform, ensure_pe_transform_schema, query_pe_transform,
    save_pe_transform, save_pe_transform_entries,
};
use crate::rs_surreal::spatial::is_virtual_node;
use crate::{
    DBType, NamedAttrMap, RefnoEnum, SUL_DB, SurrealQueryExt, get_children_refnos, get_db_option,
    get_mdb_world_site_ele_nodes, get_named_attmap,
    pdms_data::{PlinParam, PlinParamData},
    tool::{direction_parse::parse_expr_to_dir, math_tool::*},
};
use anyhow::anyhow;
use bevy_transform::prelude::*;
use cached::proc_macro::cached;
use glam::{DMat3, DMat4, DQuat, DVec3};

use glam::{Quat, Vec3};
use std::collections::VecDeque;

/// Compute a Transform that rotates from a standard up axis to the target PLAX.
/// This should be applied in geo_relate.trans (orientation layer), not at mesh time.
pub fn calculate_plax_transform(plax: Vec3, standard_up: Vec3) -> Transform {
    use std::f32::consts::PI;
    let target = if plax.length_squared() > 0.0 {
        plax.normalize()
    } else {
        standard_up
    };
    let source = if standard_up.length_squared() > 0.0 {
        standard_up.normalize()
    } else {
        Vec3::Z
    };
    let dot = source.dot(target).clamp(-1.0, 1.0);

    let rotation = if (1.0 - dot).abs() < 1e-6 {
        Quat::IDENTITY
    } else if (1.0 + dot).abs() < 1e-6 {
        let axis = if source.x.abs() < 0.9 {
            Vec3::X
        } else {
            Vec3::Y
        };
        Quat::from_axis_angle(axis, PI)
    } else {
        let axis = source.cross(target).normalize();
        let angle = source.angle_between(target);
        Quat::from_axis_angle(axis, angle)
    };

    Transform {
        translation: Vec3::ZERO,
        rotation,
        ..Default::default()
    }
}

/// Gets the local transform for an entity
///
/// This function calculates the local transform matrix for a given entity
/// using the transform strategy system.
///
/// # Arguments
/// * `refno` - Reference number of the entity
///
/// # Returns
/// * `Ok(Some(Transform))` - The local transform if calculation succeeds
/// * `Ok(None)` - If the transform cannot be calculated
/// * `Err` - If an error occurs during calculation
#[cached(result = true)]
pub async fn get_local_transform(refno: RefnoEnum) -> anyhow::Result<Option<Transform>> {
    get_transform_mat4(refno, true)
        .await
        .map(|m| m.map(|x| Transform::from_matrix(x.as_mat4())))
}

pub mod strategies;

use strategies::TransformStrategyFactory;

/// 递归获取有效的父节点属性，处理虚拟节点属性合并
///
/// 当父节点是虚拟节点（如SPINE）时，需要递归向上查找非虚拟祖先节点，
/// 并将非虚拟祖先的属性与虚拟节点属性合并（虚拟节点属性优先）。
///
/// # Arguments
/// * `parent_refno` - 父节点引用号
///
/// # Returns
/// * 合并后的属性映射
pub async fn get_effective_parent_att(parent_refno: RefnoEnum) -> anyhow::Result<NamedAttrMap> {
    let mut current_refno = parent_refno;
    let mut virtual_attrs: Vec<NamedAttrMap> = Vec::new();
    let mut depth = 0;
    const MAX_DEPTH: usize = 10; // 防止循环引用

    // 向上遍历，收集所有虚拟节点的属性
    while depth < MAX_DEPTH {
        let current_att = get_named_attmap(current_refno).await?;
        let current_type = current_att.get_type_str();

        if !is_virtual_node(current_type) {
            // 找到非虚拟节点，作为合并的基础
            let mut merged_att = current_att;

            // 反向合并虚拟节点属性（子节点属性优先）
            for attrs in virtual_attrs.iter().rev() {
                for (key, value) in attrs.iter() {
                    merged_att.insert(key.clone(), value.clone());
                }
            }

            return Ok(merged_att);
        }

        // 当前节点是虚拟节点，保存其属性并继续向上查找
        virtual_attrs.push(current_att);

        // 获取父节点
        let next_refno = virtual_attrs.last().unwrap().get_owner();
        if next_refno.is_unset() {
            // 没有更多父节点，返回最后一个虚拟节点的属性
            if let Some(last_att) = virtual_attrs.pop() {
                return Ok(last_att);
            } else {
                return Err(anyhow!("No valid parent attributes found"));
            }
        }

        current_refno = next_refno;
        depth += 1;
    }

    Err(anyhow!(
        "Maximum depth exceeded while searching for effective parent attributes"
    ))
}

/// Calculate the local transformation matrix for an entity relative to its parent
///
/// # Arguments
/// * `refno` - Reference number of the entity
/// * `parent_refno` - Reference number of the parent entity
///
/// # Returns
/// * `Ok(Some(DMat4))` - The local transformation matrix if calculation succeeds
/// * `Ok(None)` - If the transform cannot be calculated
/// * `Err` - If an error occurs during calculation
#[cached(result = true)]
pub async fn get_local_mat4(refno: RefnoEnum) -> anyhow::Result<Option<DMat4>> {
    // Get attribute maps for the entity and its parent
    let att = get_named_attmap(refno).await?;
    let parent_refno = att.get_owner();
    let parent_att = get_effective_parent_att(parent_refno).await?;

    // Use strategy factory to get the appropriate strategy
    let mut strategy = TransformStrategyFactory::get_strategy_from_ref(&att, &parent_att);
    strategy.get_local_transform().await
}

/// 获取变换矩阵（统一入口）
///
/// 此函数是获取本地变换和世界变换的统一入口点，使用策略系统（TransformStrategy）
/// 来计算变换矩阵，提供更好的可维护性和扩展性。
///
/// # Arguments
/// * `refno` - 目标构件的参考号
/// * `is_local` - 如果为 true，返回相对于父节点的局部变换；否则返回世界变换
///
/// # Returns
/// * `Ok(Some(DMat4))` - 计算得到的变换矩阵
/// * `Ok(None)` - 如果无法计算变换
/// * `Err` - 如果计算过程中发生错误
///
/// # 特性
/// - 使用策略模式支持不同构件类型的专门计算逻辑
/// - 与重构后的 `get_local_mat4` 函数集成
/// - 支持缓存优化（从 pe_transform 表读取/写入缓存）
pub async fn get_transform_mat4(refno: RefnoEnum, is_local: bool) -> anyhow::Result<Option<DMat4>> {
    // 尝试从缓存获取，如果查询失败则继续惰性计算
    let cache = query_pe_transform(refno).await.ok().flatten();
    let cached_local = cache.as_ref().and_then(|c| c.local.clone());
    let cached_world = cache.as_ref().and_then(|c| c.world.clone());

    if is_local {
        if let Some(local) = cached_local {
            let mat4 = bevy_transform_to_dmat4(&local);
            #[cfg(feature = "debug_spatial")]
            println!("🎯 Cache hit for pe_transform.local: {}", refno);
            return Ok(Some(mat4));
        }
    } else if let Some(world) = cached_world {
        let mat4 = bevy_transform_to_dmat4(&world);
        #[cfg(feature = "debug_spatial")]
        println!("🎯 Cache hit for pe_transform.world: {}", refno);
        return Ok(Some(mat4));
    }

    // 兜底：直接从 PE 记录读取 world_trans（某些库/节点 pe_transform 缓存未预热或策略计算失败时使用）。
    // 这样可以避免 PRIM/BOX 等在模型生成时因为 world_transform=None/Err 而被跳过。
    if !is_local {
        if let Some(world) = crate::rs_surreal::query_pe_world_trans(refno).await.ok().flatten() {
            let mat4 = bevy_transform_to_dmat4(&world);
            let refno_clone = refno;
            let world_clone = world.clone();
            tokio::spawn(async move {
                let _ = save_pe_transform(refno_clone, None, Some(world_clone)).await;
            });
            return Ok(Some(mat4));
        }
    }

    let local_mat = match cached_local {
        Some(local) => Some(bevy_transform_to_dmat4(&local)),
        None => get_local_mat4(refno).await?,
    };
    let world_mat = if is_local {
        None
    } else {
        compute_world_from_parent(refno, local_mat).await?
    };

    let local_trans = dmat4_to_transform_option(local_mat);
    let world_trans = dmat4_to_transform_option(world_mat);

    if local_trans.is_some() || world_trans.is_some() {
        let refno_clone = refno;
        tokio::spawn(async move {
            let _ = save_pe_transform(refno_clone, local_trans, world_trans).await;
            #[cfg(feature = "debug_spatial")]
            println!("💾 Cached pe_transform for: {}", refno_clone);
        });
    }

    Ok(if is_local { local_mat } else { world_mat })
}

/// 惰性计算世界变换矩阵
///
/// 当父节点没有缓存时，向上查找最近有 world_trans 缓存的祖先，
/// 然后从该祖先开始逐层累乘 local_mat 得到世界变换。
async fn compute_world_from_parent(
    refno: RefnoEnum,
    local_mat: Option<DMat4>,
) -> anyhow::Result<Option<DMat4>> {
    use crate::rs_surreal::{find_nearest_cached_ancestor, query_ancestor_refnos};

    let att = get_named_attmap(refno).await?;
    let parent_refno = att.get_owner();
    if parent_refno.is_unset() {
        // 根节点，直接返回 local_mat
        return Ok(Some(local_mat.unwrap_or(DMat4::IDENTITY)));
    }

    // 尝试直接从父节点缓存获取（快速路径）
    let mut parent_world = query_pe_transform(parent_refno).await?.and_then(|c| c.world);
    if parent_world.is_none() {
        // pe_transform 未命中时，优先使用 pe.world_trans（若存在），并回灌 pe_transform。
        parent_world = crate::rs_surreal::query_pe_world_trans(parent_refno).await.ok().flatten();
        if let Some(w) = parent_world.clone() {
            let r = parent_refno;
            tokio::spawn(async move {
                let _ = save_pe_transform(r, None, Some(w)).await;
            });
        }
    }
    if let Some(parent_world) = parent_world {
        let parent_mat = bevy_transform_to_dmat4(&parent_world);
        return Ok(Some(match local_mat {
            Some(local) => parent_mat * local,
            None => parent_mat,
        }));
    }

    // 父节点无缓存，启用惰性计算：查找最近有缓存的祖先
    #[cfg(feature = "debug_spatial")]
    println!(
        "🔄 惰性计算 world_mat for {}: 父节点 {} 无缓存",
        refno, parent_refno
    );

    // 获取祖先链并反转（query_ancestor_refnos 返回从当前到根，需要反转为从根到父）
    // 注意：query_ancestor_refnos 包含当前节点自己，需要排除
    let mut ancestors = query_ancestor_refnos(refno).await?;
    ancestors.retain(|r| *r != refno); // 排除当前节点
    ancestors.reverse(); // 反转为从根到父的顺序
    if ancestors.is_empty() {
        return Ok(Some(local_mat.unwrap_or(DMat4::IDENTITY)));
    }

    // 查找最近有 world_trans 缓存的祖先
    let cached_ancestor = find_nearest_cached_ancestor(refno).await?;
    let ancestors_len = ancestors.len();

    // 确定计算起点（注意：find_nearest_cached_ancestor 返回的索引是原始顺序，需要转换为反转后的索引）
    let (start_idx, mut world_mat) = match cached_ancestor {
        Some((orig_idx, ancestor_refno)) => {
            let mut start_world =
                query_pe_transform(ancestor_refno).await?.and_then(|c| c.world);
            if start_world.is_none() {
                start_world =
                    crate::rs_surreal::query_pe_world_trans(ancestor_refno).await.ok().flatten();
                if let Some(w) = start_world.clone() {
                    let r = ancestor_refno;
                    tokio::spawn(async move {
                        let _ = save_pe_transform(r, None, Some(w)).await;
                    });
                }
            }
            let world = start_world
                .as_ref()
                .map(bevy_transform_to_dmat4)
                .unwrap_or(DMat4::IDENTITY);
            // 原始索引是从当前到根，反转后索引 = len - 1 - orig_idx
            let reversed_idx = ancestors_len - 1 - orig_idx;
            #[cfg(feature = "debug_spatial")]
            println!(
                "  ✅ 找到缓存祖先: {} (orig_idx={}, reversed_idx={})",
                ancestor_refno, orig_idx, reversed_idx
            );
            (reversed_idx + 1, world) // 从缓存祖先的下一个开始计算
        }
        None => {
            #[cfg(feature = "debug_spatial")]
            println!("  ⚠️ 无缓存祖先，从根节点开始计算");
            (0, DMat4::IDENTITY) // 从根节点开始
        }
    };

    // 收集需要写入缓存的中间节点
    let mut entries_to_save: Vec<PeTransformEntry> = Vec::new();

    // 从起点逐层累乘 local_mat
    for (i, ancestor) in ancestors.iter().enumerate().skip(start_idx) {
        let ancestor_local = get_local_mat4(*ancestor).await?.unwrap_or(DMat4::IDENTITY);
        world_mat = world_mat * ancestor_local;

        // 沿途缓存（提升后续命中率）
        let world_trans = dmat4_to_transform_option(Some(world_mat));
        let local_trans = dmat4_to_transform_option(Some(ancestor_local));
        if world_trans.is_some() {
            entries_to_save.push(PeTransformEntry {
                refno: *ancestor,
                local: local_trans,
                world: world_trans,
            });
        }
    }

    // 最后乘上当前节点的 local_mat
    let result = match local_mat {
        Some(local) => world_mat * local,
        None => world_mat,
    };

    // 异步保存沿途缓存
    if !entries_to_save.is_empty() {
        tokio::spawn(async move {
            let _ = save_pe_transform_entries(&entries_to_save).await;
            #[cfg(feature = "debug_spatial")]
            println!("💾 惰性计算：已缓存 {} 个中间节点", entries_to_save.len());
        });
    }

    Ok(Some(result))
}

/// 获取世界变换矩阵（向后兼容别名）
///
/// 此函数是 `get_transform_mat4` 的别名，为了保持向后兼容性而保留。
/// 新代码建议直接使用 `get_transform_mat4`。
///
/// # Arguments
/// * `refno` - 目标构件的参考号
/// * `is_local` - 如果为 true，返回相对于父节点的局部变换；否则返回世界变换
#[inline]
pub async fn get_world_mat4(refno: RefnoEnum, is_local: bool) -> anyhow::Result<Option<DMat4>> {
    get_transform_mat4(refno, is_local).await
}

/// 将 Bevy Transform 转换为 DMat4
///
/// # 参数
/// * `transform` - Bevy Transform 对象
///
/// # 返回值
/// 对应的 4x4 变换矩阵
fn bevy_transform_to_dmat4(transform: &Transform) -> DMat4 {
    DMat4::from_scale_rotation_translation(
        transform.scale.as_dvec3(),
        transform.rotation.as_dquat(),
        transform.translation.as_dvec3(),
    )
}

/// 将 DMat4 转换为 Bevy Transform
///
/// # 参数
/// * `mat4` - 4x4 变换矩阵
///
/// # 返回值
/// 对应的 Bevy Transform 对象
fn dmat4_to_bevy_transform(mat4: &DMat4) -> Transform {
    let (scale, rotation, translation) = mat4.to_scale_rotation_translation();
    Transform {
        translation: translation.as_vec3(),
        rotation: rotation.as_quat(),
        scale: scale.as_vec3(),
    }
}

fn dmat4_to_transform_option(mat4: Option<DMat4>) -> Option<Transform> {
    mat4.and_then(|m| {
        if m.is_nan() {
            None
        } else {
            Some(dmat4_to_bevy_transform(&m))
        }
    })
}

/// 清除指定 refno 的世界变换缓存
///
/// 当元件的位置或方向属性（POS、ORI等）发生变化时，需要调用此函数清除缓存
///
/// # 参数
/// * `refno` - 要清除缓存的参考号
///
/// # 返回值
/// * `Ok(())` - 成功清除缓存
/// * `Err` - 如果清除过程中发生错误
pub async fn invalidate_world_trans_cache(refno: RefnoEnum) -> anyhow::Result<()> {
    clear_pe_transform(refno).await?;
    #[cfg(feature = "debug_spatial")]
    println!("🗑️  Invalidated pe_transform cache for: {}", refno);
    Ok(())
}

/// 刷新 MDB(DESI) 下的 pe_transform 缓存（包含 SITE）
///
/// # 参数
/// * `mdb` - 可选 MDB 名称（None 则使用 DbOption.toml）
///
/// # 返回值
/// * 处理的节点数量
pub async fn refresh_pe_transform_for_mdb(mdb: Option<String>) -> anyhow::Result<usize> {
    ensure_pe_transform_schema().await?;
    let mdb_name = mdb.unwrap_or_else(|| get_db_option().mdb_name.clone());

    // 查询该 MDB 下的总节点数
    let count_sql = format!(
        "SELECT VALUE count() FROM pe WHERE mdb = '{}' GROUP ALL",
        mdb_name
    );
    let mut count_response = SUL_DB.query_response(&count_sql).await?;
    let total_nodes: Vec<i64> = count_response.take(0)?;
    let total_nodes = total_nodes.first().copied().unwrap_or(0) as usize;

    println!("📊 MDB {} 总节点数: {}", mdb_name, total_nodes);

    let sites = get_mdb_world_site_ele_nodes(mdb_name, DBType::DESI).await?;
    if sites.is_empty() {
        return Ok(0);
    }

    const BATCH_SIZE: usize = 500;
    let mut entries: Vec<PeTransformEntry> = Vec::with_capacity(BATCH_SIZE);
    let mut total = 0usize;
    let mut last_print_count = 0usize;

    fn push_entry(
        entries: &mut Vec<PeTransformEntry>,
        total: &mut usize,
        refno: RefnoEnum,
        local_mat: Option<DMat4>,
        world_mat: Option<DMat4>,
    ) {
        let local = dmat4_to_transform_option(local_mat);
        let world = dmat4_to_transform_option(world_mat);
        if local.is_none() && world.is_none() {
            return;
        }
        entries.push(PeTransformEntry {
            refno,
            local,
            world,
        });
        *total += 1;
    }

    for site in sites {
        let site_refno = site.refno;
        let mut queue: VecDeque<(RefnoEnum, DMat4)> = VecDeque::new();

        let local_mat = match get_local_mat4(site_refno).await {
            Ok(mat) => mat.filter(|m| !m.is_nan()),
            Err(e) => {
                #[cfg(feature = "debug_spatial")]
                eprintln!("刷新 SITE 本地变换失败: {} -> {}", site_refno, e);
                None
            }
        };
        let world_mat = local_mat.unwrap_or(DMat4::IDENTITY);
        push_entry(
            &mut entries,
            &mut total,
            site_refno,
            local_mat,
            Some(world_mat),
        );
        queue.push_back((site_refno, world_mat));

        while let Some((parent_refno, parent_world)) = queue.pop_front() {
            let children = match get_children_refnos(parent_refno).await {
                Ok(children) => children,
                Err(e) => {
                    #[cfg(feature = "debug_spatial")]
                    eprintln!("获取子节点失败: {} -> {}", parent_refno, e);
                    continue;
                }
            };

            for child in children {
                let local_mat = match get_local_mat4(child).await {
                    Ok(mat) => mat.filter(|m| !m.is_nan()),
                    Err(e) => {
                        #[cfg(feature = "debug_spatial")]
                        eprintln!("刷新本地变换失败: {} -> {}", child, e);
                        None
                    }
                };
                let world_mat = match local_mat {
                    Some(local) => parent_world * local,
                    None => parent_world,
                };
                push_entry(&mut entries, &mut total, child, local_mat, Some(world_mat));
                queue.push_back((child, world_mat));

                // 每处理 10 个节点更新一次进度
                if total - last_print_count >= 10 {
                    let percentage = if total_nodes > 0 {
                        (total as f64 / total_nodes as f64 * 100.0) as usize
                    } else {
                        0
                    };
                    print!(
                        "\r📊 进度: {}/{} ({:3}%)...",
                        total, total_nodes, percentage
                    );
                    use std::io::Write;
                    std::io::stdout().flush().ok();
                    last_print_count = total;
                }

                if entries.len() >= BATCH_SIZE {
                    save_pe_transform_entries(&entries).await?;
                    entries.clear();
                    // 批量保存时也更新进度
                    let percentage = if total_nodes > 0 {
                        (total as f64 / total_nodes as f64 * 100.0) as usize
                    } else {
                        0
                    };
                    print!(
                        "\r📊 进度: {}/{} ({:3}%) [已保存批次]...",
                        total, total_nodes, percentage
                    );
                    use std::io::Write;
                    std::io::stdout().flush().ok();
                    last_print_count = total;
                }
            }
        }
    }

    if !entries.is_empty() {
        save_pe_transform_entries(&entries).await?;
    }

    // 打印最终完成信息（带换行）
    println!("\r✅ 完成！共处理 {} 个节点                    ", total);

    Ok(total)
}

/// 刷新指定 dbnum 列表的 pe_transform 缓存
///
/// # 参数
/// * `dbnums` - 数据库编号列表 (如 &[1112, 7997])
///
/// # 返回值
/// * 处理的节点数量
///
/// # 示例
/// ```
/// let count = refresh_pe_transform_for_dbnums(&[1112]).await?;
/// ```
pub async fn refresh_pe_transform_for_dbnums(dbnums: &[u32]) -> anyhow::Result<usize> {
    ensure_pe_transform_schema().await?;

    const BATCH_SIZE: usize = 500;
    let mut entries: Vec<PeTransformEntry> = Vec::with_capacity(BATCH_SIZE);
    let mut total = 0usize;

    fn push_entry(
        entries: &mut Vec<PeTransformEntry>,
        total: &mut usize,
        refno: RefnoEnum,
        local_mat: Option<DMat4>,
        world_mat: Option<DMat4>,
    ) {
        let local = dmat4_to_transform_option(local_mat);
        let world = dmat4_to_transform_option(world_mat);
        if local.is_none() && world.is_none() {
            return;
        }
        entries.push(PeTransformEntry {
            refno,
            local,
            world,
        });
        *total += 1;
    }

    if dbnums.is_empty() {
        println!("⚠️  未提供 dbnum 列表");
        return Ok(0);
    }

    println!("📋 刷新 dbnums: {:?}", dbnums);

    // 对每个 dbnum，查询其根节点并处理子树
    for dbnum in dbnums {
        // 先查询该 dbnum 下的总节点数
        let count_sql = format!(
            "SELECT VALUE count() FROM pe WHERE dbnum = {} GROUP ALL",
            dbnum
        );
        let mut count_response = SUL_DB.query_response(&count_sql).await?;
        let total_nodes: Vec<i64> = count_response.take(0)?;
        let total_nodes = total_nodes.first().copied().unwrap_or(0) as usize;

        println!("📊 dbnum {} 总节点数: {}", dbnum, total_nodes);

        // 查询该 dbnum 下的所有根节点（通常是 SITE 或 WORL）
        // 使用 SELECT VALUE 直接返回 refno 值列表
        let sql = format!(
            "SELECT VALUE refno FROM pe WHERE dbnum = {} AND (noun = 'SITE' OR noun = 'WORL') AND owner.refno = NONE",
            dbnum
        );

        let mut response = SUL_DB.query_response(&sql).await?;
        let roots: Vec<RefnoEnum> = response.take(0)?;

        if roots.is_empty() {
            println!("⚠️  dbnum {} 没有找到根节点", dbnum);
            continue;
        }

        println!("🔍 处理 dbnum {}, 找到 {} 个根节点", dbnum, roots.len());

        // 当前 dbnum 的局部计数器（用于进度显示）
        let mut dbnum_processed = 0usize;
        let mut dbnum_last_print = 0usize;

        for root_refno in roots {
            let mut queue: VecDeque<(RefnoEnum, DMat4)> = VecDeque::new();

            let local_mat = match get_local_mat4(root_refno).await {
                Ok(mat) => mat.filter(|m| !m.is_nan()),
                Err(e) => {
                    #[cfg(feature = "debug_spatial")]
                    eprintln!("刷新根节点本地变换失败: {} -> {}", root_refno, e);
                    None
                }
            };
            let world_mat = local_mat.unwrap_or(DMat4::IDENTITY);
            push_entry(
                &mut entries,
                &mut total,
                root_refno,
                local_mat,
                Some(world_mat),
            );
            dbnum_processed += 1;
            queue.push_back((root_refno, world_mat));

            while let Some((parent_refno, parent_world)) = queue.pop_front() {
                let children = match get_children_refnos(parent_refno).await {
                    Ok(children) => children,
                    Err(e) => {
                        #[cfg(feature = "debug_spatial")]
                        eprintln!("获取子节点失败: {} -> {}", parent_refno, e);
                        continue;
                    }
                };

                for child in children {
                    let local_mat = match get_local_mat4(child).await {
                        Ok(mat) => mat.filter(|m| !m.is_nan()),
                        Err(e) => {
                            #[cfg(feature = "debug_spatial")]
                            eprintln!("刷新本地变换失败: {} -> {}", child, e);
                            None
                        }
                    };
                    let world_mat = match local_mat {
                        Some(local) => parent_world * local,
                        None => parent_world,
                    };
                    push_entry(&mut entries, &mut total, child, local_mat, Some(world_mat));
                    dbnum_processed += 1;
                    queue.push_back((child, world_mat));

                    // 每处理 10 个节点更新一次进度
                    if dbnum_processed - dbnum_last_print >= 10 {
                        let percentage = if total_nodes > 0 {
                            (dbnum_processed as f64 / total_nodes as f64 * 100.0) as usize
                        } else {
                            0
                        };
                        print!(
                            "\r📊 进度: {}/{} ({:3}%)...",
                            dbnum_processed, total_nodes, percentage
                        );
                        use std::io::Write;
                        std::io::stdout().flush().ok();
                        dbnum_last_print = dbnum_processed;
                    }

                    if entries.len() >= BATCH_SIZE {
                        save_pe_transform_entries(&entries).await?;
                        entries.clear();
                        // 批量保存时也更新进度
                        let percentage = if total_nodes > 0 {
                            (dbnum_processed as f64 / total_nodes as f64 * 100.0) as usize
                        } else {
                            0
                        };
                        print!(
                            "\r📊 进度: {}/{} ({:3}%) [已保存批次]...",
                            dbnum_processed, total_nodes, percentage
                        );
                        use std::io::Write;
                        std::io::stdout().flush().ok();
                        dbnum_last_print = dbnum_processed;
                    }
                }
            }
        }
        // 当前 dbnum 处理完成，打印换行
        println!();
    }

    if !entries.is_empty() {
        save_pe_transform_entries(&entries).await?;
    }

    // 打印最终完成信息（带换行）
    println!("\r✅ 完成！共处理 {} 个节点                    ", total);

    Ok(total)
}
