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

/// Calculate the local transform for an entity relative to its parent
///
/// # Arguments
/// * `refno` - Reference number of the entity
/// * `parent_refno` - Reference number of the parent entity
///
/// # Returns
/// * `Ok(Some(Transform))` - The local transform if calculation succeeds
/// * `Ok(None)` - If the transform cannot be calculated
/// * `Err` - If an error occurs during calculation
#[cached(result = true)]
pub async fn get_local_transform(
    refno: RefnoEnum,
    parent_refno: RefnoEnum,
) -> anyhow::Result<Option<Transform>> {
    get_local_mat4(refno, parent_refno)
        .await
        .map(|m| m.map(|x| Transform::from_matrix(x.as_mat4())))
}

pub mod strategies;

use strategies::TransformStrategyFactory;

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
pub async fn get_local_mat4(
    refno: RefnoEnum,
    parent_refno: RefnoEnum,
) -> anyhow::Result<Option<DMat4>> {
    // Get attribute maps for the entity and its parent
    let att = get_named_attmap(refno).await?;
    let parent_att = get_named_attmap(parent_refno).await?;

    let cur_type = att.get_type_str();

    // Use strategy factory to get the appropriate strategy
    let strategy = TransformStrategyFactory::get_strategy(cur_type);
    strategy
        .get_local_transform(refno, parent_refno, &att, &parent_att)
        .await
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
/// - 支持缓存优化
/// - 生产安全的特性标志回退机制
pub async fn get_world_mat4(
    refno: RefnoEnum,
    is_local: bool,
) -> anyhow::Result<Option<DMat4>> {
    // 新的策略系统实现
    get_world_mat4_with_strategies_impl(refno, is_local).await
}


/// 新策略系统的具体实现
///
/// 此函数包含使用策略模式的世界矩阵计算逻辑
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
            return get_local_mat4(cur_refno, parent_refno).await;
        }
        return Ok(Some(DMat4::IDENTITY));
    }

    // 遍历祖先链，累加所有局部变换
    let mut mat4 = DMat4::IDENTITY;
    
    for i in 1..ancestors.len() {
        let cur_refno = ancestors[i].get_refno_or_default();
        let parent_refno = ancestors[i-1].get_refno_or_default();
        
        match get_local_mat4(cur_refno, parent_refno).await {
            Ok(Some(local_mat)) => {
                mat4 = mat4 * local_mat;
            },
            Ok(None) => {
                #[cfg(feature = "debug_spatial")]
                println!("DEBUG: No transform calculated for {} -> {}", parent_refno, cur_refno);
            },
            Err(e) => {
                #[cfg(feature = "debug_spatial")]
                println!("DEBUG: Error calculating transform for {} -> {}: {}", parent_refno, cur_refno, e);
            }
        }
    }

    if mat4.is_nan() {
        return Ok(None);
    }

    Ok(Some(mat4))
}