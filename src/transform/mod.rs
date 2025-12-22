//! Transform module for calculating local and world transforms for entities
//!
//! This module provides functions to calculate transforms (local positions and orientations)
//! for different entity types. It extracts functionality from the `get_world_transform` method
//! to calculate only the local transform of each node relative to its parent, which can then
//! be combined to get the world transform without recalculating from the root each time.

use crate::rs_surreal::spatial::*;
use crate::{
    NamedAttrMap, RefnoEnum, SUL_DB, get_named_attmap,
    pdms_data::{PlinParam, PlinParamData},
    tool::{direction_parse::parse_expr_to_dir, math_tool::*},
};
use anyhow::anyhow;
use bevy_transform::prelude::*;
use cached::proc_macro::cached;
use glam::{DMat3, DMat4, DQuat, DVec3};

use glam::{Quat, Vec3};

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
    get_local_mat4(refno)
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

/// 使用策略模式重构的世界矩阵计算函数
///
/// 这是 `get_world_mat4` 的重构版本，使用新的策略系统（TransformStrategy）
/// 来计算变换矩阵，提供更好的可维护性和扩展性。
///
/// # 特性标志
///
/// 此函数的行为受 `use_strategy_transform` 特性标志控制：
/// - **启用时**：使用新的策略系统
/// - **禁用时**：回退到旧的 `get_world_mat4` 实现
///
/// 默认情况下该特性是关闭的（opt-in 迁移策略），需要显式启用：
/// ```bash
/// cargo run --features use_strategy_transform
/// ```
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
/// - 保持与原函数相同的 API 接口
/// - 支持缓存优化（从 PE 表的 world_trans 字段读取/写入缓存）
/// - 生产安全的特性标志回退机制
pub async fn get_world_mat4(refno: RefnoEnum, is_local: bool) -> anyhow::Result<Option<DMat4>> {
    // 如果不是 local 模式，先尝试从数据库缓存读取
    if !is_local {
        if let Ok(Some(pe)) = crate::get_pe(refno).await {
            if let Some(world_trans) = pe.world_trans {
                // 从 PlantTransform 转换为 DMat4
                let mat4 = bevy_transform_to_dmat4(&world_trans.0);
                #[cfg(feature = "debug_spatial")]
                println!("🎯 Cache hit for world_trans: {}", refno);
                return Ok(Some(mat4));
            }
        }
    }

    // 缓存未命中，计算世界变换矩阵
    let result = get_world_mat4_with_strategies_impl(refno, is_local).await?;

    // 如果计算成功且不是 local 模式，缓存结果到数据库
    if !is_local {
        if let Some(mat4) = result {
            let transform = dmat4_to_bevy_transform(&mat4);
            let plant_trans = crate::rs_surreal::PlantTransform(transform);

            // 异步更新 PE 表的 world_trans 字段（不阻塞返回）
            let refno_clone = refno;
            tokio::spawn(async move {
                let sql = format!(
                    "UPDATE {} SET world_trans = $trans",
                    refno_clone.to_pe_key()
                );
                let _ = SUL_DB.query(&sql)
                    .bind(("trans", plant_trans))
                    .await;
                #[cfg(feature = "debug_spatial")]
                println!("💾 Cached world_trans for: {}", refno_clone);
            });
        }
    }

    Ok(result)
}

/// 新策略系统的具体实现
///
/// 此函数包含使用策略模式的世界矩阵计算逻辑
///
/// # 优化策略
/// - 在遍历祖先链时，检查是否有祖先节点已缓存 world_trans
/// - 如果找到缓存的祖先，从该点开始计算，避免重复计算
async fn get_world_mat4_with_strategies_impl(
    refno: RefnoEnum,
    is_local: bool,
) -> anyhow::Result<Option<DMat4>> {
    #[cfg(feature = "profile")]
    let start_ancestors = std::time::Instant::now();

    let mut ancestors: Vec<NamedAttrMap> = super::get_ancestor_attmaps(refno).await?;

    #[cfg(feature = "profile")]
    let elapsed_ancestors = start_ancestors.elapsed();
    #[cfg(feature = "profile")]
    println!("get_ancestor_attmaps took {:?}", elapsed_ancestors);

    #[cfg(feature = "profile")]
    let start_refnos = std::time::Instant::now();
    let ancestor_refnos = crate::query_ancestor_refnos(refno).await?;
    #[cfg(feature = "profile")]
    let elapsed_refnos = start_refnos.elapsed();
    #[cfg(feature = "profile")]
    println!("query_ancestor_refnos took {:?}", elapsed_refnos);

    // 检查 ancestors 是否包含 self，如果不包含则添加
    // get_ancestor_attmaps 通常返回 [Parent, GrandParent, ... Root]
    // 我们需要将其补充为 [Self, Parent, ... Root]
    let has_self = ancestors.iter().any(|a| a.get_refno_or_default() == refno);
    if !has_self {
        let self_att = get_named_attmap(refno).await?;
        ancestors.insert(0, self_att);
    }

    if ancestor_refnos.len() <= 1 {
        return Ok(Some(DMat4::IDENTITY));
    }

    ancestors.reverse();

    // 如果只需要局部变换，直接调用 get_local_mat4
    if is_local {
        if ancestors.len() >= 2 {
            let parent_refno = ancestors[ancestors.len() - 2].get_refno_or_default();
            let cur_refno = ancestors.last().unwrap().get_refno_or_default();
            return get_local_mat4(cur_refno).await;
        }
        return Ok(Some(DMat4::IDENTITY));
    }

    // 优化：查找祖先链中最近的有缓存 world_trans 的节点
    let mut start_index = 0;
    let mut mat4 = DMat4::IDENTITY;

    #[cfg(feature = "profile")]
    let cache_search_start = std::time::Instant::now();

    // 从最接近目标节点的祖先开始查找（逆序遍历，从后往前）
    for i in (1..ancestors.len()).rev() {
        let ancestor_refno = ancestors[i].get_refno_or_default();

        // 尝试从数据库读取该祖先的缓存
        if let Ok(Some(pe)) = crate::get_pe(ancestor_refno).await {
            if let Some(world_trans) = pe.world_trans {
                // 找到缓存！使用这个作为起点
                mat4 = bevy_transform_to_dmat4(&world_trans.0);
                start_index = i;
                #[cfg(feature = "debug_spatial")]
                println!("🎯 Found cached world_trans at ancestor[{}]: {}", i, ancestor_refno);
                break;
            }
        }
    }

    #[cfg(feature = "profile")]
    {
        let cache_search_elapsed = cache_search_start.elapsed();
        println!("Cache search took {:?}, start_index={}", cache_search_elapsed, start_index);
    }

    // 从找到的缓存点（或根节点）开始，累加到目标节点的局部变换
    for i in (start_index + 1)..ancestors.len() {
        let cur_refno = ancestors[i].get_refno_or_default();
        let parent_refno = ancestors[i - 1].get_refno_or_default();

        match get_local_mat4(cur_refno).await {
            Ok(Some(local_mat)) => {
                mat4 = mat4 * local_mat;
            }
            Ok(None) => {
                #[cfg(feature = "debug_spatial")]
                println!(
                    "DEBUG: No transform calculated for {} -> {}",
                    parent_refno, cur_refno
                );
            }
            Err(e) => {
                #[cfg(feature = "debug_spatial")]
                println!(
                    "DEBUG: Error calculating transform for {} -> {}: {}",
                    parent_refno, cur_refno, e
                );
            }
        }
    }

    if mat4.is_nan() {
        return Ok(None);
    }

    Ok(Some(mat4))
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
    let sql = format!(
        "UPDATE {} SET world_trans = NONE",
        refno.to_pe_key()
    );
    SUL_DB.query(&sql).await?;
    #[cfg(feature = "debug_spatial")]
    println!("🗑️  Invalidated world_trans cache for: {}", refno);
    Ok(())
}
