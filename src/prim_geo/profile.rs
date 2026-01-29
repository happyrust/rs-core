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
use crate::rs_surreal::spatial::{
    construct_basis_z_default, construct_basis_z_y_hint, get_spline_pts,
};
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

const FRAME_EPS: f32 = 1e-6;

/// 将多个 Spine3D 段转换为归一化的路径段和对应的变换
///
/// **关键架构改进**：将路径几何与实例化变换分离
/// - **归一化路径**：所有段都从原点 (0,0,0) 开始，沿单位方向延伸
/// - **完整变换**：包含每段起点的 position（相对坐标）、rotation 和 scale
///
/// **重要**：输入的 segments 中的坐标应该是**相对坐标**（相对于第一个 POINSP 或 POSS 的位置）。
/// 这样第一段的 `transform.translation` 就是 `Vec3::ZERO`，几何体的世界位置由外部的
/// `inst_info.world_transform` 控制。
///
/// 参数：
/// - segments: Spine3D 段列表（坐标为相对于参考原点的偏移）
/// - plax: 截面参考方向，用于计算 Frenet 标架
/// - bangle: 绕路径方向的旋转角度（度数）
///
/// 返回：
/// - Ok((Vec<SegmentPath>, Vec<Transform>)): (归一化路径段, 每段的完整变换)
/// - Err: 如果段不连续或转换失败
async fn normalize_spine_segments(
    segments: Vec<Spine3D>,
    plax: Vec3,
    bangle: f32,
) -> anyhow::Result<(Vec<SegmentPath>, Vec<Transform>)> {
    const EPSILON: f32 = 1e-3;
    let mut normalized_segments = Vec::new();
    let mut transforms = Vec::new();

    if segments.is_empty() {
        return Ok((normalized_segments, transforms));
    }

    // 若路径包含 CURVE（THRU/CENT），则不做“单位化路径 + segment_transforms 还原”：
    // - 直接用真实几何（相对坐标）构建 SegmentPath，避免圆弧单位化带来的中心/起点/扭转复杂度。
    // - segment_transforms 置空（或 identity），真实尺度由 SegmentPath 自身携带。
    //
    // 仍保留：仅当全为 LINE 时才走旧的单位化逻辑（便于复用与缩放）。
    let has_curve = segments.iter().any(|s| {
        matches!(s.curve_type, SpineCurveType::THRU | SpineCurveType::CENT)
    });

    if has_curve {
        // 连续性检查（可选）：维持原行为，仅 warning，不中断
        for i in 1..segments.len() {
            let prev_end = segments[i - 1].pt1;
            let curr_start = segments[i].pt0;
            let distance = prev_end.distance(curr_start);
            if distance > EPSILON {
                tracing::warn!(
                    "Spine 段不连续(非单位化分支): 段 {} 到段 {} 的距离为 {:.6}",
                    i - 1,
                    i,
                    distance
                );
            }
        }

        for spine in segments.iter() {
            match spine.curve_type {
                SpineCurveType::LINE => {
                    normalized_segments.push(SegmentPath::Line(Line3D {
                        start: spine.pt0,
                        end: spine.pt1,
                        is_spine: true,
                    }));
                }
                SpineCurveType::THRU => {
                    // 与旧的单位化逻辑保持一致：用 THRU 三点推导 arc.angle 与 arc.axis，
                    // 以避免 180°（起终点对径）时 v0×v1 退化为 0 带来的不稳定。
                    let center = circum_center(spine.pt0, spine.pt1, spine.thru_pt);
                    let radius = center.distance(spine.pt0);
                    let vec0 = spine.pt0 - spine.thru_pt;
                    let vec1 = spine.pt1 - spine.thru_pt;
                    let angle = (PI - vec0.angle_between(vec1)) * 2.0;
                    let mut axis = vec1.cross(vec0).normalize_or_zero();
                    if axis.length_squared() < 1e-6 {
                        // 兜底：用圆心->起点 与 圆心->thru 构造法向（180° 也不退化）
                        axis = (spine.pt0 - center)
                            .cross(spine.thru_pt - center)
                            .normalize_or_zero();
                    }
                    if axis.length_squared() < 1e-6 {
                        axis = Vec3::Z;
                    }

                    normalized_segments.push(SegmentPath::Arc(Arc3D {
                        center,
                        radius,
                        angle,
                        start_pt: spine.pt0,    // 实际起点
                        clock_wise: false,      // 方向由 axis（符号）决定，避免与 angle 符号耦合
                        axis,
                        pref_axis: spine.preferred_dir,
                    }));
                }
                SpineCurveType::CENT => {
                    let center = spine.center_pt;
                    let radius = center.distance(spine.pt0);

                    // 兼容旧逻辑：center 已知但 angle/axis 仍按“反向补角×2”推导（与历史数据/行为对齐）
                    let vec0 = spine.pt0 - center;
                    let vec1 = spine.pt1 - center;
                    let angle = (PI - vec0.angle_between(vec1)) * 2.0;
                    let mut axis = vec1.cross(vec0).normalize_or_zero();
                    if axis.length_squared() < 1e-6 {
                        // 若 pt0/pt1 近似对径，用 center->pt0 与 center->thru 兜底
                        axis = (spine.pt0 - center)
                            .cross(spine.thru_pt - center)
                            .normalize_or_zero();
                    }
                    if axis.length_squared() < 1e-6 {
                        axis = Vec3::Z;
                    }

                    normalized_segments.push(SegmentPath::Arc(Arc3D {
                        center,
                        radius,
                        angle,
                        start_pt: spine.pt0,
                        clock_wise: false,
                        axis,
                        pref_axis: spine.preferred_dir,
                    }));
                }
                SpineCurveType::UNKNOWN => {
                    let refno = spine.refno;
                    let error_msg = format!(
                        "Spine 段 {} 的曲线类型为 UNKNOWN，无法生成路径。",
                        refno
                    );
                    tracing::error!("{}", error_msg);
                    return Err(anyhow::anyhow!(error_msg));
                }
            }
        }

        // 曲线路径不再依赖 segment_transforms（bangle 将在截面阶段处理），返回空 transforms
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

                // 归一化路径：从原点沿 Z 轴100.0单位长度（与sweep_solid.rs保持一致）
                normalized_segments.push(SegmentPath::Line(Line3D {
                    start: Vec3::ZERO,
                    end: Vec3::Z * 100.0,
                    is_spine: true,
                }));

                // 使用 YDIR (spine.preferred_dir) 计算方位，与 SpineStrategy.initialize_rotation 保持一致
                let ydir = spine.preferred_dir.as_dvec3();
                let base_rotation =
                    construct_basis_z_y_hint(direction.as_dvec3(), Some(ydir), false);

                // 计算 bangle 旋转（绕路径方向）
                let bangle_rotation = Quat::from_axis_angle(direction, bangle.to_radians());

                // 组合旋转：基础方位 × bangle 旋转
                let final_rotation = base_rotation.as_quat() * bangle_rotation;

                // 完整变换：包含位置、方位 + bangle 旋转和缩放
                transforms.push(Transform {
                    translation: spine.pt0,                     // 起点位置
                    rotation: final_rotation,                   // 方位 × bangle 旋转
                    scale: Vec3::new(1.0, 1.0, length / 100.0), // Z 方向缩放：实际长度/100.0
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
                let clock_wise = axis.z < 0.0;

                // 归一化圆弧：从原点开始，单位半径
                normalized_segments.push(SegmentPath::Arc(Arc3D {
                    center: Vec3::ZERO,
                    radius: 1.0,
                    angle,
                    start_pt: Vec3::X, // 单位圆上的起点
                    clock_wise,
                    axis,
                    pref_axis: spine.preferred_dir,
                }));

                // 计算圆弧起点处的切线方向
                // 1. 径向量：从圆心指向起点
                let radial = (spine.pt0 - center).normalize_or_zero();

                // 2. 切线方向：axis × radial（顺时针则取反）
                let tangent = if clock_wise {
                    -axis.cross(radial).normalize_or_zero()
                } else {
                    axis.cross(radial).normalize_or_zero()
                };

                // 3. 使用 YDIR (spine.preferred_dir) 计算方位
                let ydir = spine.preferred_dir.as_dvec3();
                let base_rotation = construct_basis_z_y_hint(tangent.as_dvec3(), Some(ydir), false);

                // 4. 计算 bangle 旋转（绕切线方向）
                let bangle_rotation = Quat::from_axis_angle(tangent, bangle.to_radians());

                // 5. 组合旋转：基础方位 × bangle 旋转
                let final_rotation = base_rotation.as_quat() * bangle_rotation;

                // 完整变换：位置、方位 + bangle 旋转和缩放
                transforms.push(Transform {
                    translation: center,        // 圆心位置
                    rotation: final_rotation,   // 方位 × bangle 旋转
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
                let clock_wise = axis.z < 0.0;

                // 归一化圆弧：从原点开始，单位半径
                normalized_segments.push(SegmentPath::Arc(Arc3D {
                    center: Vec3::ZERO,
                    radius: 1.0,
                    angle,
                    start_pt: Vec3::X, // 单位圆上的起点
                    clock_wise,
                    axis,
                    pref_axis: spine.preferred_dir,
                }));

                // 计算圆弧起点处的切线方向
                // 1. 径向量：从圆心指向起点
                let radial = (spine.pt0 - center).normalize_or_zero();

                // 2. 切线方向：axis × radial（顺时针则取反）
                let tangent = if clock_wise {
                    -axis.cross(radial).normalize_or_zero()
                } else {
                    axis.cross(radial).normalize_or_zero()
                };

                // 3. 使用 YDIR (spine.preferred_dir) 计算方位
                let ydir = spine.preferred_dir.as_dvec3();
                let base_rotation = construct_basis_z_y_hint(tangent.as_dvec3(), Some(ydir), false);

                // 4. 计算 bangle 旋转（绕切线方向）
                let bangle_rotation = Quat::from_axis_angle(tangent, bangle.to_radians());

                // 5. 组合旋转：基础方位 × bangle 旋转
                let final_rotation = base_rotation.as_quat() * bangle_rotation;

                // 完整变换：位置、方位 + bangle 旋转和缩放
                transforms.push(Transform {
                    translation: center,        // 圆心位置
                    rotation: final_rotation,   // 方位 × bangle 旋转
                    scale: Vec3::splat(radius), // 统一缩放到实际半径
                });
            }
            SpineCurveType::UNKNOWN => {
                let refno = spine.refno;
                let error_msg = format!(
                    "Spine 段 {} 的曲线类型为 UNKNOWN，无法生成归一化路径。请检查 CURVE 元素的 CURTYP 属性是否有效（应为 CENT 或 THRU）",
                    refno
                );
                tracing::error!("{}", error_msg);
                return Err(anyhow::anyhow!(error_msg));
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
    // 记录第一个点的世界坐标作为参考原点，用于将所有路径点转换为相对坐标
    let mut spine_origin: Option<Vec3> = None;

    let mut spine_paths = if type_name == "GENSEC" || type_name == "WALL" || type_name == "STWALL" {
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

            // 获取第一个 POINSP 的位置作为参考原点（如果尚未设置）
            if spine_origin.is_none() {
                for att in ch_atts.iter() {
                    if att.get_type_str() == "POINSP" {
                        if let Some(pos) = att.get_position() {
                            spine_origin = Some(pos);
                            break;
                        }
                    }
                }
            }
            let origin = spine_origin.unwrap_or(Vec3::ZERO);

            let mut i = 0;
            while i < ch_atts.len() - 1 {
                let att1 = &ch_atts[i];
                let t1 = att1.get_type_str();
                let att2 = &ch_atts[(i + 1) % len];
                let t2 = att2.get_type_str();
                if t1 == "POINSP" && t2 == "POINSP" {
                    // 使用相对于参考原点的坐标
                    let pt0_world = att1.get_position().unwrap_or_default();
                    let pt1_world = att2.get_position().unwrap_or_default();
                    paths.push(Spine3D {
                        refno: att1.get_refno().unwrap(), // 起点 POINSP 的 refno
                        pt0: pt0_world - origin,          // 相对坐标
                        pt1: pt1_world - origin,          // 相对坐标
                        curve_type: SpineCurveType::LINE,
                        preferred_dir: spine_att.get_vec3("YDIR").unwrap_or(Vec3::Z),
                        ..Default::default()
                    });
                    i += 1;
                } else if t1 == "POINSP" && t2 == "CURVE" {
                    let att3 = &ch_atts[(i + 2) % len];
                    let pt0_world = att1.get_position().unwrap_or_default();
                    let pt1_world = att3.get_position().unwrap_or_default();
                    let mid_pt_world = att2.get_position().unwrap_or_default();
                    let cur_type_str = att2.get_str("CURTYP").unwrap_or("unset");
                    let curve_type = match cur_type_str {
                        "CENT" => SpineCurveType::CENT,
                        "THRU" => SpineCurveType::THRU,
                        _ => SpineCurveType::UNKNOWN,
                    };
                    paths.push(Spine3D {
                        refno: att1.get_refno().unwrap(), // 修正：使用起点 POINSP 的 refno，而不是 CURVE 的 refno
                        pt0: pt0_world - origin,          // 相对坐标
                        pt1: pt1_world - origin,          // 相对坐标
                        thru_pt: mid_pt_world - origin,   // 相对坐标
                        center_pt: mid_pt_world - origin, // 相对坐标
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

    // 如果没有 SPINE 子元素，尝试通过 POSS/POSE 创建简单拉伸路径
    if spine_paths.len() == 0 {
        if let Some(poss) = att.get_poss()
            && let Some(pose) = att.get_pose()
        {
            let delta = pose - poss;
            if delta.length_squared() < FRAME_EPS {
                tracing::warn!("POSS 和 POSE 重合，无法计算拉伸方向，refno = {:?}", refno);
                return Ok(false);
            }

            // 设置 POSS 作为参考原点
            spine_origin = Some(poss);

            // 将 POSS/POSE 转换为 Spine3D::LINE 段，使用相对坐标
            // pt0 = poss - poss = Vec3::ZERO
            // pt1 = pose - poss = delta (相对偏移)
            spine_paths.push(Spine3D {
                refno,
                pt0: Vec3::ZERO, // 相对坐标：起点为原点
                pt1: delta,      // 相对坐标：终点为相对偏移
                curve_type: SpineCurveType::LINE,
                preferred_dir: Vec3::Y, // 使用默认参考方向，后续会用 plax 覆盖
                ..Default::default()
            });
        }
    }

    // 统一处理所有路径（包括 SPINE 和 POSS/POSE 转换的路径）
    if spine_paths.len() > 0 {
        // 将所有 Spine3D 段连接成一条连续路径
        // 提前获取第一个 profile 的 plax 和元素的 bangle
        let first_plax = geos
            .iter()
            .find_map(|g| {
                if let CateGeoParam::Profile(profile) = g {
                    Some(profile.get_plax())
                } else {
                    None
                }
            })
            .unwrap_or(Vec3::Y);
        // 对于 SCTN 和 STWALL，BANG 影响的是 local transform，而不是几何体本身
        // 这些类型的 BANG 旋转已在 TransformStrategy 中处理
        let bangle = if type_name == "SCTN" || type_name == "STWALL" {
            0.0
        } else {
            att.get_f32("BANG").unwrap_or_default()
        };

        match normalize_spine_segments(spine_paths.clone(), first_plax, bangle).await {
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
                            drns, // 使用实际读取的 DRNS 方向向量（与 core.dll 处理一致）
                            drne, // 使用实际读取的 DRNE 方向向量（与 core.dll 处理一致）
                            plax,
                            bangle, // 使用前面已计算的 bangle（对于 SCTN/STWALL 为 0）
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
                        // let hash = profile_refno.hash_with_another_refno(first_spine_refno);

                        // 获取第一段的完整变换用于实例化
                        let first_transform =
                            segment_transforms.first().cloned().unwrap_or_else(|| {
                                tracing::warn!("segment_transforms 为空，使用 Transform::IDENTITY");
                                Transform::IDENTITY
                            });

                        // let orientation_str = crate::tool::math_tool::quat_to_pdms_ori_str(
                        //     &first_transform.rotation,
                        //     false,
                        // );

                        // 实例 transform 的使用策略（与 inst_geo.param 的 unit 化策略必须配套）：
                        // - 单段直线且无倾斜：可复用单位几何（inst_geo.param 会 unit 化），实例 transform 负责方向与长度缩放；
                        // - 圆弧/多段/倾斜：inst_geo.param 必须保留 segment_transforms 参与路径采样，实例 transform 必须为 identity，
                        //   否则整体缩放会把截面一起放大（典型：WALL 的 radius=28000）。
                        let is_simple_line =
                            loft.path.as_single_line().is_some() && !loft.is_sloped();

                        // 根据元素类型决定是否使用 rotation
                        // - GENSEC/WALL（有 SPINE）：单段直线时使用第一个点的方位；否则使用 identity
                        // - SCTN/STWALL（POSS/POSE）：只有偏移，不旋转
                        let transform = if type_name == "GENSEC" || type_name == "WALL" {
                            if is_simple_line {
                                first_transform
                            } else {
                                Transform::IDENTITY
                            }
                        } else {
                            Transform {
                                translation: first_transform.translation,
                                rotation: Quat::IDENTITY,
                                scale: first_transform.scale,
                            }
                        };

                        csg_shapes_map
                            .entry(refno)
                            .or_insert(Vec::new())
                            .push(CateCsgShape {
                                // refno: RefU64(hash).into(),
                                refno: profile_refno,
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
    } else {
        // 既没有 SPINE 也没有 POSS/POSE，无法生成几何
        tracing::debug!(
            "元素 {:?} 既没有 SPINE 子元素也没有 POSS/POSE 属性，跳过几何生成",
            refno
        );
        return Ok(false);
    }
    Ok(true)
}
