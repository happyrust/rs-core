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
use crate::rs_surreal::query::{get_owner_refno_by_type, get_owner_type_name};
use crate::rs_surreal::spatial::{construct_basis_z_default, get_spline_pts};
use crate::shape::pdms_shape::BrepShapeTrait;
use crate::tool::dir_tool::parse_ori_str_to_quat;
use crate::tool::float_tool::{f32_round_3, vec3_round_3};
use crate::tool::math_tool::{
    dquat_to_pdms_ori_xyz_str, quat_to_pdms_ori_str, to_pdms_ori_str, to_pdms_vec_str,
};
use crate::transform::{calculate_plax_transform, get_local_transform};
use crate::{RefU64, get_world_transform};
use anyhow::anyhow;
use bevy_transform::prelude::Transform;
use dashmap::{DashMap, DashSet};
use glam::{DMat4, DQuat, DVec3, Mat3, Quat, Vec3};

use std::vec::Vec;

/// 将多个 Spine3D 段转换为归一化的路径段和对应的变换
///
/// **关键架构改进**：将路径几何与实例化变换分离
/// - **归一化路径**：所有段都从原点 (0,0,0) 开始，沿单位方向延伸
/// - **完整变换**：包含每段起点的 position、rotation 和 scale
///
/// 参数：
/// - segments: Spine3D 段的列表（包含实际世界坐标）
///
/// 返回：
/// - Ok((Vec<SegmentPath>, Vec<Transform>)): (归一化路径段, 每段的完整变换)
/// - Err: 如果段不连续或转换失败
async fn normalize_spine_segments(
    segments: Vec<Spine3D>,
) -> anyhow::Result<(Vec<SegmentPath>, Vec<Transform>)> {
    const EPSILON: f32 = 1e-3;
    let mut normalized_segments = Vec::new();
    let mut transforms = Vec::new();

    if segments.is_empty() {
        return Ok((normalized_segments, transforms));
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

        // 根据曲线类型创建归一化的 SegmentPath 和对应的 Transform
        match spine.curve_type {
            SpineCurveType::LINE => {
                // 计算实际方向和长度
                let direction = (spine.pt1 - spine.pt0).normalize_or_zero();
                let length = spine.pt0.distance(spine.pt1);

                // 归一化路径：从原点沿 Z 轴单位长度
                normalized_segments.push(SegmentPath::Line(Line3D {
                    start: Vec3::ZERO,
                    end: Vec3::Z,
                    is_spine: true,
                }));

                // 获取该段起点 POINSP 的局部旋转
                let local_rotation = crate::transform::get_local_transform(spine.refno)
                    .await
                    .ok()
                    .flatten()
                    .map(|t| t.rotation)
                    .unwrap_or(Quat::IDENTITY);

                // 完整变换：包含位置、旋转和缩放
                transforms.push(Transform {
                    translation: spine.pt0,             // 起点位置
                    rotation: local_rotation,           // 起点旋转（包含 bangle）
                    scale: Vec3::new(1.0, 1.0, length), // Z 方向缩放到实际长度
                });
            }
            SpineCurveType::THRU => {
                // 通过三点确定圆弧
                let center = circum_center(spine.pt0, spine.pt1, spine.thru_pt);
                let radius = center.distance(spine.pt0);
                let vec0 = spine.pt0 - spine.thru_pt;
                let vec1 = spine.pt1 - spine.thru_pt;
                let angle = (PI - vec0.angle_between(vec1)) * 2.0;
                let axis = vec1.cross(vec0).normalize();

                // 归一化圆弧：从原点开始，单位半径
                normalized_segments.push(SegmentPath::Arc(Arc3D {
                    center: Vec3::ZERO,
                    radius: 1.0,
                    angle,
                    start_pt: Vec3::X, // 单位圆上的起点
                    clock_wise: axis.z < 0.0,
                    axis,
                    pref_axis: spine.preferred_dir,
                }));

                // 获取该段起点 POINSP 的局部旋转
                let local_rotation = crate::transform::get_local_transform(spine.refno)
                    .await
                    .ok()
                    .flatten()
                    .map(|t| t.rotation)
                    .unwrap_or(Quat::IDENTITY);

                // 完整变换：位置、旋转和缩放
                transforms.push(Transform {
                    translation: center,        // 圆心位置
                    rotation: local_rotation,   // 起点旋转
                    scale: Vec3::splat(radius), // 统一缩放到实际半径
                });
            }
            SpineCurveType::CENT => {
                // 中心点已知的圆弧
                let center = spine.center_pt;
                let radius = center.distance(spine.pt0);
                let vec0 = spine.pt0 - center;
                let vec1 = spine.pt1 - center;
                let angle = (PI - vec0.angle_between(vec1)) * 2.0;
                let axis = vec1.cross(vec0).normalize();

                // 归一化圆弧：从原点开始，单位半径
                normalized_segments.push(SegmentPath::Arc(Arc3D {
                    center: Vec3::ZERO,
                    radius: 1.0,
                    angle,
                    start_pt: Vec3::X, // 单位圆上的起点
                    clock_wise: axis.z < 0.0,
                    axis,
                    pref_axis: spine.preferred_dir,
                }));

                // 获取该段起点 POINSP 的局部旋转
                let local_rotation = crate::transform::get_local_transform(spine.refno)
                    .await
                    .ok()
                    .flatten()
                    .map(|t| t.rotation)
                    .unwrap_or(Quat::IDENTITY);

                // 完整变换：位置、旋转和缩放
                transforms.push(Transform {
                    translation: center,        // 圆心位置
                    rotation: local_rotation,   // 起点旋转
                    scale: Vec3::splat(radius), // 统一缩放到实际半径
                });
            }
            SpineCurveType::UNKNOWN => {
                tracing::warn!("遇到 UNKNOWN 类型的 Spine 曲线，跳过");
            }
        }
    }

    Ok((normalized_segments, transforms))
}

/// 为给定 PDMS 元素构建与剖面（Profile）相关的 CSG 几何体。
///
/// 该函数会根据元素的属性与几何描述生成 Sweep / Loft 实体：
/// - 当只有 `POSS` / `POSE` 两点时，沿两点连线进行直线拉伸生成剖面实体；
/// - 当存在 `SPINE` / `POINSP` / `CURVE` 子元素时，将多段 Spine 曲线连接成一条
///   连续路径，并沿该路径进行放样生成剖面实体；
/// - 对 `GENSEC` 元素，会优先根据 SPINE 方向计算旋转朝向，其它类型则使用
///   `PLAX` 属性计算朝向。
///
/// 参数：
/// - `refno`：当前元素的 Refno，用于查询属性且作为结果映射的键；
/// - `geom_info`：类别几何信息，其中的 `Profile` 描述剖面形状及其 `PLAX` 等参数；
/// - `csg_shapes_map`：输出用的 CSG 形体映射，本函数会向其中追加生成的
///   `CateCsgShape`。
///
/// 返回值：
/// - `Ok(true)`：已根据当前元素尝试生成剖面几何（可能包含直线拉伸或沿 Spine 放样）；
/// - `Ok(false)`：不存在可用的几何或 Spine 路径（例如没有 Profile 或路径为空），
///   跳过当前元素；
/// - `Err`：在查询属性或组装路径/几何过程中发生错误。
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

    // 使用统一的变换策略计算局部旋转：
    // 优先通过 get_local_transform(refno, owner) 获取当前构件相对于父节点的局部 Transform，
    // 然后用其 rotation 的逆把 Parent 空间下的 DRNS/DRNE 转换到本地截面坐标系。
    // 如果局部变换无法计算，则回退到 ORI 提供的旋转逻辑。
    let parent_refno = att.get_owner();
    let inv_local_rot = if parent_refno.is_unset() {
        att.get_rotation().unwrap_or(DQuat::IDENTITY).inverse()
    } else {
        match get_local_transform(refno).await? {
            Some(local_t) => local_t.rotation.as_dquat().inverse(),
            None => att.get_rotation().unwrap_or(DQuat::IDENTITY).inverse(),
        }
    };

    let mut drns = att
        .get_dvec3("DRNS")
        .map(|x| inv_local_rot.mul_vec3(x.normalize()));
    let mut drne = att
        .get_dvec3("DRNE")
        .map(|x| inv_local_rot.mul_vec3(x.normalize()));
    // dbg!((refno, drns, drne));

    // 性能优化：提前缓存元素类型信息，避免在循环中重复处理
    let is_gensec_element = type_name == "GENSEC";
    let gensec_refno = if is_gensec_element {
        // 如果是GENSEC，使用当前refno
        Some(refno)
    } else {
        None
    };
    // let parent_refno = att.get_owner();
    let mut spine_paths = if type_name == "GENSEC" || type_name == "WALL" {
        let children_refnos = crate::collect_descendant_filter_ids(&[refno], &["SPINE"], None)
            .await
            .unwrap_or_default();
        let mut paths = vec![];
        for &spine_refno in children_refnos.iter() {
            let spine_att = crate::get_named_attmap(spine_refno).await?;
            //如果是墙，会有这两个属性
            drns = spine_att.get_dvec3("DRNS").map(|x| x.normalize());
            if drns.is_some() && drns.unwrap().is_nan() {
                drns = None;
            }
            drne = spine_att.get_dvec3("DRNE").map(|x| x.normalize());
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
                        refno: att1.get_refno().unwrap(), // 起点 POINSP 的 refno
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
                        refno: att1.get_refno().unwrap(), // 修正：使用起点 POINSP 的 refno，而不是 CURVE 的 refno
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
        //if is SCTN (无 SPINE，通过 POSS/POSE 定义的简单拉伸)
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

                    // 为共享 mesh 生成创建单位长度的路径，实际长度存储在 height 中用于实例化时缩放
                    let path = Line3D {
                        start: Default::default(),
                        end: Vec3::Z * 1.0, // 使用真正的单位长度路径
                        is_spine: false,
                    };

                    // SCTN 类型（无 SPINE）：不需要局部变换
                    let solid = SweepSolid {
                        profile: profile.clone(),
                        drns: None,
                        drne: None,
                        plax,
                        extrude_dir,
                        height: 1.0,
                        path: SweepPath3D::from_line(path),
                        lmirror: att.get_bool("LMIRR").unwrap_or_default(),
                        spine_segments: vec![],     // 无 SPINE 段
                        segment_transforms: vec![], // SCTN 无需局部变换
                    };

                    // SCTN 类型：直接使用 POSS 位置和缩放，不需要旋转
                    let transform = Transform {
                        rotation: Quat::IDENTITY,
                        scale: solid.get_scaled_vec3(),
                        translation: poss,
                    };
                    csg_shapes_map
                        .entry(refno)
                        .or_insert(Vec::new())
                        .push(CateCsgShape {
                            refno: profile_refno,
                            csg_shape: Box::new(solid),
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
    } else {
        // 将所有 Spine3D 段连接成一条连续路径
        match normalize_spine_segments(spine_paths.clone()).await {
            Ok((normalized_paths, segment_transforms)) => {
                if normalized_paths.is_empty() {
                    tracing::warn!("归一化 Spine 段后为空，跳过处理");
                    return Ok(false);
                }

                // 为每个 profile 创建一个包含归一化路径的 SweepSolid
                for (i, geom) in geos.iter().enumerate() {
                    if let CateGeoParam::Profile(profile) = geom {
                        let Some(profile_refno) = profile.get_refno() else {
                            continue;
                        };

                        plax = profile.get_plax();

                        let sweep_path = SweepPath3D::from_segments(normalized_paths.clone());

                        // 验证路径连续性
                        let (is_continuous, discontinuity_index) = sweep_path.validate_continuity();
                        if !is_continuous {
                            tracing::warn!(
                                "多段路径在索引 {:?} 处不连续，继续生成",
                                discontinuity_index
                            );
                        }

                        // 注意：height 现在是归一化路径的长度
                        // 实际长度由 segment_transforms 中的 scale 控制
                        let height = sweep_path.length();

                        let loft = SweepSolid {
                            profile: profile.clone(),
                            drns: None,
                            drne: None,
                            plax,
                            extrude_dir,
                            height,
                            path: sweep_path,
                            lmirror: att.get_bool("LMIRR").unwrap_or_default(),
                            spine_segments: spine_paths.clone(), // 存储原始 Spine3D 段信息（用于调试）
                            segment_transforms: segment_transforms.clone(), // 存储完整变换（位置+旋转+缩放）
                        };

                        // 使用第一个 spine 的 refno 生成 hash
                        let first_spine_refno = spine_paths
                            .first()
                            .map(|s| s.refno)
                            .unwrap_or(RefnoEnum::from(RefU64(0)));
                        let hash = profile_refno.hash_with_another_refno(first_spine_refno);

                        // 获取第一段的完整变换用于实例化
                        let first_transform = segment_transforms
                            .first()
                            .cloned()
                            .unwrap_or(Transform::IDENTITY);

                        let orientation_str = crate::tool::math_tool::quat_to_pdms_ori_str(
                            &first_transform.rotation,
                            false,
                        );
                        println!(
                            "DEBUG: SweepSolid first segment - pos: {:?}, rotation ENU: {}, scale: {:?}",
                            first_transform.translation, orientation_str, first_transform.scale
                        );

                        // 使用第一段的完整变换进行实例化（包含位置、旋转和缩放）
                        let transform = first_transform;

                        csg_shapes_map
                            .entry(refno)
                            .or_insert(Vec::new())
                            .push(CateCsgShape {
                                //先暂时不做几何体共享
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
