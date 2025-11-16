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

fn append_cap(
    cap: &CapTriangulation,
    origin: Vec3,
    right: Vec3,
    up: Vec3,
    path_dir: Vec3,
    normal: Vec3,
    is_start: bool,
    vertices: &mut Vec<Vec3>,
    normals: &mut Vec<Vec3>,
    indices: &mut Vec<u32>,
) {
    let base = vertices.len() as u32;

    for point in &cap.points {
        let normal_z = normal.dot(path_dir);
        let normal_y = normal.dot(up);
        let z_offset = if normal_z.abs() > 0.001 {
            let sign = if is_start { -1.0 } else { 1.0 };
            sign * point.y * (normal_y / normal_z)
        } else {
            0.0
        };

        let local = right * point.x + up * point.y;
        let vertex = origin + local + path_dir * z_offset;
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
    let path_dir = (line.end - line.start).normalize();
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

    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();

    // 计算端面方向
    let start_normal = if let Some(dir) = drns {
        -dir.as_vec3().normalize()
    } else {
        -path_dir
    };

    let end_normal = if let Some(dir) = drne {
        dir.as_vec3().normalize()
    } else {
        path_dir
    };

    // 生成起始位置的顶点（考虑drns倾斜）
    for &profile_pt in profile_points.iter() {
        let local_3d = right * profile_pt.x + up * profile_pt.y;

        // 计算该点沿路径方向的偏移，使其位于倾斜的端面上
        // 对于斜切端面，不同Y坐标的点需要不同的Z偏移
        let z_offset = if drns.is_some() {
            // 计算倾斜导致的路径方向偏移
            // start_normal 和 path_dir 之间的夹角决定了倾斜程度
            let normal_z = start_normal.dot(path_dir);
            let normal_y = start_normal.dot(up);

            if normal_z.abs() > 0.001 {
                // y坐标越大，沿path_dir的偏移越大
                -profile_pt.y * (normal_y / normal_z)
            } else {
                0.0
            }
        } else {
            0.0
        };

        let vertex = line.start + local_3d + path_dir * z_offset;
        vertices.push(vertex);

        let normal = local_3d.normalize();
        normals.push(normal);
    }

    // 生成结束位置的顶点（考虑drne倾斜）
    for &profile_pt in profile_points.iter() {
        let local_3d = right * profile_pt.x + up * profile_pt.y;

        // 计算该点沿路径方向的偏移
        let z_offset = if drne.is_some() {
            let normal_z = end_normal.dot(path_dir);
            let normal_y = end_normal.dot(up);

            if normal_z.abs() > 0.001 {
                profile_pt.y * (normal_y / normal_z)
            } else {
                0.0
            }
        } else {
            0.0
        };

        let vertex = line.end + local_3d + path_dir * z_offset;
        vertices.push(vertex);

        let normal = local_3d.normalize();
        normals.push(normal);
    }

    // 生成侧面索引
    for i in 0..(n_profile - 1) {
        let i0 = i as u32;
        let i1 = (i + 1) as u32;
        let i2 = (i + n_profile) as u32;
        let i3 = (i + 1 + n_profile) as u32;

        // 两个三角形组成一个四边形
        indices.extend_from_slice(&[i0, i2, i1, i1, i2, i3]);
    }

    // 封闭轮廓（连接最后一点和第一点）
    if profile_points.len() > 3 {
        let i0 = (n_profile - 1) as u32;
        let i1 = 0u32;
        let i2 = (2 * n_profile - 1) as u32;
        let i3 = n_profile as u32;
        indices.extend_from_slice(&[i0, i2, i1, i1, i2, i3]);
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

    Some(PlantMesh {
        indices,
        vertices,
        normals,
        wire_vertices: Vec::new(),
        edges: Vec::new(),
        aabb: None,
    })
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

            // 法向量指向外部（径向方向）
            let normal = local_3d.normalize();
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

    // 添加起始端面
    let start_tangent = arc_segment.tangent_at(0.0);
    let start_center_idx = vertices.len() as u32;
    let profile_center =
        profile_points.iter().fold(Vec2::ZERO, |acc, &p| acc + p) / profile_points.len() as f32;

    let (right, up) = {
        let ref_vec = if start_tangent.y.abs() < 0.9 {
            Vec3::Y
        } else {
            Vec3::X
        };
        let right = ref_vec.cross(start_tangent).normalize();
        let up = start_tangent.cross(right).normalize();
        (right, up)
    };

    let center_3d_start =
        arc_segment.point_at(0.0) + right * profile_center.x + up * profile_center.y;
    vertices.push(center_3d_start);
    normals.push(-start_tangent);

    for i in 0..(n_profile - 1) {
        indices.extend_from_slice(&[start_center_idx, i as u32, (i + 1) as u32]);
    }
    if profile_points.len() > 3 {
        indices.extend_from_slice(&[start_center_idx, (n_profile - 1) as u32, 0]);
    }

    // 添加结束端面
    let end_tangent = arc_segment.tangent_at(1.0);
    let end_center_idx = vertices.len() as u32;
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

    let center_3d_end =
        arc_segment.point_at(1.0) + right_end * profile_center.x + up_end * profile_center.y;
    vertices.push(center_3d_end);
    normals.push(end_tangent);

    let last_ring_base = (arc_segments * n_profile) as u32;
    for i in 0..(n_profile - 1) {
        indices.extend_from_slice(&[
            end_center_idx,
            last_ring_base + (i + 1) as u32,
            last_ring_base + i as u32,
        ]);
    }
    if profile_points.len() > 3 {
        indices.extend_from_slice(&[
            end_center_idx,
            last_ring_base,
            last_ring_base + (n_profile - 1) as u32,
        ]);
    }

    Some(PlantMesh {
        indices,
        vertices,
        normals,
        wire_vertices: Vec::new(),
        edges: Vec::new(),
        aabb: None,
    })
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

    // 添加起始端面
    let first_tangent = path_samples[0].1;
    let start_center_idx = vertices.len() as u32;
    let profile_center =
        profile_points.iter().fold(Vec2::ZERO, |acc, &p| acc + p) / profile_points.len() as f32;

    let (right, up) = {
        let ref_vec = if first_tangent.y.abs() < 0.9 {
            Vec3::Y
        } else {
            Vec3::X
        };
        let right = ref_vec.cross(first_tangent).normalize();
        let up = first_tangent.cross(right).normalize();
        (right, up)
    };

    let center_3d_start = path_samples[0].0 + right * profile_center.x + up * profile_center.y;
    vertices.push(center_3d_start);
    normals.push(-first_tangent);

    for i in 0..(n_profile - 1) {
        indices.extend_from_slice(&[start_center_idx, i as u32, (i + 1) as u32]);
    }
    if profile_points.len() > 3 {
        indices.extend_from_slice(&[start_center_idx, (n_profile - 1) as u32, 0]);
    }

    // 添加结束端面
    let last_tangent = path_samples.last().unwrap().1;
    let end_center_idx = vertices.len() as u32;
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

    let center_3d_end =
        path_samples.last().unwrap().0 + right_end * profile_center.x + up_end * profile_center.y;
    vertices.push(center_3d_end);
    normals.push(last_tangent);

    let last_ring_base = ((path_samples.len() - 1) * n_profile) as u32;
    for i in 0..(n_profile - 1) {
        indices.extend_from_slice(&[
            end_center_idx,
            last_ring_base + (i + 1) as u32,
            last_ring_base + i as u32,
        ]);
    }
    if profile_points.len() > 3 {
        indices.extend_from_slice(&[
            end_center_idx,
            last_ring_base,
            last_ring_base + (n_profile - 1) as u32,
        ]);
    }

    Some(PlantMesh {
        indices,
        vertices,
        normals,
        wire_vertices: Vec::new(),
        edges: Vec::new(),
        aabb: None,
    })
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
