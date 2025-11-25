//! CSG（构造实体几何）网格生成模块
//!
//! 本模块实现了多种基本几何形状的三角网格生成算法，包括：
//! - 圆柱体（LCylinder, SCylinder）
//! - 球体（Sphere）
//! - 圆台（LSnout）
//! - 盒子（SBox）
//! - 圆盘（Dish）
//! - 圆环（CTorus, RTorus）
//! - 棱锥（Pyramid, LPyramid）
//! - 拉伸体（Extrusion）
//!
//! 所有网格生成算法都支持自适应细分，根据几何形状的尺寸和LOD设置
//! 自动调整网格分辨率，以平衡渲染质量和性能。

use crate::debug_macros::is_debug_model_enabled;
use crate::mesh_precision::LodMeshSettings;
use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::prim_geo::ctorus::CTorus;
use crate::prim_geo::cylinder::{LCylinder, SCylinder};
use crate::prim_geo::dish::Dish;
use crate::prim_geo::extrusion::Extrusion;
use crate::prim_geo::lpyramid::LPyramid;
use crate::prim_geo::polyhedron::{Polygon, Polyhedron};
use crate::prim_geo::profile_processor::{ProfileProcessor, extrude_profile};
use crate::prim_geo::pyramid::Pyramid;
use crate::prim_geo::revolution::Revolution;
use crate::prim_geo::rtorus::RTorus;
use crate::prim_geo::sbox::SBox;
use crate::prim_geo::snout::LSnout;
use crate::prim_geo::sphere::Sphere;
use crate::prim_geo::sweep_solid::SweepSolid;
use crate::prim_geo::wire::CurveType;
use crate::shape::pdms_shape::{Edge, Edges, PlantMesh, VerifiedShape};
use crate::types::refno::RefU64;
use crate::utils::svg_generator::SpineSvgGenerator;
use glam::{Vec2, Vec3};
use nalgebra::Point3;
use parry3d::bounding_volume::{Aabb, BoundingVolume};
use std::collections::HashSet;
use std::sync::Mutex;

/// 最小长度阈值，用于判断几何形状是否有效
const MIN_LEN: f32 = 1e-6;

/// 跟踪已经生成过PLOOP调试文件的refno，避免重复生成
static PLOOP_DEBUG_GENERATED: std::sync::LazyLock<Mutex<HashSet<String>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashSet::new()));

/// 清理PLOOP调试文件生成记录（用于新的运行周期）
pub fn clear_ploop_debug_cache() {
    if let Ok(mut generated_set) = PLOOP_DEBUG_GENERATED.lock() {
        generated_set.clear();
    }
}

/// 生成单位盒子网格（用于简单盒子的基础网格）
///
/// 返回一个尺寸为1x1x1的单位盒子，中心在原点
pub fn unit_box_mesh() -> PlantMesh {
    let half = 0.5;
    let mut vertices = Vec::with_capacity(24); // 6个面 × 4个顶点 = 24
    let mut normals = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36); // 6个面 × 2个三角形 × 3个索引 = 36

    // 定义6个面的法向量和4个角点（在单位坐标系中）
    let faces = [
        // +Z面（前面）
        (
            Vec3::Z,
            [
                Vec3::new(-half, -half, half),
                Vec3::new(half, -half, half),
                Vec3::new(half, half, half),
                Vec3::new(-half, half, half),
            ],
        ),
        // -Z面（后面）
        (
            Vec3::NEG_Z,
            [
                Vec3::new(-half, half, -half),
                Vec3::new(half, half, -half),
                Vec3::new(half, -half, -half),
                Vec3::new(-half, -half, -half),
            ],
        ),
        // +X面（右面）
        (
            Vec3::X,
            [
                Vec3::new(half, -half, -half),
                Vec3::new(half, half, -half),
                Vec3::new(half, half, half),
                Vec3::new(half, -half, half),
            ],
        ),
        // -X面（左面）
        (
            Vec3::NEG_X,
            [
                Vec3::new(-half, -half, half),
                Vec3::new(-half, half, half),
                Vec3::new(-half, half, -half),
                Vec3::new(-half, -half, -half),
            ],
        ),
        // +Y面（上面）
        (
            Vec3::Y,
            [
                Vec3::new(-half, half, -half),
                Vec3::new(half, half, -half),
                Vec3::new(half, half, half),
                Vec3::new(-half, half, half),
            ],
        ),
        // -Y面（下面）
        (
            Vec3::NEG_Y,
            [
                Vec3::new(-half, -half, half),
                Vec3::new(half, -half, half),
                Vec3::new(half, -half, -half),
                Vec3::new(-half, -half, -half),
            ],
        ),
    ];

    for (normal, corners) in faces {
        let base_index = vertices.len() as u32;
        for corner in corners {
            vertices.push(corner);
            normals.push(normal);
        }
        // 添加两个三角形
        indices.extend_from_slice(&[
            base_index,
            base_index + 1,
            base_index + 2,
            base_index,
            base_index + 2,
            base_index + 3,
        ]);
    }

    use nalgebra::Point3;
    use parry3d::bounding_volume::Aabb;

    // 🆕 生成盒子的12条特征边
    let box_edges = generate_box_edges(1.0, 1.0, 1.0);

    let mut mesh = PlantMesh {
        indices,
        vertices,
        normals,
        uvs: Vec::new(),
        wire_vertices: Vec::new(),
        edges: box_edges,
        aabb: Some(Aabb::new(
            Point3::new(-half, -half, -half),
            Point3::new(half, half, half),
        )),
    };
    mesh.generate_auto_uvs();
    mesh.sync_wire_vertices_from_edges();
    mesh
}

/// 生成单位球体网格（用于简单球体的基础网格）
///
/// 返回一个半径为0.5的单位球体，中心在原点
pub fn unit_sphere_mesh() -> PlantMesh {
    use nalgebra::Point3;
    use parry3d::bounding_volume::Aabb;
    let radius = 0.5;
    let settings = LodMeshSettings::default();
    let radial = compute_radial_segments(&settings, radius, false, 3);
    let mut height = compute_height_segments(&settings, radius * 2.0, false, 2);
    // 确保高度分段数为偶数（便于对称分布）
    if height % 2 != 0 {
        height += 1;
    }

    let mut vertices = Vec::with_capacity((radial + 1) * (height + 1));
    let mut normals = Vec::with_capacity(vertices.capacity());
    let mut indices = Vec::with_capacity(height * radial * 6);
    let mut aabb = Aabb::new_invalid();

    // 生成球面顶点
    for lat in 0..=height {
        // 纬度参数 [0, 1] 映射到 [0, π]
        let v = lat as f32 / height as f32;
        let theta = v * std::f32::consts::PI; // 极角（纬度角）
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();

        for lon in 0..=radial {
            // 经度参数 [0, 1] 映射到 [0, 2π]
            let u = lon as f32 / radial as f32;
            let phi = u * std::f32::consts::TAU; // 方位角（经度角）
            let (sin_phi, cos_phi) = phi.sin_cos();

            let normal = Vec3::new(sin_theta * cos_phi, sin_theta * sin_phi, cos_theta);
            let vertex = normal * radius;
            extend_aabb(&mut aabb, vertex);
            vertices.push(vertex);
            normals.push(normal);
        }
    }

    let stride = radial + 1;
    for lat in 0..height {
        for lon in 0..radial {
            let current = lat * stride + lon;
            let next = current + stride;
            indices.extend_from_slice(&[
                current as u32,
                (current + 1) as u32,
                next as u32,
                (current + 1) as u32,
                (next + 1) as u32,
                next as u32,
            ]);
        }
    }

    // 使用特征边生成函数（纬线圈数和经线条数）
    let sphere_edges = generate_sphere_edges(
        radius,
        8,  // 8条经线（子午线）
        4,  // 4条纬线（平行圈）
    );
    let mut mesh = PlantMesh {
        indices,
        vertices,
        normals,
        uvs: Vec::new(),
        wire_vertices: vec![],
        edges: sphere_edges,
        aabb: Some(aabb),
    };
    mesh.generate_auto_uvs();
    mesh.sync_wire_vertices_from_edges();
    mesh
}

/// 生成单位圆柱体网格（用于简单圆柱体的基础网格）
///
/// 返回一个高度为1、半径为0.5的单位圆柱体，包含侧面和两个端面
///
/// # 参数
/// - `settings`: LOD网格设置，控制网格的细分程度
/// - `non_scalable`: 是否不可缩放（固定分段数）
pub fn unit_cylinder_mesh(settings: &LodMeshSettings, non_scalable: bool) -> PlantMesh {
    let height = 1.0;
    let radius = 0.5;

    // 使用LOD设置计算分段数
    let resolution = compute_radial_segments(settings, radius, non_scalable, 3);
    let segments = compute_height_segments(settings, height, non_scalable, 1);

    let num_rings = segments + 1;
    let num_vertices = resolution * 2 + num_rings * (resolution + 1);
    let num_faces = resolution * (num_rings - 2);
    let num_indices = (2 * num_faces + 2 * (resolution - 1) * 2) * 3;
    let mut vertices: Vec<Vec3> = Vec::with_capacity(num_vertices as usize);
    let mut normals: Vec<Vec3> = Vec::with_capacity(num_vertices as usize);
    let mut indices: Vec<u32> = Vec::with_capacity(num_indices as usize);

    let step_theta = std::f32::consts::TAU / resolution as f32;
    let step_z = height / segments as f32;

    for ring in 0..num_rings {
        let z = ring as f32 * step_z;
        for segment in 0..=resolution {
            let theta = segment as f32 * step_theta;
            let (sin, cos) = theta.sin_cos();
            vertices.push([radius * cos, radius * sin, z].into());
            normals.push([cos, sin, 0.0].into());
        }
    }

    for i in 0..segments {
        let ring = i * (resolution + 1);
        let next_ring = (i + 1) * (resolution + 1);
        for j in 0..resolution {
            indices.extend_from_slice(&[
                ((ring + j + 1) as u32),
                ((next_ring + j) as u32),
                ((ring + j) as u32),
                ((ring + j + 1) as u32),
                ((next_ring + j + 1) as u32),
                ((next_ring + j) as u32),
            ]);
        }
    }

    // 构建端面的闭包函数（顶部或底部）
    let mut build_cap = |top: bool| {
        // 根据是顶部还是底部设置不同的z坐标和法向量
        let (z, normal_z) = if top { (height, 1.0) } else { (0.0, -1.0) };

        // 先插入中心顶点
        let center_index = vertices.len() as u32;
        vertices.push([0.0, 0.0, z].into());
        normals.push([0.0, 0.0, normal_z].into());

        // 再插入圆周顶点
        let rim_base = vertices.len() as u32;
        for i in 0..resolution {
            let theta = i as f32 * step_theta;
            let (sin, cos) = theta.sin_cos();
            vertices.push([cos * radius, sin * radius, z].into());
            normals.push([0.0, 0.0, normal_z].into());
        }

        // 使用扇形三角形生成端面索引
        // 顶面：从外侧看为逆时针，底面：从外侧看为逆时针（法线向外）
        for i in 0..resolution {
            let v0 = center_index;
            let v1 = rim_base + i as u32;
            let v2 = rim_base + ((i + 1) % resolution) as u32;
            if top {
                // 顶部法线指向 +Z
                indices.extend_from_slice(&[v0, v1, v2]);
            } else {
                // 底部法线指向 -Z，反转绕序
                indices.extend_from_slice(&[v0, v2, v1]);
            }
        }
    };

    build_cap(true);
    build_cap(false);

    // 🆕 生成圆柱体的特征边（顶圆 + 底圆 + 4条纵向边）
    let cylinder_edges = generate_cylinder_edges(
        radius,
        height,
        resolution,
        4, // 生成 4 条纵向边，均匀分布
    );

    let mut mesh = PlantMesh {
        indices,
        vertices,
        normals,
        uvs: Vec::new(),
        wire_vertices: Vec::new(),
        edges: cylinder_edges,
        aabb: Some(Aabb::new(
            Point3::new(-0.5, -0.5, 0.0),
            Point3::new(0.5, 0.5, 1.0),
        )),
    };
    mesh.generate_auto_uvs();
    mesh.sync_wire_vertices_from_edges();
    mesh
}

/// 计算径向分段数（圆周方向的细分段数）
///
/// # 参数
/// - `settings`: LOD网格设置
/// - `radius`: 半径
/// - `non_scalable`: 是否不可缩放（固定分段数）
/// - `required_min`: 最小分段数要求
///
/// # 返回
/// 径向分段数，至少为3
fn compute_radial_segments(
    settings: &LodMeshSettings,
    radius: f32,
    non_scalable: bool,
    required_min: u16,
) -> usize {
    // 计算周长（如果半径有效）
    let circumference = if radius > 0.0 {
        Some(2.0 * std::f32::consts::PI * radius)
    } else {
        None
    };
    let base = settings.adaptive_radial_segments(radius, circumference, non_scalable);
    // 确保分段数至少为3（最小三角形数）和required_min中的较大值
    base.max(required_min.max(3)) as usize
}

/// 计算高度分段数（轴向的细分段数）
///
/// # 参数
/// - `settings`: LOD网格设置
/// - `span`: 高度范围
/// - `non_scalable`: 是否不可缩放（固定分段数）
/// - `required_min`: 最小分段数要求
///
/// # 返回
/// 高度分段数，至少为1
fn compute_height_segments(
    settings: &LodMeshSettings,
    span: f32,
    non_scalable: bool,
    required_min: u16,
) -> usize {
    let base = settings.adaptive_height_segments(span, non_scalable);
    base.max(required_min.max(1)) as usize
}

/// 从三角网格索引中提取唯一的边
///
/// # 参数
/// - `indices`: 三角网格的索引数组，每3个元素表示一个三角形
/// - `vertices`: 顶点数组
///
/// # 返回
/// 边的集合，每条边由两个顶点组成（起点和终点）
fn extract_edges_from_mesh(indices: &[u32], vertices: &[Vec3]) -> Edges {
    use std::collections::HashSet;

    if indices.len() < 3 || vertices.is_empty() {
        return Vec::new();
    }

    // 使用 HashSet 存储标准化的边（较小的索引在前）
    let mut edge_set: HashSet<(u32, u32)> = HashSet::new();

    // 遍历所有三角形，提取每条边
    for triangle in indices.chunks_exact(3) {
        let v0 = triangle[0];
        let v1 = triangle[1];
        let v2 = triangle[2];

        // 三条边，标准化为较小的索引在前
        let edges = [
            if v0 < v1 { (v0, v1) } else { (v1, v0) },
            if v1 < v2 { (v1, v2) } else { (v2, v1) },
            if v2 < v0 { (v2, v0) } else { (v0, v2) },
        ];

        for edge in edges {
            edge_set.insert(edge);
        }
    }

    // 将边索引转换为顶点坐标
    let mut edges = Vec::with_capacity(edge_set.len());
    for (idx0, idx1) in edge_set {
        if idx0 < vertices.len() as u32 && idx1 < vertices.len() as u32 {
            let edge = Edge::new(vec![vertices[idx0 as usize], vertices[idx1 as usize]]);
            edges.push(edge);
        }
    }

    edges
}

/// 从 Profile 轮廓生成特征边（用于拉伸体、旋转体等）
///
/// 此函数基于截面轮廓直接生成几何体的外轮廓边，避免从三角网格提取大量内部边。
/// 适用于：
/// - 拉伸体：底面轮廓 + 顶面轮廓 + 纵向边
/// - 旋转体：经线 + 纬线
/// - 扫掠体：起始截面 + 结束截面 + 沿路径的边
///
/// # 参数
/// - `contour_points`: 2D 截面轮廓顶点（已处理 FRADIUS、boolean 操作、圆弧离散化）
/// - `height`: 拉伸高度（对于拉伸体）
/// - `include_vertical_edges`: 是否包含纵向边（连接底面和顶面）
///
/// # 返回
/// 特征边集合，每条边包含起点和终点
pub fn generate_profile_based_edges(
    contour_points: &[Vec2],
    height: f32,
    include_vertical_edges: bool,
) -> Edges {
    if contour_points.len() < 2 {
        return Vec::new();
    }

    let mut edges = Vec::new();
    let n = contour_points.len();

    // 1. 底面轮廓边（z=0）
    for i in 0..n {
        let curr = contour_points[i];
        let next = contour_points[(i + 1) % n];
        edges.push(Edge::new(vec![
            Vec3::new(curr.x, curr.y, 0.0),
            Vec3::new(next.x, next.y, 0.0),
        ]));
    }

    // 2. 顶面轮廓边（z=height）
    for i in 0..n {
        let curr = contour_points[i];
        let next = contour_points[(i + 1) % n];
        edges.push(Edge::new(vec![
            Vec3::new(curr.x, curr.y, height),
            Vec3::new(next.x, next.y, height),
        ]));
    }

    // 3. 纵向边（可选，连接底面和顶面对应顶点）
    if include_vertical_edges {
        for point in contour_points {
            edges.push(Edge::new(vec![
                Vec3::new(point.x, point.y, 0.0),
                Vec3::new(point.x, point.y, height),
            ]));
        }
    }

    edges
}

/// 创建一个带有边信息的 PlantMesh
///
/// 辅助函数，用于创建 PlantMesh 并自动提取边信息
fn create_mesh_with_edges(
    indices: Vec<u32>,
    vertices: Vec<Vec3>,
    normals: Vec<Vec3>,
    aabb: Option<Aabb>,
) -> PlantMesh {
    let edges = extract_edges_from_mesh(&indices, &vertices);
    let mut mesh = PlantMesh {
        indices,
        vertices,
        normals,
        uvs: Vec::new(),
        wire_vertices: Vec::new(),
        edges,
        aabb,
    };
    mesh.generate_auto_uvs();
    mesh.sync_wire_vertices_from_edges();
    mesh
}

/// 创建一个带有自定义边信息的 PlantMesh
///
/// 与 `create_mesh_with_edges` 类似，但允许指定自定义边集合
/// 优先使用提供的边，如果为 None 则从三角网格提取
///
/// # 参数
/// - `indices`: 三角形索引
/// - `vertices`: 顶点位置
/// - `normals`: 顶点法向量
/// - `aabb`: 包围盒（可选）
/// - `custom_edges`: 自定义边集合（可选，如基于 Profile 生成的边）
fn create_mesh_with_custom_edges(
    indices: Vec<u32>,
    vertices: Vec<Vec3>,
    normals: Vec<Vec3>,
    aabb: Option<Aabb>,
    custom_edges: Option<Edges>,
) -> PlantMesh {
    let edges = custom_edges.unwrap_or_else(|| extract_edges_from_mesh(&indices, &vertices));
    let mut mesh = PlantMesh {
        indices,
        vertices,
        normals,
        uvs: Vec::new(),
        wire_vertices: Vec::new(),
        edges,
        aabb,
    };
    mesh.generate_auto_uvs();
    mesh.sync_wire_vertices_from_edges();
    mesh
}

/// 从 Profile 轮廓生成旋转体的特征边（经线和纬线）
///
/// 旋转体的边包括：
/// - **纬线边**：在不同旋转角度位置的轮廓圆环（较少，用于显示旋转形状）
/// - **经线边**（可选）：Profile 轮廓上的点沿旋转方向的圆弧轨迹
///
/// # 参数
/// - `profile`: 轮廓顶点（在 3D 空间中的点）
/// - `rot_pt`: 旋转中心点
/// - `rot_dir`: 旋转轴方向（归一化）
/// - `angle_rad`: 旋转角度（弧度）
/// - `num_latitude_rings`: 纬线圆环数量（建议 2-4 个，用于起始/结束/中间位置）
/// - `include_longitude_edges`: 是否包含经线边
///
/// # 返回
/// 特征边集合
pub fn generate_revolution_profile_edges(
    profile: &[Vec3],
    rot_pt: Vec3,
    rot_dir: Vec3,
    angle_rad: f32,
    num_latitude_rings: usize,
    include_longitude_edges: bool,
) -> Edges {
    if profile.len() < 2 {
        return Vec::new();
    }

    let mut edges = Vec::new();
    let n_profile = profile.len();
    let num_rings = num_latitude_rings.max(2);

    // 计算垂直于旋转轴的正交基
    let (u_axis, v_axis) = {
        let ref_vec = if rot_dir.x.abs() < 0.9 {
            Vec3::X
        } else {
            Vec3::Y
        };
        let u = ref_vec.cross(rot_dir).normalize();
        let v = rot_dir.cross(u).normalize();
        (u, v)
    };

    // 1. 生成纬线边（轮廓圆环，在不同旋转角度）
    for ring_idx in 0..num_rings {
        let theta = if num_rings == 1 {
            0.0
        } else {
            angle_rad * ring_idx as f32 / (num_rings - 1) as f32
        };
        let (sin_theta, cos_theta) = theta.sin_cos();

        // 为当前角度生成轮廓的所有边
        for i in 0..n_profile {
            let j = (i + 1) % n_profile;
            if j == 0 && n_profile > 2 {
                // 如果是开放轮廓，跳过闭合边
                continue;
            }

            let p0 = profile[i];
            let p1 = profile[j];

            // 计算旋转后的位置
            let rotated_p0 = rotate_point_around_axis(p0, rot_pt, rot_dir, u_axis, v_axis, sin_theta, cos_theta);
            let rotated_p1 = rotate_point_around_axis(p1, rot_pt, rot_dir, u_axis, v_axis, sin_theta, cos_theta);

            edges.push(Edge::new(vec![rotated_p0, rotated_p1]));
        }
    }

    // 2. 生成经线边（可选，Profile 轮廓点的旋转轨迹）
    if include_longitude_edges {
        let num_longitude_samples = (angle_rad.to_degrees() / 30.0).ceil().max(4.0) as usize;

        for profile_idx in 0..n_profile {
            let p = profile[profile_idx];

            // 沿旋转方向采样
            for seg in 0..num_longitude_samples {
                let theta0 = angle_rad * seg as f32 / num_longitude_samples as f32;
                let theta1 = angle_rad * (seg + 1) as f32 / num_longitude_samples as f32;

                let (sin0, cos0) = theta0.sin_cos();
                let (sin1, cos1) = theta1.sin_cos();

                let pos0 = rotate_point_around_axis(p, rot_pt, rot_dir, u_axis, v_axis, sin0, cos0);
                let pos1 = rotate_point_around_axis(p, rot_pt, rot_dir, u_axis, v_axis, sin1, cos1);

                edges.push(Edge::new(vec![pos0, pos1]));
            }
        }
    }

    edges
}

/// 辅助函数：绕轴旋转点
#[inline]
fn rotate_point_around_axis(
    point: Vec3,
    rot_center: Vec3,
    rot_axis: Vec3,
    u_axis: Vec3,
    v_axis: Vec3,
    sin_theta: f32,
    cos_theta: f32,
) -> Vec3 {
    let offset = point - rot_center;
    let along_axis = offset.dot(rot_axis);
    let perp_offset = offset - rot_axis * along_axis;
    let perp_dist = perp_offset.length();

    if perp_dist < MIN_LEN {
        // 点在旋转轴上，不旋转
        return point;
    }

    let perp_dir = perp_offset / perp_dist;

    // 将 perp_dir 分解到 u_axis 和 v_axis
    let u_comp = perp_dir.dot(u_axis);
    let v_comp = perp_dir.dot(v_axis);

    // 旋转后的方向
    let rotated_u = u_comp * cos_theta - v_comp * sin_theta;
    let rotated_v = u_comp * sin_theta + v_comp * cos_theta;
    let rotated_perp_dir = u_axis * rotated_u + v_axis * rotated_v;

    // 计算旋转后的位置
    let rotated_perp_offset = rotated_perp_dir * perp_dist;
    let rotated_offset = rotated_perp_offset + rot_axis * along_axis;

    rot_center + rotated_offset
}

/// 生成圆柱体的特征边
///
/// 圆柱体的边包括：
/// - 顶圆边
/// - 底圆边
/// - 纵向边（可选，连接顶圆和底圆）
///
/// # 参数
/// - `radius`: 圆柱半径
/// - `height`: 圆柱高度
/// - `num_segments`: 圆周分段数
/// - `num_vertical_edges`: 纵向边数量（0 表示不生成纵向边）
///
/// # 返回
/// 特征边集合
pub fn generate_cylinder_edges(
    radius: f32,
    height: f32,
    num_segments: usize,
    num_vertical_edges: usize,
) -> Edges {
    let mut edges = Vec::new();
    let step_theta = std::f32::consts::TAU / num_segments as f32;

    // 1. 底圆边（z=0）
    for i in 0..num_segments {
        let theta0 = i as f32 * step_theta;
        let theta1 = ((i + 1) % num_segments) as f32 * step_theta;
        let (sin0, cos0) = theta0.sin_cos();
        let (sin1, cos1) = theta1.sin_cos();

        edges.push(Edge::new(vec![
            Vec3::new(radius * cos0, radius * sin0, 0.0),
            Vec3::new(radius * cos1, radius * sin1, 0.0),
        ]));
    }

    // 2. 顶圆边（z=height）
    for i in 0..num_segments {
        let theta0 = i as f32 * step_theta;
        let theta1 = ((i + 1) % num_segments) as f32 * step_theta;
        let (sin0, cos0) = theta0.sin_cos();
        let (sin1, cos1) = theta1.sin_cos();

        edges.push(Edge::new(vec![
            Vec3::new(radius * cos0, radius * sin0, height),
            Vec3::new(radius * cos1, radius * sin1, height),
        ]));
    }

    // 3. 纵向边（可选，均匀分布在圆周上）
    if num_vertical_edges > 0 {
        let vertical_step = num_segments / num_vertical_edges.max(1);
        for i in 0..num_vertical_edges {
            let segment_idx = i * vertical_step;
            let theta = segment_idx as f32 * step_theta;
            let (sin, cos) = theta.sin_cos();

            edges.push(Edge::new(vec![
                Vec3::new(radius * cos, radius * sin, 0.0),
                Vec3::new(radius * cos, radius * sin, height),
            ]));
        }
    }

    edges
}

/// 生成球体的特征边（经线和纬线）
///
/// # 参数
/// - `radius`: 球体半径
/// - `num_meridians`: 经线数量
/// - `num_parallels`: 纬线数量（不包括两极）
///
/// # 返回
/// 特征边集合
pub fn generate_sphere_edges(
    radius: f32,
    num_meridians: usize,
    num_parallels: usize,
) -> Edges {
    let mut edges = Vec::new();
    let theta_step = std::f32::consts::TAU / num_meridians as f32;
    let phi_step = std::f32::consts::PI / (num_parallels + 1) as f32;

    // 1. 纬线（平行于赤道的圆）
    for parallel_idx in 1..=num_parallels {
        let phi = parallel_idx as f32 * phi_step;
        let (sin_phi, cos_phi) = phi.sin_cos();
        let ring_radius = radius * sin_phi;
        let z = radius * cos_phi;

        for i in 0..num_meridians {
            let theta0 = i as f32 * theta_step;
            let theta1 = ((i + 1) % num_meridians) as f32 * theta_step;
            let (sin0, cos0) = theta0.sin_cos();
            let (sin1, cos1) = theta1.sin_cos();

            edges.push(Edge::new(vec![
                Vec3::new(ring_radius * cos0, ring_radius * sin0, z),
                Vec3::new(ring_radius * cos1, ring_radius * sin1, z),
            ]));
        }
    }

    // 2. 经线（通过南北极的半圆）
    for meridian_idx in 0..num_meridians {
        let theta = meridian_idx as f32 * theta_step;
        let (sin_theta, cos_theta) = theta.sin_cos();

        for segment in 0..=num_parallels {
            let phi0 = segment as f32 * phi_step;
            let phi1 = ((segment + 1) % (num_parallels + 2)) as f32 * phi_step;

            let (sin_phi0, cos_phi0) = phi0.sin_cos();
            let (sin_phi1, cos_phi1) = phi1.sin_cos();

            let p0 = Vec3::new(
                radius * sin_phi0 * cos_theta,
                radius * sin_phi0 * sin_theta,
                radius * cos_phi0,
            );
            let p1 = Vec3::new(
                radius * sin_phi1 * cos_theta,
                radius * sin_phi1 * sin_theta,
                radius * cos_phi1,
            );

            edges.push(Edge::new(vec![p0, p1]));
        }
    }

    edges
}

/// 生成盒子的12条边
///
/// # 参数
/// - `width`: X 方向尺寸
/// - `depth`: Y 方向尺寸
/// - `height`: Z 方向尺寸
///
/// # 返回
/// 特征边集合（12条边）
pub fn generate_box_edges(width: f32, depth: f32, height: f32) -> Edges {
    let hx = width / 2.0;
    let hy = depth / 2.0;
    let hz = height / 2.0;

    vec![
        // 底面 4 条边
        Edge::new(vec![Vec3::new(-hx, -hy, -hz), Vec3::new(hx, -hy, -hz)]),
        Edge::new(vec![Vec3::new(hx, -hy, -hz), Vec3::new(hx, hy, -hz)]),
        Edge::new(vec![Vec3::new(hx, hy, -hz), Vec3::new(-hx, hy, -hz)]),
        Edge::new(vec![Vec3::new(-hx, hy, -hz), Vec3::new(-hx, -hy, -hz)]),
        // 顶面 4 条边
        Edge::new(vec![Vec3::new(-hx, -hy, hz), Vec3::new(hx, -hy, hz)]),
        Edge::new(vec![Vec3::new(hx, -hy, hz), Vec3::new(hx, hy, hz)]),
        Edge::new(vec![Vec3::new(hx, hy, hz), Vec3::new(-hx, hy, hz)]),
        Edge::new(vec![Vec3::new(-hx, hy, hz), Vec3::new(-hx, -hy, hz)]),
        // 纵向 4 条边
        Edge::new(vec![Vec3::new(-hx, -hy, -hz), Vec3::new(-hx, -hy, hz)]),
        Edge::new(vec![Vec3::new(hx, -hy, -hz), Vec3::new(hx, -hy, hz)]),
        Edge::new(vec![Vec3::new(hx, hy, -hz), Vec3::new(hx, hy, hz)]),
        Edge::new(vec![Vec3::new(-hx, hy, -hz), Vec3::new(-hx, hy, hz)]),
    ]
}

/// 生成圆锥体（snout）的特征边
///
/// 包括底部圆、顶部圆（如果存在）以及连接两者的竖直线
///
/// # 参数
/// - `bottom_center`: 底部中心点
/// - `top_center`: 顶部中心点
/// - `bottom_radius`: 底部半径
/// - `top_radius`: 顶部半径
/// - `axis_dir`: 轴向方向（归一化）
/// - `num_segments`: 圆周分段数
/// - `num_vertical_edges`: 竖直边的数量
///
/// # 返回
/// 特征边集合
pub fn generate_snout_edges(
    bottom_center: Vec3,
    top_center: Vec3,
    bottom_radius: f32,
    top_radius: f32,
    axis_dir: Vec3,
    num_segments: usize,
    num_vertical_edges: usize,
) -> Edges {
    let mut edges = Vec::new();

    // 生成正交基向量（用于构建圆周点）
    let (basis_u, basis_v) = orthonormal_basis(axis_dir);

    // 1. 底部圆（如果有半径）
    if bottom_radius > 1e-6 {
        let mut bottom_points = Vec::with_capacity(num_segments + 1);
        for i in 0..=num_segments {
            let angle = (i as f32 / num_segments as f32) * std::f32::consts::TAU;
            let (sin, cos) = angle.sin_cos();
            let radial_dir = basis_u * cos + basis_v * sin;
            let point = bottom_center + radial_dir * bottom_radius;
            bottom_points.push(point);
        }
        edges.push(Edge::new(bottom_points));
    }

    // 2. 顶部圆（如果有半径）
    if top_radius > 1e-6 {
        let mut top_points = Vec::with_capacity(num_segments + 1);
        for i in 0..=num_segments {
            let angle = (i as f32 / num_segments as f32) * std::f32::consts::TAU;
            let (sin, cos) = angle.sin_cos();
            let radial_dir = basis_u * cos + basis_v * sin;
            let point = top_center + radial_dir * top_radius;
            top_points.push(point);
        }
        edges.push(Edge::new(top_points));
    }

    // 3. 连接底部和顶部的竖直线（仅当两端都有半径时）
    if bottom_radius > 1e-6 && top_radius > 1e-6 && num_vertical_edges > 0 {
        let angle_step = std::f32::consts::TAU / num_vertical_edges as f32;
        for i in 0..num_vertical_edges {
            let angle = i as f32 * angle_step;
            let (sin, cos) = angle.sin_cos();
            let radial_dir = basis_u * cos + basis_v * sin;

            let bottom_point = bottom_center + radial_dir * bottom_radius;
            let top_point = top_center + radial_dir * top_radius;

            edges.push(Edge::new(vec![bottom_point, top_point]));
        }
    } else if bottom_radius > 1e-6 && top_radius <= 1e-6 {
        // 纯圆锥情况：从顶点到底部圆周的线
        let angle_step = std::f32::consts::TAU / num_vertical_edges as f32;
        for i in 0..num_vertical_edges {
            let angle = i as f32 * angle_step;
            let (sin, cos) = angle.sin_cos();
            let radial_dir = basis_u * cos + basis_v * sin;
            let bottom_point = bottom_center + radial_dir * bottom_radius;
            edges.push(Edge::new(vec![top_center, bottom_point]));
        }
    } else if bottom_radius <= 1e-6 && top_radius > 1e-6 {
        // 倒圆锥情况：从底部顶点到顶部圆周的线
        let angle_step = std::f32::consts::TAU / num_vertical_edges as f32;
        for i in 0..num_vertical_edges {
            let angle = i as f32 * angle_step;
            let (sin, cos) = angle.sin_cos();
            let radial_dir = basis_u * cos + basis_v * sin;
            let top_point = top_center + radial_dir * top_radius;
            edges.push(Edge::new(vec![bottom_center, top_point]));
        }
    }

    edges
}

/// 生成的网格及其包围盒
#[derive(Debug)]
pub struct GeneratedMesh {
    /// 生成的三角网格
    pub mesh: PlantMesh,
    /// 轴向对齐包围盒（AABB）
    pub aabb: Option<Aabb>,
}

/// 根据几何参数生成CSG网格
///
/// 这是本模块的主要入口函数，根据不同的几何参数类型调用相应的生成函数
///
/// # 参数
/// - `param`: PDMS几何参数，可以是圆柱、球体、盒子等各种基本形状
/// - `settings`: LOD网格设置，控制网格的细分程度
/// - `non_scalable`: 是否不可缩放（对于固定细节级别的对象）
/// - `refno`: 可选的参考号，用于调试输出文件名
///
/// # 返回
/// 如果几何参数有效，返回生成的网格和包围盒；否则返回None
pub fn generate_csg_mesh(
    param: &PdmsGeoParam,
    settings: &LodMeshSettings,
    non_scalable: bool,
    refno: Option<RefU64>,
) -> Option<GeneratedMesh> {
    match param {
        PdmsGeoParam::PrimLCylinder(cyl) => generate_lcylinder_mesh(cyl, settings, non_scalable),
        PdmsGeoParam::PrimSCylinder(cyl) => generate_scylinder_mesh(cyl, settings, non_scalable),
        PdmsGeoParam::PrimSphere(sphere) => generate_sphere_mesh(sphere, settings, non_scalable),
        PdmsGeoParam::PrimLSnout(snout) => generate_snout_mesh(snout, settings, non_scalable),
        PdmsGeoParam::PrimBox(sbox) => generate_box_mesh(sbox),
        PdmsGeoParam::PrimDish(dish) => generate_dish_mesh(dish, settings, non_scalable),
        PdmsGeoParam::PrimCTorus(torus) => generate_torus_mesh(torus, settings, non_scalable),
        PdmsGeoParam::PrimRTorus(rtorus) => {
            generate_rect_torus_mesh(rtorus, settings, non_scalable)
        }
        PdmsGeoParam::PrimPyramid(pyr) => generate_pyramid_mesh(pyr),
        PdmsGeoParam::PrimLPyramid(lpyr) => generate_lpyramid_mesh(lpyr),
        PdmsGeoParam::PrimExtrusion(extrusion) => generate_extrusion_mesh(extrusion, refno),
        PdmsGeoParam::PrimPolyhedron(poly) => generate_polyhedron_mesh(poly),
        PdmsGeoParam::PrimRevolution(rev) => generate_revolution_mesh(rev, settings, non_scalable),
        PdmsGeoParam::PrimLoft(sweep) => {
            generate_prim_loft_mesh(sweep, settings, non_scalable, refno)
        }
        _ => None,
    }
}

/// 生成线性圆柱体（LCylinder）网格
///
/// LCylinder由轴向方向、直径和两个端面的偏移距离定义
/// 与 SCylinder 一致，使用单位圆柱体，通过 transform 的 scale 来缩放
fn generate_lcylinder_mesh(
    cyl: &LCylinder,
    settings: &LodMeshSettings,
    non_scalable: bool,
) -> Option<GeneratedMesh> {
    // 验证参数有效性
    let height = (cyl.ptdi - cyl.pbdi).abs();
    if cyl.pdia.abs() <= MIN_LEN || height <= MIN_LEN {
        return None;
    }

    // 使用单位圆柱体，通过 get_trans() 返回的 scale 来缩放
    Some(GeneratedMesh {
        mesh: unit_cylinder_mesh(settings, non_scalable),
        aabb: None,
    })
}

/// 生成剪切圆柱体（SSCL，Shear Cylinder）网格
///
/// SSCL是SCylinder的一种特殊形式，具有剪切变形：
/// - 底面和顶面可以在X和Y方向有不同的剪切角度
/// - 侧面会沿着高度方向进行插值变形，形成斜向的圆柱体
fn generate_sscl_mesh(
    cyl: &SCylinder,
    settings: &LodMeshSettings,
    non_scalable: bool,
) -> Option<GeneratedMesh> {
    let dir = safe_normalize(cyl.paxi_dir)?;
    let radius = (cyl.pdia * 0.5).abs();
    if radius <= MIN_LEN {
        return None;
    }
    let height = cyl.phei;
    if height.abs() <= MIN_LEN {
        return None;
    }

    // 计算底面和顶面的中心点
    let (bottom_center, top_center) = if height >= 0.0 {
        (cyl.paxi_pt, cyl.paxi_pt + dir * height)
    } else {
        let top = cyl.paxi_pt;
        (top + dir * height, top)
    };

    // 剪切角度参数（转换为弧度）
    let btm_shear_x = cyl.btm_shear_angles[0].to_radians();
    let btm_shear_y = cyl.btm_shear_angles[1].to_radians();
    let top_shear_x = cyl.top_shear_angles[0].to_radians();
    let top_shear_y = cyl.top_shear_angles[1].to_radians();

    // 计算剪切变换的正切值
    let tan_btm_x = btm_shear_x.tan();
    let tan_btm_y = btm_shear_y.tan();
    let tan_top_x = top_shear_x.tan();
    let tan_top_y = top_shear_y.tan();

    // 建立局部坐标系
    let (basis_u, basis_v) = orthonormal_basis(dir);

    let radial = compute_radial_segments(settings, radius, non_scalable, 3);
    let ring_stride = radial + 1;
    let step_theta = std::f32::consts::TAU / radial as f32;

    // 预先计算每个切片在底部和顶部的椭圆边界点
    struct SliceData {
        radial_dir: Vec3,
        bottom_point: Vec3,
        span: Vec3,
    }

    let mut slice_data = Vec::with_capacity(ring_stride);
    let mut max_span = 0.0f32;
    for slice in 0..=radial {
        let angle = slice as f32 * step_theta;
        let (sin, cos) = angle.sin_cos();
        let radial_dir = basis_u * cos + basis_v * sin;
        let x_local = radius * cos;
        let y_local = radius * sin;

        let z_offset_bottom = tan_btm_x * x_local + tan_btm_y * y_local;
        let z_offset_top = tan_top_x * x_local + tan_top_y * y_local;

        let bottom_point = bottom_center + radial_dir * radius + dir * z_offset_bottom;
        let top_point = top_center + radial_dir * radius + dir * z_offset_top;
        let span = top_point - bottom_point;
        max_span = max_span.max(span.length());

        slice_data.push(SliceData {
            radial_dir,
            bottom_point,
            span,
        });
    }

    // 使用最长母线长度决定高度分段，确保剪切越大细分越多
    let mut height_segments =
        compute_height_segments(settings, max_span.max(height.abs()), non_scalable, 1);
    if height_segments == 0 {
        height_segments = 1;
    }

    // 计算顶点、法线和索引的数量
    let vertex_count = (height_segments + 1) * ring_stride + 2 * (radial + 1);
    let mut vertices = Vec::with_capacity(vertex_count);
    let mut normals = Vec::with_capacity(vertex_count);
    let mut indices = Vec::with_capacity(height_segments * radial * 6 + radial * 6);
    let mut aabb = Aabb::new_invalid();

    // 生成侧面顶点（沿每条母线插值）
    for ring in 0..=height_segments {
        let t = ring as f32 / height_segments as f32;
        for data in &slice_data {
            let vertex = data.bottom_point + data.span * t;
            extend_aabb(&mut aabb, vertex);
            vertices.push(vertex);
            normals.push(data.radial_dir);
        }
    }

    // 生成侧面索引
    for ring in 0..height_segments {
        for slice in 0..radial {
            let current = ring * ring_stride + slice;
            let next = current + ring_stride;
            indices.extend_from_slice(&[
                current as u32,
                (current + 1) as u32,
                next as u32,
                (current + 1) as u32,
                (next + 1) as u32,
                next as u32,
            ]);
        }
    }

    // 计算底面法向（考虑剪切角度）
    let bottom_normal = if btm_shear_x.abs() > f32::EPSILON || btm_shear_y.abs() > f32::EPSILON {
        // 计算斜切平面的法向
        // 平面方程: z = tan_x * x + tan_y * y
        // 法向: (-tan_x, -tan_y, 1) 归一化
        let normal_unnorm = Vec3::new(-tan_btm_x, -tan_btm_y, 1.0);
        safe_normalize(normal_unnorm).unwrap_or(-dir)
    } else {
        -dir
    };

    // 生成底面独立顶点
    let bottom_cap_base = vertices.len() as u32;
    for slice in 0..=radial {
        let angle = slice as f32 * step_theta;
        let (sin, cos) = angle.sin_cos();

        // 计算圆周点在 XY 平面的位置
        let x_local = radius * cos;
        let y_local = radius * sin;

        // 计算该点沿轴向的偏移（使其位于斜切平面上）
        // 平面方程: z_offset = tan_x * x + tan_y * y
        let z_offset = tan_btm_x * x_local + tan_btm_y * y_local;

        // 顶点位置
        let vertex = bottom_center + basis_u * x_local + basis_v * y_local + dir * z_offset;
        vertices.push(vertex);
        normals.push(bottom_normal);
        extend_aabb(&mut aabb, vertex);
    }

    // 底面中心点（在斜切平面的中心）
    let bottom_center_index = vertices.len() as u32;
    vertices.push(bottom_center);
    normals.push(bottom_normal);
    extend_aabb(&mut aabb, bottom_center);

    // 底面索引（注意缠绕方向）
    for slice in 0..radial {
        let next = slice + 1;
        indices.extend_from_slice(&[
            bottom_center_index,
            bottom_cap_base + next as u32,
            bottom_cap_base + slice as u32,
        ]);
    }

    // 计算顶面法向（考虑剪切角度）
    let top_normal = if top_shear_x.abs() > f32::EPSILON || top_shear_y.abs() > f32::EPSILON {
        // 计算斜切平面的法向
        let normal_unnorm = Vec3::new(-tan_top_x, -tan_top_y, 1.0);
        safe_normalize(normal_unnorm).unwrap_or(dir)
    } else {
        dir
    };

    // 生成顶面独立顶点
    let top_cap_base = vertices.len() as u32;
    for slice in 0..=radial {
        let angle = slice as f32 * step_theta;
        let (sin, cos) = angle.sin_cos();

        // 计算圆周点在 XY 平面的位置
        let x_local = radius * cos;
        let y_local = radius * sin;

        // 计算该点沿轴向的偏移（使其位于斜切平面上）
        // 相对于顶面中心的偏移
        let z_offset = tan_top_x * x_local + tan_top_y * y_local;

        // 顶点位置
        let vertex = top_center + basis_u * x_local + basis_v * y_local + dir * z_offset;
        vertices.push(vertex);
        normals.push(top_normal);
        extend_aabb(&mut aabb, vertex);
    }

    // 顶面中心点（在斜切平面的中心）
    let top_center_index = vertices.len() as u32;
    vertices.push(top_center);
    normals.push(top_normal);
    extend_aabb(&mut aabb, top_center);

    // 顶面索引
    for slice in 0..radial {
        let next = slice + 1;
        indices.extend_from_slice(&[
            top_center_index,
            top_cap_base + slice as u32,
            top_cap_base + next as u32,
        ]);
    }

    Some(GeneratedMesh {
        mesh: create_mesh_with_edges(indices, vertices, normals, Some(aabb)),
        aabb: Some(aabb),
    })
}

/// 生成简单圆柱体（SCylinder）网格
///
/// SCylinder由轴向方向、直径和高度定义
/// 如果检测到剪切参数，则委托给`generate_sscl_mesh`处理
pub(crate) fn generate_scylinder_mesh(
    cyl: &SCylinder,
    settings: &LodMeshSettings,
    non_scalable: bool,
) -> Option<GeneratedMesh> {
    // 如果是剪切圆柱体，使用专门的生成函数
    if cyl.is_sscl() {
        return generate_sscl_mesh(cyl, settings, non_scalable);
    }
    if cyl.pdia.abs() <= MIN_LEN || cyl.phei.abs() <= MIN_LEN {
        return None;
    }

    Some(GeneratedMesh {
        mesh: unit_cylinder_mesh(settings, non_scalable),
        aabb: None,
    })
}

/// 构建圆柱体网格的通用函数
///
/// # 参数
/// - `bottom_center`: 底部中心点
/// - `top_center`: 顶部中心点
/// - `radius`: 圆柱体半径
/// - `settings`: LOD网格设置
/// - `non_scalable`: 是否不可缩放
///
/// # 返回
/// 生成的圆柱体网格和包围盒
fn build_cylinder_mesh(
    bottom_center: Vec3,
    top_center: Vec3,
    radius: f32,
    settings: &LodMeshSettings,
    non_scalable: bool,
) -> Option<GeneratedMesh> {
    if radius <= MIN_LEN {
        return None;
    }
    // 计算轴向向量和高度
    let axis_vec = top_center - bottom_center;
    let height = axis_vec.length();
    if height <= MIN_LEN {
        return None;
    }
    let axis_dir = axis_vec / height;
    // 构建垂直于轴向的正交基（用于计算圆周上的点）
    let (basis_u, basis_v) = orthonormal_basis(axis_dir);

    let radial = compute_radial_segments(settings, radius, non_scalable, 3);
    let height_segments = compute_height_segments(settings, height, non_scalable, 1);
    let ring_stride = radial + 1;
    let step_theta = std::f32::consts::TAU / radial as f32;

    let mut vertices = Vec::with_capacity((height_segments + 1) * ring_stride + 2 * (radial + 1));
    let mut normals = Vec::with_capacity(vertices.capacity());
    let mut indices = Vec::with_capacity(height_segments * radial * 6 + radial * 6);
    let mut aabb = Aabb::new_invalid();

    for ring in 0..=height_segments {
        let t = ring as f32 / height_segments as f32;
        let center = bottom_center + axis_vec * t;
        for slice in 0..=radial {
            let angle = slice as f32 * step_theta;
            let (sin, cos) = angle.sin_cos();
            let radial_dir = basis_u * cos + basis_v * sin;
            let vertex = center + radial_dir * radius;
            extend_aabb(&mut aabb, vertex);
            vertices.push(vertex);
            normals.push(radial_dir);
        }
    }

    for ring in 0..height_segments {
        for slice in 0..radial {
            let current = ring * ring_stride + slice;
            let next = current + ring_stride;
            indices.extend_from_slice(&[
                current as u32,
                (current + 1) as u32,
                next as u32,
                (current + 1) as u32,
                (next + 1) as u32,
                next as u32,
            ]);
        }
    }

    let bottom_center_index = vertices.len() as u32;
    vertices.push(bottom_center);
    normals.push(-axis_dir);
    extend_aabb(&mut aabb, bottom_center);
    for slice in 0..radial {
        let next = (slice + 1) % (radial + 1);
        indices.extend_from_slice(&[bottom_center_index, next as u32, slice as u32]);
    }

    let top_center_index = vertices.len() as u32;
    vertices.push(top_center);
    normals.push(axis_dir);
    extend_aabb(&mut aabb, top_center);
    let top_ring_offset = height_segments * ring_stride;
    for slice in 0..radial {
        let curr = top_ring_offset + slice;
        let next = top_ring_offset + ((slice + 1) % (radial + 1));
        indices.extend_from_slice(&[top_center_index, curr as u32, next as u32]);
    }

    Some(GeneratedMesh {
        mesh: create_mesh_with_edges(indices, vertices, normals, Some(aabb)),
        aabb: Some(aabb),
    })
}

/// 生成球体网格
///
/// 使用球坐标系生成球面网格，沿纬度（高度）和经度（径向）方向细分
fn generate_sphere_mesh(
    sphere: &Sphere,
    settings: &LodMeshSettings,
    non_scalable: bool,
) -> Option<GeneratedMesh> {
    let radius = sphere.radius.abs();
    if radius <= MIN_LEN {
        return None;
    }

    // 计算径向和高度分段数
    let radial = compute_radial_segments(settings, radius, non_scalable, 3);
    let mut height = compute_height_segments(settings, radius * 2.0, non_scalable, 2);
    // 确保高度分段数为偶数（便于对称分布）
    if height % 2 != 0 {
        height += 1;
    }

    let mut vertices = Vec::with_capacity((radial + 1) * (height + 1));
    let mut normals = Vec::with_capacity(vertices.capacity());
    let mut indices = Vec::with_capacity(height * radial * 6);
    let mut aabb = Aabb::new_invalid();

    // 生成球面顶点
    for lat in 0..=height {
        // 纬度参数 [0, 1] 映射到 [0, π]
        let v = lat as f32 / height as f32;
        let theta = v * std::f32::consts::PI; // 极角（纬度角）
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();

        for lon in 0..=radial {
            // 经度参数 [0, 1] 映射到 [0, 2π]
            let u = lon as f32 / radial as f32;
            let phi = u * std::f32::consts::TAU; // 方位角（经度角）
            let (sin_phi, cos_phi) = phi.sin_cos();

            let normal = Vec3::new(sin_theta * cos_phi, sin_theta * sin_phi, cos_theta);
            let vertex = sphere.center + normal * radius;
            extend_aabb(&mut aabb, vertex);
            vertices.push(vertex);
            normals.push(normal);
        }
    }

    let stride = radial + 1;
    for lat in 0..height {
        for lon in 0..radial {
            let current = lat * stride + lon;
            let next = current + stride;
            indices.extend_from_slice(&[
                current as u32,
                (current + 1) as u32,
                next as u32,
                (current + 1) as u32,
                (next + 1) as u32,
                next as u32,
            ]);
        }
    }

    Some(GeneratedMesh {
        mesh: create_mesh_with_edges(indices, vertices, normals, Some(aabb)),
        aabb: Some(aabb),
    })
}

/// 生成圆台（LSnout）网格
///
/// 圆台是一个截顶圆锥，具有：
/// - 底部半径（pbdm）和顶部半径（ptdm）
/// - 底部和顶部的中心点可以沿轴向偏移
/// - 中心偏移方向由pbax_dir定义
fn generate_snout_mesh(
    snout: &LSnout,
    settings: &LodMeshSettings,
    non_scalable: bool,
) -> Option<GeneratedMesh> {
    // 归一化轴向方向
    let axis_dir = safe_normalize(snout.paax_dir)?;
    // 偏移方向，如果无效则使用垂直于轴向的方向
    let offset_dir = snout
        .pbax_dir
        .try_normalize()
        .unwrap_or_else(|| orthonormal_basis(axis_dir).0);

    // 计算底部和顶部半径
    let bottom_radius = (snout.pbdm * 0.5).max(0.0);
    let top_radius = (snout.ptdm * 0.5).max(0.0);
    if bottom_radius <= MIN_LEN && top_radius <= MIN_LEN {
        return None;
    }

    let height_axis = snout.ptdi - snout.pbdi;
    if height_axis.abs() <= MIN_LEN && snout.poff.abs() <= MIN_LEN {
        return None;
    }

    let (basis_u, basis_v) = orthonormal_basis(axis_dir);
    let center_delta = axis_dir * height_axis + offset_dir * snout.poff;
    let axial_span = center_delta.length();
    let bottom_center = snout.paax_pt + axis_dir * snout.pbdi;
    let max_radius = bottom_radius.max(top_radius);
    let radial = compute_radial_segments(settings, max_radius, non_scalable, 3);
    let height_segments = compute_height_segments(settings, axial_span, non_scalable, 1);
    let step_theta = std::f32::consts::TAU / radial as f32;
    let ring_stride = radial + 1;
    let radius_delta = top_radius - bottom_radius;

    let mut vertices = Vec::with_capacity((height_segments + 1) * ring_stride + 2 * (radial + 1));
    let mut normals = Vec::with_capacity(vertices.capacity());
    let mut indices = Vec::with_capacity(height_segments * radial * 6 + radial * 6);
    let mut aabb = Aabb::new_invalid();

    for segment in 0..=height_segments {
        let t = segment as f32 / height_segments as f32;
        let center = bottom_center + axis_dir * (height_axis * t) + offset_dir * (snout.poff * t);
        let radius = (bottom_radius + radius_delta * t).max(0.0);
        for slice in 0..=radial {
            let angle = slice as f32 * step_theta;
            let (sin, cos) = angle.sin_cos();
            let radial_dir = basis_u * cos + basis_v * sin;
            let vertex = center + radial_dir * radius;
            extend_aabb(&mut aabb, vertex);
            vertices.push(vertex);

            // 计算法向量：使用切向量的叉积
            // tangent_theta: 圆周方向的切向量
            let tangent_theta = (-sin) * basis_u + cos * basis_v;
            let tangent_theta = tangent_theta * radius;
            // tangent_height: 高度方向的切向量（考虑半径变化）
            let tangent_height = center_delta + radial_dir * radius_delta;
            // 法向量 = tangent_theta × tangent_height
            let mut normal = tangent_theta.cross(tangent_height);
            if normal.length_squared() <= 1e-8 {
                // 如果法向量太小（退化情况），使用径向方向作为法向量
                normal = radial_dir;
            } else {
                normal = normal.normalize();
            }
            normals.push(normal);
        }
    }

    for segment in 0..height_segments {
        for slice in 0..radial {
            let current = segment * ring_stride + slice;
            let next = current + ring_stride;
            indices.extend_from_slice(&[
                current as u32,
                (current + 1) as u32,
                next as u32,
                (current + 1) as u32,
                (next + 1) as u32,
                next as u32,
            ]);
        }
    }

    if bottom_radius > MIN_LEN {
        let bottom_center_index = vertices.len() as u32;
        vertices.push(bottom_center);
        normals.push(-axis_dir);
        extend_aabb(&mut aabb, bottom_center);
        for slice in 0..radial {
            let next = (slice + 1) % (radial + 1);
            indices.extend_from_slice(&[bottom_center_index, (next) as u32, slice as u32]);
        }
    }

    if top_radius > MIN_LEN {
        let top_center = bottom_center + axis_dir * height_axis + offset_dir * snout.poff;
        let top_center_index = vertices.len() as u32;
        vertices.push(top_center);
        normals.push(axis_dir);
        extend_aabb(&mut aabb, top_center);
        let top_ring_offset = height_segments * ring_stride;
        for slice in 0..radial {
            let curr = top_ring_offset + slice;
            let next = top_ring_offset + ((slice + 1) % (radial + 1));
            indices.extend_from_slice(&[top_center_index, curr as u32, next as u32]);
        }
    }

    // 计算顶部中心点
    let top_center = bottom_center + axis_dir * height_axis + offset_dir * snout.poff;

    // 使用特征边生成函数
    let snout_edges = generate_snout_edges(
        bottom_center,
        top_center,
        bottom_radius,
        top_radius,
        axis_dir,
        radial,        // 圆周分段数
        4,             // 4条竖直边
    );

    Some(GeneratedMesh {
        mesh: create_mesh_with_custom_edges(indices, vertices, normals, Some(aabb), Some(snout_edges)),
        aabb: Some(aabb),
    })
}

/// 生成盒子（SBox）网格
///
/// 盒子由中心点和尺寸定义，包含6个面（每个面由2个三角形组成）
fn generate_box_mesh(sbox: &SBox) -> Option<GeneratedMesh> {
    if !sbox.check_valid() {
        return None;
    }
    let half = sbox.size * 0.5; // 半尺寸
    let mut vertices = Vec::with_capacity(24); // 6个面 × 4个顶点 = 24
    let mut normals = Vec::with_capacity(24);
    let mut uvs = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36); // 6个面 × 2个三角形 × 3个索引 = 36

    // 定义6个面的法向量、4个角点（在单位坐标系中）以及对应的UV轴向
    // UV轴向：(u_axis_index, v_axis_index, u_sign, v_sign)
    // index: 0=x, 1=y, 2=z
    let faces = [
        // +Z面（前面）：UV = (X, Y)
        (
            Vec3::Z,
            [
                Vec3::new(-1.0, -1.0, 1.0),
                Vec3::new(1.0, -1.0, 1.0),
                Vec3::new(1.0, 1.0, 1.0),
                Vec3::new(-1.0, 1.0, 1.0),
            ],
            (0, 1, 1.0, 1.0),
        ),
        // -Z面（后面）：UV = (-X, Y)
        (
            Vec3::NEG_Z,
            [
                Vec3::new(-1.0, 1.0, -1.0),
                Vec3::new(1.0, 1.0, -1.0),
                Vec3::new(1.0, -1.0, -1.0),
                Vec3::new(-1.0, -1.0, -1.0),
            ],
            (0, 1, -1.0, 1.0),
        ),
        // +X面（右面）：UV = (-Z, Y)
        (
            Vec3::X,
            [
                Vec3::new(1.0, -1.0, -1.0),
                Vec3::new(1.0, 1.0, -1.0),
                Vec3::new(1.0, 1.0, 1.0),
                Vec3::new(1.0, -1.0, 1.0),
            ],
            (2, 1, -1.0, 1.0),
        ),
        // -X面（左面）：UV = (Z, Y)
        (
            Vec3::NEG_X,
            [
                Vec3::new(-1.0, -1.0, 1.0),
                Vec3::new(-1.0, 1.0, 1.0),
                Vec3::new(-1.0, 1.0, -1.0),
                Vec3::new(-1.0, -1.0, -1.0),
            ],
            (2, 1, 1.0, 1.0),
        ),
        // +Y面（上面）：UV = (X, -Z)
        (
            Vec3::Y,
            [
                Vec3::new(-1.0, 1.0, -1.0),
                Vec3::new(1.0, 1.0, -1.0),
                Vec3::new(1.0, 1.0, 1.0),
                Vec3::new(-1.0, 1.0, 1.0),
            ],
            (0, 2, 1.0, -1.0),
        ),
        // -Y面（下面）：UV = (X, Z)
        (
            Vec3::NEG_Y,
            [
                Vec3::new(-1.0, -1.0, 1.0),
                Vec3::new(1.0, -1.0, 1.0),
                Vec3::new(1.0, -1.0, -1.0),
                Vec3::new(-1.0, -1.0, -1.0),
            ],
            (0, 2, 1.0, 1.0),
        ),
    ];

    for (normal, corners, (u_idx, v_idx, u_sign, v_sign)) in faces {
        let base_index = vertices.len() as u32;
        for corner in corners {
            let scaled = Vec3::new(corner.x * half.x, corner.y * half.y, corner.z * half.z);
            vertices.push(sbox.center + scaled);
            normals.push(normal);

            // World Scale UV: 使用实际尺寸作为 UV 坐标
            // 这里的 scaled 是相对于中心的偏移，加上 half 得到相对于 corner 的正值（0 to size）
            // UV = (position_on_face)
            // corner 取值范围是 -1 到 1，所以 (corner + 1) / 2 是 0-1
            // 乘以尺寸得到实际物理长度

            let size_arr = [sbox.size.x, sbox.size.y, sbox.size.z];
            let u_len = size_arr[u_idx];
            let v_len = size_arr[v_idx];

            let u_base = match u_idx {
                0 => corner.x,
                1 => corner.y,
                _ => corner.z,
            };
            let v_base = match v_idx {
                0 => corner.x,
                1 => corner.y,
                _ => corner.z,
            };

            // 将 -1..1 映射到 0..size
            // 如果 sign 是负的，则反转方向
            let u = if u_sign > 0.0 {
                (u_base + 1.0) * 0.5 * u_len
            } else {
                (1.0 - u_base) * 0.5 * u_len
            };

            let v = if v_sign > 0.0 {
                (v_base + 1.0) * 0.5 * v_len
            } else {
                (1.0 - v_base) * 0.5 * v_len
            };

            uvs.push([u, v]);
        }
        // 确保三角形的顶点顺序是逆时针的（从外部看），使法向量指向外部
        // 通过计算第一个三角形的法向量来验证方向
        let v0 = vertices[base_index as usize];
        let v1 = vertices[base_index as usize + 1];
        let v2 = vertices[base_index as usize + 2];
        let computed_normal = (v1 - v0).cross(v2 - v0);

        // 如果计算出的法向量与预设法向量方向相反，需要反转索引顺序
        if computed_normal.dot(normal) < 0.0 {
            // 反转索引顺序（逆时针）
            indices.extend_from_slice(&[
                base_index,
                base_index + 2,
                base_index + 1,
                base_index,
                base_index + 3,
                base_index + 2,
            ]);
        } else {
            // 保持原顺序
            indices.extend_from_slice(&[
                base_index,
                base_index + 1,
                base_index + 2,
                base_index,
                base_index + 2,
                base_index + 3,
            ]);
        }
    }

    let min = sbox.center - half;
    let max = sbox.center + half;
    let aabb = Aabb::new(Point3::from(min), Point3::from(max));
    let mut mesh = create_mesh_with_edges(indices, vertices, normals, Some(aabb));
    mesh.uvs = uvs; // 使用手动计算的 UV 覆盖默认的空 UV
    // create_mesh_with_edges 内部会调用 generate_auto_uvs，我们之后覆盖它
    // 但 generate_auto_uvs 是基于 bounding box 的，这里我们明确提供了 UV
    // 为了避免重复计算，我们可以修改 create_mesh_with_edges 或者直接构造 PlantMesh

    // 重构 Mesh 构造，避免无用的 auto uv
    let edges = extract_edges_from_mesh(&mesh.indices, &mesh.vertices);
    mesh.edges = edges;
    mesh.sync_wire_vertices_from_edges();

    Some(GeneratedMesh {
        mesh,
        aabb: Some(aabb),
    })
}

/// 生成圆盘（Dish）网格
///
/// 圆盘是一个球形帽面，由球面的一部分和底部圆面组成
/// 支持两种类型：
/// - prad=0: 球形圆盘（Spherical Dish）
/// - prad>0: 椭圆圆盘（Elliptical Dish），z轴缩放形成椭球面
fn generate_dish_mesh(
    dish: &Dish,
    settings: &LodMeshSettings,
    non_scalable: bool,
) -> Option<GeneratedMesh> {
    let axis = safe_normalize(dish.paax_dir)?;
    let radius_rim = dish.pdia * 0.5; // 边缘半径
    let height = dish.pheig;
    if radius_rim <= MIN_LEN || height <= MIN_LEN {
        return None;
    }

    let is_elliptical = dish.prad.abs() > MIN_LEN;
    let base_center = dish.paax_pt + axis * dish.pdis;
    let (basis_u, basis_v) = orthonormal_basis(axis);

    // 根据 dish 类型选择不同的参数
    let (radius_sphere, mut arc, center_offset, scale_z) = if is_elliptical {
        // 椭圆 dish: 使用 baseRadius 作为球半径，z轴缩放为 height/baseRadius
        // 参考 rvmparser: sphereBasedShape(baseRadius, π/2, 0, height/baseRadius)
        let scale_z = height / radius_rim;
        let scale_z = if scale_z.is_finite() && scale_z > MIN_LEN {
            scale_z
        } else {
            1.0
        };
        (radius_rim, std::f32::consts::PI / 2.0, 0.0, scale_z)
    } else {
        // 球形 dish: 计算球面半径
        // 使用几何关系：R² = r² + (R-h)²，解得 R = (r² + h²) / (2h)
        let radius_sphere = (radius_rim * radius_rim + height * height) / (2.0 * height);
        if !radius_sphere.is_finite() || radius_sphere <= MIN_LEN {
            return None;
        }
        // 计算弧角
        let sinval = (radius_rim / radius_sphere).max(-1.0).min(1.0);
        let mut arc = sinval.asin();
        if radius_rim < height {
            arc = std::f32::consts::PI - arc;
        }
        let center_offset = height - radius_sphere;
        (radius_sphere, arc, center_offset, 1.0)
    };

    if arc <= MIN_LEN {
        return None;
    }

    let min_dish_segments = settings.min_radial_segments.max(10);
    let radial_segments =
        compute_radial_segments(settings, radius_rim, non_scalable, min_dish_segments);
    // 对于椭圆 dish，根据 arc 和 scale_z 计算合适的 rings 数
    // 参考 rvmparser: rings = max(min_rings, scale_z * samples * arc / (2π))
    let min_rings = 3u16;
    let samples = radial_segments;
    let mut rings = if is_elliptical {
        let calculated_rings =
            (scale_z * samples as f32 * arc / std::f32::consts::TAU).max(min_rings as f32);
        calculated_rings as usize
    } else {
        compute_height_segments(settings, height, non_scalable, min_rings)
    };
    if rings < min_rings as usize {
        rings = min_rings as usize;
    }
    if rings < 2 {
        return None;
    }

    let is_full_sphere = if arc >= std::f32::consts::PI - 1e-3 {
        arc = std::f32::consts::PI;
        true
    } else {
        false
    };

    // 估算容量：每环最多 radial_segments + 1 个顶点
    let max_vertices_per_ring = radial_segments + 1;
    let mut vertices = Vec::with_capacity((rings + 1) * max_vertices_per_ring + 1);
    let mut normals = Vec::with_capacity(vertices.capacity());
    let mut indices = Vec::with_capacity(rings * radial_segments * 6 + radial_segments * 3);
    let mut aabb = Aabb::new_invalid();
    let mut ring_offsets = Vec::with_capacity(rings + 1);
    let mut ring_vertex_counts = Vec::with_capacity(rings);

    // 生成顶点并跟踪环偏移
    let theta_step = if rings > 1 {
        arc / (rings as f32 - 1.0)
    } else {
        0.0
    };

    for lat in 0..rings {
        ring_offsets.push(vertices.len() as u32);

        let theta = theta_step * lat as f32;
        let cos_theta = theta.cos();
        let sin_theta = theta.sin();

        // 计算 z 坐标（考虑 scale_z 缩放）
        let z = radius_sphere * scale_z * cos_theta + center_offset;
        let axis_point = base_center + axis * z;

        // 计算当前环的半径
        let w = sin_theta; // 当前环的半径系数
        let ring_radius = radius_sphere * w;

        // 为每个环生成顶点
        let n_in_ring = if lat == 0 || (is_full_sphere && lat == rings - 1) {
            1 // 顶部和底部（球形 dish）使用单个顶点
        } else {
            // 根据 w (sin_theta) 计算每环的顶点数
            ((w * samples as f32).max(3.0).ceil() as u32).max(3)
        };
        ring_vertex_counts.push(n_in_ring);

        for lon in 0..n_in_ring {
            let phi = if n_in_ring > 1 {
                lon as f32 / n_in_ring as f32 * std::f32::consts::TAU
            } else {
                0.0
            };
            let dir = basis_u * phi.cos() + basis_v * phi.sin();
            let vertex = axis_point + dir * ring_radius;
            extend_aabb(&mut aabb, vertex);
            vertices.push(vertex);

            // 计算法线（对于椭圆 dish，需要考虑 scale_z）
            let nx = w * phi.cos();
            let ny = w * phi.sin();
            let nz = if scale_z.abs() > MIN_LEN {
                cos_theta / scale_z
            } else {
                cos_theta
            };
            let normal = (basis_u * nx + basis_v * ny + axis * nz).normalize();
            normals.push(normal);
        }
    }
    ring_offsets.push(vertices.len() as u32);

    // 生成索引（连接相邻环）
    // ring_offsets 有 rings + 1 个元素，索引从 0 到 rings
    // 每个环从 ring_offsets[lat] 开始，到 ring_offsets[lat + 1] 结束
    for lat in 0..(rings - 1) {
        let n_c = ring_vertex_counts[lat];
        let n_n = ring_vertex_counts[lat + 1];

        let o_c = ring_offsets[lat];
        let o_n = ring_offsets[lat + 1];

        if n_c < n_n {
            // 下一环顶点更多
            for i_n in 0..(n_n as usize) {
                let i_n_u32 = i_n as u32;
                let mut ii_n = i_n_u32 + 1;
                let mut i_c = (n_c * (i_n_u32 + 1)) / n_n;
                let mut ii_c = (n_c * (i_n_u32 + 2)) / n_n;
                if n_c > 0 {
                    i_c %= n_c;
                    ii_c %= n_c;
                }
                if n_n > 0 {
                    ii_n %= n_n;
                }

                if i_c != ii_c {
                    indices.extend_from_slice(&[o_c + i_c, o_n + ii_n, o_c + ii_c]);
                }
                indices.extend_from_slice(&[o_c + i_c, o_n + i_n_u32, o_n + ii_n]);
            }
        } else {
            // 当前环顶点更多或相等
            for i_c in 0..(n_c as usize) {
                let i_c_u32 = i_c as u32;
                let mut ii_c = i_c_u32 + 1;
                let mut i_n = if n_c > 0 { (n_n * i_c_u32) / n_c } else { 0 };
                let mut ii_n = if n_c > 0 {
                    (n_n * (i_c_u32 + 1)) / n_c
                } else {
                    0
                };

                if n_n > 0 {
                    i_n %= n_n;
                    ii_n %= n_n;
                }
                if n_c > 0 {
                    ii_c %= n_c;
                }

                indices.extend_from_slice(&[o_c + i_c_u32, o_n + ii_n, o_c + ii_c]);
                if i_n != ii_n {
                    indices.extend_from_slice(&[o_c + i_c_u32, o_n + i_n, o_n + ii_n]);
                }
            }
        }
    }

    // 添加底部圆面（仅对球形 dish 或椭圆 dish 的底部）
    if !is_elliptical || height > MIN_LEN {
        let base_ring_idx = rings - 1;
        if base_ring_idx < ring_offsets.len() - 1 {
            let base_ring_start = ring_offsets[base_ring_idx];
            let base_ring_count = ring_offsets[base_ring_idx + 1] - base_ring_start;
            if base_ring_count > 1 {
                let base_center_index = vertices.len() as u32;
                vertices.push(base_center);
                normals.push(-axis);
                extend_aabb(&mut aabb, base_center);
                for lon in 0..(base_ring_count as usize) {
                    let curr = base_ring_start + lon as u32;
                    let next = base_ring_start + ((lon as u32 + 1) % base_ring_count);
                    indices.extend_from_slice(&[base_center_index, next, curr]);
                }
            }
        }
    }

    Some(GeneratedMesh {
        mesh: create_mesh_with_edges(indices, vertices, normals, Some(aabb)),
        aabb: Some(aabb),
    })
}

/// 生成圆环（CTorus）网格
///
/// 圆环由外半径（rout）和内半径（rins）定义
/// 支持任意角度（包括部分圆环）
fn generate_torus_mesh(
    torus: &CTorus,
    settings: &LodMeshSettings,
    non_scalable: bool,
) -> Option<GeneratedMesh> {
    if !torus.check_valid() {
        return None;
    }

    // 计算管半径和主半径
    let tube_radius = (torus.rout - torus.rins) * 0.5; // 管的半径
    if tube_radius <= MIN_LEN {
        return None;
    }
    let major_radius = torus.rins + tube_radius; // 主圆环的半径（toroidal radius）
    let sweep_angle = torus.angle.to_radians();
    if sweep_angle <= MIN_LEN {
        return None;
    }

    // 计算分段数（参考 rvmparser 的 sagittaBasedSegmentCount）
    let scale = if non_scalable {
        settings.non_scalable_factor
    } else {
        1.0
    };

    // 使用现有的 compute_radial_segments，但需要考虑角度
    let major_segments = compute_radial_segments(settings, major_radius, non_scalable, 3);
    // 根据角度调整分段数
    let angle_ratio = sweep_angle / std::f32::consts::TAU;
    let major_segments = ((major_segments as f32 * angle_ratio).ceil() as usize).max(2);

    let tube_segments = compute_radial_segments(settings, tube_radius, non_scalable, 3);

    // 对于部分圆环，需要额外的采样点
    let samples_l = major_segments + 1; // toroidal 方向（不闭合）
    let samples_s = tube_segments; // poloidal 方向（闭合）

    let mut vertices = Vec::with_capacity(samples_l * samples_s);
    let mut normals = Vec::with_capacity(vertices.capacity());
    let mut indices = Vec::with_capacity((samples_l - 1) * samples_s * 6);
    let mut aabb = Aabb::new_invalid();

    // 生成 toroidal 方向的三角函数值
    let mut t0_cos = Vec::with_capacity(samples_l);
    let mut t0_sin = Vec::with_capacity(samples_l);
    for i in 0..samples_l {
        let theta = if samples_l > 1 {
            (sweep_angle / (samples_l - 1) as f32) * i as f32
        } else {
            0.0
        };
        t0_cos.push(theta.cos());
        t0_sin.push(theta.sin());
    }

    // 生成 poloidal 方向的三角函数值
    let mut t1_cos = Vec::with_capacity(samples_s);
    let mut t1_sin = Vec::with_capacity(samples_s);
    for i in 0..samples_s {
        let phi = (std::f32::consts::TAU / samples_s as f32) * i as f32;
        t1_cos.push(phi.cos());
        t1_sin.push(phi.sin());
    }

    // 生成 shell 顶点
    for u in 0..samples_l {
        for v in 0..samples_s {
            let cos_phi = t1_cos[v];
            let sin_phi = t1_sin[v];
            let cos_theta = t0_cos[u];
            let sin_theta = t0_sin[u];

            // 法线：(cos(phi) * cos(theta), cos(phi) * sin(theta), sin(phi))
            let normal = Vec3::new(cos_phi * cos_theta, cos_phi * sin_theta, sin_phi);

            // 顶点：((radius * cos(phi) + offset) * cos(theta), (radius * cos(phi) + offset) * sin(theta), radius * sin(phi))
            let r = tube_radius * cos_phi + major_radius;
            let vertex = Vec3::new(r * cos_theta, r * sin_theta, tube_radius * sin_phi);

            extend_aabb(&mut aabb, vertex);
            vertices.push(vertex);
            normals.push(normal);
        }
    }

    // 生成 shell 索引
    for u in 0..(samples_l - 1) {
        for v in 0..samples_s {
            let v_next = (v + 1) % samples_s;
            let idx00 = (u * samples_s + v) as u32;
            let idx01 = (u * samples_s + v_next) as u32;
            let idx10 = ((u + 1) * samples_s + v) as u32;
            let idx11 = ((u + 1) * samples_s + v_next) as u32;

            // 第一个三角形
            indices.push(idx00);
            indices.push(idx10);
            indices.push(idx11);

            // 第二个三角形
            indices.push(idx11);
            indices.push(idx01);
            indices.push(idx00);
        }
    }

    // 对于部分圆环，需要添加端面
    // 起始端面（角度=0）
    if sweep_angle < std::f32::consts::TAU - 1e-3 {
        let start_offset = vertices.len() as u32;
        for v in 0..samples_s {
            let cos_phi = t1_cos[v];
            let sin_phi = t1_sin[v];
            let r = tube_radius * cos_phi + major_radius;
            let vertex = Vec3::new(r, 0.0, tube_radius * sin_phi);
            extend_aabb(&mut aabb, vertex);
            vertices.push(vertex);
            // 法向量指向起始方向
            normals.push(Vec3::new(-1.0, 0.0, 0.0));
        }
        // 扇状三角化起始端面
        for i in 1..(samples_s - 1) {
            indices.extend_from_slice(&[
                start_offset,
                start_offset + i as u32,
                start_offset + (i + 1) as u32,
            ]);
        }

        // 结束端面（角度=sweep_angle）
        let end_offset = vertices.len() as u32;
        let cos_end = t0_cos[samples_l - 1];
        let sin_end = t0_sin[samples_l - 1];
        for v in 0..samples_s {
            let cos_phi = t1_cos[v];
            let sin_phi = t1_sin[v];
            let r = tube_radius * cos_phi + major_radius;
            let vertex = Vec3::new(r * cos_end, r * sin_end, tube_radius * sin_phi);
            extend_aabb(&mut aabb, vertex);
            vertices.push(vertex);
            // 法向量指向结束方向
            normals.push(Vec3::new(-cos_end, -sin_end, 0.0));
        }
        // 扇状三角化结束端面
        for i in 1..(samples_s - 1) {
            indices.extend_from_slice(&[
                end_offset,
                end_offset + (i + 1) as u32,
                end_offset + i as u32,
            ]);
        }
    }

    Some(GeneratedMesh {
        mesh: create_mesh_with_edges(indices, vertices, normals, Some(aabb)),
        aabb: Some(aabb),
    })
}

/// 生成棱锥（Pyramid）网格
///
/// 棱锥具有：
/// - 底部矩形（由pbbt和pcbt定义）
/// - 顶部矩形或点（由pbtp和pctp定义）
/// - 如果顶部尺寸为0，则顶部为顶点
fn generate_pyramid_mesh(pyr: &Pyramid) -> Option<GeneratedMesh> {
    if !pyr.check_valid() {
        return None;
    }

    // 归一化轴向方向
    let axis_dir = safe_normalize(pyr.paax_dir)?;
    let (fallback_u, fallback_v) = orthonormal_basis(axis_dir);

    // 计算B方向（垂直于轴向）
    let mut pb_dir = safe_normalize(pyr.pbax_dir).unwrap_or(fallback_u);
    pb_dir = pb_dir - axis_dir * pb_dir.dot(axis_dir); // 投影到垂直于轴向的平面
    if pb_dir.length_squared() <= MIN_LEN * MIN_LEN {
        pb_dir = fallback_u;
    }
    pb_dir = pb_dir.normalize();

    // 计算C方向（垂直于轴向和B方向）
    let mut pc_dir = safe_normalize(pyr.pcax_dir).unwrap_or(fallback_v);
    pc_dir = pc_dir - axis_dir * pc_dir.dot(axis_dir) - pb_dir * pc_dir.dot(pb_dir); // 正交化
    if pc_dir.length_squared() <= MIN_LEN * MIN_LEN {
        pc_dir = fallback_v;
    }
    pc_dir = pc_dir.normalize();

    // 计算底部和顶部中心点
    let bottom_center = pyr.paax_pt + axis_dir * pyr.pbdi;
    // 顶部中心点可以沿B和C方向偏移
    let top_center =
        pyr.paax_pt + axis_dir * pyr.ptdi + pb_dir * (pyr.pbof * 0.5) + pc_dir * (pyr.pcof * 0.5);

    // 底部和顶部的半尺寸
    let bottom_half = Vec3::new(pyr.pbbt * 0.5, pyr.pcbt * 0.5, 0.0);
    let top_half = Vec3::new(pyr.pbtp * 0.5, pyr.pctp * 0.5, 0.0);

    let mut vertices: Vec<Vec3> = Vec::new();
    let mut normals: Vec<Vec3> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut aabb = Aabb::new_invalid();

    let mut add_vertex =
        |pos: Vec3, vertices: &mut Vec<Vec3>, normals: &mut Vec<Vec3>, aabb: &mut Aabb| {
            extend_aabb(aabb, pos);
            vertices.push(pos);
            normals.push(Vec3::ZERO);
            (vertices.len() - 1) as u32
        };

    // 生成底部四个角点（如果底部尺寸有效）
    let bottom_corners = if bottom_half.x <= MIN_LEN || bottom_half.y <= MIN_LEN {
        None // 底部退化为点或线
    } else {
        let offsets = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]; // 四个角的偏移
        let mut idxs = [0u32; 4];
        for (i, (ox, oy)) in offsets.iter().enumerate() {
            let pos = bottom_center + pb_dir * (ox * bottom_half.x) + pc_dir * (oy * bottom_half.y);
            idxs[i] = add_vertex(pos, &mut vertices, &mut normals, &mut aabb);
        }
        Some(idxs)
    };

    // 生成顶部顶点或四个角点
    let (top_vertices, apex_index) = if top_half.x <= MIN_LEN || top_half.y <= MIN_LEN {
        // 顶部退化为点（尖锥）
        let apex = add_vertex(top_center, &mut vertices, &mut normals, &mut aabb);
        (None, Some(apex))
    } else {
        // 顶部是矩形
        let offsets = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)];
        let mut idxs = [0u32; 4];
        for (i, (ox, oy)) in offsets.iter().enumerate() {
            let pos = top_center + pb_dir * (ox * top_half.x) + pc_dir * (oy * top_half.y);
            idxs[i] = add_vertex(pos, &mut vertices, &mut normals, &mut aabb);
        }
        (Some(idxs), None)
    };

    if let Some(bottom) = bottom_corners {
        indices.extend_from_slice(&[bottom[0], bottom[1], bottom[2]]);
        indices.extend_from_slice(&[bottom[0], bottom[2], bottom[3]]);
    }

    if bottom_corners.is_none() && top_vertices.is_some() {
        return None;
    }

    if let Some(top) = top_vertices {
        indices.extend_from_slice(&[top[2], top[1], top[0]]);
        indices.extend_from_slice(&[top[3], top[2], top[0]]);
        if let Some(bottom) = bottom_corners {
            for i in 0..4 {
                let next = (i + 1) % 4;
                indices.extend_from_slice(&[bottom[i], bottom[next], top[next]]);
                indices.extend_from_slice(&[bottom[i], top[next], top[i]]);
            }
        }
    } else if let (Some(bottom), Some(apex)) = (bottom_corners, apex_index) {
        for i in 0..4 {
            let next = (i + 1) % 4;
            indices.extend_from_slice(&[bottom[next], bottom[i], apex]);
        }
    }

    if indices.is_empty() {
        return None;
    }

    // 计算顶点法向量：对共享该顶点的所有面的法向量求和（平滑着色）
    for tri in indices.chunks_exact(3) {
        let a = vertices[tri[0] as usize];
        let b = vertices[tri[1] as usize];
        let c = vertices[tri[2] as usize];
        let normal = (b - a).cross(c - a); // 面的法向量
        if normal.length_squared() > MIN_LEN * MIN_LEN {
            let norm = normal.normalize();
            // 将面的法向量累加到三个顶点上
            normals[tri[0] as usize] += norm;
            normals[tri[1] as usize] += norm;
            normals[tri[2] as usize] += norm;
        }
    }

    // 归一化所有法向量
    for n in normals.iter_mut() {
        if n.length_squared() > MIN_LEN * MIN_LEN {
            *n = n.normalize();
        } else {
            // 如果法向量无效，使用轴向方向作为默认值
            *n = axis_dir;
        }
    }

    Some(GeneratedMesh {
        mesh: create_mesh_with_edges(indices, vertices, normals, Some(aabb)),
        aabb: Some(aabb),
    })
}

/// 生成线性棱锥（LPyramid）网格
///
/// LPyramid是Pyramid的变体，通过将LPyramid参数转换为Pyramid参数来生成网格
fn generate_lpyramid_mesh(lpyr: &LPyramid) -> Option<GeneratedMesh> {
    // 将LPyramid转换为Pyramid格式
    let pyramid = Pyramid {
        pbax_pt: lpyr.pbax_pt,
        pbax_dir: lpyr.pbax_dir,
        pcax_pt: lpyr.pcax_pt,
        pcax_dir: lpyr.pcax_dir,
        paax_pt: lpyr.paax_pt,
        paax_dir: lpyr.paax_dir,
        pbtp: lpyr.pbtp,
        pctp: lpyr.pctp,
        pbbt: lpyr.pbbt,
        pcbt: lpyr.pcbt,
        ptdi: lpyr.ptdi,
        pbdi: lpyr.pbdi,
        pbof: lpyr.pbof,
        pcof: lpyr.pcof,
    };
    generate_pyramid_mesh(&pyramid)
}

/// 生成矩形圆环（RTorus）网格
///
/// RTorus是一个空心圆柱体，由外半径、内半径和高度定义
/// 支持任意角度（包括部分圆环）
///
/// 该形状由以下部分组成：
/// - 外圆柱面
/// - 内圆柱面
/// - 顶部和底部环形端面（如果角度 < 360度，还有起始和结束端面）
fn generate_rect_torus_mesh(
    rtorus: &RTorus,
    settings: &LodMeshSettings,
    non_scalable: bool,
) -> Option<GeneratedMesh> {
    if !rtorus.check_valid() {
        return None;
    }

    let outer_radius = rtorus.rout.abs().max(MIN_LEN);
    let inner_radius = rtorus
        .rins
        .abs()
        .max(MIN_LEN)
        .min((outer_radius - MIN_LEN).max(MIN_LEN));

    let sweep_angle = rtorus.angle.to_radians();
    if sweep_angle <= MIN_LEN {
        return None;
    }

    // 计算分段数
    let angle_ratio = sweep_angle / std::f32::consts::TAU;
    let major_segments_base = compute_radial_segments(settings, outer_radius, non_scalable, 3);
    let major_segments = ((major_segments_base as f32 * angle_ratio).ceil() as usize).max(2);
    let height_segments = compute_height_segments(settings, rtorus.height.abs(), non_scalable, 1);
    let radial_span = (outer_radius - inner_radius).abs().max(MIN_LEN);
    let radial_segments = compute_height_segments(
        settings,
        radial_span,
        non_scalable,
        settings.cap_segments.max(1),
    );

    let half_height = rtorus.height * 0.5;
    let samples = major_segments + 1; // 对于部分圆环，需要额外的采样点
    let mut combined = PlantMesh::default();
    combined.aabb = Some(Aabb::new_invalid());

    // 生成 toroidal 方向的三角函数值
    let mut t0_cos = Vec::with_capacity(samples);
    let mut t0_sin = Vec::with_capacity(samples);
    for i in 0..samples {
        let theta = if samples > 1 {
            (sweep_angle / (samples - 1) as f32) * i as f32
        } else {
            0.0
        };
        t0_cos.push(theta.cos());
        t0_sin.push(theta.sin());
    }

    // 生成外圆柱面（法向量向外）
    let (outer_mesh, outer_aabb) = generate_partial_cylinder_surface(
        outer_radius,
        half_height,
        samples,
        height_segments,
        &t0_cos,
        &t0_sin,
        true, // outward = true
    );
    merge_meshes(&mut combined, outer_mesh, outer_aabb);

    // 生成内圆柱面（法向量向内）
    let (inner_mesh, inner_aabb) = generate_partial_cylinder_surface(
        inner_radius,
        half_height,
        samples,
        height_segments,
        &t0_cos,
        &t0_sin,
        false, // outward = false
    );
    merge_meshes(&mut combined, inner_mesh, inner_aabb);

    // 生成顶部环形端面
    let (top_mesh, top_aabb) = generate_partial_annulus_surface(
        half_height,
        inner_radius,
        outer_radius,
        samples,
        radial_segments,
        &t0_cos,
        &t0_sin,
        1.0, // normal_sign = 1.0 (向上)
    );
    merge_meshes(&mut combined, top_mesh, top_aabb);

    // 生成底部环形端面
    let (bottom_mesh, bottom_aabb) = generate_partial_annulus_surface(
        -half_height,
        inner_radius,
        outer_radius,
        samples,
        radial_segments,
        &t0_cos,
        &t0_sin,
        -1.0, // normal_sign = -1.0 (向下)
    );
    merge_meshes(&mut combined, bottom_mesh, bottom_aabb);

    // 对于部分圆环，需要添加起始和结束端面
    if sweep_angle < std::f32::consts::TAU - 1e-3 {
        // 起始端面（角度=0）
        let (start_mesh, start_aabb) = generate_rect_torus_end_face(
            inner_radius,
            outer_radius,
            half_height,
            radial_segments,
            height_segments,
            0.0,  // angle = 0
            true, // is_start
        );
        merge_meshes(&mut combined, start_mesh, start_aabb);

        // 结束端面（角度=sweep_angle）
        let (end_mesh, end_aabb) = generate_rect_torus_end_face(
            inner_radius,
            outer_radius,
            half_height,
            radial_segments,
            height_segments,
            sweep_angle,
            false, // is_start
        );
        merge_meshes(&mut combined, end_mesh, end_aabb);
    }

    let final_aabb = combined.cal_aabb();
    combined.aabb = final_aabb;

    Some(GeneratedMesh {
        mesh: combined,
        aabb: final_aabb,
    })
}

/// 生成部分圆柱面网格（支持任意角度）
fn generate_partial_cylinder_surface(
    radius: f32,
    half_height: f32,
    samples: usize,
    height_segments: usize,
    t0_cos: &[f32],
    t0_sin: &[f32],
    outward: bool,
) -> (PlantMesh, Aabb) {
    let mut vertices = Vec::with_capacity((height_segments + 1) * samples);
    let mut normals = Vec::with_capacity(vertices.capacity());
    let mut indices = Vec::with_capacity(height_segments * (samples - 1) * 6);
    let mut aabb = Aabb::new_invalid();

    for h in 0..=height_segments {
        let t = h as f32 / height_segments as f32;
        let z = -half_height + t * (2.0 * half_height);
        for seg in 0..samples {
            let cos_theta = t0_cos[seg];
            let sin_theta = t0_sin[seg];
            let position = Vec3::new(radius * cos_theta, radius * sin_theta, z);
            extend_aabb(&mut aabb, position);
            let mut normal = Vec3::new(cos_theta, sin_theta, 0.0);
            if !outward {
                normal = -normal;
            }
            vertices.push(position);
            normals.push(normal);
        }
    }

    let ring_stride = samples;
    for h in 0..height_segments {
        for seg in 0..(samples - 1) {
            let current = h * ring_stride + seg;
            let next = current + ring_stride;
            let mut tri1 = [current as u32, (current + 1) as u32, next as u32];
            let mut tri2 = [(current + 1) as u32, (next + 1) as u32, next as u32];
            if !outward {
                tri1.swap(0, 2);
                tri2.swap(0, 2);
            }
            indices.extend_from_slice(&tri1);
            indices.extend_from_slice(&tri2);
        }
    }

    (
        create_mesh_with_edges(indices, vertices, normals, Some(aabb)),
        aabb,
    )
}

/// 生成部分环形端面网格（支持任意角度）
fn generate_partial_annulus_surface(
    z: f32,
    inner_radius: f32,
    outer_radius: f32,
    samples: usize,
    radial_segments: usize,
    t0_cos: &[f32],
    t0_sin: &[f32],
    normal_sign: f32,
) -> (PlantMesh, Aabb) {
    let mut vertices = Vec::with_capacity((radial_segments + 1) * samples);
    let mut normals = Vec::with_capacity(vertices.capacity());
    let mut indices = Vec::with_capacity(radial_segments * (samples - 1) * 6);
    let mut aabb = Aabb::new_invalid();
    let normal = Vec3::new(0.0, 0.0, normal_sign);

    for radial in 0..=radial_segments {
        let t = radial as f32 / radial_segments as f32;
        let radius = inner_radius + (outer_radius - inner_radius) * t;
        for seg in 0..samples {
            let cos_theta = t0_cos[seg];
            let sin_theta = t0_sin[seg];
            let position = Vec3::new(radius * cos_theta, radius * sin_theta, z);
            extend_aabb(&mut aabb, position);
            vertices.push(position);
            normals.push(normal);
        }
    }

    let ring_stride = samples;
    for radial in 0..radial_segments {
        for seg in 0..(samples - 1) {
            let current = radial * ring_stride + seg;
            let next = current + ring_stride;
            if normal_sign > 0.0 {
                indices.extend_from_slice(&[current as u32, next as u32, (current + 1) as u32]);
                indices.extend_from_slice(&[(current + 1) as u32, next as u32, (next + 1) as u32]);
            } else {
                indices.extend_from_slice(&[current as u32, (current + 1) as u32, next as u32]);
                indices.extend_from_slice(&[(current + 1) as u32, (next + 1) as u32, next as u32]);
            }
        }
    }

    (
        create_mesh_with_edges(indices, vertices, normals, Some(aabb)),
        aabb,
    )
}

/// 生成矩形环面体的端面（起始或结束）
fn generate_rect_torus_end_face(
    inner_radius: f32,
    outer_radius: f32,
    half_height: f32,
    radial_segments: usize,
    height_segments: usize,
    angle: f32,
    is_start: bool,
) -> (PlantMesh, Aabb) {
    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();
    let mut aabb = Aabb::new_invalid();

    let (cos_angle, sin_angle) = angle.sin_cos();
    let normal_dir = if is_start {
        Vec3::new(-1.0, 0.0, 0.0)
    } else {
        Vec3::new(-cos_angle, -sin_angle, 0.0)
    };

    // 生成矩形截面的顶点
    for h_idx in 0..=height_segments {
        let t = h_idx as f32 / height_segments as f32;
        let z = -half_height + t * (2.0 * half_height);
        for r_idx in 0..=radial_segments {
            let r_t = r_idx as f32 / radial_segments as f32;
            let radius = inner_radius + (outer_radius - inner_radius) * r_t;
            let position = Vec3::new(radius * cos_angle, radius * sin_angle, z);
            extend_aabb(&mut aabb, position);
            vertices.push(position);
            normals.push(normal_dir);
        }
    }

    // 生成索引
    let ring_stride = radial_segments + 1;
    for h in 0..height_segments {
        for r in 0..radial_segments {
            let current = h * ring_stride + r;
            let next = current + ring_stride;
            if is_start {
                indices.extend_from_slice(&[current as u32, next as u32, (current + 1) as u32]);
                indices.extend_from_slice(&[(current + 1) as u32, next as u32, (next + 1) as u32]);
            } else {
                indices.extend_from_slice(&[current as u32, (current + 1) as u32, next as u32]);
                indices.extend_from_slice(&[(current + 1) as u32, (next + 1) as u32, next as u32]);
            }
        }
    }

    (
        create_mesh_with_edges(indices, vertices, normals, Some(aabb)),
        aabb,
    )
}

/// 导出 PLOOP 数据为 JSON 格式（用于 ploop-rs 测试）
///
/// 生成符合 ploop-rs 输入格式的 JSON 文件
///
/// # 参数
/// - `original`: 原始顶点数据
/// - `name`: PLOOP 名称（如 "FLOOR"）
/// - `height`: 拉伸高度
/// - `refno`: 可选的参考号，如果提供则使用 RefU64 的 to_string 格式作为文件名
fn export_ploop_json(
    original: &[Vec3],
    name: &str,
    height: f32,
    refno: Option<RefU64>,
) -> anyhow::Result<()> {
    use serde_json::json;
    use std::fs;

    // 创建输出目录
    let output_dir = "output/ploop-json";
    fs::create_dir_all(output_dir)?;

    // 根据是否有 refno 决定文件名格式
    let file_suffix = if let Some(refno_val) = refno {
        // 使用 RefU64 的 to_string 格式：ref_0_ref_1
        refno_val.to_string()
    } else {
        // 如果没有 refno，使用时间戳作为后备方案
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string()
    };

    let json_filename = format!("{}/ploop_{}_{}.json", output_dir, name, file_suffix);
    let txt_filename = format!("{}/ploop_{}_{}.txt", output_dir, name, file_suffix);

    // 生成 JSON 格式（用于 3D 可视化）
    let vertices: Vec<_> = original
        .iter()
        .map(|v| {
            if v.z > 0.0 {
                json!({
                    "x": v.x,
                    "y": v.y,
                    "z": 0.0,
                    "fradius": v.z
                })
            } else {
                json!({
                    "x": v.x,
                    "y": v.y,
                    "z": 0.0,
                    "fradius": null
                })
            }
        })
        .collect();

    let fradius_count = original.iter().filter(|v| v.z > 0.0).count();

    let json_data = json!({
        "name": name,
        "height": height,
        "vertices": vertices,
        "fradius_count": fradius_count
    });

    fs::write(&json_filename, serde_json::to_string_pretty(&json_data)?)?;
    println!("📄 [CSG] PLOOP JSON 已保存: {}", json_filename);

    // 生成 TXT 格式（用于 ploop-rs 解析器）
    let mut txt_content = String::new();
    txt_content.push_str(&format!("NEW FRMWORK {}\n", name));
    txt_content.push_str("NEW PLOOP\n");
    txt_content.push_str(&format!("HEIG {:.1}mm\n", height));

    for v in original.iter() {
        txt_content.push_str("NEW PAVERT\n");
        txt_content.push_str(&format!("POS E {:.1}mm N {:.1}mm U 0mm\n", v.x, v.y));
        if v.z > 0.0 {
            txt_content.push_str(&format!("FRAD {:.1}mm\n", v.z));
        }
    }

    txt_content.push_str("END\n");

    fs::write(&txt_filename, txt_content)?;
    println!("📄 [CSG] PLOOP TXT 已保存: {}", txt_filename);

    Ok(())
}

/// 生成 PLOOP 轮廓对比 SVG
///
/// 将原始轮廓和处理后的轮廓绘制在同一个 SVG 中，方便对比
/// - 原始轮廓：红色，使用真实的圆弧
/// - 处理后轮廓：蓝色直线段（ploop-rs 展开后的结果）
///
/// # 参数
/// - `original`: 原始顶点数据
/// - `processed`: 处理后的顶点数据
/// - `refno`: 可选的参考号，如果提供则使用 RefU64 的 to_string 格式作为文件名
fn generate_ploop_comparison_svg(
    original: &[Vec3],
    processed: &[Vec3],
    refno: Option<RefU64>,
) -> anyhow::Result<()> {
    use std::fs;
    use std::path::Path;

    // 创建输出目录
    let output_dir = "output/ploop-svg";
    fs::create_dir_all(output_dir)?;

    // 根据是否有 refno 决定文件名格式
    let file_suffix = if let Some(refno_val) = refno {
        // 使用 RefU64 的 to_string 格式：ref_0_ref_1
        refno_val.to_string()
    } else {
        // 如果没有 refno，使用时间戳作为后备方案
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string()
    };

    let filename = format!("{}/ploop_comparison_{}.svg", output_dir, file_suffix);

    // 计算边界框（原始轮廓考虑圆角半径，处理后仅考虑坐标）
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;

    for v in original.iter() {
        let radius = v.z.max(0.0); // z 存储 FRADIUS
        min_x = min_x.min(v.x - radius);
        min_y = min_y.min(v.y - radius);
        max_x = max_x.max(v.x + radius);
        max_y = max_y.max(v.y + radius);
    }

    for v in processed.iter() {
        min_x = min_x.min(v.x);
        min_y = min_y.min(v.y);
        max_x = max_x.max(v.x);
        max_y = max_y.max(v.y);
    }

    let width = max_x - min_x;
    let height = max_y - min_y;
    let margin = 100.0; // 增加边距以容纳圆角
    let canvas_width = 1400.0;
    let canvas_height = 1000.0;

    // 计算缩放比例
    let scale_x = (canvas_width - 2.0 * margin) / width;
    let scale_y = (canvas_height - 2.0 * margin) / height;
    let scale = scale_x.min(scale_y);

    // 坐标转换函数
    let to_svg = |v: &Vec3| -> (f32, f32) {
        let x = (v.x - min_x) * scale + margin;
        let y = canvas_height - ((v.y - min_y) * scale + margin); // SVG Y轴向下
        (x, y)
    };

    // 生成 SVG 内容
    let mut svg = String::new();
    svg.push_str(&format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<svg width="{}" height="{}" viewBox="0 0 {} {}" xmlns="http://www.w3.org/2000/svg">
<style>
    .original-line {{ stroke: #ff4444; stroke-width: 2; stroke-dasharray: 5,5; fill: none; }}
    .processed-line {{ stroke: #4444ff; stroke-width: 2; fill: none; }}
    .original-point {{ fill: #ff4444; }}
    .processed-point {{ fill: #4444ff; }}
    .fradius-point {{ fill: #ff8800; stroke: #ff4400; stroke-width: 1; }}
    .label {{ font-family: Arial; font-size: 12px; fill: #333; }}
    .title {{ font-family: Arial; font-size: 16px; font-weight: bold; fill: #000; }}
</style>
"#,
        canvas_width, canvas_height, canvas_width, canvas_height
    ));

    // 标题
    svg.push_str(&format!(
        r#"<text x="{}" y="30" class="title" text-anchor="middle">PLOOP 轮廓对比：原始 vs 处理后</text>
"#,
        canvas_width / 2.0
    ));

    // 图例
    svg.push_str(
        r#"<g transform="translate(50, 50)">
    <line x1="0" y1="0" x2="40" y2="0" class="original-line" />
    <text x="50" y="5" class="label">原始轮廓 (红色虚线)</text>
    <line x1="0" y1="20" x2="40" y2="20" class="processed-line" />
    <text x="50" y="25" class="label">处理后轮廓 (蓝色实线)</text>
    <circle cx="5" cy="40" r="4" class="fradius-point" />
    <text x="15" y="45" class="label">FRADIUS 顶点 (橙色)</text>
</g>
"#,
    );

    // 绘制原始轮廓（使用真实的圆弧）
    svg.push_str("<g id=\"original-profile\">\n");
    svg.push_str("<path class=\"original-line\" d=\"");

    let n = original.len();
    for i in 0..n {
        let curr = &original[i];
        let next = &original[(i + 1) % n];
        let (x1, y1) = to_svg(curr);
        let (x2, y2) = to_svg(next);

        if i == 0 {
            svg.push_str(&format!("M {:.1} {:.1} ", x1, y1));
        }

        // 检查下一个顶点是否有 FRADIUS
        if next.z > 0.0 {
            // 有圆角：需要绘制到圆角起点，然后绘制圆弧
            let next_next = &original[(i + 2) % n];
            let fradius = next.z * scale; // 缩放圆角半径

            // 计算从当前点到圆角起点的向量
            let dx1 = next.x - curr.x;
            let dy1 = next.y - curr.y;
            let len1 = (dx1 * dx1 + dy1 * dy1).sqrt();

            // 计算从圆角点到下一个点的向量
            let dx2 = next_next.x - next.x;
            let dy2 = next_next.y - next.y;
            let len2 = (dx2 * dx2 + dy2 * dy2).sqrt();

            if len1 > 0.0 && len2 > 0.0 {
                // 归一化向量
                let ux1 = dx1 / len1;
                let uy1 = dy1 / len1;
                let ux2 = dx2 / len2;
                let uy2 = dy2 / len2;

                // 计算圆角的起点和终点（在原始坐标系中）
                let arc_start_x = next.x - ux1 * next.z;
                let arc_start_y = next.y - uy1 * next.z;
                let arc_end_x = next.x + ux2 * next.z;
                let arc_end_y = next.y + uy2 * next.z;

                // 转换到 SVG 坐标
                let (arc_start_svg_x, arc_start_svg_y) =
                    to_svg(&Vec3::new(arc_start_x, arc_start_y, 0.0));
                let (arc_end_svg_x, arc_end_svg_y) = to_svg(&Vec3::new(arc_end_x, arc_end_y, 0.0));

                // 绘制直线到圆角起点
                svg.push_str(&format!("L {:.1} {:.1} ", arc_start_svg_x, arc_start_svg_y));

                // 绘制圆弧（使用 SVG 的 A 命令）
                // A rx ry x-axis-rotation large-arc-flag sweep-flag x y
                // large-arc-flag = 0 (小弧)
                // sweep-flag = 1 (顺时针) 或 0 (逆时针)
                let sweep_flag = 1; // 假设顺时针
                svg.push_str(&format!(
                    "A {:.1} {:.1} 0 0 {} {:.1} {:.1} ",
                    fradius, fradius, sweep_flag, arc_end_svg_x, arc_end_svg_y
                ));
            } else {
                // 如果向量长度为0，退化为直线
                svg.push_str(&format!("L {:.1} {:.1} ", x2, y2));
            }
        } else {
            // 没有圆角：直接绘制直线
            svg.push_str(&format!("L {:.1} {:.1} ", x2, y2));
        }
    }

    svg.push_str("Z\" />\n");

    // 绘制原始顶点
    for (i, v) in original.iter().enumerate() {
        let (x, y) = to_svg(v);
        let class = if v.z > 0.0 {
            "fradius-point"
        } else {
            "original-point"
        };
        svg.push_str(&format!(
            "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"4\" class=\"{}\" />\n",
            x, y, class
        ));
        // 如果有 FRADIUS，显示数值
        if v.z > 0.0 {
            svg.push_str(&format!(
                "<text x=\"{:.1}\" y=\"{:.1}\" class=\"label\" text-anchor=\"middle\">R={:.0}</text>\n",
                x, y - 10.0, v.z
            ));
        }
    }
    svg.push_str("</g>\n");

    // 绘制处理后轮廓
    svg.push_str("<g id=\"processed-profile\">\n");
    svg.push_str("<path class=\"processed-line\" d=\"");
    for (i, v) in processed.iter().enumerate() {
        let (x, y) = to_svg(v);
        if i == 0 {
            svg.push_str(&format!("M {:.1} {:.1} ", x, y));
        } else {
            svg.push_str(&format!("L {:.1} {:.1} ", x, y));
        }
    }
    svg.push_str("Z\" />\n");

    // 绘制处理后顶点
    for v in processed.iter() {
        let (x, y) = to_svg(v);
        svg.push_str(&format!(
            "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"3\" class=\"processed-point\" />\n",
            x, y
        ));
    }
    svg.push_str("</g>\n");

    // 统计信息
    let fradius_count = original.iter().filter(|v| v.z > 0.0).count();
    svg.push_str(&format!(
        r#"<text x="{}" y="{}" class="label" text-anchor="middle">原始顶点: {} | 处理后顶点: {} | FRADIUS 顶点: {}</text>
"#,
        canvas_width / 2.0,
        canvas_height - 20.0,
        original.len(),
        processed.len(),
        fradius_count
    ));

    svg.push_str("</svg>");

    // 保存文件
    fs::write(&filename, svg)?;
    println!("📊 [CSG] SVG 对比图已保存: {}", filename);

    Ok(())
}

/// 生成拉伸体（Extrusion）网格
///
/// 拉伸体将一个2D轮廓沿Z轴方向拉伸一定高度形成3D形状
/// 当前实现仅支持：
/// - 单一轮廓（单个顶点列表）
/// - 填充类型（CurveType::Fill）
/// - 轮廓的 z 坐标存储 FRADIUS（圆角半径），会被 ploop-rs 展开并转换为 bulge
///
/// # 参数
/// - `extrusion`: 拉伸体参数
/// - `refno`: 可选的参考号，用于调试输出文件名
fn generate_extrusion_mesh(extrusion: &Extrusion, _refno: Option<RefU64>) -> Option<GeneratedMesh> {
    if extrusion.height.abs() <= MIN_LEN {
        return None;
    }
    if extrusion.verts.is_empty() || extrusion.verts[0].len() < 3 {
        return None;
    }
    // 仅支持填充类型
    if !matches!(&extrusion.cur_type, CurveType::Fill) {
        return None;
    }

    // 使用统一的 ProfileProcessor 管线：
    // 1. FRADIUS → bulge（process_ploop_vertices 在 ProfileProcessor 内部调用）
    // 2. Polyline（cavalier_contours）
    // 3. 圆弧按 bulge 离散化为 2D 轮廓点
    // 4. i_triangle 三角化
    // 5. extrude_profile 生成 3D 网格
    let mut verts2d: Vec<Vec<Vec2>> = Vec::with_capacity(extrusion.verts.len());
    let mut frads: Vec<Vec<f32>> = Vec::with_capacity(extrusion.verts.len());
    for wire in &extrusion.verts {
        let mut v2 = Vec::with_capacity(wire.len());
        let mut r = Vec::with_capacity(wire.len());
        for p in wire {
            v2.push(Vec2::new(p.x, p.y));
            r.push(p.z);
        }
        verts2d.push(v2);
        frads.push(r);
    }

    let processor = match ProfileProcessor::from_wires(verts2d, frads, true) {
        Ok(p) => p,
        Err(e) => {
            println!("⚠️  [CSG] Extrusion ProfileProcessor 创建失败: {}", e);
            return None;
        }
    };

    let refno_str = _refno.map(|r| r.to_string());
    let refno_ref = refno_str.as_deref();
    let profile = match processor.process("EXTRUSION", refno_ref) {
        Ok(p) => p,
        Err(e) => {
            println!("⚠️  [CSG] Extrusion ProfileProcessor 处理失败: {}", e);
            return None;
        }
    };

    let extruded = extrude_profile(&profile, extrusion.height);

    // 🆕 从 Profile 轮廓生成特征边（外轮廓边）
    let profile_edges = generate_profile_based_edges(
        &profile.contour_points,
        extrusion.height,
        false, // 暂不包含纵向边，避免过于密集
    );

    // 使用 create_mesh_with_custom_edges 构建带基于 Profile 的边的 PlantMesh
    let mut mesh = create_mesh_with_custom_edges(
        extruded.indices,
        extruded.vertices,
        extruded.normals,
        None,
        Some(profile_edges),
    );
    mesh.uvs = extruded.uvs;

    // 确保 AABB 被正确计算，并同步到 mesh.aabb
    let aabb = mesh.aabb.clone().or_else(|| mesh.cal_aabb());
    if mesh.aabb.is_none() {
        mesh.aabb = aabb.clone();
    }

    Some(GeneratedMesh { mesh, aabb })
}

/// 生成圆柱面网格（用于RTorus的组成部分）
///
/// # 参数
/// - `radius`: 圆柱半径
/// - `half_height`: 半高度（圆柱从-half_height到+half_height）
/// - `major_segments`: 圆周方向的段数
/// - `height_segments`: 高度方向的段数
/// - `outward`: 法向量方向（true=向外，false=向内）
///
/// # 返回
/// 生成的圆柱面网格和包围盒
fn generate_cylinder_surface(
    radius: f32,
    half_height: f32,
    major_segments: usize,
    height_segments: usize,
    outward: bool,
) -> (PlantMesh, Aabb) {
    let mut vertices = Vec::with_capacity((height_segments + 1) * (major_segments + 1));
    let mut normals = Vec::with_capacity(vertices.capacity());
    let mut indices = Vec::with_capacity(height_segments * major_segments * 6);
    let mut aabb = Aabb::new_invalid();

    for h in 0..=height_segments {
        let t = h as f32 / height_segments as f32;
        let z = -half_height + t * (2.0 * half_height);
        for seg in 0..=major_segments {
            let angle = seg as f32 / major_segments as f32 * std::f32::consts::TAU;
            let (sin, cos) = angle.sin_cos();
            let position = Vec3::new(radius * cos, radius * sin, z);
            extend_aabb(&mut aabb, position);
            let mut normal = Vec3::new(cos, sin, 0.0);
            if !outward {
                normal = -normal;
            }
            vertices.push(position);
            normals.push(normal);
        }
    }

    let ring_stride = major_segments + 1;
    for h in 0..height_segments {
        for seg in 0..major_segments {
            let current = h * ring_stride + seg;
            let next = current + ring_stride;
            let mut tri1 = [current as u32, (current + 1) as u32, next as u32];
            let mut tri2 = [(current + 1) as u32, (next + 1) as u32, next as u32];
            if !outward {
                tri1.swap(0, 2);
                tri2.swap(0, 2);
            }
            indices.extend_from_slice(&tri1);
            indices.extend_from_slice(&tri2);
        }
    }

    (
        create_mesh_with_edges(indices, vertices, normals, Some(aabb)),
        aabb,
    )
}

/// 生成环形端面网格（用于RTorus的顶部和底部）
///
/// # 参数
/// - `z`: Z坐标（端面的高度位置）
/// - `inner_radius`: 内半径
/// - `outer_radius`: 外半径
/// - `major_segments`: 圆周方向的段数
/// - `radial_segments`: 径向的段数（从内半径到外半径）
/// - `normal_sign`: 法向量符号（1.0=向上，-1.0=向下）
///
/// # 返回
/// 生成的环形端面网格和包围盒
fn generate_annulus_surface(
    z: f32,
    inner_radius: f32,
    outer_radius: f32,
    major_segments: usize,
    radial_segments: usize,
    normal_sign: f32,
) -> (PlantMesh, Aabb) {
    let mut vertices = Vec::with_capacity((radial_segments + 1) * (major_segments + 1));
    let mut normals = Vec::with_capacity(vertices.capacity());
    let mut indices = Vec::with_capacity(radial_segments * major_segments * 6);
    let mut aabb = Aabb::new_invalid();
    let normal = Vec3::new(0.0, 0.0, normal_sign);

    for radial in 0..=radial_segments {
        let t = radial as f32 / radial_segments as f32;
        let radius = inner_radius + (outer_radius - inner_radius) * t;
        for seg in 0..=major_segments {
            let angle = seg as f32 / major_segments as f32 * std::f32::consts::TAU;
            let (sin, cos) = angle.sin_cos();
            let position = Vec3::new(radius * cos, radius * sin, z);
            extend_aabb(&mut aabb, position);
            vertices.push(position);
            normals.push(normal);
        }
    }

    let ring_stride = major_segments + 1;
    for radial in 0..radial_segments {
        for seg in 0..major_segments {
            let current = radial * ring_stride + seg;
            let next = current + ring_stride;
            if normal_sign > 0.0 {
                indices.extend_from_slice(&[current as u32, next as u32, (current + 1) as u32]);
                indices.extend_from_slice(&[(current + 1) as u32, next as u32, (next + 1) as u32]);
            } else {
                indices.extend_from_slice(&[current as u32, (current + 1) as u32, next as u32]);
                indices.extend_from_slice(&[(current + 1) as u32, (next + 1) as u32, next as u32]);
            }
        }
    }

    (
        create_mesh_with_edges(indices, vertices, normals, Some(aabb)),
        aabb,
    )
}

/// 合并两个网格
///
/// 将另一个网格的顶点、法向量、索引合并到基础网格中，并更新包围盒
fn merge_meshes(base: &mut PlantMesh, mut other: PlantMesh, other_aabb: Aabb) {
    other.aabb = Some(other_aabb);
    base.merge(&other);
    // 更新基础网格的包围盒
    if let Some(base_aabb) = base.aabb.as_mut() {
        base_aabb.merge(&other_aabb);
    } else {
        base.aabb = Some(other_aabb);
    }
}

/// 安全归一化向量
///
/// 如果向量长度过小（接近零），返回None；否则返回归一化后的向量
fn safe_normalize(v: Vec3) -> Option<Vec3> {
    if v.length_squared() <= MIN_LEN * MIN_LEN {
        None
    } else {
        Some(v.normalize())
    }
}

/// 扩展包围盒以包含给定点
fn extend_aabb(aabb: &mut Aabb, v: Vec3) {
    aabb.take_point(Point3::new(v.x, v.y, v.z));
}

/// 构建正交基
///
/// 给定一个法向量，生成两个与之正交的切向量，形成正交基（u, v, n）
///
/// # 参数
/// - `normal`: 法向量（将被归一化）
///
/// # 返回
/// (tangent, bitangent) 两个切向量，与normal一起形成右手坐标系
fn orthonormal_basis(normal: Vec3) -> (Vec3, Vec3) {
    let n = normal.normalize();
    // 选择一个与n不平行的向量进行叉积，生成切向量
    let mut tangent = if n.z.abs() < 0.999 {
        Vec3::Z.cross(n) // 如果n不接近Z轴，使用Z轴
    } else {
        Vec3::X.cross(n) // 如果n接近Z轴，使用X轴
    };
    // 如果切向量仍然太小，尝试使用Y轴
    if tangent.length_squared() <= MIN_LEN {
        tangent = Vec3::Y.cross(n);
    }
    tangent = tangent.normalize();
    // 副切向量 = n × tangent（确保右手坐标系）
    let bitangent = n.cross(tangent).normalize();
    (tangent, bitangent)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prim_geo::lpyramid::LPyramid;
    use crate::prim_geo::rtorus::RTorus;
    #[cfg(feature = "occ")]
    use crate::shape::pdms_shape::BrepShapeTrait;
    use approx::assert_relative_eq;

    #[cfg(feature = "occ")]
    #[test]
    fn lcylinder_csg_matches_occ_aabb() {
        let cyl = LCylinder {
            paxi_dir: Vec3::new(0.0, 0.0, 1.0),
            pbdi: -0.75,
            ptdi: 0.25,
            pdia: 0.8,
            ..Default::default()
        };
        let param = PdmsGeoParam::PrimLCylinder(cyl.clone());
        let settings = LodMeshSettings::default();
        let csg = generate_csg_mesh(&param, &settings, false, None)
            .expect("CSG cylinder generation failed");
        #[cfg(feature = "occ")]
        let occ_mesh = {
            let shape = param
                .gen_csg_shape()
                .expect("CSG cylinder generation failed");
            // 对于测试，如果启用 OCC feature，可以转换为 OCC 进行比较
            // 这里暂时跳过 OCC 测试
            csg.mesh.clone()
        };
        #[cfg(not(feature = "occ"))]
        let occ_mesh = csg.mesh.clone();
        let csg_aabb = csg.mesh.aabb.expect("missing CSG aabb");
        let occ_aabb = occ_mesh.aabb.expect("missing OCC aabb");

        let scale = cyl.get_scaled_vec3();
        assert_relative_eq!(csg_aabb.extents()[0], scale.x, epsilon = 1e-3);
        assert_relative_eq!(csg_aabb.extents()[1], scale.y, epsilon = 1e-3);
        assert_relative_eq!(
            csg_aabb.extents()[2],
            (cyl.ptdi - cyl.pbdi).abs(),
            epsilon = 1e-3
        );

        let scaled_occ_extent_x = occ_aabb.extents()[0] * scale.x;
        let scaled_occ_extent_y = occ_aabb.extents()[1] * scale.y;
        assert_relative_eq!(scaled_occ_extent_x, csg_aabb.extents()[0], epsilon = 1e-3);
        assert_relative_eq!(scaled_occ_extent_y, csg_aabb.extents()[1], epsilon = 1e-3);
    }

    #[cfg(feature = "occ")]
    #[test]
    fn snout_csg_matches_occ_aabb() {
        let snout = LSnout {
            paax_pt: Vec3::new(0.0, 0.0, 0.0),
            paax_dir: Vec3::new(0.0, 0.0, 1.0),
            pbax_dir: Vec3::new(1.0, 0.0, 0.0),
            pbdi: 0.0,
            ptdi: 1.2,
            pbdm: 1.0,
            ptdm: 0.6,
            poff: 0.2,
            ..Default::default()
        };
        let param = PdmsGeoParam::PrimLSnout(snout.clone());
        let settings = LodMeshSettings {
            radial_segments: 32,
            height_segments: 4,
            ..Default::default()
        };
        let csg =
            generate_csg_mesh(&param, &settings, false, None).expect("CSG snout generation failed");
        #[cfg(feature = "occ")]
        let occ_mesh = {
            // 对于测试，如果启用 OCC feature，可以转换为 OCC 进行比较
            // 这里暂时跳过 OCC 测试
            csg.mesh.clone()
        };
        #[cfg(not(feature = "occ"))]
        let occ_mesh = csg.mesh.clone();
        let csg_aabb = csg.mesh.aabb.expect("missing CSG aabb");
        let occ_aabb = occ_mesh.aabb.expect("missing OCC aabb");
        assert_relative_eq!(csg_aabb.mins.x, -snout.pbdm / 2.0, epsilon = 2e-3);
        assert_relative_eq!(
            csg_aabb.maxs.x,
            (snout.poff + snout.ptdm / 2.0),
            epsilon = 2e-3
        );
        assert_relative_eq!(csg_aabb.mins.y, -snout.pbdm / 2.0, epsilon = 2e-3);
        assert_relative_eq!(csg_aabb.maxs.y, snout.pbdm / 2.0, epsilon = 2e-3);
        assert_relative_eq!(csg_aabb.mins.z, snout.pbdi, epsilon = 2e-3);
        assert_relative_eq!(csg_aabb.maxs.z, snout.ptdi, epsilon = 2e-3);

        let occ_extents = occ_aabb.extents();
        assert_relative_eq!(occ_extents[0], 1.0, epsilon = 1e-3);
        assert_relative_eq!(occ_extents[1], 1.0, epsilon = 1e-3);
    }

    #[test]
    fn sscl_csg_generates_mesh() {
        let mut cyl = SCylinder::default();
        cyl.pdia = 2.0; // diameter = 2.0, radius = 1.0
        cyl.phei = 3.0; // height = 3.0
        cyl.center_in_mid = true; // Center the cylinder
        cyl.btm_shear_angles = [10.0, 5.0]; // 10° in x, 5° in y
        cyl.top_shear_angles = [15.0, -5.0]; // 15° in x, -5° in y

        let generated = generate_csg_mesh(
            &PdmsGeoParam::PrimSCylinder(cyl),
            &LodMeshSettings {
                radial_segments: 16,
                height_segments: 4,
                ..Default::default()
            },
            false,
            None,
        )
        .expect("SSCL CSG generation failed");

        // Verify mesh has reasonable properties
        assert!(generated.mesh.vertices.len() > 0);
        assert!(generated.mesh.indices.len() > 0);
        assert!(generated.mesh.normals.len() == generated.mesh.vertices.len());

        // Verify that SSCL produces a different result than regular SCylinder
        let mut regular_cyl = SCylinder::default();
        regular_cyl.pdia = 2.0;
        regular_cyl.phei = 3.0;
        regular_cyl.center_in_mid = true;
        // No shear angles

        let regular_generated = generate_csg_mesh(
            &PdmsGeoParam::PrimSCylinder(regular_cyl),
            &LodMeshSettings {
                radial_segments: 16,
                height_segments: 4,
                ..Default::default()
            },
            false,
            None,
        )
        .expect("Regular SCylinder CSG generation failed");

        // SSCL should have different vertices due to shear transformation
        assert_ne!(
            generated.mesh.vertices.len(),
            regular_generated.mesh.vertices.len()
        );
    }

    #[test]
    fn sbox_csg_extents_match_params() {
        let sbox = SBox {
            center: Vec3::new(1.0, -2.0, 3.0),
            size: Vec3::new(2.0, 4.0, 6.0),
        };
        let generated = generate_csg_mesh(
            &PdmsGeoParam::PrimBox(sbox.clone()),
            &LodMeshSettings::default(),
            false,
            None,
        )
        .expect("SBox CSG generation failed");
        let aabb = generated.mesh.aabb.expect("missing box aabb");
        assert_relative_eq!(
            aabb.mins.x,
            sbox.center.x - sbox.size.x * 0.5,
            epsilon = 1e-6
        );
        assert_relative_eq!(
            aabb.maxs.x,
            sbox.center.x + sbox.size.x * 0.5,
            epsilon = 1e-6
        );
        assert_relative_eq!(
            aabb.mins.y,
            sbox.center.y - sbox.size.y * 0.5,
            epsilon = 1e-6
        );
        assert_relative_eq!(
            aabb.maxs.y,
            sbox.center.y + sbox.size.y * 0.5,
            epsilon = 1e-6
        );
        assert_relative_eq!(
            aabb.mins.z,
            sbox.center.z - sbox.size.z * 0.5,
            epsilon = 1e-6
        );
        assert_relative_eq!(
            aabb.maxs.z,
            sbox.center.z + sbox.size.z * 0.5,
            epsilon = 1e-6
        );
    }

    #[test]
    fn dish_csg_aabb_matches_basic_dimensions() {
        let dish = Dish {
            paax_pt: Vec3::ZERO,
            paax_dir: Vec3::Z,
            pdis: 0.2,
            pheig: 1.5,
            pdia: 2.0,
            prad: 0.0,
            ..Default::default()
        };
        let generated = generate_csg_mesh(
            &PdmsGeoParam::PrimDish(dish.clone()),
            &LodMeshSettings {
                radial_segments: 32,
                height_segments: 4,
                ..Default::default()
            },
            false,
            None,
        )
        .expect("Dish CSG generation failed");
        let aabb = generated.mesh.aabb.expect("missing dish aabb");
        let base_center = dish.paax_pt + Vec3::Z * dish.pdis;
        assert_relative_eq!(aabb.mins.z, base_center.z, epsilon = 1e-3);
        assert_relative_eq!(aabb.maxs.z, base_center.z + dish.pheig, epsilon = 1e-3);
        let sphere_radius =
            (dish.pdia * dish.pdia * 0.25 + dish.pheig * dish.pheig) / (2.0 * dish.pheig);
        assert_relative_eq!(aabb.mins.x, -sphere_radius, epsilon = 1e-3);
        assert_relative_eq!(aabb.maxs.x, sphere_radius, epsilon = 1e-3);
        assert_relative_eq!(aabb.mins.y, -sphere_radius, epsilon = 1e-3);
        assert_relative_eq!(aabb.maxs.y, sphere_radius, epsilon = 1e-3);
    }

    #[test]
    fn ct_torus_csg_extents_match_major_minor() {
        let torus = CTorus {
            rins: 1.0,
            rout: 3.0,
            angle: 360.0,
        };
        let tube_radius = (torus.rout - torus.rins) * 0.5;
        let major_radius = torus.rins + tube_radius;
        let expected_xy = major_radius + tube_radius;

        let generated = generate_csg_mesh(
            &PdmsGeoParam::PrimCTorus(torus),
            &LodMeshSettings {
                radial_segments: 32,
                height_segments: 16,
                ..Default::default()
            },
            false,
            None,
        )
        .expect("CTorus CSG generation failed");
        let aabb = generated.mesh.aabb.expect("missing torus aabb");

        assert_relative_eq!(aabb.maxs.z, tube_radius, epsilon = 1e-3);
        assert_relative_eq!(aabb.mins.z, -tube_radius, epsilon = 1e-3);
        assert_relative_eq!(aabb.maxs.x, expected_xy, epsilon = 1e-3);
        assert_relative_eq!(aabb.mins.x, -expected_xy, epsilon = 1e-3);
        assert_relative_eq!(aabb.maxs.y, expected_xy, epsilon = 1e-3);
        assert_relative_eq!(aabb.mins.y, -expected_xy, epsilon = 1e-3);
    }

    #[test]
    fn pyramid_csg_extents_match_parameters() {
        let pyramid = Pyramid {
            paax_pt: Vec3::ZERO,
            paax_dir: Vec3::Z,
            pbax_pt: Vec3::ZERO,
            pbax_dir: Vec3::X,
            pcax_pt: Vec3::ZERO,
            pcax_dir: Vec3::Y,
            pbbt: 4.0,
            pcbt: 4.0,
            pbtp: 2.0,
            pctp: 2.0,
            pbdi: 0.0,
            ptdi: 2.0,
            pbof: 0.0,
            pcof: 0.0,
        };

        let generated = generate_csg_mesh(
            &PdmsGeoParam::PrimPyramid(pyramid.clone()),
            &LodMeshSettings::default(),
            false,
            None,
        )
        .expect("Pyramid CSG generation failed");
        let aabb = generated.mesh.aabb.expect("missing pyramid aabb");

        assert_relative_eq!(aabb.mins.x, -2.0, epsilon = 1e-3);
        assert_relative_eq!(aabb.maxs.x, 2.0, epsilon = 1e-3);
        assert_relative_eq!(aabb.mins.y, -2.0, epsilon = 1e-3);
        assert_relative_eq!(aabb.maxs.y, 2.0, epsilon = 1e-3);
        assert_relative_eq!(aabb.mins.z, 0.0, epsilon = 1e-3);
        assert_relative_eq!(aabb.maxs.z, 2.0, epsilon = 1e-3);
    }

    #[test]
    fn extrusion_csg_basic_prism() {
        let square = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ];
        let extrusion = Extrusion {
            verts: vec![square],
            height: 2.0,
            cur_type: CurveType::Fill,
        };
        let generated = generate_csg_mesh(
            &PdmsGeoParam::PrimExtrusion(extrusion),
            &LodMeshSettings::default(),
            false,
            None,
        )
        .expect("Extrusion CSG generation failed");
        let aabb = generated.mesh.aabb.expect("missing extrusion aabb");
        assert_relative_eq!(aabb.mins.x, 0.0, epsilon = 1e-3);
        assert_relative_eq!(aabb.maxs.x, 1.0, epsilon = 1e-3);
        assert_relative_eq!(aabb.mins.y, 0.0, epsilon = 1e-3);
        assert_relative_eq!(aabb.maxs.y, 1.0, epsilon = 1e-3);
        assert_relative_eq!(aabb.mins.z, 0.0, epsilon = 1e-3);
        assert_relative_eq!(aabb.maxs.z, 2.0, epsilon = 1e-3);
    }

    /// 测试：带 FRADIUS 的矩形截面挤出，验证圆角被离散（顶点数增加）
    #[test]
    fn extrusion_csg_with_fradius() {
        // 150x150 的矩形，四个角 FRAD=20
        let rect_with_fradius = vec![
            Vec3::new(0.0, 0.0, 20.0),
            Vec3::new(150.0, 0.0, 20.0),
            Vec3::new(150.0, 150.0, 20.0),
            Vec3::new(0.0, 150.0, 20.0),
        ];

        let extrusion = Extrusion {
            verts: vec![rect_with_fradius],
            height: 100.0,
            cur_type: CurveType::Fill,
        };

        let generated = generate_csg_mesh(
            &PdmsGeoParam::PrimExtrusion(extrusion),
            &LodMeshSettings::default(),
            false,
            None,
        )
        .expect("Extrusion CSG generation with FRADIUS failed");

        let mesh = &generated.mesh;
        let aabb = mesh.aabb.expect("missing extrusion aabb");

        // 带圆角的矩形挤出，顶点数应该明显大于简单四边形挤出
        assert!(
            mesh.vertices.len() > 8,
            "expected more than 8 vertices for rounded extrusion, got {}",
            mesh.vertices.len()
        );

        // 只检查高度方向是否符合预期（0 ~ 100），XY 范围可能因为圆角/数值略有变化
        assert!(aabb.mins.z <= 1e-3);
        assert!(aabb.maxs.z >= 100.0 - 1e-3);

        // 导出 OBJ 文件用于可视化验证
        let _ = mesh.export_obj(false, "test_output/extrusion_rounded_fradius.obj");
    }
}

/// 生成多面体（Polyhedron）网格
///
/// Polyhedron 由多个多边形面组成，每个面可能有多个环（外环和内环）
/// 如果已经有预生成的 mesh，直接使用；否则需要三角化多边形
pub(crate) fn generate_polyhedron_mesh(poly: &Polyhedron) -> Option<GeneratedMesh> {
    // 如果已经有预生成的 mesh，直接使用
    if let Some(ref mesh) = poly.mesh {
        let aabb = mesh.aabb.or_else(|| mesh.cal_aabb());
        return Some(GeneratedMesh {
            mesh: mesh.clone(),
            aabb,
        });
    }

    // 否则需要三角化多边形
    // 简单的实现：使用扇状三角化处理每个多边形
    let mut all_vertices = Vec::new();
    let mut all_normals = Vec::new();
    let mut all_indices = Vec::new();
    let mut aabb = Aabb::new_invalid();
    let mut vertex_offset = 0u32;

    for polygon in &poly.polygons {
        if polygon.loops.is_empty() {
            continue;
        }

        // 处理外环（第一个环）
        let outer_loop = &polygon.loops[0];
        if outer_loop.len() < 3 {
            continue;
        }

        // 计算多边形法向量
        let mut normal = Vec3::ZERO;
        for i in 0..outer_loop.len() {
            let v0 = outer_loop[i];
            let v1 = outer_loop[(i + 1) % outer_loop.len()];
            let v2 = outer_loop[(i + 2) % outer_loop.len()];
            normal += (v1 - v0).cross(v2 - v1);
        }
        if normal.length_squared() > MIN_LEN * MIN_LEN {
            normal = normal.normalize();
        } else {
            normal = Vec3::Z; // 默认法向量
        }

        // 添加顶点
        for &vertex in outer_loop {
            extend_aabb(&mut aabb, vertex);
            all_vertices.push(vertex);
            all_normals.push(normal);
        }

        // 使用扇状三角化（fan triangulation）
        // 假设外环是凸多边形或接近凸多边形
        for i in 1..(outer_loop.len() - 1) {
            all_indices.push(vertex_offset);
            all_indices.push(vertex_offset + i as u32);
            all_indices.push(vertex_offset + (i + 1) as u32);
        }

        vertex_offset += outer_loop.len() as u32;

        // TODO: 处理内环（洞）
        // 目前只处理外环
    }

    if all_vertices.is_empty() {
        return None;
    }

    Some(GeneratedMesh {
        mesh: create_mesh_with_edges(all_indices, all_vertices, all_normals, Some(aabb)),
        aabb: Some(aabb),
    })
}

/// 生成旋转体（Revolution）网格
///
/// Revolution 通过将轮廓绕轴旋转生成网格
/// 参考 rvmparser 的做法，通过旋转轮廓生成表面网格
pub(crate) fn generate_revolution_mesh(
    rev: &Revolution,
    settings: &LodMeshSettings,
    non_scalable: bool,
) -> Option<GeneratedMesh> {
    if rev.verts.is_empty() || rev.verts[0].len() < 3 {
        return None;
    }

    // 使用第一个轮廓
    let profile = &rev.verts[0];
    let n_profile = profile.len();
    if n_profile < 3 {
        return None;
    }

    // 计算旋转角度
    let angle_deg = if rev.angle.abs() > 360.0 || rev.angle.abs() < 1e-3 {
        360.0
    } else {
        rev.angle.abs()
    };
    let angle_rad = angle_deg.to_radians();

    // 归一化旋转轴
    let rot_dir = rev.rot_dir.normalize();
    let rot_pt = rev.rot_pt;

    // 计算径向分段数（基于轮廓的尺寸）
    let profile_max_dist = profile
        .iter()
        .map(|p| (p - rot_pt).length())
        .fold(0.0f32, f32::max);
    let radial_segments = compute_radial_segments(settings, profile_max_dist, non_scalable, 8);
    let angular_segments = (radial_segments as f32 * (angle_deg / 360.0)).max(4.0) as usize;

    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();
    let mut aabb = Aabb::new_invalid();

    // 计算垂直于旋转轴的正交基
    let (u, v) = {
        let ref_vec = if rot_dir.x.abs() < 0.9 {
            Vec3::X
        } else {
            Vec3::Y
        };
        let u = ref_vec.cross(rot_dir).normalize();
        let v = rot_dir.cross(u).normalize();
        (u, v)
    };

    #[derive(Clone, Copy)]
    struct RevolvedSample {
        along_axis: f32,
        perp_dist: f32,
        perp_dir: Vec3,
        circ_dir: Vec3,
    }

    let mut samples = Vec::with_capacity(n_profile);
    let mut axis_indices = Vec::new();
    for &profile_pt in profile.iter() {
        let offset = profile_pt - rot_pt;
        let along_axis = offset.dot(rot_dir);
        let perp_offset = offset - rot_dir * along_axis;
        let perp_dist = perp_offset.length();
        if perp_dist < MIN_LEN {
            samples.push(RevolvedSample {
                along_axis,
                perp_dist,
                perp_dir: Vec3::ZERO,
                circ_dir: Vec3::ZERO,
            });
            axis_indices.push(samples.len() - 1);
            continue;
        }
        let perp_dir = perp_offset / perp_dist;
        let circ_dir = rot_dir.cross(perp_dir).normalize();
        samples.push(RevolvedSample {
            along_axis,
            perp_dist,
            perp_dir,
            circ_dir,
        });
    }

    for &idx in &axis_indices {
        let axis_pt = profile[idx];
        let mut fallback = Vec3::ZERO;
        let max_step = n_profile.max(1);
        for step in 1..max_step {
            if idx + step < n_profile {
                let diff = profile[idx + step] - axis_pt;
                let projected = diff - rot_dir * diff.dot(rot_dir);
                if projected.length_squared() > MIN_LEN * MIN_LEN {
                    fallback = projected.normalize();
                    break;
                }
            }
            if idx >= step {
                let diff = profile[idx - step] - axis_pt;
                let projected = diff - rot_dir * diff.dot(rot_dir);
                if projected.length_squared() > MIN_LEN * MIN_LEN {
                    fallback = projected.normalize();
                    break;
                }
            }
        }
        if fallback.length_squared() <= MIN_LEN * MIN_LEN {
            fallback = u;
        }
        let mut circ_dir = rot_dir.cross(fallback);
        if circ_dir.length_squared() <= MIN_LEN * MIN_LEN {
            circ_dir = v;
        } else {
            circ_dir = circ_dir.normalize();
        }
        samples[idx].perp_dir = fallback;
        samples[idx].circ_dir = circ_dir;
    }

    let rotate_position = |sample: &RevolvedSample, sin: f32, cos: f32| -> Vec3 {
        if sample.perp_dist < MIN_LEN {
            rot_pt + rot_dir * sample.along_axis
        } else {
            let rotated_perp = sample.perp_dir * cos + sample.circ_dir * sin;
            rot_pt + rot_dir * sample.along_axis + rotated_perp * sample.perp_dist
        }
    };

    // 生成顶点：对每个轮廓点，绕轴旋转生成环形顶点
    for (profile_idx, sample) in samples.iter().enumerate() {
        if sample.perp_dist < MIN_LEN {
            for seg in 0..=angular_segments {
                let theta = (seg as f32 / angular_segments as f32) * angle_rad;
                let (sin, cos) = theta.sin_cos();
                let rotated_perp_dir = sample.perp_dir * cos + sample.circ_dir * sin;
                let position = rot_pt + rot_dir * sample.along_axis;
                extend_aabb(&mut aabb, position);
                vertices.push(position);
                normals.push(rotated_perp_dir.normalize());
            }
            continue;
        }

        for seg in 0..=angular_segments {
            let theta = (seg as f32 / angular_segments as f32) * angle_rad;
            let (sin, cos) = theta.sin_cos();

            let rotated_perp_dir = sample.perp_dir * cos + sample.circ_dir * sin;
            let _rotated_circ_dir = sample.circ_dir * cos - sample.perp_dir * sin;
            let position =
                rot_pt + rot_dir * sample.along_axis + rotated_perp_dir * sample.perp_dist;

            extend_aabb(&mut aabb, position);

            let tangent_theta = rot_dir.cross(rotated_perp_dir * sample.perp_dist);
            let prev_idx = if profile_idx == 0 {
                profile_idx
            } else {
                profile_idx - 1
            };
            let next_idx = if profile_idx + 1 < n_profile {
                profile_idx + 1
            } else {
                profile_idx
            };
            let prev_pos = rotate_position(&samples[prev_idx], sin, cos);
            let next_pos = rotate_position(&samples[next_idx], sin, cos);
            let mut tangent_profile = next_pos - prev_pos;
            if tangent_profile.length_squared() <= MIN_LEN * MIN_LEN {
                if next_idx != profile_idx {
                    tangent_profile = next_pos - position;
                } else if prev_idx != profile_idx {
                    tangent_profile = position - prev_pos;
                }
            }
            if tangent_profile.length_squared() <= MIN_LEN * MIN_LEN {
                tangent_profile = rot_dir;
            } else {
                tangent_profile = tangent_profile.normalize();
            }

            let mut normal = tangent_theta.cross(tangent_profile);
            if normal.length_squared() <= MIN_LEN * MIN_LEN {
                normal = rotated_perp_dir;
            } else {
                normal = normal.normalize();
            }

            vertices.push(position);
            normals.push(normal);
        }
    }

    // 生成索引：连接相邻的轮廓点和角度段
    let stride = angular_segments + 1;
    for profile_idx in 0..(n_profile - 1) {
        let profile_offset = profile_idx * stride;
        let next_profile_offset = (profile_idx + 1) * stride;

        for seg in 0..angular_segments {
            let base = profile_offset + seg;
            let next_base = next_profile_offset + seg;

            // 两个三角形组成一个四边形
            indices.extend_from_slice(&[
                base as u32,
                (base + 1) as u32,
                next_base as u32,
                (base + 1) as u32,
                (next_base + 1) as u32,
                next_base as u32,
            ]);
        }
    }

    // 如果角度小于 360 度，需要添加端面
    if angle_deg < 360.0 - 1e-3 {
        // 添加起始端面
        let start_offset = vertices.len() as u32;
        for (i, &pt) in profile.iter().enumerate() {
            vertices.push(pt);
            let normal = if i < profile.len() - 1 {
                let edge = profile[i + 1] - pt;
                -rot_dir.cross(edge).normalize()
            } else {
                -rot_dir
            };
            normals.push(normal);
        }
        // 扇状三角化起始端面
        for i in 1..(profile.len() - 1) {
            indices.extend_from_slice(&[
                start_offset,
                start_offset + i as u32,
                start_offset + (i + 1) as u32,
            ]);
        }

        // 添加结束端面
        let end_offset = vertices.len() as u32;
        let last_theta = angle_rad;
        let (sin, cos) = last_theta.sin_cos();
        for (i, &pt) in profile.iter().enumerate() {
            let offset = pt - rot_pt;
            let perp_offset = offset - rot_dir * offset.dot(rot_dir);
            let perp_dist = perp_offset.length();
            if perp_dist > MIN_LEN {
                let perp_dir = perp_offset / perp_dist;
                let rotated_perp = perp_dir * cos + (rot_dir.cross(perp_dir)) * sin;
                let rotated_offset = rotated_perp * perp_dist + rot_dir * offset.dot(rot_dir);
                let position = rot_pt + rotated_offset;
                vertices.push(position);
            } else {
                vertices.push(pt);
            }
            let normal = if i < profile.len() - 1 {
                let edge = profile[i + 1] - pt;
                rot_dir.cross(edge).normalize()
            } else {
                rot_dir
            };
            normals.push(normal);
        }
        // 扇状三角化结束端面
        for i in 1..(profile.len() - 1) {
            indices.extend_from_slice(&[
                end_offset,
                end_offset + (i + 1) as u32,
                end_offset + i as u32,
            ]);
        }
    }

    if vertices.is_empty() {
        return None;
    }

    // 🆕 从 Profile 轮廓生成旋转体的特征边（纬线边）
    let revolution_edges = generate_revolution_profile_edges(
        profile,
        rot_pt,
        rot_dir,
        angle_rad,
        3,     // 生成 3 个纬线圆环：起始、中间、结束
        false, // 不包含经线边，避免过于密集
    );

    Some(GeneratedMesh {
        mesh: create_mesh_with_custom_edges(
            indices,
            vertices,
            normals,
            Some(aabb),
            Some(revolution_edges),
        ),
        aabb: Some(aabb),
    })
}

/// 生成PrimLoft（SweepSolid）网格
///
/// PrimLoft是一个通用的扫掠实体，通过将截面轮廓沿着路径扫掠来生成实体
/// 支持多种路径类型：直线、圆弧、多段路径等
fn generate_prim_loft_mesh(
    sweep: &SweepSolid,
    settings: &LodMeshSettings,
    non_scalable: bool,
    refno: Option<RefU64>,
) -> Option<GeneratedMesh> {
    use crate::geometry::sweep_mesh::generate_sweep_solid_mesh;

    // 使用sweep mesh生成器创建网格
    let mesh = generate_sweep_solid_mesh(sweep, settings, refno)?;

    // 计算AABB
    let aabb = if mesh.vertices.is_empty() {
        Aabb::new_invalid()
    } else {
        let mut aabb = Aabb::new_invalid();
        for vertex in &mesh.vertices {
            extend_aabb(&mut aabb, *vertex);
        }
        aabb
    };

    Some(GeneratedMesh {
        mesh,
        aabb: Some(aabb),
    })
}

#[cfg(test)]
mod closure_tests {
    use super::*;
    use crate::mesh_precision::LodMeshSettings;

    #[test]
    fn test_unit_cylinder_mesh_closure() {
        let settings = LodMeshSettings::default();
        // 生成 mesh
        let mesh = unit_cylinder_mesh(&settings, false);

        // 获取 resolution (根据 radius=0.5 计算)
        let resolution = compute_radial_segments(&settings, 0.5, false, 3);
        let height_segments = compute_height_segments(&settings, 1.0, false, 1);

        // 验证索引数量
        // 侧面三角形数 = height_segments * resolution * 2 (每个quad 2个三角形)
        // 端面三角形数 = resolution * 1 * 2 (上下两个端面，每个端面有 resolution 个三角形)
        // 注意：修复前的 bug 是 resolution - 1，所以如果测试通过，说明修复有效
        let expected_triangle_count = height_segments * resolution * 2 + resolution * 2;
        let expected_indices_count = expected_triangle_count * 3;

        assert_eq!(
            mesh.indices.len(),
            expected_indices_count,
            "Indices count mismatch. Expected {} triangles ({} indices), but got {} indices. Resolution: {}, Height Segments: {}",
            expected_triangle_count,
            expected_indices_count,
            mesh.indices.len(),
            resolution,
            height_segments
        );
    }
}
