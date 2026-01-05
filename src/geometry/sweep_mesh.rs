use crate::debug_macros::is_debug_model_enabled;
use crate::mesh_precision::LodMeshSettings;
use crate::parsed_data::CateProfileParam;
use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::prim_geo::profile_processor::ProfileProcessor;
use crate::prim_geo::spine::{Arc3D, Line3D, SegmentPath};
use crate::prim_geo::spine::{Spine3D, SweepPath3D};
use crate::prim_geo::sweep_solid::SweepSolid;
use crate::prim_geo::wire::CurveType;
use crate::shape::pdms_shape::PlantMesh;
use crate::types::refno::RefnoEnum;
use bevy_transform::prelude::Transform;
use glam::{DMat4, DQuat, DVec3, Mat3, Quat, Vec2, Vec3};
use i_triangle::float::triangulatable::Triangulatable;

/// 截面顶点数据
#[derive(Clone, Debug)]
struct ProfileVertex {
    pos: Vec2,
    normal: Vec2,
    u: f32, // 沿截面的纹理坐标
}

struct ProfileData {
    vertices: Vec<ProfileVertex>,
    is_smooth: bool,
    is_closed: bool, // 是否首尾相连 (如果是 true，会自动连接 last->first；如果是 false，视为条带)
}

/// 获取截面数据（顶点、法线、是否平滑）
/// 使用统一的 ProfileProcessor 处理，与 Extrusion 保持一致
fn get_profile_data(profile: &CateProfileParam, _refno: RefnoEnum) -> Option<ProfileData> {
    // 将 CateProfileParam 转换为 ProfileProcessor 需要的格式
    let (wires, profile_refno) = match profile {
        CateProfileParam::SPRO(spro) => {
            // 使用profile内部的refno，而不是传入的refno
            let profile_refno = Some(spro.refno);

            // SPRO: verts 是 Vec<Vec2>，frads 是 Vec<f32>
            // 需要转换为 Vec<Vec3>，其中 z 分量是 FRADIUS
            if spro.verts.len() != spro.frads.len() {
                return None;
            }
            let wire: Vec<Vec3> = spro
                .verts
                .iter()
                .zip(spro.frads.iter())
                .map(|(v, &frad)| Vec3::new(v.x, v.y, frad))
                .collect();
            (vec![wire], profile_refno)
        }
        CateProfileParam::SREC(srect) => {
            // SREC: 转换为矩形轮廓
            let half_size = srect.size / 2.0;
            let center = srect.center + srect.dxy;
            let wire = vec![
                Vec3::new(center.x - half_size.x, center.y - half_size.y, 0.0),
                Vec3::new(center.x + half_size.x, center.y - half_size.y, 0.0),
                Vec3::new(center.x + half_size.x, center.y + half_size.y, 0.0),
                Vec3::new(center.x - half_size.x, center.y + half_size.y, 0.0),
            ];
            (vec![wire], None)
        }
        CateProfileParam::SANN(sann) => {
            // SANN: 特殊处理，保持原有逻辑（圆弧截面）
            let radius = sann.pradius;
            let segments = 32;
            let angle = sann.pangle.to_radians();
            let start_angle = 0.0;

            let mut vertices: Vec<ProfileVertex> = Vec::with_capacity(segments + 1);
            let mut total_len = 0.0;

            // 无论是闭合圆还是圆弧，都生成 segments+1 个点
            // 对于闭合圆，最后一个点与第一个点位置重合，但 U 不同 (1.0)
            for i in 0..=segments {
                let theta = start_angle + (i as f32 / segments as f32) * angle;
                let cos_t = theta.cos();
                let sin_t = theta.sin();

                let x = radius * cos_t;
                let y = radius * sin_t;
                let pos = Vec2::new(x, y) + sann.plin_pos;
                let normal = Vec2::new(cos_t, sin_t); // 径向法线

                if i > 0 {
                    total_len += (pos - vertices[i - 1].pos).length();
                }

                vertices.push(ProfileVertex {
                    pos,
                    normal,
                    u: total_len,
                });
            }

            // 归一化 U
            if total_len > 0.0 {
                for v in &mut vertices {
                    v.u /= total_len;
                }
            }

            return Some(ProfileData {
                vertices,
                is_smooth: true,
                is_closed: false, // 已生成重合点，视为 Strip
            });
        }
        _ => return None,
    };

    // 使用 ProfileProcessor 处理截面（与 Extrusion 一致）
    let mut verts2d: Vec<Vec<Vec2>> = Vec::with_capacity(wires.len());
    let mut frads: Vec<Vec<f32>> = Vec::with_capacity(wires.len());
    for wire in &wires {
        let mut v2 = Vec::with_capacity(wire.len());
        let mut r = Vec::with_capacity(wire.len());
        for p in wire {
            v2.push(Vec2::new(p.x, p.y));
            r.push(p.z);
        }
        verts2d.push(v2);
        frads.push(r);
    }

    let processor = ProfileProcessor::from_wires(verts2d, frads, true).ok()?;
    let profile_refno_str = profile_refno.map(|r| r.to_string());
    let profile_refno_ref = profile_refno_str.as_deref();
    let processed = processor.process("SWEEP", profile_refno_ref).ok()?;

    // 从 ProcessedProfile 转换为 ProfileData
    // 使用 contour_points 作为轮廓点
    let mut vertices = Vec::new();
    let mut total_len = 0.0;
    let n = processed.contour_points.len();

    if n < 3 {
        return None;
    }

    // 计算轮廓总长度
    let mut perimeter = 0.0;
    for i in 0..n {
        let curr = processed.contour_points[i];
        let next = processed.contour_points[(i + 1) % n];
        perimeter += curr.distance(next);
    }

    // 生成顶点，计算累积长度作为 U 坐标
    let mut curr_len = 0.0;
    for i in 0..n {
        let curr = processed.contour_points[i];
        let next = processed.contour_points[(i + 1) % n];

        vertices.push(ProfileVertex {
            pos: curr,
            normal: Vec2::ZERO, // 法线由面生成
            u: if perimeter > 0.0 {
                curr_len / perimeter
            } else {
                0.0
            },
        });

        curr_len += curr.distance(next);
    }

    // 添加闭合点（如果首尾不重合）
    if !vertices.is_empty() && vertices[0].pos.distance(vertices.last().unwrap().pos) > 1e-6 {
        let first = vertices[0].clone();
        vertices.push(ProfileVertex {
            pos: first.pos,
            normal: first.normal,
            u: 1.0,
        });
    }

    Some(ProfileData {
        vertices,
        is_smooth: false, // ProfileProcessor 处理后的轮廓通常是硬表面
        is_closed: false, // 已包含闭合点，视为 Strip
    })
}

/// 构建截面变换矩阵（与 OCC 模式保持一致）
///
/// 变换顺序：
/// 1. 平移：应用 plin_pos 偏移（负值，因为要移到原点）
/// 2. 旋转：应用 bangle 绕 Z 轴旋转
/// 3. 镜像：如果 lmirror，X 轴取反
fn build_profile_transform_matrix(plin_pos: Vec2, bangle: f32, lmirror: bool) -> DMat4 {
    // 1. 平移：移到原点（负 plin_pos）
    let translation =
        DMat4::from_translation(DVec3::new(-plin_pos.x as f64, -plin_pos.y as f64, 0.0));

    // 2. 旋转：bangle 绕 Z 轴
    let rotation = if bangle.abs() > 0.001 {
        DQuat::from_rotation_z(bangle.to_radians() as f64)
    } else {
        DQuat::IDENTITY
    };
    let rotation_mat = DMat4::from_quat(rotation);

    // 3. 镜像：lmirror 时 X 轴取反
    let mirror_mat = if lmirror {
        DMat4::from_scale(DVec3::new(-1.0, 1.0, 1.0))
    } else {
        DMat4::IDENTITY
    };

    // 组合变换：先平移，再旋转，最后镜像
    mirror_mat * rotation_mat * translation
}

/// 对截面应用 plin_pos/lmirror 变换（BANG 已在 segment_transforms 的 Frenet 标架旋转中应用，此处不再重复旋转）
fn apply_profile_transform(mut profile: ProfileData, plin_pos: Vec2, lmirror: bool) -> ProfileData {
    // bangle 已在 normalize_spine_segments() 中通过 Frenet 标架计算并存储在 segment_transforms 中
    // 截面阶段只做平移和镜像，不应用 bangle
    let mat = build_profile_transform_matrix(plin_pos, 0.0, lmirror);

    for v in &mut profile.vertices {
        let p = mat.transform_point3(DVec3::new(v.pos.x as f64, v.pos.y as f64, 0.0));
        v.pos = Vec2::new(p.x as f32, p.y as f32);

        if v.normal.length_squared() > 0.0 {
            let n = mat.transform_vector3(DVec3::new(v.normal.x as f64, v.normal.y as f64, 0.0));
            v.normal = Vec2::new(n.x as f32, n.y as f32).normalize();
        }
    }

    profile
}

/// 路径采样点
struct PathSample {
    pos: Vec3,
    tangent: Vec3,
    rot: Mat3, // 局部坐标系 [Right, Up, Tangent]
    dist: f32, // 沿路径距离
}

/// 为圆弧路径计算径向坐标系（与 OCC 和 core.dll 保持一致）
///
/// OCC 对圆弧的处理:
/// - X 轴(right): 径向,从圆心指向当前点
/// - Y 轴(up): pref_axis (固定,用户指定)
/// - Z 轴(tangent): plax (切线方向)
fn sample_arc_frames(arc: &Arc3D, arc_segments: usize, plax: Vec3) -> Option<Vec<PathSample>> {
    let samples = arc_segments.max(4);
    let mut result = Vec::with_capacity(samples + 1);
    let mut total_dist = 0.0;
    let mut last_pos = arc.start_pt;

    // OCC 的截面坐标系定义:
    // y_axis = arc.pref_axis (截面的"上"方向,固定不变)
    // z_axis = plax (截面的法向,如果 clock_wise 则取反)
    // x_axis = y_axis.cross(z_axis) (截面的"右"方向)
    let profile_up = arc.pref_axis.normalize();
    let mut profile_normal = plax.normalize();
    if arc.clock_wise {
        profile_normal = -profile_normal;
    }

    // 检查 pref_axis 和 plax 是否平行（避免零向量叉积导致 NaN）
    let dot = profile_up.dot(profile_normal).abs();
    let (profile_right, profile_up_ortho) = if dot > 0.999 {
        // pref_axis 和 plax 几乎平行,使用 arc.axis 来构建坐标系
        eprintln!(
            "警告: pref_axis ({:?}) 和 plax ({:?}) 平行, 使用 arc.axis ({:?})",
            arc.pref_axis, plax, arc.axis
        );
        let right = arc.axis.cross(profile_normal).normalize();
        let up = profile_normal.cross(right).normalize();
        (right, up)
    } else {
        // 正常情况：pref_axis 和 plax 不平行
        let right = profile_up.cross(profile_normal).normalize();
        // 重新正交化 up 向量,确保坐标系是正交的
        let up = profile_normal.cross(right).normalize();
        (right, up)
    };

    for i in 0..=samples {
        let t = i as f32 / samples as f32;
        let angle_at_t = arc.angle * t;

        // 计算当前点的位置
        let rot_quat = Quat::from_axis_angle(arc.axis, angle_at_t);
        let pos = arc.center + rot_quat.mul_vec3(arc.start_pt - arc.center);

        // 计算切线
        let radial = (pos - arc.center).normalize();
        let tangent = arc.axis.cross(radial).normalize();
        let tangent = if arc.clock_wise { -tangent } else { tangent };

        // PathSample 的坐标系定义:
        // - right: 截面上的横向 (profile_right)
        // - up: 截面上的纵向 (profile_up_ortho)
        // - tangent: 路径切线方向 (实际切线,不是 plax)
        // 对于圆弧,截面保持固定方向(不随路径旋转)
        let rot = Mat3::from_cols(profile_right, profile_up_ortho, tangent);

        if i > 0 {
            total_dist += pos.distance(last_pos);
        }

        result.push(PathSample {
            pos,
            tangent,
            rot,
            dist: total_dist,
        });

        last_pos = pos;
    }

    Some(result)
}

/// 变换 Line3D 几何体
fn transform_line(line: &Line3D, transform: &Transform) -> Line3D {
    Line3D {
        start: transform.transform_point(line.start),
        end: transform.transform_point(line.end),
        is_spine: line.is_spine,
    }
}

/// 变换 Arc3D 几何体
fn transform_arc(arc: &Arc3D, transform: &Transform) -> SegmentPath {
    // 检查缩放类型
    let scale = transform.scale;
    let is_uniform_scale = (scale.x - scale.y).abs() < 1e-6 && (scale.y - scale.z).abs() < 1e-6;

    if is_uniform_scale {
        // 均匀缩放：直接变换参数
        SegmentPath::Arc(Arc3D {
            center: transform.transform_point(arc.center),
            start_pt: transform.transform_point(arc.start_pt),
            radius: arc.radius * scale.x,
            axis: (transform.rotation * arc.axis).normalize(),
            angle: arc.angle,
            clock_wise: arc.clock_wise,
            pref_axis: (transform.rotation * arc.pref_axis).normalize(),
        })
    } else {
        // 非均匀缩放：转换为多段线近似
        // TODO: 实现圆弧到多段线的转换
        SegmentPath::Arc(Arc3D {
            center: transform.transform_point(arc.center),
            start_pt: transform.transform_point(arc.start_pt),
            radius: arc.radius * scale.x, // 简化处理
            axis: (transform.rotation * arc.axis).normalize(),
            angle: arc.angle,
            clock_wise: arc.clock_wise,
            pref_axis: (transform.rotation * arc.pref_axis).normalize(),
        })
    }
}

/// 同步版本的路径采样，使用预计算的变换
fn sample_path_frames_sync(
    segments: &[SegmentPath],
    arc_segments_per_segment: usize,
    plax: Vec3, // 标准参考方向（调用方应传 Vec3::Z；圆弧分支内部使用 pref_axis/YDIR）
    segment_transforms: &[Transform], // 预计算的每段变换
) -> Option<Vec<PathSample>> {
    if segments.is_empty() {
        return None;
    }

    // 特殊处理：单段圆弧路径使用径向坐标系
    if segments.len() == 1 {
        if let SegmentPath::Arc(arc) = &segments[0] {
            // 变换圆弧段，安全处理空变换数组
            let transform = segment_transforms.first().unwrap_or(&Transform::IDENTITY);
            let transformed_arc = match transform_arc(arc, transform) {
                SegmentPath::Arc(arc) => arc,
                _ => return None,
            };

            return sample_arc_frames(&transformed_arc, arc_segments_per_segment, plax);
        }
    }

    // 1. 变换所有段
    let mut transformed_segments = Vec::new();
    for (i, segment) in segments.iter().enumerate() {
        // 安全获取变换，如果数组为空则使用单位变换
        let transform = segment_transforms.get(i).unwrap_or(&Transform::IDENTITY);

        let transformed_segment = match segment {
            SegmentPath::Line(line) => SegmentPath::Line(transform_line(line, transform)),
            SegmentPath::Arc(arc) => transform_arc(arc, transform),
        };
        transformed_segments.push(transformed_segment);
    }

    // 2. 从变换后的段收集采样点和切线
    let mut raw_samples = Vec::new();
    let mut total_dist = 0.0;
    let mut last_pos = transformed_segments[0].start_point();

    for segment in &transformed_segments {
        match segment {
            SegmentPath::Line(line) => {
                let start = line.start;
                let end = line.end;
                let dir = (end - start).normalize_or_zero();
                let len = line.length();

                if raw_samples.is_empty() {
                    raw_samples.push((start, dir, 0.0));
                }
                total_dist += len;
                raw_samples.push((end, dir, total_dist));
                last_pos = end;
            }
            SegmentPath::Arc(arc) => {
                let samples = arc_segments_per_segment.max(4);
                let arc_seg = SegmentPath::Arc(arc.clone());

                if raw_samples.is_empty() {
                    let p = arc_seg.point_at(0.0);
                    let t = arc_seg.tangent_at(0.0);
                    raw_samples.push((p, t, 0.0));
                }

                for i in 1..=samples {
                    let t_param = i as f32 / samples as f32;
                    let pos = arc_seg.point_at(t_param);
                    let tan = arc_seg.tangent_at(t_param);

                    let step_dist = pos.distance(last_pos);
                    total_dist += step_dist;

                    raw_samples.push((pos, tan, total_dist));
                    last_pos = pos;
                }
            }
        }
    }

    if raw_samples.len() < 2 {
        return None;
    }

    // 2. 计算第一点的坐标系
    let first_tan = raw_samples[0].1;

    // 修复：根据路径类型选择合适的参考方向（与 OCC 和 core.dll 保持一致）
    // - 对于圆弧路径：使用 arc.pref_axis (YDIR) 作为 Y 轴
    // - 对于 SPINE 直线路径：从 segments 中查找 pref_axis（参考 OCC 做法）
    // - 对于普通直线路径：使用 plax 作为参考方向
    let ref_up = match segments.first() {
        Some(SegmentPath::Arc(arc)) => {
            // 圆弧路径：使用 pref_axis 作为 Y 轴（对应 PDMS 的 YDIR 属性）
            // 这与 OCC 代码中的 `let y_axis = arc.pref_axis.as_dvec3()` 一致
            arc.pref_axis
        }
        Some(SegmentPath::Line(line)) if line.is_spine => {
            // SPINE 直线路径：参考 OCC 版本，从所有 segments 中查找 pref_axis
            // 遍历所有 segments，找到第一个 Arc 的 pref_axis；如果没有 Arc，使用默认值
            segments
                .iter()
                .find_map(|seg| {
                    if let SegmentPath::Arc(arc) = seg {
                        Some(arc.pref_axis)
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| {
                    // 如果没有找到 Arc，使用 plax 作为参考方向（与 OCC 版本保持一致）
                    // 如果 plax 与切线几乎平行，选择垂直方向
                    if first_tan.dot(plax).abs() > 0.9 {
                        // plax 与切线平行，选择一个垂直于切线的方向
                        let perp = if first_tan.dot(Vec3::X).abs() < 0.9 {
                            Vec3::X
                        } else {
                            Vec3::Y
                        };
                        // 使用叉积构建垂直于切线的 up 向量
                        let temp_right = perp.cross(first_tan).normalize();
                        first_tan.cross(temp_right).normalize()
                    } else {
                        // 使用 plax 作为 up 方向的参考
                        plax
                    }
                })
        }
        _ => {
            // 普通直线路径：使用 plax 作为参考方向
            // 如果 plax 与切线几乎平行，选择垂直方向
            if first_tan.dot(plax).abs() > 0.9 {
                // plax 与切线平行，选择一个垂直于切线的方向
                let perp = if first_tan.dot(Vec3::X).abs() < 0.9 {
                    Vec3::X
                } else {
                    Vec3::Y
                };
                // 使用叉积构建垂直于切线的 up 向量
                let temp_right = perp.cross(first_tan).normalize();
                first_tan.cross(temp_right).normalize()
            } else {
                // 使用 plax 作为 up 方向的参考
                plax
            }
        }
    };

    let first_right = ref_up.cross(first_tan).normalize();
    let first_up = first_tan.cross(first_right).normalize();

    let mut samples = Vec::with_capacity(raw_samples.len());
    let first_rot = Mat3::from_cols(first_right, first_up, first_tan);

    samples.push(PathSample {
        pos: raw_samples[0].0,
        tangent: first_tan,
        rot: first_rot,
        dist: 0.0,
    });

    // 3. 使用平行传输递推后续坐标系
    for i in 0..raw_samples.len() - 1 {
        let curr = &samples[i];
        let next_raw = &raw_samples[i + 1];

        let t1 = curr.tangent;
        let t2 = next_raw.1;

        let mut next_rot = curr.rot;

        let dot = t1.dot(t2).clamp(-1.0, 1.0);
        if dot < 0.9999 {
            let axis = t1.cross(t2);
            if axis.length_squared() > 0.0001 {
                let angle = dot.acos();
                let rot_quat = Quat::from_axis_angle(axis.normalize(), angle);

                let new_right = rot_quat * curr.rot.x_axis;
                let new_up = rot_quat * curr.rot.y_axis;

                // 重新正交化
                let final_right = new_up.cross(t2).normalize();
                let final_up = t2.cross(final_right).normalize();

                next_rot = Mat3::from_cols(final_right, final_up, t2);
            }
        }

        samples.push(PathSample {
            pos: next_raw.0,
            tangent: next_raw.1,
            rot: next_rot,
            dist: next_raw.2,
        });
    }

    Some(samples)
}

/// 计算平面裁剪偏移
fn compute_offset(local: Vec3, path_dir: Vec3, plane_normal: Vec3) -> f32 {
    let denom = plane_normal.dot(path_dir);
    if denom.abs() > 1e-6 {
        -plane_normal.dot(local) / denom
    } else {
        0.0
    }
}

/// 生成 Mesh
fn generate_mesh_from_frames(
    profile: &ProfileData,
    path_samples: &[PathSample],
    drns: Option<DVec3>,
    drne: Option<DVec3>,
) -> PlantMesh {
    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut indices = Vec::new();

    // 解析 Start/End 法线 (用于斜切)
    let start_tan = path_samples.first().unwrap().tangent;
    let end_tan = path_samples.last().unwrap().tangent;

    let resolve_cap_normal = |dir: Option<DVec3>, tangent: Vec3, fallback: Vec3| {
        if let Some(d) = dir {
            let v = d.as_vec3();
            if v.length_squared() > 0.001 {
                let mut n = v.normalize();
                // 若与路径方向几乎垂直，直接退回默认法线，避免偏移放大
                if n.dot(tangent).abs() < 0.1 {
                    return fallback;
                }
                // 确保法线朝向外 (背离路径方向)
                if fallback.dot(tangent) < 0.0 {
                    // Start
                    if n.dot(tangent) > 0.0 {
                        n = -n;
                    }
                } else {
                    // End
                    if n.dot(tangent) < 0.0 {
                        n = -n;
                    }
                }
                return n;
            }
        }
        fallback
    };

    let start_plane_normal = resolve_cap_normal(drns, start_tan, -start_tan);
    let end_plane_normal = resolve_cap_normal(drne, end_tan, end_tan);

    let num_rings = path_samples.len();
    let num_prof_verts = profile.vertices.len();

    if profile.is_smooth {
        // === 平滑模式 (Shared Vertices) ===
        for (i, sample) in path_samples.iter().enumerate() {
            let is_first = i == 0;
            let is_last = i == num_rings - 1;

            for pv in &profile.vertices {
                let local = sample.rot.x_axis * pv.pos.x + sample.rot.y_axis * pv.pos.y;
                let mut offset = 0.0;

                if is_first {
                    offset = compute_offset(local, sample.tangent, start_plane_normal);
                } else if is_last {
                    offset = compute_offset(local, sample.tangent, end_plane_normal);
                }

                let pos = sample.pos + local + sample.tangent * offset;
                let norm_3d =
                    (sample.rot.x_axis * pv.normal.x + sample.rot.y_axis * pv.normal.y).normalize();

                vertices.push(pos);
                normals.push(norm_3d);
                uvs.push([pv.u, sample.dist]);
            }
        }

        for i in 0..num_rings - 1 {
            for j in 0..num_prof_verts {
                if !profile.is_closed && j == num_prof_verts - 1 {
                    continue;
                }

                let curr = j;
                let next = (j + 1) % num_prof_verts;

                let base_curr = (i * num_prof_verts + curr) as u32;
                let base_next = (i * num_prof_verts + next) as u32;
                let next_ring_curr = ((i + 1) * num_prof_verts + curr) as u32;
                let next_ring_next = ((i + 1) * num_prof_verts + next) as u32;

                indices.extend_from_slice(&[
                    base_curr,
                    base_next,
                    next_ring_next,
                    base_curr,
                    next_ring_next,
                    next_ring_curr,
                ]);
            }
        }
    } else {
        // === 硬表面模式 (Faceted) ===
        for i in 0..num_rings - 1 {
            let s1 = &path_samples[i];
            let s2 = &path_samples[i + 1];

            let is_first_ring = i == 0;
            let is_last_ring = i == num_rings - 2;

            for j in 0..num_prof_verts {
                if !profile.is_closed && j == num_prof_verts - 1 {
                    continue;
                }
                let curr_idx = j;
                let next_idx = (j + 1) % num_prof_verts;

                let p1_2d = profile.vertices[curr_idx].pos;
                let p2_2d = profile.vertices[next_idx].pos;

                let calc_pos =
                    |sample: &PathSample, p2d: Vec2, is_start: bool, is_end: bool| -> Vec3 {
                        let local = sample.rot.x_axis * p2d.x + sample.rot.y_axis * p2d.y;
                        let mut offset = 0.0;
                        if is_start {
                            offset = compute_offset(local, sample.tangent, start_plane_normal);
                        } else if is_end {
                            offset = compute_offset(local, sample.tangent, end_plane_normal);
                        }
                        sample.pos + local + sample.tangent * offset
                    };

                let v1 = calc_pos(s1, p1_2d, is_first_ring, false);
                let v2 = calc_pos(s1, p2_2d, is_first_ring, false);
                let v3 = calc_pos(s2, p2_2d, false, is_last_ring);
                let v4 = calc_pos(s2, p1_2d, false, is_last_ring);

                let normal = (v2 - v1).cross(v4 - v1).normalize_or_zero();

                let base = vertices.len() as u32;
                vertices.push(v1);
                vertices.push(v2);
                vertices.push(v3);
                vertices.push(v4);
                normals.push(normal);
                normals.push(normal);
                normals.push(normal);
                normals.push(normal);

                let u1 = profile.vertices[curr_idx].u;
                let u2 = profile.vertices[next_idx].u;
                uvs.push([u1, s1.dist]);
                uvs.push([u2, s1.dist]);
                uvs.push([u2, s2.dist]);
                uvs.push([u1, s2.dist]);

                indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
            }
        }
    }

    // === 生成封口 (Caps) ===
    // 去除末尾重复点进行三角化
    let cap_points: Vec<Vec2> = profile
        .vertices
        .iter()
        .take(if profile.vertices.len() > 0 {
            profile.vertices.len() - 1
        } else {
            0
        })
        .map(|v| v.pos)
        .collect();

    if let Some(cap_mesh) = triangulate_polygon(&cap_points) {
        add_cap(
            &mut vertices,
            &mut normals,
            &mut uvs,
            &mut indices,
            &cap_mesh,
            &path_samples[0],
            start_plane_normal,
            true,
        );

        add_cap(
            &mut vertices,
            &mut normals,
            &mut uvs,
            &mut indices,
            &cap_mesh,
            path_samples.last().unwrap(),
            end_plane_normal,
            false,
        );
    }

    // 🆕 从 Profile 生成扫掠体的轮廓边
    let sweep_edges = generate_sweep_profile_edges(profile, path_samples);

    let mut mesh = PlantMesh {
        indices,
        vertices,
        normals,
        uvs,
        wire_vertices: Vec::new(),
        edges: sweep_edges,
        aabb: None,
    };

    // 同步 wire_vertices
    mesh.sync_wire_vertices_from_edges();

    mesh
}

/// 从 Profile 和路径采样点生成扫掠体的特征边
///
/// 生成的边包括：
/// - 起始截面的轮廓边
/// - 结束截面的轮廓边
///
/// 注意：不生成纵向边，以避免边数过多
fn generate_sweep_profile_edges(
    profile: &ProfileData,
    path_samples: &[PathSample],
) -> Vec<crate::shape::pdms_shape::Edge> {
    use crate::shape::pdms_shape::Edge;

    if path_samples.len() < 2 || profile.vertices.is_empty() {
        return Vec::new();
    }

    let mut edges = Vec::new();
    let n = profile.vertices.len();

    // 1. 起始截面的轮廓边
    let start_sample = &path_samples[0];
    for i in 0..n {
        let j = (i + 1) % n;
        if !profile.is_closed && j == 0 {
            break; // 开放轮廓不需要闭合边
        }

        let v0 = profile.vertices[i].pos;
        let v1 = profile.vertices[j].pos;

        let local0 = start_sample.rot.x_axis * v0.x + start_sample.rot.y_axis * v0.y;
        let local1 = start_sample.rot.x_axis * v1.x + start_sample.rot.y_axis * v1.y;

        let pos0 = start_sample.pos + local0;
        let pos1 = start_sample.pos + local1;

        edges.push(Edge::new(vec![pos0, pos1]));
    }

    // 2. 结束截面的轮廓边
    let end_sample = path_samples.last().unwrap();
    for i in 0..n {
        let j = (i + 1) % n;
        if !profile.is_closed && j == 0 {
            break;
        }

        let v0 = profile.vertices[i].pos;
        let v1 = profile.vertices[j].pos;

        let local0 = end_sample.rot.x_axis * v0.x + end_sample.rot.y_axis * v0.y;
        let local1 = end_sample.rot.x_axis * v1.x + end_sample.rot.y_axis * v1.y;

        let pos0 = end_sample.pos + local0;
        let pos1 = end_sample.pos + local1;

        edges.push(Edge::new(vec![pos0, pos1]));
    }

    edges
}

pub struct CapTriangulation {
    pub points: Vec<Vec2>,
    pub indices: Vec<u32>,
}

fn triangulate_polygon(points: &[Vec2]) -> Option<CapTriangulation> {
    if points.len() < 3 {
        return None;
    }
    let contour: Vec<[f32; 2]> = points.iter().map(|p| [p.x, p.y]).collect();
    let raw = contour.as_slice().triangulate();
    let triangulation = raw.to_triangulation::<u32>();
    if triangulation.indices.is_empty() {
        return None;
    }

    Some(CapTriangulation {
        points: triangulation
            .points
            .into_iter()
            .map(|p| Vec2::new(p[0], p[1]))
            .collect(),
        indices: triangulation.indices,
    })
}

fn add_cap(
    vertices: &mut Vec<Vec3>,
    normals: &mut Vec<Vec3>,
    uvs: &mut Vec<[f32; 2]>,
    indices: &mut Vec<u32>,
    cap: &CapTriangulation,
    sample: &PathSample,
    plane_normal: Vec3,
    _is_start: bool,
) {
    let base = vertices.len() as u32;

    for pt in &cap.points {
        let local = sample.rot.x_axis * pt.x + sample.rot.y_axis * pt.y;
        let offset = compute_offset(local, sample.tangent, plane_normal);
        let pos = sample.pos + local + sample.tangent * offset;

        vertices.push(pos);
        normals.push(plane_normal);
        uvs.push([pt.x, pt.y]);
    }

    let mut tri_indices = cap.indices.clone();
    if tri_indices.len() >= 3 {
        let p0 = vertices[base as usize + tri_indices[0] as usize];
        let p1 = vertices[base as usize + tri_indices[1] as usize];
        let p2 = vertices[base as usize + tri_indices[2] as usize];
        let n = (p1 - p0).cross(p2 - p0);
        // 确保面法线与封口法线方向一致
        if n.dot(plane_normal) < 0.0 {
            for chunk in tri_indices.chunks_exact_mut(3) {
                chunk.swap(1, 2);
            }
        }
    }

    for idx in tri_indices {
        indices.push(base + idx);
    }
}

fn compute_arc_segments(settings: &LodMeshSettings, arc_length: f32, radius: f32) -> usize {
    let base_segments = settings.radial_segments as usize;
    if let Some(target_len) = settings.target_segment_length {
        let computed = (arc_length / target_len).ceil() as usize;
        return computed.clamp(
            settings.min_radial_segments as usize,
            settings.max_radial_segments.unwrap_or(64) as usize,
        );
    }
    let length_factor = (arc_length / 100.0).clamp(0.5, 3.0);
    let radius_factor = (radius / 50.0).clamp(0.5, 2.0);
    ((base_segments as f32 * length_factor * radius_factor) as usize).clamp(
        settings.min_radial_segments as usize,
        settings.max_radial_segments.unwrap_or(64) as usize,
    )
}

pub fn generate_sweep_solid_mesh(
    sweep: &SweepSolid,
    settings: &LodMeshSettings,
    refno: RefnoEnum,
) -> Option<PlantMesh> {
    // 正常生成截面数据并应用截面自身变换（plin_pos/bangle/lmirror）
    let profile = get_profile_data(&sweep.profile, refno)?;
    let profile = apply_profile_transform(profile, sweep.profile.get_plin_pos(), sweep.lmirror);

    let arc_segments = if sweep.path.is_single_segment() {
        if let Some(arc) = sweep.path.as_single_arc() {
            compute_arc_segments(settings, arc.angle.abs() * arc.radius, arc.radius)
        } else {
            1
        }
    } else {
        (settings.radial_segments as usize / 2).clamp(settings.min_radial_segments as usize, 32)
    };

    // 使用预计算的变换进行路径采样
    // plax 由 SweepSolid 提供，决定直线路径的参考朝向
    let frames = sample_path_frames_sync(
        &sweep.path.segments,
        arc_segments,
        sweep.plax,
        &sweep.segment_transforms,
    )?;

    // 正常生成 mesh（不再需要后处理变换）
    let mesh = generate_mesh_from_frames(&profile, &frames, sweep.drns, sweep.drne);

    Some(mesh)
}

/// 从 SweepPath 提取 Spine3D 段信息（临时实现）
fn extract_spine_segments_from_sweep_path(_path: &SweepPath3D) -> Option<Vec<Spine3D>> {
    // TODO: 需要从调用方传递完整的 Spine3D 信息
    // 暂时返回空，这会导致变换失败
    // 需要修改调用链来传递 Spine3D 信息
    None
}
