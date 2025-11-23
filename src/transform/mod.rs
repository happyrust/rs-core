//! Transform module for calculating local and world transforms for entities
//!
//! This module provides functions to calculate transforms (local positions and orientations)
//! for different entity types. It extracts functionality from the `get_world_transform` method
//! to calculate only the local transform of each node relative to its parent, which can then
//! be combined to get the world transform without recalculating from the root each time.

use crate::rs_surreal::spatial::{
    SectionEnd, cal_cutp_ori, cal_ori_by_opdir, cal_ori_by_ydir, cal_ori_by_z_axis_ref_x,
    cal_ori_by_z_axis_ref_y, cal_spine_orientation_basis, cal_spine_orientation_basis_with_ydir,
    cal_zdis_pkdi_in_section_by_spine, get_spline_path, get_spline_pts, query_pline,
};
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
    let target = if plax.length_squared() > 0.0 { plax.normalize() } else { standard_up };
    let source = if standard_up.length_squared() > 0.0 { standard_up.normalize() } else { Vec3::Z };
    let dot = source.dot(target).clamp(-1.0, 1.0);

    let rotation = if (1.0 - dot).abs() < 1e-6 {
        Quat::IDENTITY
    } else if (1.0 + dot).abs() < 1e-6 {
        let axis = if source.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
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
    strategy.get_local_transform(refno, parent_refno, &att, &parent_att).await
}

/// Calculate the world transform for an entity by combining local transforms
///
/// This function calculates the world transform by traversing the hierarchy
/// from the entity up to the root, calculating local transforms along the way,
/// and then combining them.
///
/// # Arguments
/// * `refno` - Reference number of the entity
///
/// # Returns
/// * `Ok(Some(Transform))` - The world transform if calculation succeeds
/// * `Ok(None)` - If the transform cannot be calculated
/// * `Err` - If an error occurs during calculation
#[cached(result = true)]
pub async fn get_world_transform(refno: RefnoEnum) -> anyhow::Result<Option<Transform>> {
    get_world_mat4(refno)
        .await
        .map(|m| m.map(|x| Transform::from_matrix(x.as_mat4())))
}

/// Calculate the world transformation matrix for an entity by combining local transforms
///
/// # Arguments
/// * `refno` - Reference number of the entity
///
/// # Returns
/// * `Ok(Some(DMat4))` - The world transformation matrix if calculation succeeds
/// * `Ok(None)` - If the transform cannot be calculated
/// * `Err` - If an error occurs during calculation
#[cached(result = true)]
pub async fn get_world_mat4(refno: RefnoEnum) -> anyhow::Result<Option<DMat4>> {
    // Get ancestors
    let ancestors: Vec<RefnoEnum> = crate::query_ancestor_refnos(refno).await?;

    if ancestors.len() <= 1 {
        return Ok(Some(DMat4::IDENTITY));
    }

    // Start with identity matrix
    let mut world_mat = DMat4::IDENTITY;

    // Traverse ancestors from root to leaf
    for i in 0..ancestors.len() - 1 {
        let parent_refno = ancestors[i];
        let child_refno = ancestors[i + 1];

        // Get local transform
        if let Some(local_mat) = get_local_mat4(child_refno, parent_refno).await? {
            // Combine with world transform
            world_mat = world_mat * local_mat;
        } else {
            return Ok(None);
        }
    }

    Ok(Some(world_mat))
}
