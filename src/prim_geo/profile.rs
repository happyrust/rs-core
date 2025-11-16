use std::default;
use std::f32::consts::{FRAC_PI_2, PI};

use crate::parsed_data::geo_params_data::{CateGeoParam, PdmsGeoParam};
use crate::parsed_data::{CateGeomsInfo, CateProfileParam};
use crate::pdms_types::*;
use crate::prim_geo::category::CateCsgShape;
use crate::prim_geo::spine::{
    Arc3D, Line3D, SegmentPath, Spine3D, SpineCurveType, SweepPath3D, circum_center,
};
use crate::prim_geo::{CateCsgShapeMap, SweepSolid};
use crate::shape::pdms_shape::BrepShapeTrait;
use crate::tool::dir_tool::parse_ori_str_to_quat;
use crate::tool::float_tool::{f32_round_3, vec3_round_3};
use crate::tool::math_tool::{
    dquat_to_pdms_ori_xyz_str, quat_to_pdms_ori_str, to_pdms_ori_str, to_pdms_vec_str,
};
use crate::{RefU64, get_world_transform};
use anyhow::anyhow;
use bevy_transform::prelude::Transform;
use dashmap::{DashMap, DashSet};
use glam::{DMat4, DQuat, DVec3, Mat3, Quat, Vec3};
use std::vec::Vec;

/// 将多个 Spine3D 段连接成连续的可序列化路径
///
/// 参数：
/// - segments: Spine3D 段的列表
///
/// 返回：
/// - Ok(Vec<SegmentPath>): 转换后的连续路径段列表
/// - Err: 如果段不连续或转换失败
fn connect_spine_segments(segments: Vec<Spine3D>) -> anyhow::Result<Vec<SegmentPath>> {
    const EPSILON: f32 = 1e-3;
    let mut result = Vec::new();

    if segments.is_empty() {
        return Ok(result);
    }

    for (i, spine) in segments.iter().enumerate() {
        // 验证连续性（除了第一段）
        if i > 0 {
            let prev_end = if i == 1 {
                segments[0].pt1
            } else {
                segments[i - 1].pt1
            };

            let curr_start = spine.pt0;
            let distance = prev_end.distance(curr_start);

            if distance > EPSILON {
                tracing::warn!(
                    "Spine 段不连续: 段 {} 到段 {} 的距离为 {:.6}",
                    i - 1,
                    i,
                    distance
                );
                // 不返回错误，继续处理
            }
        }

        // 根据曲线类型转换为 SegmentPath
        match spine.curve_type {
            SpineCurveType::LINE => {
                result.push(SegmentPath::Line(Line3D {
                    start: spine.pt0,
                    end: spine.pt1,
                    is_spine: true,
                }));
            }
            SpineCurveType::THRU => {
                // 通过三点确定圆弧
                let center = circum_center(spine.pt0, spine.pt1, spine.thru_pt);
                let vec0 = spine.pt0 - spine.thru_pt;
                let vec1 = spine.pt1 - spine.thru_pt;
                let angle = (PI - vec0.angle_between(vec1)) * 2.0;
                let axis = vec1.cross(vec0).normalize();

                result.push(SegmentPath::Arc(Arc3D {
                    center: vec3_round_3(center),
                    radius: f32_round_3(center.distance(spine.pt0)),
                    angle,
                    start_pt: spine.pt0,
                    clock_wise: axis.z < 0.0,
                    axis,
                    pref_axis: spine.preferred_dir,
                }));
            }
            SpineCurveType::CENT => {
                // 中心点已知的圆弧
                let center = spine.center_pt;
                let vec0 = spine.pt0 - center;
                let vec1 = spine.pt1 - center;
                let angle = (PI - vec0.angle_between(vec1)) * 2.0;
                let axis = vec1.cross(vec0).normalize();

                result.push(SegmentPath::Arc(Arc3D {
                    center: vec3_round_3(center),
                    radius: f32_round_3(center.distance(spine.pt0)),
                    angle,
                    start_pt: spine.pt0,
                    clock_wise: axis.z < 0.0,
                    axis,
                    pref_axis: spine.preferred_dir,
                }));
            }
            SpineCurveType::UNKNOWN => {
                tracing::warn!("遇到 UNKNOWN 类型的 Spine 曲线，跳过");
            }
        }
    }

    Ok(result)
}

pub async fn create_profile_geos(
    refno: RefnoEnum,
    geom_info: &CateGeomsInfo,
    csg_shapes_map: &CateCsgShapeMap,
) -> anyhow::Result<bool> {
    let geos = &geom_info.geometries;
    if geos.len() == 0 {
        return Ok(false);
    }
    let att = crate::get_named_attmap(refno).await?;
    let type_name = att.get_type_str();
    let mut plax = Vec3::Y;
    let mut extrude_dir = DVec3::Z;
    let mat = crate::get_world_mat4(refno, true)
        .await?
        .unwrap_or_default();
    // dbg!(&mat);
    let (_, rot, _) = mat.to_scale_rotation_translation();
    // dbg!(dquat_to_pdms_ori_xyz_str(&rot));
    let inv_quat = rot.inverse();
    // dbg!((refno, att.get_dvec3("DRNS"), att.get_dvec3("DRNE")));
    let mut drns = att
        .get_dvec3("DRNS")
        .map(|x| inv_quat.mul_vec3(x.normalize()));
    let mut drne = att
        .get_dvec3("DRNE")
        .map(|x| inv_quat.mul_vec3(x.normalize()));
    // dbg!((refno, drns, drne));
    let parent_refno = att.get_owner();
    let mut spine_paths = if type_name == "GENSEC" || type_name == "WALL" {
        let children_refnos = crate::collect_descendant_filter_ids(&[refno], &["SPINE"], None)
            .await
            .unwrap_or_default();
        let mut paths = vec![];
        for &spine_refno in children_refnos.iter() {
            let spine_att = crate::get_named_attmap(spine_refno).await?;
            let spine_mat = crate::get_world_mat4(spine_refno, true)
                .await?
                .unwrap_or_default();
            let inv_mat = spine_mat.inverse();
            //如果是墙，会有这两个属性
            drns = spine_att
                .get_dvec3("DRNS")
                .map(|x| inv_mat.transform_vector3(x.normalize()));
            if drns.is_some() && drns.unwrap().is_nan() {
                drns = None;
            }
            drne = spine_att
                .get_dvec3("DRNE")
                .map(|x| inv_mat.transform_vector3(x.normalize()));
            if drne.is_some() && drne.unwrap().is_nan() {
                drne = None;
            }
            // dbg!((drns, drne));
            let ch_atts = crate::get_children_named_attmaps(spine_refno)
                .await
                .unwrap_or_default();
            let len = ch_atts.len();
            if len < 1 {
                continue;
            }

            let mut i = 0;
            while i < ch_atts.len() - 1 {
                let att1 = &ch_atts[i];
                let t1 = att1.get_type_str();
                let att2 = &ch_atts[(i + 1) % len];
                let t2 = att2.get_type_str();
                if t1 == "POINSP" && t2 == "POINSP" {
                    paths.push(Spine3D {
                        refno: att1.get_refno().unwrap(),
                        pt0: att1.get_position().unwrap_or_default(),
                        pt1: att2.get_position().unwrap_or_default(),
                        curve_type: SpineCurveType::LINE,
                        preferred_dir: spine_att.get_vec3("YDIR").unwrap_or(Vec3::Z),
                        ..Default::default()
                    });
                    i += 1;
                } else if t1 == "POINSP" && t2 == "CURVE" {
                    let att3 = &ch_atts[(i + 2) % len];
                    let pt0 = att1.get_position().unwrap_or_default();
                    let pt1 = att3.get_position().unwrap_or_default();
                    let mid_pt = att2.get_position().unwrap_or_default();
                    let cur_type_str = att2.get_str("CURTYP").unwrap_or("unset");
                    let curve_type = match cur_type_str {
                        "CENT" => SpineCurveType::CENT,
                        "THRU" => SpineCurveType::THRU,
                        _ => SpineCurveType::UNKNOWN,
                    };
                    paths.push(Spine3D {
                        refno: att2.get_refno().unwrap(),
                        pt0,
                        pt1,
                        thru_pt: mid_pt,
                        center_pt: mid_pt,
                        cond_pos: att2.get_vec3("CPOS").unwrap_or_default(),
                        curve_type,
                        preferred_dir: spine_att.get_vec3("YDIR").unwrap_or(Vec3::Z),
                        radius: att2.get_f32("RAD").unwrap_or_default(),
                    });
                    i += 2;
                }
            }
        }
        paths
    } else {
        vec![]
    };

    if spine_paths.len() == 0 {
        if let Some(poss) = att.get_poss()
            && let Some(pose) = att.get_pose()
        {
            let height = pose.distance(poss);
            //还原成相对坐标系下的拉升方向
            for (i, geom) in geos.iter().enumerate() {
                if let CateGeoParam::Profile(profile) = geom {
                    let Some(profile_refno) = profile.get_refno() else {
                        continue;
                    };
                    plax = profile.get_plax();
                    let bangle = att.get_f32("BANG").unwrap_or_default();

                    let path = Line3D {
                        start: Default::default(),
                        end: pose - poss,
                        is_spine: false,
                    };

                    let solid = SweepSolid {
                        profile: profile.clone(),
                        drns,
                        drne,
                        bangle,
                        plax,
                        extrude_dir,
                        height,
                        path: SweepPath3D::from_line(path),
                        lmirror: att.get_bool("LMIRR").unwrap_or_default(),
                    };
                    csg_shapes_map
                        .entry(refno)
                        .or_insert(Vec::new())
                        .push(CateCsgShape {
                            refno: profile_refno,
                            csg_shape: Box::new(solid),
                            transform: Transform::IDENTITY,
                            visible: true,
                            is_tubi: false,
                            shape_err: None,
                            pts: vec![],
                            is_ngmr: false,
                        });
                }
            }
        }
    } else {
        // 将所有 Spine3D 段连接成一条连续路径
        match connect_spine_segments(spine_paths.clone()) {
            Ok(connected_paths) => {
                if connected_paths.is_empty() {
                    tracing::warn!("连接 Spine 段后为空，跳过处理");
                    return Ok(false);
                }

                // 为每个 profile 创建一个包含完整路径的 SweepSolid
                for (i, geom) in geos.iter().enumerate() {
                    if let CateGeoParam::Profile(profile) = geom {
                        let Some(profile_refno) = profile.get_refno() else {
                            continue;
                        };

                        plax = profile.get_plax();
                        let bangle = att.get_f32("BANG").unwrap_or_default();

                        // 创建路径
                        let sweep_path = SweepPath3D::from_segments(connected_paths.clone());

                        // 验证路径连续性
                        let (is_continuous, discontinuity_index) = sweep_path.validate_continuity();
                        if !is_continuous {
                            tracing::warn!(
                                "多段路径在索引 {:?} 处不连续，继续生成",
                                discontinuity_index
                            );
                        }

                        let height = sweep_path.length();
                        let loft = SweepSolid {
                            profile: profile.clone(),
                            drns,
                            drne,
                            bangle,
                            plax,
                            extrude_dir,
                            height,
                            path: sweep_path,
                            lmirror: att.get_bool("LMIRR").unwrap_or_default(),
                        };

                        // 使用第一个 spine 的 refno 生成 hash
                        let first_spine_refno = spine_paths
                            .first()
                            .map(|s| s.refno)
                            .unwrap_or(RefnoEnum::from(RefU64(0)));
                        let hash = profile_refno.hash_with_another_refno(first_spine_refno);

                        // Transform 使用 IDENTITY，因为多段路径已经包含了世界坐标
                        let mut transform = Transform::IDENTITY;
                        transform.scale = loft.get_scaled_vec3();

                        csg_shapes_map
                            .entry(refno)
                            .or_insert(Vec::new())
                            .push(CateCsgShape {
                                refno: RefU64(hash).into(),
                                csg_shape: Box::new(loft),
                                transform,
                                visible: true,
                                is_tubi: false,
                                shape_err: None,
                                pts: vec![],
                                is_ngmr: false,
                            });
                    }
                }
            }
            Err(e) => {
                tracing::error!("连接 Spine 段失败: {:?}", e);
                return Err(e);
            }
        }
    }
    Ok(true)
}
