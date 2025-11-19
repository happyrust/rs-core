use crate::mesh_precision::LodMeshSettings;
use crate::parsed_data::CateProfileParam;
/// Sweep Mesh Generation
///
/// 为 SweepSolid 生成 CSG 网格，不依赖 truck feature
/// 参考 revolution 的实现方式，直接生成顶点、法线和索引
use crate::prim_geo::spine::{Arc3D, Line3D, SegmentPath, SweepPath3D};
use crate::prim_geo::sweep_solid::SweepSolid;
use crate::shape::pdms_shape::PlantMesh;
use glam::{DVec3, Mat3, Vec2, Vec3};
use i_triangle::float::triangulatable::Triangulatable;

/// 生成给定截面的2D轮廓点
fn get_profile_points(profile: &CateProfileParam) -> Option<Vec<Vec2>> {
    match profile {
        CateProfileParam::SANN(sann) => {
            // 圆形截面
            let radius = sann.pradius;
            let segments = 32; // 圆形分段数
            let angle = sann.pangle.to_radians();

            let mut points = Vec::new();
            for i in 0..segments {
                let theta = (i as f32 / segments as f32) * angle;
                let x = radius * theta.cos();
                let y = radius * theta.sin();
                points.push(Vec2::new(x, y) + sann.plin_pos);
            }
            Some(points)
        }
        CateProfileParam::SREC(srect) => {
            // 矩形截面
            let half_size = srect.size / 2.0;
            let center = srect.center + srect.dxy;

            Some(vec![
                center + Vec2::new(-half_size.x, -half_size.y),
                center + Vec2::new(half_size.x, -half_size.y),
                center + Vec2::new(half_size.x, half_size.y),
                center + Vec2::new(-half_size.x, half_size.y),
            ])
        }
        CateProfileParam::SPRO(spro) => {
            // 通用轮廓
            Some(spro.verts.clone())
        }
        _ => None,
    }
}

struct CapTriangulation {
    points: Vec<Vec2>,
    indices: Vec<u32>,
}

fn triangulate_polygon(profile_points: &[Vec2]) -> Option<CapTriangulation> {
    if profile_points.len() < 3 {
        return None;
    }

    let contour: Vec<[f32; 2]> = profile_points.iter().map(|p| [p.x, p.y]).collect();

    let raw = contour.as_slice().triangulate();
    let triangulation = raw.to_triangulation::<u32>();

    if triangulation.indices.is_empty() {
        return None;
    }

    let points = triangulation
        .points
        .into_iter()
        .map(|p| Vec2::new(p[0], p[1]))
        .collect();

    Some(CapTriangulation {
        points,
        indices: triangulation.indices,
    })
}

fn polygon_signed_area(points: &[Vec2]) -> f32 {
    let mut area = 0.0f32;
    if points.len() < 3 {
        return area;
    }

    for i in 0..points.len() {
        let p0 = points[i];
        let p1 = points[(i + 1) % points.len()];
        area += p0.x * p1.y - p1.x * p0.y;
    }

    area * 0.5
}

fn compute_path_offset(local: Vec3, path_dir: Vec3, plane_normal: Vec3) -> f32 {
    let denom = plane_normal.dot(path_dir);
    if denom.abs() > 1e-6 {
        -plane_normal.dot(local) / denom
    } else {
        0.0
    }
}

fn append_cap(
    cap: &CapTriangulation,
    origin: Vec3,
    right: Vec3,
    up: Vec3,
    path_dir: Vec3,
    normal: Vec3,
    _is_start: bool,
    vertices: &mut Vec<Vec3>,
    normals: &mut Vec<Vec3>,
    indices: &mut Vec<u32>,
) {
    let base = vertices.len() as u32;

    for point in &cap.points {
        let local = right * point.x + up * point.y;
        let offset = compute_path_offset(local, path_dir, normal);
        let vertex = origin + local + path_dir * offset;
        vertices.push(vertex);
        normals.push(normal);
    }

    let mut cap_tris: Vec<[u32; 3]> = cap
        .indices
        .chunks(3)
        .filter_map(|tri| {
            if tri.len() == 3 {
                Some([base + tri[0], base + tri[1], base + tri[2]])
            } else {
                None
            }
        })
        .collect();

    if let Some(first) = cap_tris.first() {
        let a = vertices[first[0] as usize];
        let b = vertices[first[1] as usize];
        let c = vertices[first[2] as usize];
        let face_normal = (b - a).cross(c - a);
        if face_normal.dot(normal) < 0.0 {
            for tri in &mut cap_tris {
                tri.swap(1, 2);
            }
        }
    }

    for tri in cap_tris {
        indices.extend_from_slice(&tri);
    }
}

/// 沿单段直线路径生成 sweep mesh
fn generate_line_sweep(
    profile_points: &[Vec2],
    line: &Line3D,
    _transform: &Mat3,
    drns: Option<DVec3>,
    drne: Option<DVec3>,
) -> Option<PlantMesh> {
    if profile_points.len() < 3 {
        return None;
    }

    let n_profile = profile_points.len();
    let path_vec = line.end - line.start;
    if path_vec.length_squared() < 1e-6 {
        return None;
    }

    let path_dir = path_vec.normalize();
    // 构建局部坐标系
    let (right, up) = {
        let ref_vec = if path_dir.y.abs() < 0.9 {
            Vec3::Y
        } else {
            Vec3::X
        };
        let right = ref_vec.cross(path_dir).normalize();
        let up = path_dir.cross(right).normalize();
        (right, up)
    };

    let resolve_cap_normal = |dir: Option<DVec3>, fallback: Vec3| {
        if let Some(candidate) = dir {
            let vec = candidate.as_vec3();
            if let Some(mut norm) = vec.try_normalize() {
                if norm.dot(path_dir).abs() > 0.9 {
                    if fallback.dot(path_dir).is_sign_negative() {
                        if norm.dot(path_dir) > 0.0 {
                            norm = -norm;
                        }
                    } else if norm.dot(path_dir) < 0.0 {
                        norm = -norm;
                    }
                    return norm;
                }
            }
        }
        fallback
    };

    let mut start_normal = resolve_cap_normal(drns, -path_dir);
    let mut end_normal = resolve_cap_normal(drne, path_dir);

    if start_normal.length_squared() < 1e-6 {
        start_normal = -path_dir;
    }
    if end_normal.length_squared() < 1e-6 {
        end_normal = path_dir;
    }

    let mut start_positions = Vec::with_capacity(n_profile);
    let mut end_positions = Vec::with_capacity(n_profile);

    for &profile_pt in profile_points.iter() {
        let local_3d = right * profile_pt.x + up * profile_pt.y;

        let start_offset = compute_path_offset(local_3d, path_dir, start_normal);
        let end_offset = compute_path_offset(local_3d, path_dir, end_normal);

        let start_vertex = line.start + local_3d + path_dir * start_offset;
        let end_vertex = line.end + local_3d + path_dir * end_offset;

        start_positions.push(start_vertex);
        end_positions.push(end_vertex);
    }

    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();

    let area = polygon_signed_area(profile_points);
    let is_ccw = area >= 0.0;

    for i in 0..n_profile {
        let next = (i + 1) % n_profile;

        let edge_2d = profile_points[next] - profile_points[i];
        if edge_2d.length_squared() < 1e-8 {
            continue;
        }

        let normal_2d = if is_ccw {
            Vec2::new(edge_2d.y, -edge_2d.x)
        } else {
            Vec2::new(-edge_2d.y, edge_2d.x)
        };

        if normal_2d.length_squared() < 1e-8 {
            continue;
        }

        let mut face_normal = right * normal_2d.x + up * normal_2d.y;
        let Some(mut face_normal) = face_normal.try_normalize() else {
            continue;
        };

        let start_a = start_positions[i];
        let end_a = end_positions[i];
        let start_b = start_positions[next];
        let end_b = end_positions[next];

        let tri_normal = (end_a - start_a).cross(start_b - start_a);
        if tri_normal.length_squared() < 1e-8 {
            continue;
        }

        if tri_normal.dot(face_normal) < 0.0 {
            face_normal = -face_normal;
        }

        let base = vertices.len() as u32;

        vertices.push(start_a);
        normals.push(face_normal);
        vertices.push(end_a);
        normals.push(face_normal);
        vertices.push(start_b);
        normals.push(face_normal);
        vertices.push(end_b);
        normals.push(face_normal);

        indices.extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 1, base + 3]);
    }

    let cap_triangulation = triangulate_polygon(profile_points);
    if let Some(cap) = cap_triangulation.as_ref() {
        append_cap(
            cap,
            line.start,
            right,
            up,
            path_dir,
            start_normal,
            true,
            &mut vertices,
            &mut normals,
            &mut indices,
        );

        append_cap(
            cap,
            line.end,
            right,
            up,
            path_dir,
            end_normal,
            false,
            &mut vertices,
            &mut normals,
            &mut indices,
        );
    }

    let mut mesh = PlantMesh {
        indices,
        vertices,
        normals,
        uvs: Vec::new(),
        wire_vertices: Vec::new(),
        edges: Vec::new(),
        aabb: None,
    };
    mesh.generate_auto_uvs();
    Some(mesh)
}

/// 沿单段圆弧路径生成 sweep mesh
fn generate_arc_sweep(
    profile_points: &[Vec2],
    arc: &Arc3D,
    arc_segments: usize,
) -> Option<PlantMesh> {
    if profile_points.len() < 3 {
        return None;
    }

    let n_profile = profile_points.len();
    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();

    // 沿圆弧采样点和切线
    let arc_segment = SegmentPath::Arc(arc.clone());
    for i in 0..=arc_segments {
        let t = i as f32 / arc_segments as f32;
        let position = arc_segment.point_at(t);
        let tangent = arc_segment.tangent_at(t);

        // 构建局部坐标系（与路径切线垂直）
        let (right, up) = {
            let ref_vec = if tangent.y.abs() < 0.9 {
                Vec3::Y
            } else {
                Vec3::X
            };
            let right = ref_vec.cross(tangent).normalize();
            let up = tangent.cross(right).normalize();
            (right, up)
        };

        // 生成截面环
        for &profile_pt in profile_points.iter() {
            let local_3d = right * profile_pt.x + up * profile_pt.y;
            let vertex = position + local_3d;
            vertices.push(vertex);

            // 计算表面法向量
            // 对于扫掠表面，法向量应该垂直于扫掠方向和轮廓切线方向
            // 这里使用径向方向作为法向量的近似（对于简单轮廓）
            let normal = if local_3d.length_squared() > 1e-6 {
                local_3d.normalize()
            } else {
                // 如果轮廓点太靠近中心，使用up方向作为法向量
                up
            };
            normals.push(normal);
        }
    }

    // 生成侧面索引（连接相邻的截面环）
    for ring_idx in 0..arc_segments {
        for i in 0..(n_profile - 1) {
            let base = (ring_idx * n_profile + i) as u32;
            let next_ring_base = ((ring_idx + 1) * n_profile + i) as u32;

            indices.extend_from_slice(&[
                base,
                next_ring_base,
                base + 1,
                base + 1,
                next_ring_base,
                next_ring_base + 1,
            ]);
        }

        // 封闭轮廓（连接最后一点和第一点）
        if profile_points.len() > 3 {
            let base = (ring_idx * n_profile + n_profile - 1) as u32;
            let next_ring_base = ((ring_idx + 1) * n_profile + n_profile - 1) as u32;
            let ring_start = (ring_idx * n_profile) as u32;
            let next_ring_start = ((ring_idx + 1) * n_profile) as u32;

            indices.extend_from_slice(&[
                base,
                next_ring_base,
                ring_start,
                ring_start,
                next_ring_base,
                next_ring_start,
            ]);
        }
    }

    // 添加起始端面和结束端面（使用三角剖分）
    let cap_triangulation = triangulate_polygon(profile_points);
    if let Some(cap) = cap_triangulation.as_ref() {
        // 起始端面
        let start_tangent = arc_segment.tangent_at(0.0);
        let start_position = arc_segment.point_at(0.0);
        let (right_start, up_start) = {
            let ref_vec = if start_tangent.y.abs() < 0.9 {
                Vec3::Y
            } else {
                Vec3::X
            };
            let right = ref_vec.cross(start_tangent).normalize();
            let up = start_tangent.cross(right).normalize();
            (right, up)
        };

        append_cap(
            cap,
            start_position,
            right_start,
            up_start,
            start_tangent,
            -start_tangent,
            true,
            &mut vertices,
            &mut normals,
            &mut indices,
        );

        // 结束端面
        let end_tangent = arc_segment.tangent_at(1.0);
        let end_position = arc_segment.point_at(1.0);
        let (right_end, up_end) = {
            let ref_vec = if end_tangent.y.abs() < 0.9 {
                Vec3::Y
            } else {
                Vec3::X
            };
            let right = ref_vec.cross(end_tangent).normalize();
            let up = end_tangent.cross(right).normalize();
            (right, up)
        };

        append_cap(
            cap,
            end_position,
            right_end,
            up_end,
            end_tangent,
            end_tangent,
            false,
            &mut vertices,
            &mut normals,
            &mut indices,
        );
    }

    let mut mesh = PlantMesh {
        indices,
        vertices,
        normals,
        uvs: Vec::new(),
        wire_vertices: Vec::new(),
        edges: Vec::new(),
        aabb: None,
    };
    mesh.generate_auto_uvs();
    Some(mesh)
}

/// 沿多段路径生成 sweep mesh
fn generate_multi_segment_sweep(
    profile_points: &[Vec2],
    segments: &[SegmentPath],
    arc_segments_per_segment: usize,
) -> Option<PlantMesh> {
    if profile_points.len() < 3 || segments.is_empty() {
        return None;
    }

    let n_profile = profile_points.len();

    // 计算每段的关键点和切线
    let mut path_samples = Vec::new();

    for segment in segments {
        match segment {
            SegmentPath::Line(line) => {
                let start = line.start;
                let end = line.end;
                let dir = (end - start).normalize();

                // 直线段只需要起点
                if path_samples.is_empty() {
                    path_samples.push((start, dir));
                }
                // 总是添加终点
                path_samples.push((end, dir));
            }
            SegmentPath::Arc(arc) => {
                // 圆弧段需要采样多个点
                let samples = arc_segments_per_segment.max(4);
                let arc_seg = SegmentPath::Arc(arc.clone());

                // 如果是第一段，添加起点
                if path_samples.is_empty() {
                    let start_pos = arc_seg.point_at(0.0);
                    let start_tan = arc_seg.tangent_at(0.0);
                    path_samples.push((start_pos, start_tan));
                }

                // 添加中间采样点和终点
                for i in 1..=samples {
                    let t = i as f32 / samples as f32;
                    let pos = arc_seg.point_at(t);
                    let tan = arc_seg.tangent_at(t);
                    path_samples.push((pos, tan));
                }
            }
        }
    }

    if path_samples.len() < 2 {
        return None;
    }

    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();

    // 为每个路径采样点生成轮廓
    for &(position, tangent) in &path_samples {
        // 构建局部坐标系
        let (right, up) = {
            let ref_vec = if tangent.y.abs() < 0.9 {
                Vec3::Y
            } else {
                Vec3::X
            };
            let right = ref_vec.cross(tangent).normalize();
            let up = tangent.cross(right).normalize();
            (right, up)
        };

        for &profile_pt in profile_points.iter() {
            let local_3d = right * profile_pt.x + up * profile_pt.y;
            let vertex = position + local_3d;
            vertices.push(vertex);

            let normal = local_3d.normalize();
            normals.push(normal);
        }
    }

    // 生成侧面索引（连接相邻的轮廓环）
    for ring_idx in 0..(path_samples.len() - 1) {
        for i in 0..(n_profile - 1) {
            let base = (ring_idx * n_profile + i) as u32;
            let next_ring_base = ((ring_idx + 1) * n_profile + i) as u32;

            indices.extend_from_slice(&[
                base,
                next_ring_base,
                base + 1,
                base + 1,
                next_ring_base,
                next_ring_base + 1,
            ]);
        }

        // 封闭轮廓
        if profile_points.len() > 3 {
            let base = (ring_idx * n_profile + n_profile - 1) as u32;
            let next_ring_base = ((ring_idx + 1) * n_profile + n_profile - 1) as u32;
            let ring_start = (ring_idx * n_profile) as u32;
            let next_ring_start = ((ring_idx + 1) * n_profile) as u32;

            indices.extend_from_slice(&[
                base,
                next_ring_base,
                ring_start,
                ring_start,
                next_ring_base,
                next_ring_start,
            ]);
        }
    }

    // 添加起始端面和结束端面（使用三角剖分）
    let cap_triangulation = triangulate_polygon(profile_points);
    if let Some(cap) = cap_triangulation.as_ref() {
        // 起始端面
        let first_tangent = path_samples[0].1;
        let first_position = path_samples[0].0;
        let (right_start, up_start) = {
            let ref_vec = if first_tangent.y.abs() < 0.9 {
                Vec3::Y
            } else {
                Vec3::X
            };
            let right = ref_vec.cross(first_tangent).normalize();
            let up = first_tangent.cross(right).normalize();
            (right, up)
        };

        append_cap(
            cap,
            first_position,
            right_start,
            up_start,
            first_tangent,
            -first_tangent,
            true,
            &mut vertices,
            &mut normals,
            &mut indices,
        );

        // 结束端面
        let last_tangent = path_samples.last().unwrap().1;
        let last_position = path_samples.last().unwrap().0;
        let (right_end, up_end) = {
            let ref_vec = if last_tangent.y.abs() < 0.9 {
                Vec3::Y
            } else {
                Vec3::X
            };
            let right = ref_vec.cross(last_tangent).normalize();
            let up = last_tangent.cross(right).normalize();
            (right, up)
        };

        append_cap(
            cap,
            last_position,
            right_end,
            up_end,
            last_tangent,
            last_tangent,
            false,
            &mut vertices,
            &mut normals,
            &mut indices,
        );
    }

    let mut mesh = PlantMesh {
        indices,
        vertices,
        normals,
        uvs: Vec::new(),
        wire_vertices: Vec::new(),
        edges: Vec::new(),
        aabb: None,
    };
    mesh.generate_auto_uvs();
    Some(mesh)
}

/// 根据设置计算圆弧分段数
fn compute_arc_segments(settings: &LodMeshSettings, arc_length: f32, radius: f32) -> usize {
    // 使用 radial_segments 作为基准分段数
    let base_segments = settings.radial_segments as usize;

    // 如果设置了 target_segment_length，基于目标段长计算
    if let Some(target_len) = settings.target_segment_length {
        let computed = (arc_length / target_len).ceil() as usize;
        return computed
            .max(settings.min_radial_segments as usize)
            .min(settings.max_radial_segments.unwrap_or(64) as usize);
    }

    // 根据圆弧长度和半径进行自适应调整
    let length_factor = (arc_length / 100.0).max(0.5).min(3.0);
    let radius_factor = (radius / 50.0).max(0.5).min(2.0);

    ((base_segments as f32 * length_factor * radius_factor) as usize)
        .max(settings.min_radial_segments as usize)
        .min(settings.max_radial_segments.unwrap_or(64) as usize)
}

/// 应用截面旋转和方向控制
fn apply_profile_transform(
    profile_points: &[Vec2],
    plax: Vec3,
    bangle: f32,
    lmirror: bool,
) -> Vec<Vec2> {
    let mut transformed = profile_points.to_vec();

    // 应用旋转角度 bangle
    if bangle.abs() > 0.001 {
        let cos_b = bangle.to_radians().cos();
        let sin_b = bangle.to_radians().sin();
        for pt in &mut transformed {
            let x = pt.x * cos_b - pt.y * sin_b;
            let y = pt.x * sin_b + pt.y * cos_b;
            *pt = Vec2::new(x, y);
        }
    }

    // 应用镜像
    if lmirror {
        for pt in &mut transformed {
            pt.x = -pt.x;
        }
    }

    transformed
}

/// 为 SweepSolid 生成 CSG mesh（支持完整功能）
pub fn generate_sweep_solid_mesh(
    sweep: &SweepSolid,
    settings: &LodMeshSettings,
) -> Option<PlantMesh> {
    // 获取截面轮廓点
    let mut profile_points = get_profile_points(&sweep.profile)?;

    // 应用截面变换（plax, bangle, lmirror）
    profile_points =
        apply_profile_transform(&profile_points, sweep.plax, sweep.bangle, sweep.lmirror);

    // 根据路径类型生成mesh
    if sweep.path.is_single_segment() {
        if let Some(line) = sweep.path.as_single_line() {
            // 单段直线路径
            let transform = Mat3::IDENTITY;
            return generate_line_sweep(&profile_points, line, &transform, sweep.drns, sweep.drne);
        } else if let Some(arc) = sweep.path.as_single_arc() {
            // 单段圆弧路径 - 使用LOD设置计算分段数
            let arc_segment = SegmentPath::Arc(arc.clone());
            let arc_length = arc_segment.length();
            let arc_segments = compute_arc_segments(settings, arc_length, arc.radius);

            tracing::debug!(
                "生成圆弧sweep: 半径={:.2}, 角度={:.2}°, 长度={:.2}, 分段数={}",
                arc.radius,
                arc.angle.to_degrees(),
                arc_length,
                arc_segments
            );

            return generate_arc_sweep(&profile_points, arc, arc_segments);
        }
    } else {
        // 多段路径 - 为每个圆弧段计算合适的分段数
        // 使用 radial_segments 的一半作为每个圆弧段的默认分段数
        let arc_segments = (settings.radial_segments as usize / 2)
            .max(settings.min_radial_segments as usize)
            .min(32);

        return generate_multi_segment_sweep(&profile_points, &sweep.path.segments, arc_segments);
    }

    None
}
