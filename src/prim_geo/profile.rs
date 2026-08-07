use crate::plant_transform::Transform;
use std::default;
use std::f32::consts::{FRAC_PI_2, PI};

use crate::geometry::sweep_mesh::sweep_reference_endpoints;
use crate::mesh_precision::LodMeshSettings;
use crate::parsed_data::geo_params_data::{CateGeoParam, PdmsGeoParam};
use crate::parsed_data::{CateGeomsInfo, CateProfileParam, PlineSnapPoint};
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
use dashmap::{DashMap, DashSet};
use glam::{DMat4, DQuat, DVec3, Mat3, Quat, Vec3};

use std::vec::Vec;

const FRAME_EPS: f32 = 1e-6;

/// 将 SPINE 路径（POINSP/CURVE）转换为 SegmentPath 列表
/// 所有段使用实际几何坐标，不做单位化
///
/// # 参数
/// - segments: Spine3D 段列表（坐标为相对于参考原点的偏移）
/// - plax: 截面参考方向，用于计算 Frenet 标架
/// - bangle: 绕路径方向的旋转角度（度数）
///
/// # 返回
/// - Vec<SegmentPath>: 使用实际几何坐标的路径段列表
fn convert_spine_to_segments(
    segments: &[Spine3D],
    plax: Vec3,
    bangle: f32,
) -> anyhow::Result<Vec<SegmentPath>> {
    const EPSILON: f32 = 1e-3;
    let mut result = Vec::new();

    if segments.is_empty() {
        return Ok(result);
    }

    // 连续性检查（仅 warning）
    for i in 1..segments.len() {
        let prev_end = segments[i - 1].pt1;
        let curr_start = segments[i].pt0;
        if prev_end.distance(curr_start) > EPSILON {
            tracing::warn!(
                "Spine 段不连续: 段 {} 到段 {}, 距离={}",
                i - 1,
                i,
                prev_end.distance(curr_start)
            );
        }
    }

    // 遍历所有段，按实际几何生成 SegmentPath
    for spine in segments.iter() {
        match spine.curve_type {
            SpineCurveType::LINE => {
                result.push(SegmentPath::Line(Line3D {
                    start: spine.pt0,
                    end: spine.pt1,
                    is_spine: true,
                }));
            }
            SpineCurveType::THRU => {
                // 三点圆弧：计算圆心、半径、角度、轴
                let center = circum_center(spine.pt0, spine.pt1, spine.thru_pt);
                let radius = center.distance(spine.pt0);

                // 计算圆弧的角度和轴
                let vec0 = (spine.pt0 - center).normalize_or_zero();
                let vec1 = (spine.pt1 - center).normalize_or_zero();
                let angle = vec0.angle_between(vec1);
                let axis = vec0.cross(vec1).normalize_or_zero();

                println!(
                    "[convert_spine] THRU: pt0={:?} pt1={:?} thru={:?} center={:?} radius={:.3} angle={:.3}deg axis={:?} pref_axis={:?}",
                    spine.pt0,
                    spine.pt1,
                    spine.thru_pt,
                    center,
                    radius,
                    angle.to_degrees(),
                    axis,
                    spine.preferred_dir
                );

                result.push(SegmentPath::Arc(Arc3D {
                    center,
                    radius,
                    angle,
                    start_pt: spine.pt0,
                    clock_wise: false,
                    axis,
                    pref_axis: spine.preferred_dir,
                }));
            }
            SpineCurveType::CENT => {
                // 中心点已知的圆弧
                let center = spine.center_pt;
                let radius = center.distance(spine.pt0);

                let vec0 = (spine.pt0 - center).normalize_or_zero();
                let vec1 = (spine.pt1 - center).normalize_or_zero();
                let angle = vec0.angle_between(vec1);
                let axis = vec0.cross(vec1).normalize_or_zero();

                result.push(SegmentPath::Arc(Arc3D {
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
                return Err(anyhow!("未知的曲线类型"));
            }
        }
    }

    Ok(result)
}

/// 将 POSS/POSE 两点转换为 SegmentPath
/// 直接使用实际坐标，不做单位化
///
/// # 参数
/// - poss: 起点位置
/// - pose: 终点位置
///
/// # 返回
/// - SegmentPath: 使用实际几何坐标的直线段
fn convert_poss_pose_to_segment(poss: Vec3, pose: Vec3) -> SegmentPath {
    SegmentPath::Line(Line3D {
        start: poss,
        end: pose,
        is_spine: true,
    })
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
    pline_snap_map: &DashMap<RefnoEnum, Vec<PlineSnapPoint>>,
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
                        "CENT" => Some(SpineCurveType::CENT),
                        "THRU" | "FILL" | "RADI" | "BULG" => Some(SpineCurveType::THRU),
                        "LINE" => Some(SpineCurveType::LINE),
                        // 与 core.dll 一致：NULL 段不生成路径
                        "NULL" => None,
                        _ => Some(SpineCurveType::UNKNOWN),
                    };
                    if curve_type.is_none() {
                        i += 2;
                        continue;
                    }
                    let ydir = spine_att.get_vec3("YDIR").unwrap_or(Vec3::Z);
                    println!(
                        "[profile][arc-spine] refno={} CURTYP={} pt0_world={:?} pt1_world={:?} mid_pt={:?} origin={:?} ydir={:?} rad={:?}",
                        refno,
                        cur_type_str,
                        pt0_world,
                        pt1_world,
                        mid_pt_world,
                        origin,
                        ydir,
                        att2.get_f32("RAD")
                    );
                    paths.push(Spine3D {
                        refno: att1.get_refno().unwrap(), // 修正：使用起点 POINSP 的 refno，而不是 CURVE 的 refno
                        pt0: pt0_world - origin,          // 相对坐标
                        pt1: pt1_world - origin,          // 相对坐标
                        thru_pt: mid_pt_world - origin,   // 相对坐标
                        center_pt: mid_pt_world - origin, // 相对坐标
                        cond_pos: att2.get_vec3("CPOS").unwrap_or_default(),
                        curve_type: curve_type.unwrap_or_default(),
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

    // 优先检查是否有 POSS/POSE 属性（如果有，使用 POSS/POSE 的处理方式）
    let has_poss_pose = att.get_poss().is_some() && att.get_pose().is_some();

    println!(
        "[profile] refno={} type={} has_poss_pose={} spine_paths.len={}",
        refno,
        type_name,
        has_poss_pose,
        spine_paths.len()
    );

    // 如果有 POSS/POSE 属性，优先使用 POSS/POSE 创建路径（清空 SPINE 创建的路径）
    if has_poss_pose {
        if let Some(poss) = att.get_poss()
            && let Some(pose) = att.get_pose()
        {
            println!(
                "[profile][poss/pose] refno={} type={} poss={} pose={} delta={} len={}",
                refno,
                type_name,
                to_pdms_vec_str(&poss, false),
                to_pdms_vec_str(&pose, false),
                to_pdms_vec_str(&(pose - poss), false),
                (pose - poss).length()
            );
            let delta = pose - poss;
            if delta.length_squared() < FRAME_EPS {
                tracing::warn!("POSS 和 POSE 重合，无法计算拉伸方向，refno = {:?}", refno);
                return Ok(false);
            }

            // 清空 SPINE 创建的路径，只使用 POSS/POSE
            spine_paths.clear();

            // 设置 POSS 作为参考原点
            // spine_origin = Some(poss);

            // 将 POSS/POSE 转换为 Spine3D::LINE 段
            spine_paths.push(Spine3D {
                refno,
                pt0: Vec3::ZERO,
                pt1: delta,
                curve_type: SpineCurveType::LINE,
                preferred_dir: Vec3::Y,
                ..Default::default()
            });
        }
    }

    // 统一处理所有路径（包括 SPINE 和 POSS/POSE 转换的路径）
    if spine_paths.len() > 0 {
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

        println!(
            "[profile] refno={} type={} has_poss_pose={} drns={:?} drne={:?} plax={:?} bangle={:.3}",
            refno,
            type_name,
            has_poss_pose,
            drns.map(|v| vec3_round_3(v.as_vec3())),
            drne.map(|v| vec3_round_3(v.as_vec3())),
            first_plax,
            bangle,
        );

        // 根据路径来源选择不同的转换函数
        // - POSS/POSE：路径沿 Z 轴 (0,0,0) -> (0,0,length)，world_transform 包含最终方位
        // - SPINE：使用相对坐标（相对于第一个 POINSP）
        let (segments, poss_pose_length) = if has_poss_pose {
            // POSS/POSE 场景：路径沿 Z 轴，长度为 delta.length()
            let poss = att.get_poss().unwrap();
            let pose = att.get_pose().unwrap();
            let length = (pose - poss).length();
            // 路径沿 Z 轴：(0,0,0) -> (0,0,length)
            (
                vec![convert_poss_pose_to_segment(Vec3::ZERO, Vec3::Z * length)],
                length,
            )
        } else {
            // SPINE 场景：使用相对坐标的路径段
            match convert_spine_to_segments(&spine_paths, first_plax, bangle) {
                Ok(segs) => (segs, 0.0),
                Err(e) => {
                    tracing::error!("转换 Spine 段失败: {:?}", e);
                    return Err(e);
                }
            }
        };

        if segments.is_empty() {
            tracing::warn!("转换后的路径段为空，跳过处理");
            return Ok(false);
        }

        // 为每个 profile 创建一个包含实际几何路径的 SweepSolid
        for (_i, geom) in geos.iter().enumerate() {
            if let CateGeoParam::Profile(profile) = geom {
                let Some(profile_refno) = profile.get_refno() else {
                    continue;
                };

                plax = profile.get_plax();

                let sweep_path = SweepPath3D::from_segments(segments.clone());

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
                    drns, // 使用实际读取的 DRNS 方向向量（与 core.dll 处理一致）
                    drne, // 使用实际读取的 DRNE 方向向量（与 core.dll 处理一致）
                    plax,
                    bangle, // 使用前面已计算的 bangle（对于 SCTN/STWALL 为 0）
                    extrude_dir,
                    height,
                    path: sweep_path,
                    lmirror: att.get_bool("LMIRR").unwrap_or_default(),
                    spine_segments: spine_paths.clone(), // 存储原始 Spine3D 段信息（用于调试）
                };

                // 判断是否为可复用的单线段+无倾斜场景
                let is_simple_line = loft.path.as_single_line().is_some() && !loft.is_sloped();
                let is_sloped_line = loft.path.as_single_line().is_some() && loft.is_sloped();

                // 计算 geo_transform：
                // - 单线段+无倾斜：使用 translation + rotation + scale（用于复用）
                // - 单线段+有倾斜：使用 translation + rotation，scale=ONE（长度在 mesh 中）
                // - 其他：IDENTITY（实际几何已在正确坐标系）
                let geo_transform = if has_poss_pose && is_simple_line {
                    // POSS/POSE 单线段+无倾斜：可复用
                    // world_transform 已包含最终方位，geo_transform 只保留 scale 用于单位 mesh 复用
                    Transform {
                        translation: Vec3::ZERO,
                        rotation: Quat::IDENTITY,
                        scale: Vec3::new(1.0, 1.0, poss_pose_length / 100.0), // 单位 mesh 长度为 100
                    }
                } else if has_poss_pose && is_sloped_line {
                    // POSS/POSE 单线段+有倾斜：不复用，mesh 已是实际长度
                    // world_transform 已包含最终方位，geo_transform 为 IDENTITY
                    Transform::IDENTITY
                } else if !has_poss_pose && is_simple_line {
                    // SPINE 单线段+无倾斜：从路径第一段计算 transform
                    if let Some(SegmentPath::Line(line)) = segments.first() {
                        let delta = line.end - line.start;
                        let length = delta.length();
                        let direction = delta.normalize_or_zero();

                        let rotation = if direction.abs_diff_eq(Vec3::Z, 1e-6) {
                            Quat::IDENTITY
                        } else if direction.abs_diff_eq(-Vec3::Z, 1e-6) {
                            Quat::from_rotation_x(std::f32::consts::PI)
                        } else {
                            Quat::from_rotation_arc(Vec3::Z, direction)
                        };

                        let bangle_rad = bangle.to_radians();
                        let final_rotation = if bangle_rad.abs() > 1e-6 {
                            rotation * Quat::from_rotation_z(bangle_rad)
                        } else {
                            rotation
                        };

                        // 对于 SPINE，起点是相对坐标，需要加上 spine_origin
                        let origin = spine_origin.unwrap_or(Vec3::ZERO);

                        Transform {
                            translation: origin + line.start,
                            rotation: final_rotation,
                            scale: Vec3::new(1.0, 1.0, length / 100.0),
                        }
                    } else {
                        Transform::IDENTITY
                    }
                } else if !has_poss_pose && is_sloped_line {
                    // SPINE 单线段+有倾斜：只用旋转
                    if let Some(SegmentPath::Line(line)) = segments.first() {
                        let delta = line.end - line.start;
                        let direction = delta.normalize_or_zero();

                        let rotation = if direction.abs_diff_eq(Vec3::Z, 1e-6) {
                            Quat::IDENTITY
                        } else if direction.abs_diff_eq(-Vec3::Z, 1e-6) {
                            Quat::from_rotation_x(std::f32::consts::PI)
                        } else {
                            Quat::from_rotation_arc(Vec3::Z, direction)
                        };

                        let bangle_rad = bangle.to_radians();
                        let final_rotation = if bangle_rad.abs() > 1e-6 {
                            rotation * Quat::from_rotation_z(bangle_rad)
                        } else {
                            rotation
                        };

                        let origin = spine_origin.unwrap_or(Vec3::ZERO);

                        Transform {
                            translation: origin + line.start,
                            rotation: final_rotation,
                            scale: Vec3::ONE,
                        }
                    } else {
                        Transform::IDENTITY
                    }
                } else {
                    // 多段/圆弧：路径坐标是相对于 spine_origin 的，需要将 spine_origin 作为偏移
                    // 否则 world_transform * IDENTITY 会丢失 spine_origin 的 XY 偏移
                    let origin = spine_origin.unwrap_or(Vec3::ZERO);
                    Transform {
                        translation: origin,
                        rotation: Quat::IDENTITY,
                        scale: Vec3::ONE,
                    }
                };

                // 根据元素类型调整 transform
                let transform = if type_name == "GENSEC" || type_name == "WALL" {
                    geo_transform
                } else if type_name == "STWALL" {
                    if is_simple_line {
                        // STWALL 无端面倾斜：使用 translation+scale，不旋转
                        Transform {
                            translation: geo_transform.translation,
                            rotation: Quat::IDENTITY,
                            scale: geo_transform.scale,
                        }
                    } else if is_sloped_line {
                        geo_transform
                    } else {
                        Transform::IDENTITY
                    }
                } else {
                    // SCTN 保持原有行为：只有偏移，不旋转
                    Transform {
                        translation: geo_transform.translation,
                        rotation: Quat::IDENTITY,
                        scale: geo_transform.scale,
                    }
                };

                if !geom_info.plin_points.is_empty() && !pline_snap_map.contains_key(&refno) {
                    let settings = LodMeshSettings::default();
                    let points = geom_info
                        .plin_points
                        .iter()
                        .filter_map(|pline| {
                            let [start, end] =
                                sweep_reference_endpoints(&loft, pline.position, &settings)?;
                            let start = transform.transform_point(start);
                            let end = transform.transform_point(end);
                            Some([
                                PlineSnapPoint {
                                    pkey: pline.pkey.clone(),
                                    kind: "pline_start".to_string(),
                                    point: start.to_array(),
                                },
                                PlineSnapPoint {
                                    pkey: pline.pkey.clone(),
                                    kind: "pline_end".to_string(),
                                    point: end.to_array(),
                                },
                            ])
                        })
                        .flatten()
                        .collect::<Vec<_>>();
                    pline_snap_map.insert(refno, points);
                }

                csg_shapes_map
                    .entry(refno)
                    .or_insert(Vec::new())
                    .push(CateCsgShape {
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
