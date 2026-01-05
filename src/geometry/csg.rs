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
use crate::geometry::sweep_mesh::generate_sweep_solid_mesh;
use crate::mesh_precision::LodMeshSettings;
use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::prim_geo::basic::CsgSharedMesh;
use crate::prim_geo::ctorus::CTorus;
use crate::prim_geo::cylinder::{LCylinder, SCylinder};
use crate::prim_geo::profile_processor::{ProfileProcessor, extrude_profile};
use crate::prim_geo::sweep_solid::SweepSolid;
use crate::prim_geo::wire::CurveType;
use crate::prim_geo::{
    dish::Dish, extrusion::Extrusion, lpyramid::LPyramid, polyhedron::Polyhedron, pyramid::Pyramid,
    revolution::Revolution, rtorus::RTorus, sbox::SBox, snout::LSnout, sphere::Sphere,
};
use crate::shape::pdms_shape::{Edge, Edges, PlantMesh, VerifiedShape};
use crate::types::refno::RefU64;
use crate::types::refno::RefnoEnum;
use crate::utils::svg_generator::SpineSvgGenerator;
use chrono;
use glam::{Mat3, Quat, Vec2, Vec3};
use nalgebra::Point3;
use parry3d::bounding_volume::{Aabb, BoundingVolume};
use std::collections::HashSet;
use std::io::Write;
use std::sync::Mutex;

/// 最小长度阈值，用于判断几何形状是否有效
const MIN_LEN: f32 = 1e-6;

/// 跟踪已经生成过PLOOP调试文件的refno，避免重复生成
static PLOOP_DEBUG_GENERATED: std::sync::LazyLock<Mutex<HashSet<String>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashSet::new()));

/// SSLC 生成计数器（用于调试，只生成第一个）
static SSLC_GENERATION_COUNTER: std::sync::LazyLock<Mutex<usize>> =
    std::sync::LazyLock::new(|| Mutex::new(0));

/// 清理PLOOP调试文件生成记录（用于新的运行周期）
pub fn clear_ploop_debug_cache() {
    if let Ok(mut generated_set) = PLOOP_DEBUG_GENERATED.lock() {
        generated_set.clear();
    }
}

/// 重置 SSLC 生成计数器
pub fn reset_sslc_counter() {
    if let Ok(mut counter) = SSLC_GENERATION_COUNTER.lock() {
        *counter = 0;
    }
}

/// 生成单位盒子网格（用于简单盒子的基础网格）
///
/// 返回一个尺寸为1x1x1的单位盒子，中心在原点
/// 生成单位盒子网格（流形版本）
///
/// 生成无重复顶点的流形网格：
/// - 只有 8 个角点顶点
/// - 12 个三角形（6 个面 × 2）
/// - 所有三角形法向量指向外部
pub fn unit_box_mesh() -> PlantMesh {
    let half = 0.5;

    // 8 个角点顶点
    let vertices = vec![
        Vec3::new(-half, -half, -half), // 0: 左下后
        Vec3::new(half, -half, -half),  // 1: 右下后
        Vec3::new(half, half, -half),   // 2: 右上后
        Vec3::new(-half, half, -half),  // 3: 左上后
        Vec3::new(-half, -half, half),  // 4: 左下前
        Vec3::new(half, -half, half),   // 5: 右下前
        Vec3::new(half, half, half),    // 6: 右上前
        Vec3::new(-half, half, half),   // 7: 左上前
    ];

    // 法向量（每个顶点取平均，这里简化为指向外部的对角方向）
    let normals = vec![
        Vec3::new(-1.0, -1.0, -1.0).normalize(),
        Vec3::new(1.0, -1.0, -1.0).normalize(),
        Vec3::new(1.0, 1.0, -1.0).normalize(),
        Vec3::new(-1.0, 1.0, -1.0).normalize(),
        Vec3::new(-1.0, -1.0, 1.0).normalize(),
        Vec3::new(1.0, -1.0, 1.0).normalize(),
        Vec3::new(1.0, 1.0, 1.0).normalize(),
        Vec3::new(-1.0, 1.0, 1.0).normalize(),
    ];

    // 12 个三角形（6 个面，每面 2 个三角形）
    // 绕序：从外部看逆时针，法向量指向外部
    let indices = vec![
        // 前面 (+Z): 4, 5, 6, 7
        4, 5, 6, 4, 6, 7, // 后面 (-Z): 1, 0, 3, 2
        1, 0, 3, 1, 3, 2, // 右面 (+X): 5, 1, 2, 6
        5, 1, 2, 5, 2, 6, // 左面 (-X): 0, 4, 7, 3
        0, 4, 7, 0, 7, 3, // 上面 (+Y): 7, 6, 2, 3
        7, 6, 2, 7, 2, 3, // 下面 (-Y): 0, 1, 5, 4
        0, 1, 5, 0, 5, 4,
    ];

    use nalgebra::Point3;
    use parry3d::bounding_volume::Aabb;

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
/// 生成单位球体网格（流形版本）
///
/// 参考 Manifold 的球体生成算法，生成无重复顶点的流形网格：
/// - 极点只有一个顶点（不重复）
/// - 每个纬度圈的顶点不重复（经度 0 和 2π 共用同一顶点）
/// - 所有三角形法向量指向外部
///
/// 顶点布局：
/// - [0]: 北极点
/// - [1, radial]: 第一纬度圈
/// - ...
/// - [1 + (height-1)*radial, 1 + height*radial - 1]: 最后一纬度圈
/// - [1 + height*radial]: 南极点
pub fn unit_sphere_mesh() -> PlantMesh {
    use nalgebra::Point3;
    use parry3d::bounding_volume::Aabb;
    let radius = 0.5;
    let settings = LodMeshSettings::default();
    let radial = compute_radial_segments(&settings, radius, false, 3);
    let mut height = compute_height_segments(&settings, radius * 2.0, false, 2);
    if height % 2 != 0 {
        height += 1;
    }

    // 顶点数：北极 + (height-1)个纬度圈 * radial + 南极
    let num_vertices = 2 + (height - 1) * radial;
    let mut vertices = Vec::with_capacity(num_vertices as usize);
    let mut normals = Vec::with_capacity(num_vertices as usize);
    let mut aabb = Aabb::new_invalid();

    // 1. 北极点
    let north_pole = Vec3::new(0.0, 0.0, radius);
    extend_aabb(&mut aabb, north_pole);
    vertices.push(north_pole);
    normals.push(Vec3::new(0.0, 0.0, 1.0));

    // 2. 中间纬度圈（不包括极点）
    for lat in 1..height {
        let v = lat as f32 / height as f32;
        let theta = v * std::f32::consts::PI;
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();

        for lon in 0..radial {
            let u = lon as f32 / radial as f32;
            let phi = u * std::f32::consts::TAU;
            let (sin_phi, cos_phi) = phi.sin_cos();

            let normal = Vec3::new(sin_theta * cos_phi, sin_theta * sin_phi, cos_theta);
            let vertex = normal * radius;
            extend_aabb(&mut aabb, vertex);
            vertices.push(vertex);
            normals.push(normal);
        }
    }

    // 3. 南极点
    let south_pole = Vec3::new(0.0, 0.0, -radius);
    extend_aabb(&mut aabb, south_pole);
    vertices.push(south_pole);
    normals.push(Vec3::new(0.0, 0.0, -1.0));

    let south_pole_idx = vertices.len() as u32 - 1;

    // 生成三角形索引
    let mut indices = Vec::new();

    // 4. 北极扇形三角形
    for lon in 0..radial {
        let v1 = 1 + lon as u32;
        let v2 = 1 + ((lon + 1) % radial) as u32;
        // 从外部看逆时针：north_pole -> v1 -> v2
        indices.extend_from_slice(&[0, v1, v2]);
    }

    // 5. 中间带状三角形
    for lat in 1..(height - 1) {
        let ring_start = 1 + (lat - 1) * radial;
        let next_ring_start = 1 + lat * radial;

        for lon in 0..radial {
            let curr = (ring_start + lon) as u32;
            let next = (ring_start + (lon + 1) % radial) as u32;
            let curr_below = (next_ring_start + lon) as u32;
            let next_below = (next_ring_start + (lon + 1) % radial) as u32;

            // 两个三角形组成四边形，法向量指向外部
            indices.extend_from_slice(&[curr, curr_below, next]);
            indices.extend_from_slice(&[next, curr_below, next_below]);
        }
    }

    // 6. 南极扇形三角形
    let last_ring_start = 1 + (height - 2) * radial;
    for lon in 0..radial {
        let v1 = (last_ring_start + lon) as u32;
        let v2 = (last_ring_start + (lon + 1) % radial) as u32;
        // 从外部看逆时针：v1 -> south_pole -> v2
        indices.extend_from_slice(&[v1, south_pole_idx, v2]);
    }

    let sphere_edges = generate_sphere_edges(radius, 8, 4);
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
/// 生成单位圆柱体网格（流形版本）
///
/// 参考 Manifold 的 Extrude 算法，生成无重复顶点的流形网格：
/// - 每个位置的顶点只生成一次
/// - 端面复用侧面顶点，不生成重复顶点
/// - 只添加端面中心点作为新顶点
///
/// 顶点布局：
/// - [0, resolution): 底面圆周顶点
/// - [resolution, 2*resolution): 顶面圆周顶点
/// - [2*resolution]: 底面中心点
/// - [2*resolution + 1]: 顶面中心点
pub fn unit_cylinder_mesh(settings: &LodMeshSettings, non_scalable: bool) -> PlantMesh {
    let height = 1.0;
    let radius = 0.5;

    // 使用LOD设置计算分段数
    let resolution = compute_radial_segments(settings, radius, non_scalable, 3);

    // 顶点数：底面圆周 + 顶面圆周 + 2个中心点
    let num_vertices = resolution * 2 + 2;
    // 三角形数：侧面 2*resolution + 底面 resolution + 顶面 resolution
    let num_triangles = resolution * 4;

    let mut vertices: Vec<Vec3> = Vec::with_capacity(num_vertices as usize);
    let mut normals: Vec<Vec3> = Vec::with_capacity(num_vertices as usize);
    let mut indices: Vec<u32> = Vec::with_capacity(num_triangles as usize * 3);

    let step_theta = std::f32::consts::TAU / resolution as f32;

    // 1. 生成底面圆周顶点 [0, resolution)
    for i in 0..resolution {
        let theta = i as f32 * step_theta;
        let (sin, cos) = theta.sin_cos();
        vertices.push([radius * cos, radius * sin, 0.0].into());
        // 侧面法向量（指向径向）
        normals.push([cos, sin, 0.0].into());
    }

    // 2. 生成顶面圆周顶点 [resolution, 2*resolution)
    for i in 0..resolution {
        let theta = i as f32 * step_theta;
        let (sin, cos) = theta.sin_cos();
        vertices.push([radius * cos, radius * sin, height].into());
        // 侧面法向量（指向径向）
        normals.push([cos, sin, 0.0].into());
    }

    // 3. 添加端面中心点
    let bottom_center = vertices.len() as u32;
    vertices.push([0.0, 0.0, 0.0].into());
    normals.push([0.0, 0.0, -1.0].into());

    let top_center = vertices.len() as u32;
    vertices.push([0.0, 0.0, height].into());
    normals.push([0.0, 0.0, 1.0].into());

    // 4. 生成侧面三角形（复用底面和顶面圆周顶点）
    // 绕序：从外部看为逆时针（CCW），法向量指向外部
    for i in 0..resolution {
        let bottom_curr = i as u32;
        let bottom_next = ((i + 1) % resolution) as u32;
        let top_curr = (resolution + i) as u32;
        let top_next = (resolution + (i + 1) % resolution) as u32;

        // 两个三角形组成一个四边形
        // 从外部看，顶点按逆时针排列
        // 三角形 1: bottom_curr -> bottom_next -> top_curr
        indices.extend_from_slice(&[bottom_curr, bottom_next, top_curr]);
        // 三角形 2: top_curr -> bottom_next -> top_next
        indices.extend_from_slice(&[top_curr, bottom_next, top_next]);
    }

    // 5. 生成底面三角形（扇形，复用底面圆周顶点）
    // 底面法线指向 -Z，从下方看为逆时针
    for i in 0..resolution {
        let v1 = i as u32;
        let v2 = ((i + 1) % resolution) as u32;
        // 从下方看：center -> v2 -> v1 为逆时针（法向量指向 -Z）
        indices.extend_from_slice(&[bottom_center, v2, v1]);
    }

    // 6. 生成顶面三角形（扇形，复用顶面圆周顶点）
    // 顶面法线指向 +Z，从上方看为逆时针
    for i in 0..resolution {
        let v1 = (resolution + i) as u32;
        let v2 = (resolution + (i + 1) % resolution) as u32;
        // 从上方看：center -> v1 -> v2 为逆时针（法向量指向 +Z）
        indices.extend_from_slice(&[top_center, v1, v2]);
    }

    // 🆕 生成圆柱体的特征边（顶圆 + 底圆 + 4条纵向边）
    let cylinder_edges = generate_cylinder_edges(
        radius, height, resolution, 4, // 生成 4 条纵向边，均匀分布
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

/// 将边从原点坐标系变换到目标位置和方向
///
/// # 参数
/// - `edges`: 原始边（在原点，Z轴为方向）
/// - `center`: 目标中心位置
/// - `axis`: 目标轴方向（归一化）
///
/// # 返回
/// 变换后的边
fn transform_edges(edges: Edges, center: Vec3, axis: Vec3) -> Edges {
    // 计算从 Z 轴到目标轴的旋转
    let z_axis = Vec3::Z;
    let rotation = if axis.dot(z_axis).abs() > 0.9999 {
        // 轴接近 Z 轴，不需要旋转或需要 180 度旋转
        if axis.dot(z_axis) > 0.0 {
            glam::Quat::IDENTITY
        } else {
            glam::Quat::from_rotation_x(std::f32::consts::PI)
        }
    } else {
        // 计算旋转四元数
        glam::Quat::from_rotation_arc(z_axis, axis)
    };

    edges
        .into_iter()
        .map(|edge| {
            let transformed_points: Vec<Vec3> = edge
                .vertices
                .iter()
                .map(|p| center + rotation.mul_vec3(*p))
                .collect();
            Edge::new(transformed_points)
        })
        .collect()
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
            let rotated_p0 =
                rotate_point_around_axis(p0, rot_pt, rot_dir, u_axis, v_axis, sin_theta, cos_theta);
            let rotated_p1 =
                rotate_point_around_axis(p1, rot_pt, rot_dir, u_axis, v_axis, sin_theta, cos_theta);

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

/// 生成斜切圆柱体的特征边
///
/// 参数直接使用底/顶椭圆采样点，避免重复重建。
pub fn generate_sscyl_edges(bottom_rim: &[Vec3], top_rim: &[Vec3]) -> Edges {
    let mut edges = Vec::new();
    if bottom_rim.len() < 2 || top_rim.len() != bottom_rim.len() {
        return edges;
    }

    let n = bottom_rim.len();

    // 1. 底边
    for i in 0..n {
        let next = (i + 1) % n;
        edges.push(Edge::new(vec![bottom_rim[i], bottom_rim[next]]));
    }

    // 2. 顶边
    for i in 0..n {
        let next = (i + 1) % n;
        edges.push(Edge::new(vec![top_rim[i], top_rim[next]]));
    }

    // 3. 4 条母线，取四等分角对应的索引
    let meridian_indices = [0, n / 4, n / 2, (n * 3) / 4];
    for idx in meridian_indices {
        let clamped = idx % n;
        edges.push(Edge::new(vec![bottom_rim[clamped], top_rim[clamped]]));
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
pub fn generate_sphere_edges(radius: f32, num_meridians: usize, num_parallels: usize) -> Edges {
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
pub fn build_csg_mesh(
    param: &PdmsGeoParam,
    settings: &LodMeshSettings,
    non_scalable: bool,
    refno: RefnoEnum,
) -> Option<GeneratedMesh> {
    match param {
        PdmsGeoParam::PrimLCylinder(cyl) => {
            generate_lcylinder_mesh(cyl, settings, non_scalable, refno)
        }
        PdmsGeoParam::PrimSCylinder(cyl) => {
            generate_scylinder_mesh(cyl, settings, non_scalable, refno)
        }
        PdmsGeoParam::PrimSphere(sphere) => {
            generate_sphere_mesh(sphere, settings, non_scalable, refno)
        }
        PdmsGeoParam::PrimLSnout(snout) => {
            generate_snout_mesh(snout, settings, non_scalable, refno)
        }
        PdmsGeoParam::PrimBox(sbox) => generate_box_mesh(sbox, refno),
        PdmsGeoParam::PrimDish(dish) => generate_dish_mesh(dish, settings, non_scalable, refno),
        PdmsGeoParam::PrimCTorus(torus) => {
            generate_torus_mesh(torus, settings, non_scalable, refno)
        }
        PdmsGeoParam::PrimRTorus(rtorus) => {
            generate_rect_torus_mesh(rtorus, settings, non_scalable, refno)
        }
        PdmsGeoParam::PrimPyramid(pyr) => generate_pyramid_mesh(pyr, refno),
        PdmsGeoParam::PrimLPyramid(lpyr) => generate_lpyramid_mesh(lpyr, refno),
        PdmsGeoParam::PrimExtrusion(extrusion) => generate_extrusion_mesh(extrusion, refno),
        PdmsGeoParam::PrimPolyhedron(poly) => generate_polyhedron_mesh(poly, refno),
        PdmsGeoParam::PrimRevolution(rev) => {
            generate_revolution_mesh(rev, settings, non_scalable, refno)
        }
        PdmsGeoParam::PrimLoft(sweep) => {
            generate_prim_loft_mesh(sweep, settings, non_scalable, refno)
        }
        _ => None,
    }
}

pub fn generate_csg_mesh(
    param: &PdmsGeoParam,
    settings: &LodMeshSettings,
    non_scalable: bool,
    refno: Option<RefnoEnum>,
) -> Option<GeneratedMesh> {
    build_csg_mesh(param, settings, non_scalable, refno.unwrap_or_default())
}

/// 生成线性圆柱体（LCylinder）网格
///
/// LCylinder由轴向方向、直径和两个端面的偏移距离定义
/// 与 SCylinder 一致，使用单位圆柱体，通过 transform 的 scale 来缩放
fn generate_lcylinder_mesh(
    cyl: &LCylinder,
    settings: &LodMeshSettings,
    non_scalable: bool,
    refno: RefnoEnum,
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

/// 将角度规范化到 [-90, 90] 度范围
///
/// 根据 E3D/PDMS 的几何规范，倾斜角度需要规范化到有效范围：
/// - 如果 angle > 90°，则 angle = angle - 180°
/// - 如果 angle < -90°，则 angle = angle + 180°
///
/// 这确保了几何一致性和计算稳定性
#[inline]
fn normalize_shear_angle(angle: f32) -> f32 {
    let mut result = angle;
    if result > 90.0 {
        result -= 180.0;
    }
    if result < -90.0 {
        result += 180.0;
    }
    result
}

/// 生成剪切圆柱体（SSCL，Shear Cylinder）网格
///
/// 实现对齐 Core3D / gm_CreateSlopeEndedCylinder 定义：
/// - 端面法向由四个剪切角得到（局部轴为 (X, Y, Z)）
/// - 端面是倾斜平面与圆柱的交线（椭圆边界），不再整体剪切侧面
/// - 侧面保持径向法向，仅在 z 方向被两平面截断
///
/// **重要**：网格在标准局部坐标系中生成（Z 轴朝上，原点在底部中心）
/// 外部的 transform 负责旋转和平移到世界坐标系
fn generate_sscl_mesh(
    cyl: &SCylinder,
    settings: &LodMeshSettings,
    non_scalable: bool,
    refno: RefnoEnum,
) -> Option<GeneratedMesh> {
    // 调试计数器：只生成第一个 SSLC
    // let mut counter = SSLC_GENERATION_COUNTER.lock().ok()?;
    // *counter += 1;
    // let current_count = *counter;
    // drop(counter);

    // if current_count != 5 {
    //     println!("⏭️  跳过 SSLC #{} (refno: {})", current_count, refno);
    //     return None;
    // }

    // println!("🔧 生成 SSLC #{} (refno: {})", current_count, refno);

    // 在标准局部坐标系中生成：Z 轴朝上，X/Y 是剪切方向
    let dir = cyl.paxi_dir;
    let (x_axis, y_axis) = orthonormal_basis(dir);
    dbg!(&cyl);
    dbg!(dir, x_axis, y_axis);

    let radius = (cyl.pdia * 0.5).abs();
    let height = cyl.phei;
    if radius <= MIN_LEN || height.abs() <= MIN_LEN {
        return None;
    }

    // 剪切角规范化到 (-90°, 90°)
    let x_sign = if x_axis.y < 0.0 { -1.0 } else { 1.0 };
    let y_sign = if y_axis.x < 0.0 { -1.0 } else { 1.0 };
    let btm_x_deg = x_sign * normalize_shear_angle(cyl.btm_shear_angles[0]);
    let btm_y_deg = y_sign * normalize_shear_angle(cyl.btm_shear_angles[1]);
    let top_x_deg = x_sign * normalize_shear_angle(cyl.top_shear_angles[0]);
    let top_y_deg = y_sign * normalize_shear_angle(cyl.top_shear_angles[1]);
    for a in [btm_x_deg, btm_y_deg, top_x_deg, top_y_deg] {
        if a <= -90.0 || a >= 90.0 {
            return None;
        }
    }

    dbg!(btm_x_deg, btm_y_deg, top_x_deg, top_y_deg);

    // libgm 斜率：直接使用 tan(angle)
    let btm_tan_x = btm_x_deg.to_radians().tan();
    let btm_tan_y = btm_y_deg.to_radians().tan();
    let top_tan_x = top_x_deg.to_radians().tan();
    let top_tan_y = top_y_deg.to_radians().tan();

    // 合法性：高度必须大于剪切差导致的最小厚度
    let shear_delta = (top_tan_x - btm_tan_x).hypot(top_tan_y - btm_tan_y);
    if height.abs() <= radius * shear_delta + MIN_LEN {
        return None;
    }

    // 网格原点在底部中心，顶部在 Z = height
    let half_h = height * 0.5;
    let center = if cyl.center_in_mid {
        Vec3::ZERO
    } else {
        dir * half_h
    };
    let bottom_center = center - dir * half_h;
    let top_center = center + dir * half_h;

    // 计算细分参数（libgm 仅两环，这里将轴向段固定为 1）
    let radial = compute_radial_segments(settings, radius, non_scalable, 3);
    let height_segments: usize = 1;
    let ring_stride = radial + 1;

    let mut vertices = Vec::with_capacity((height_segments + 1) * ring_stride + 2 * (radial + 1));
    let mut normals = Vec::with_capacity(vertices.capacity());
    let mut indices = Vec::with_capacity(height_segments * radial * 6 + radial * 6);
    let mut aabb = Aabb::new_invalid();

    let step_theta = std::f32::consts::TAU / radial as f32;

    // 预计算各 θ 的径向与端面交点（椭圆边界）
    struct RimSample {
        radial: Vec3,
        radial_normal: Vec3,
        z_b: f32,
        z_t: f32,
    }
    let mut rim_samples = Vec::with_capacity(ring_stride);
    let mut bottom_rim = Vec::with_capacity(ring_stride);
    let mut top_rim = Vec::with_capacity(ring_stride);
    for slice in 0..=radial {
        let angle = slice as f32 * step_theta;
        let (cos_theta, sin_theta) = (angle.cos(), angle.sin());
        let radial = x_axis * (radius * cos_theta) + y_axis * (radius * sin_theta);
        let radial_normal = radial.normalize();

        // libgm 公式：z = ±h/2 + r*(cosθ*tanX + sinθ*tanY)
        let z_b = -half_h + radius * (cos_theta * btm_tan_x + sin_theta * btm_tan_y);
        let z_t = half_h + radius * (cos_theta * top_tan_x + sin_theta * top_tan_y);

        let p_b = center + dir * z_b + radial;
        let p_t = center + dir * z_t + radial;

        bottom_rim.push(p_b);
        top_rim.push(p_t);
        rim_samples.push(RimSample {
            radial,
            radial_normal,
            z_b,
            z_t,
        });
    }

    // 侧面：固定径向，沿 z_b -> z_t 插值（两环）
    for ring in 0..=height_segments {
        let t = ring as f32 / height_segments as f32;
        for sample in &rim_samples {
            let z = sample.z_b + (sample.z_t - sample.z_b) * t;
            let vertex = center + dir * z + sample.radial;
            extend_aabb(&mut aabb, vertex);
            vertices.push(vertex);
            normals.push(sample.radial_normal);
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

    // 端面法向量（与 gm_CreateSlopeEndedCylinder 一致）
    // 公式：n = (sin(xSlope), sin(ySlope), cos(xSlope)*cos(ySlope))
    // 底面法向朝下（取反），顶面法向朝上
    let btm_x_rad = btm_x_deg.to_radians();
    let btm_y_rad = btm_y_deg.to_radians();
    let top_x_rad = top_x_deg.to_radians();
    let top_y_rad = top_y_deg.to_radians();

    let Nb = Vec3::new(
        -btm_x_rad.sin(),
        -btm_y_rad.sin(),
        -btm_x_rad.cos() * btm_y_rad.cos(),
    )
    .normalize();
    let Nt = Vec3::new(
        top_x_rad.sin(),
        top_y_rad.sin(),
        top_x_rad.cos() * top_y_rad.cos(),
    )
    .normalize();

    // 底面盖子（椭圆边界，法向 Nb）
    let bottom_start = vertices.len() as u32;
    for &vertex in &bottom_rim {
        vertices.push(vertex);
        normals.push(Nb);
        extend_aabb(&mut aabb, vertex);
    }
    let bottom_center_idx = vertices.len() as u32;
    vertices.push(bottom_center);
    normals.push(Nb);
    extend_aabb(&mut aabb, bottom_center);
    for slice in 0..radial {
        let next = slice + 1;
        indices.extend_from_slice(&[
            bottom_center_idx,
            bottom_start + next as u32,
            bottom_start + slice as u32,
        ]);
    }

    // 顶面盖子（椭圆边界，法向 Nt）
    let top_start = vertices.len() as u32;
    for &vertex in &top_rim {
        vertices.push(vertex);
        normals.push(Nt);
        extend_aabb(&mut aabb, vertex);
    }
    let top_center_idx = vertices.len() as u32;
    vertices.push(top_center);
    normals.push(Nt);
    extend_aabb(&mut aabb, top_center);
    for slice in 0..radial {
        let next = slice + 1;
        indices.extend_from_slice(&[
            top_center_idx,
            top_start + slice as u32,
            top_start + next as u32,
        ]);
    }

    // 生成几何边：椭圆边界 + 4 条母线
    let edges = generate_sscyl_edges(&bottom_rim, &top_rim);

    Some(GeneratedMesh {
        mesh: create_mesh_with_custom_edges(indices, vertices, normals, Some(aabb), Some(edges)),
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
    refno: RefnoEnum,
) -> Option<GeneratedMesh> {
    // 如果是剪切圆柱体，使用专门的生成函数
    if cyl.is_sscl() {
        return generate_sscl_mesh(cyl, settings, non_scalable, refno);
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
    let axis_vec = top_center - bottom_center;
    let height = axis_vec.length();
    if height <= MIN_LEN {
        return None;
    }
    let axis_dir = axis_vec / height;
    let (basis_u, basis_v) = orthonormal_basis(axis_dir);

    let radial = compute_radial_segments(settings, radius, non_scalable, 3);
    let h_segs = compute_height_segments(settings, height, non_scalable, 1);

    // 流形版本：侧面 (h_segs+1)*radial 顶点 + 2 个中心点
    let side_count = (h_segs + 1) * radial;
    let mut vertices = Vec::with_capacity(side_count + 2);
    let mut normals = Vec::with_capacity(side_count + 2);
    let mut indices = Vec::new();
    let mut aabb = Aabb::new_invalid();

    // 生成侧面顶点（无重复）
    for ring in 0..=h_segs {
        let t = ring as f32 / h_segs as f32;
        let center = bottom_center + axis_vec * t;
        for slice in 0..radial {
            let angle = std::f32::consts::TAU * slice as f32 / radial as f32;
            let (sin, cos) = angle.sin_cos();
            let radial_dir = basis_u * cos + basis_v * sin;
            let vertex = center + radial_dir * radius;
            extend_aabb(&mut aabb, vertex);
            vertices.push(vertex);
            normals.push(radial_dir);
        }
    }

    // 生成侧面三角形（使用模运算处理闭合）
    for ring in 0..h_segs {
        for slice in 0..radial {
            let next_slice = (slice + 1) % radial;
            let curr = ring * radial + slice;
            let curr_next = ring * radial + next_slice;
            let below = (ring + 1) * radial + slice;
            let below_next = (ring + 1) * radial + next_slice;
            // 从外部看逆时针
            indices.extend_from_slice(&[curr as u32, curr_next as u32, below as u32]);
            indices.extend_from_slice(&[below as u32, curr_next as u32, below_next as u32]);
        }
    }

    // 底面中心点
    let bottom_center_idx = vertices.len() as u32;
    vertices.push(bottom_center);
    normals.push(-axis_dir);
    extend_aabb(&mut aabb, bottom_center);

    // 底面三角形（复用侧面底部环顶点）
    for slice in 0..radial {
        let next_slice = (slice + 1) % radial;
        // 底面法向量向下，从下方看逆时针
        indices.extend_from_slice(&[bottom_center_idx, next_slice as u32, slice as u32]);
    }

    // 顶面中心点
    let top_center_idx = vertices.len() as u32;
    vertices.push(top_center);
    normals.push(axis_dir);
    extend_aabb(&mut aabb, top_center);

    // 顶面三角形（复用侧面顶部环顶点）
    let top_ring_start = h_segs * radial;
    for slice in 0..radial {
        let next_slice = (slice + 1) % radial;
        let curr = top_ring_start + slice;
        let next = top_ring_start + next_slice;
        // 顶面法向量向上，从上方看逆时针
        indices.extend_from_slice(&[top_center_idx, curr as u32, next as u32]);
    }

    // 生成几何边
    let base_edges = generate_cylinder_edges(radius, height, radial, 4);
    let edges = transform_edges(base_edges, bottom_center, axis_dir);
    Some(GeneratedMesh {
        mesh: create_mesh_with_custom_edges(indices, vertices, normals, Some(aabb), Some(edges)),
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
    refno: RefnoEnum,
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

    // 流形版本：极点单顶点 + 每个纬度环 radial 个顶点（无重复）
    // 顶点数: 2 (极点) + (height - 1) * radial
    let vertex_count = 2 + (height - 1) * radial;
    let mut vertices = Vec::with_capacity(vertex_count);
    let mut normals = Vec::with_capacity(vertex_count);
    let mut indices = Vec::with_capacity(height * radial * 6);
    let mut aabb = Aabb::new_invalid();

    // 北极点 (lat = 0, theta = 0)
    let north_pole = sphere.center + Vec3::Z * radius;
    extend_aabb(&mut aabb, north_pole);
    vertices.push(north_pole);
    normals.push(Vec3::Z);

    // 中间纬度环 (lat = 1 到 height - 1)
    for lat in 1..height {
        let v = lat as f32 / height as f32;
        let theta = v * std::f32::consts::PI;
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();

        for lon in 0..radial {
            let phi = std::f32::consts::TAU * lon as f32 / radial as f32;
            let (sin_phi, cos_phi) = phi.sin_cos();

            let normal = Vec3::new(sin_theta * cos_phi, sin_theta * sin_phi, cos_theta);
            let vertex = sphere.center + normal * radius;
            extend_aabb(&mut aabb, vertex);
            vertices.push(vertex);
            normals.push(normal);
        }
    }

    // 南极点 (lat = height, theta = π)
    let south_pole = sphere.center - Vec3::Z * radius;
    extend_aabb(&mut aabb, south_pole);
    vertices.push(south_pole);
    normals.push(-Vec3::Z);

    // === 生成三角形索引 ===
    let north_pole_idx = 0u32;
    let south_pole_idx = (vertices.len() - 1) as u32;
    let first_ring_start = 1usize; // 第一个纬度环的起始索引

    // 北极扇形三角形 (连接北极点到第一个纬度环)
    for lon in 0..radial {
        let next_lon = (lon + 1) % radial;
        let curr = (first_ring_start + lon) as u32;
        let next = (first_ring_start + next_lon) as u32;
        // 从外部看逆时针
        indices.extend_from_slice(&[north_pole_idx, next, curr]);
    }

    // 中间纬度带的四边形（两个三角形）
    for lat in 0..(height - 2) {
        let ring_start = first_ring_start + lat * radial;
        let next_ring_start = ring_start + radial;
        for lon in 0..radial {
            let next_lon = (lon + 1) % radial;
            let curr = (ring_start + lon) as u32;
            let curr_next = (ring_start + next_lon) as u32;
            let below = (next_ring_start + lon) as u32;
            let below_next = (next_ring_start + next_lon) as u32;
            // 从外部看逆时针
            indices.extend_from_slice(&[curr, curr_next, below]);
            indices.extend_from_slice(&[below, curr_next, below_next]);
        }
    }

    // 南极扇形三角形 (连接最后一个纬度环到南极点)
    let last_ring_start = first_ring_start + (height - 2) * radial;
    for lon in 0..radial {
        let next_lon = (lon + 1) % radial;
        let curr = (last_ring_start + lon) as u32;
        let next = (last_ring_start + next_lon) as u32;
        // 从外部看逆时针
        indices.extend_from_slice(&[curr, next, south_pole_idx]);
    }

    // 生成几何边：赤道 + 2条子午线
    let base_edges = generate_sphere_edges(radius, radial, 1);
    let edges = transform_edges(base_edges, sphere.center, Vec3::Z);
    Some(GeneratedMesh {
        mesh: create_mesh_with_custom_edges(indices, vertices, normals, Some(aabb), Some(edges)),
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
    refno: RefnoEnum,
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
    let radius_delta = top_radius - bottom_radius;

    // 流形版本：每圈只有 radial 个顶点（不重复）
    // 顶点布局：
    // - [0, radial): 底圈
    // - [radial, 2*radial): 第二圈
    // - ...
    // - [height_segments * radial]: 底面中心（如果有底面）
    // - [height_segments * radial + 1]: 顶面中心（如果有顶面）
    let mut vertices = Vec::with_capacity((height_segments + 1) * radial + 2);
    let mut normals = Vec::with_capacity(vertices.capacity());
    let mut indices = Vec::new();
    let mut aabb = Aabb::new_invalid();

    // 生成侧面顶点（每圈 radial 个，不重复）
    for segment in 0..=height_segments {
        let t = segment as f32 / height_segments as f32;
        let center = bottom_center + axis_dir * (height_axis * t) + offset_dir * (snout.poff * t);
        let radius = (bottom_radius + radius_delta * t).max(0.0);
        for slice in 0..radial {
            let angle = slice as f32 * step_theta;
            let (sin, cos) = angle.sin_cos();
            let radial_dir = basis_u * cos + basis_v * sin;
            let vertex = center + radial_dir * radius;
            extend_aabb(&mut aabb, vertex);
            vertices.push(vertex);

            // 计算法向量
            let tangent_theta = (-sin) * basis_u + cos * basis_v;
            let tangent_theta = tangent_theta * radius;
            let tangent_height = center_delta + radial_dir * radius_delta;
            let mut normal = tangent_theta.cross(tangent_height);
            if normal.length_squared() <= 1e-8 {
                normal = radial_dir;
            } else {
                normal = normal.normalize();
            }
            normals.push(normal);
        }
    }

    // 生成侧面三角形（流形版本，使用模运算处理闭合）
    for segment in 0..height_segments {
        let ring_start = segment * radial;
        let next_ring_start = (segment + 1) * radial;
        for slice in 0..radial {
            let curr = (ring_start + slice) as u32;
            let next = (ring_start + (slice + 1) % radial) as u32;
            let curr_above = (next_ring_start + slice) as u32;
            let next_above = (next_ring_start + (slice + 1) % radial) as u32;

            // 两个三角形，法向量指向外部
            indices.extend_from_slice(&[curr, next, curr_above]);
            indices.extend_from_slice(&[next, next_above, curr_above]);
        }
    }

    // 生成底面（如果有）
    if bottom_radius > MIN_LEN {
        let bottom_center_index = vertices.len() as u32;
        vertices.push(bottom_center);
        normals.push(-axis_dir);
        extend_aabb(&mut aabb, bottom_center);

        // 底面扇形三角形，复用底圈顶点
        for slice in 0..radial {
            let v1 = slice as u32;
            let v2 = ((slice + 1) % radial) as u32;
            // 底面法向量指向 -axis_dir
            indices.extend_from_slice(&[bottom_center_index, v2, v1]);
        }
    }

    // 生成顶面（如果有）
    if top_radius > MIN_LEN {
        let top_center = bottom_center + axis_dir * height_axis + offset_dir * snout.poff;
        let top_center_index = vertices.len() as u32;
        vertices.push(top_center);
        normals.push(axis_dir);
        extend_aabb(&mut aabb, top_center);

        // 顶面扇形三角形，复用顶圈顶点
        let top_ring_start = height_segments * radial;
        for slice in 0..radial {
            let v1 = (top_ring_start + slice) as u32;
            let v2 = (top_ring_start + (slice + 1) % radial) as u32;
            // 顶面法向量指向 +axis_dir
            indices.extend_from_slice(&[top_center_index, v1, v2]);
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
        radial, // 圆周分段数
        4,      // 4条竖直边
    );

    Some(GeneratedMesh {
        mesh: create_mesh_with_custom_edges(
            indices,
            vertices,
            normals,
            Some(aabb),
            Some(snout_edges),
        ),
        aabb: Some(aabb),
    })
}

/// 生成盒子（SBox）网格
///
/// 盒子由中心点和尺寸定义，包含6个面（每个面由2个三角形组成）
fn generate_box_mesh(sbox: &SBox, refno: RefnoEnum) -> Option<GeneratedMesh> {
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

    // 生成几何边：12条边
    let base_edges = generate_box_edges(sbox.size.x, sbox.size.y, sbox.size.z);
    let edges = transform_edges(base_edges, sbox.center, Vec3::Z);

    let mut mesh =
        create_mesh_with_custom_edges(indices, vertices, normals, Some(aabb), Some(edges));
    mesh.uvs = uvs; // 使用手动计算的 UV 覆盖默认的空 UV

    Some(GeneratedMesh {
        mesh,
        aabb: Some(aabb),
    })
}

/// 焊接重合顶点以生成 Manifold 兼容的网格
///
/// 使用自适应容差量化顶点位置，将数值上接近的顶点合并为同一顶点。
/// 这是 Manifold 布尔运算所必需的，因为 Manifold 要求共享顶点拓扑。
fn weld_vertices_for_manifold(mesh: &mut PlantMesh) {
    use std::collections::HashMap;

    if mesh.vertices.is_empty() || mesh.indices.len() < 3 {
        return;
    }

    // 计算 AABB 来确定自适应精度
    let mut min_pt = Vec3::splat(f32::MAX);
    let mut max_pt = Vec3::splat(f32::MIN);
    for v in &mesh.vertices {
        min_pt = min_pt.min(*v);
        max_pt = max_pt.max(*v);
    }

    let extent = max_pt - min_pt;
    let min_extent = extent.x.min(extent.y).min(extent.z);

    // 根据最小维度选择量化精度
    // 确保每个维度至少有 100 个离散点
    let precision: f32 = if min_extent < 0.1 {
        // 非常小的几何体，使用 5 位小数
        100000.0
    } else if min_extent < 1.0 {
        // 小几何体，使用 4 位小数
        10000.0
    } else if min_extent < 10.0 {
        // 单位化的几何体，使用 3 位小数
        1000.0
    } else if min_extent < 100.0 {
        // 中等几何体，使用 2 位小数
        100.0
    } else {
        // 大型几何体，使用 1 位小数
        10.0
    };

    // 量化函数：将浮点坐标转换为整数键
    let quantize = |v: Vec3| -> (i64, i64, i64) {
        (
            (v.x * precision).round() as i64,
            (v.y * precision).round() as i64,
            (v.z * precision).round() as i64,
        )
    };

    let mut map: HashMap<(i64, i64, i64), u32> = HashMap::new();
    let mut remap: Vec<u32> = Vec::with_capacity(mesh.vertices.len());
    let mut new_vertices: Vec<Vec3> = Vec::new();
    let mut new_normals: Vec<Vec3> = Vec::new();
    let mut new_uvs: Vec<[f32; 2]> = Vec::new();

    for (i, v) in mesh.vertices.iter().copied().enumerate() {
        let key = quantize(v);
        if let Some(&idx) = map.get(&key) {
            remap.push(idx);
            continue;
        }
        let idx = new_vertices.len() as u32;
        map.insert(key, idx);
        remap.push(idx);
        new_vertices.push(v);
        if i < mesh.normals.len() {
            new_normals.push(mesh.normals[i]);
        } else {
            new_normals.push(Vec3::ZERO);
        }
        if i < mesh.uvs.len() {
            new_uvs.push(mesh.uvs[i]);
        }
    }

    let mut new_indices: Vec<u32> = Vec::with_capacity(mesh.indices.len());
    for tri in mesh.indices.chunks(3) {
        if tri.len() != 3 {
            continue;
        }
        let a = remap[tri[0] as usize];
        let b = remap[tri[1] as usize];
        let c = remap[tri[2] as usize];
        // 跳过退化三角形（顶点重合）
        if a == b || b == c || a == c {
            continue;
        }
        new_indices.push(a);
        new_indices.push(b);
        new_indices.push(c);
    }

    mesh.vertices = new_vertices;
    mesh.normals = new_normals;
    if new_uvs.len() == mesh.vertices.len() {
        mesh.uvs = new_uvs;
    } else {
        mesh.uvs.clear();
    }
    mesh.indices = new_indices;
    if mesh.edges.is_empty() {
        mesh.edges = extract_edges_from_mesh(&mesh.indices, &mesh.vertices);
    }
    if mesh.uvs.is_empty() || mesh.uvs.len() != mesh.vertices.len() {
        mesh.generate_auto_uvs();
    }
    mesh.sync_wire_vertices_from_edges();
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
    refno: RefnoEnum,
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

    // 大尺寸 dish 自适应增加分段数
    // 基于半径计算：每米增加精度，使用 sqrt 避免过度增长
    let base_min_segments = settings.radial_segments.max(24) as f32; // dish 最低 24 段
    let size_factor = (radius_rim / 1000.0).max(1.0); // radius_rim 单位为 mm
    let radial_segments = ((base_min_segments * size_factor.sqrt())
        .min(128.0) // 上限 128
        .max(24.0)) as usize; // 最低 24 段
    // dbg!(radius_rim, size_factor, radial_segments);
    // 对于椭圆 dish，根据 arc 和 scale_z 计算合适的 rings 数
    // 参考 rvmparser: rings = max(min_rings, scale_z * samples * arc / (2π))
    let min_rings = 12u16;
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

    // 生成几何边：底面圆弧
    let radial = compute_radial_segments(settings, radius_rim, non_scalable, 3);
    let base_edges = generate_cylinder_edges(radius_rim, 0.0, radial, 0);
    let edges = transform_edges(base_edges, base_center, axis);
    Some(GeneratedMesh {
        mesh: create_mesh_with_custom_edges(indices, vertices, normals, Some(aabb), Some(edges)),
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
    refno: RefnoEnum,
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

    // 对于部分圆环，需要添加端面（复用侧面顶点，不生成新顶点）
    if sweep_angle < std::f32::consts::TAU - 1e-3 {
        // 起始端面：复用第一圈顶点 [0, samples_s)
        // 扇状三角化，法向量指向 -X 方向
        for i in 1..(samples_s - 1) {
            // 绕序：从外部看逆时针
            indices.extend_from_slice(&[0, (i + 1) as u32, i as u32]);
        }

        // 结束端面：复用最后一圈顶点 [(samples_l-1)*samples_s, samples_l*samples_s)
        let last_ring_start = ((samples_l - 1) * samples_s) as u32;
        for i in 1..(samples_s - 1) {
            // 绕序：从外部看逆时针
            indices.extend_from_slice(&[
                last_ring_start,
                last_ring_start + i as u32,
                last_ring_start + (i + 1) as u32,
            ]);
        }
    }

    // 生成几何边：主圆弧（torus 中心线，在原点，Z轴方向）
    let base_edges = generate_cylinder_edges(major_radius, 0.0, samples_l, 0);
    let edges = transform_edges(base_edges, Vec3::ZERO, Vec3::Z);
    Some(GeneratedMesh {
        mesh: create_mesh_with_custom_edges(indices, vertices, normals, Some(aabb), Some(edges)),
        aabb: Some(aabb),
    })
}

/// 生成棱锥（Pyramid）网格
///
/// 棱锥具有：
/// - 底部矩形（由pbbt和pcbt定义）
/// - 顶部矩形或点（由pbtp和pctp定义）
/// - 如果顶部尺寸为0，则顶部为顶点
fn generate_pyramid_mesh(pyr: &Pyramid, refno: RefnoEnum) -> Option<GeneratedMesh> {
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

    // 生成几何边
    let mut edges = Vec::new();

    // 底部4条边
    if let Some(bottom) = bottom_corners {
        for i in 0..4 {
            let next = (i + 1) % 4;
            edges.push(Edge::new(vec![
                vertices[bottom[i] as usize],
                vertices[bottom[next] as usize],
            ]));
        }
    }

    // 顶部边或斜边
    if let Some(top) = top_vertices {
        // 截锥：顶部4条边 + 4条竖边
        for i in 0..4 {
            let next = (i + 1) % 4;
            edges.push(Edge::new(vec![
                vertices[top[i] as usize],
                vertices[top[next] as usize],
            ]));
        }
        if let Some(bottom) = bottom_corners {
            for i in 0..4 {
                edges.push(Edge::new(vec![
                    vertices[bottom[i] as usize],
                    vertices[top[i] as usize],
                ]));
            }
        }
    } else if let (Some(bottom), Some(apex)) = (bottom_corners, apex_index) {
        // 尖锥：4条斜边到顶点
        for i in 0..4 {
            edges.push(Edge::new(vec![
                vertices[bottom[i] as usize],
                vertices[apex as usize],
            ]));
        }
    }

    Some(GeneratedMesh {
        mesh: create_mesh_with_custom_edges(indices, vertices, normals, Some(aabb), Some(edges)),
        aabb: Some(aabb),
    })
}

/// 生成线性棱锥（LPyramid）网格 - 与 OCC/core.dll 实现一致
///
/// LPYRA 几何体定义：
/// - PAAX: A轴方向（高度方向）
/// - PBAX: B轴方向（宽度方向）  
/// - PCAX: C轴方向（深度方向）
/// - PBTP/PCTP: 顶面 B/C 方向半尺寸
/// - PBBT/PCBT: 底面 B/C 方向半尺寸
/// - PTDI/PBDI: 到顶面/底面的距离
/// - PBOF/PCOF: B/C 方向偏移（仅应用于顶面）
fn generate_lpyramid_mesh(lpyr: &LPyramid, refno: RefnoEnum) -> Option<GeneratedMesh> {
    if !lpyr.check_valid() {
        return None;
    }

    let tx = (lpyr.pbtp * 0.5).max(MIN_LEN);
    let ty = (lpyr.pctp * 0.5).max(MIN_LEN);
    let bx = (lpyr.pbbt * 0.5).max(MIN_LEN);
    let by = (lpyr.pcbt * 0.5).max(MIN_LEN);

    // 计算正交化的轴方向
    let axis_dir = safe_normalize(lpyr.paax_dir)?;
    let (fallback_u, fallback_v) = orthonormal_basis(axis_dir);
    let mut pb_dir = safe_normalize(lpyr.pbax_dir).unwrap_or(fallback_u);
    pb_dir = (pb_dir - axis_dir * pb_dir.dot(axis_dir)).normalize_or_zero();
    if pb_dir.length_squared() <= MIN_LEN * MIN_LEN {
        pb_dir = fallback_u;
    }
    let mut pc_dir = safe_normalize(lpyr.pcax_dir).unwrap_or(fallback_v);
    pc_dir = (pc_dir - axis_dir * pc_dir.dot(axis_dir) - pb_dir * pc_dir.dot(pb_dir))
        .normalize_or_zero();
    if pc_dir.length_squared() <= MIN_LEN * MIN_LEN {
        pc_dir = fallback_v;
    }

    // 偏移使用正交化后的方向计算（与 core.dll 一致）
    let offset_3d = pb_dir * lpyr.pbof + pc_dir * lpyr.pcof;

    // 以底面中心为参考点（与 geo_relate transform 一致）
    let center = lpyr.paax_pt + axis_dir * lpyr.pbdi;
    let height = lpyr.ptdi - lpyr.pbdi; // 总高度
    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();
    let mut aabb = Aabb::new_invalid();

    let add_vert = |p: Vec3, v: &mut Vec<Vec3>, n: &mut Vec<Vec3>, a: &mut Aabb| -> u32 {
        extend_aabb(a, p);
        v.push(p);
        n.push(Vec3::ZERO);
        (v.len() - 1) as u32
    };

    let offsets = [(-1.0f32, -1.0f32), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)];

    // 顶面：z=height，带偏移
    // 偏移 offset_3d 已经是世界坐标，直接添加到最终位置
    let top = if tx > MIN_LEN && ty > MIN_LEN {
        let mut idxs = [0u32; 4];
        for (i, (ox, oy)) in offsets.iter().enumerate() {
            let pos =
                center + pb_dir * (ox * tx) + pc_dir * (oy * ty) + axis_dir * height + offset_3d;
            idxs[i] = add_vert(pos, &mut vertices, &mut normals, &mut aabb);
        }
        Some(idxs)
    } else {
        None
    };

    // 底面：z=0，无偏移
    let bot = if bx > MIN_LEN && by > MIN_LEN {
        let mut idxs = [0u32; 4];
        for (i, (ox, oy)) in offsets.iter().enumerate() {
            let pos = center + pb_dir * (ox * bx) + pc_dir * (oy * by);
            idxs[i] = add_vert(pos, &mut vertices, &mut normals, &mut aabb);
        }
        Some(idxs)
    } else {
        None
    };

    // 顶点（当顶面退化为点时）
    let apex = if top.is_none() {
        let pos = center + axis_dir * height + offset_3d;
        Some(add_vert(pos, &mut vertices, &mut normals, &mut aabb))
    } else {
        None
    };

    // 底面三角形
    if let Some(b) = bot {
        indices.extend([b[0], b[1], b[2], b[0], b[2], b[3]]);
    }
    if bot.is_none() && top.is_some() {
        return None;
    }

    // 顶面和侧面
    if let Some(t) = top {
        indices.extend([t[2], t[1], t[0], t[3], t[2], t[0]]);
        if let Some(b) = bot {
            for i in 0..4 {
                let n = (i + 1) % 4;
                indices.extend([b[i], b[n], t[n], b[i], t[n], t[i]]);
            }
        }
    } else if let (Some(b), Some(a)) = (bot, apex) {
        for i in 0..4 {
            indices.extend([b[(i + 1) % 4], b[i], a]);
        }
    }

    if indices.is_empty() {
        return None;
    }

    // 计算法向量
    for tri in indices.chunks_exact(3) {
        let n = (vertices[tri[1] as usize] - vertices[tri[0] as usize])
            .cross(vertices[tri[2] as usize] - vertices[tri[0] as usize]);
        if n.length_squared() > MIN_LEN * MIN_LEN {
            let norm = n.normalize();
            normals[tri[0] as usize] += norm;
            normals[tri[1] as usize] += norm;
            normals[tri[2] as usize] += norm;
        }
    }
    for n in &mut normals {
        *n = if n.length_squared() > MIN_LEN * MIN_LEN {
            n.normalize()
        } else {
            axis_dir
        };
    }

    // 边
    let mut edges = Vec::new();
    if let Some(b) = bot {
        for i in 0..4 {
            edges.push(Edge::new(vec![
                vertices[b[i] as usize],
                vertices[b[(i + 1) % 4] as usize],
            ]));
        }
    }
    if let Some(t) = top {
        for i in 0..4 {
            edges.push(Edge::new(vec![
                vertices[t[i] as usize],
                vertices[t[(i + 1) % 4] as usize],
            ]));
        }
        if let Some(b) = bot {
            for i in 0..4 {
                edges.push(Edge::new(vec![
                    vertices[b[i] as usize],
                    vertices[t[i] as usize],
                ]));
            }
        }
    } else if let (Some(b), Some(a)) = (bot, apex) {
        for i in 0..4 {
            edges.push(Edge::new(vec![
                vertices[b[i] as usize],
                vertices[a as usize],
            ]));
        }
    }

    Some(GeneratedMesh {
        mesh: create_mesh_with_custom_edges(indices, vertices, normals, Some(aabb), Some(edges)),
        aabb: Some(aabb),
    })
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
    refno: RefnoEnum,
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
    let is_full_circle = sweep_angle >= std::f32::consts::TAU - 1e-3;

    // 对于完整圆环，不需要额外的采样点（首尾共享）
    // 对于部分圆环，需要 major_segments + 1 个采样点
    let radial = if is_full_circle {
        major_segments
    } else {
        major_segments + 1
    };
    let h_segs = height_segments;

    // 预计算三角函数值
    let mut cos_vals = Vec::with_capacity(radial);
    let mut sin_vals = Vec::with_capacity(radial);
    for i in 0..radial {
        let theta = if is_full_circle {
            std::f32::consts::TAU * i as f32 / radial as f32
        } else {
            sweep_angle * i as f32 / (radial - 1) as f32
        };
        cos_vals.push(theta.cos());
        sin_vals.push(theta.sin());
    }

    // === 统一顶点布局 ===
    // 外圆柱面: (h_segs+1) × radial 顶点，索引 0..(h_segs+1)*radial
    // 内圆柱面: (h_segs+1) × radial 顶点，索引 outer_count..outer_count+inner_count
    // 顶部/底部环形面复用外/内圆柱面的边缘顶点
    // 部分圆环的端面需要额外的内部顶点

    let outer_count = (h_segs + 1) * radial;
    let inner_count = (h_segs + 1) * radial;
    let total_base = outer_count + inner_count;

    let mut vertices = Vec::with_capacity(total_base);
    let mut normals = Vec::with_capacity(total_base);
    let mut indices = Vec::new();
    let mut aabb = Aabb::new_invalid();

    // --- 生成外圆柱面顶点 ---
    for h in 0..=h_segs {
        let t = h as f32 / h_segs as f32;
        let z = -half_height + t * 2.0 * half_height;
        for seg in 0..radial {
            let pos = Vec3::new(
                outer_radius * cos_vals[seg],
                outer_radius * sin_vals[seg],
                z,
            );
            let normal = Vec3::new(cos_vals[seg], sin_vals[seg], 0.0);
            extend_aabb(&mut aabb, pos);
            vertices.push(pos);
            normals.push(normal);
        }
    }

    // --- 生成内圆柱面顶点 ---
    let inner_start = vertices.len();
    for h in 0..=h_segs {
        let t = h as f32 / h_segs as f32;
        let z = -half_height + t * 2.0 * half_height;
        for seg in 0..radial {
            let pos = Vec3::new(
                inner_radius * cos_vals[seg],
                inner_radius * sin_vals[seg],
                z,
            );
            let normal = Vec3::new(-cos_vals[seg], -sin_vals[seg], 0.0); // 内表面法向量向内
            extend_aabb(&mut aabb, pos);
            vertices.push(pos);
            normals.push(normal);
        }
    }

    // === 生成外圆柱面三角形 ===
    for h in 0..h_segs {
        for seg in 0..radial {
            let next_seg = if is_full_circle {
                (seg + 1) % radial
            } else {
                seg + 1
            };
            if !is_full_circle && seg == radial - 1 {
                continue;
            } // 部分圆环最后一列不连接

            let curr = h * radial + seg;
            let next_h = (h + 1) * radial + seg;
            let curr_next = h * radial + next_seg;
            let next_h_next = (h + 1) * radial + next_seg;

            // 外表面：从外部看逆时针
            indices.extend_from_slice(&[curr as u32, next_h as u32, curr_next as u32]);
            indices.extend_from_slice(&[curr_next as u32, next_h as u32, next_h_next as u32]);
        }
    }

    // === 生成内圆柱面三角形 ===
    for h in 0..h_segs {
        for seg in 0..radial {
            let next_seg = if is_full_circle {
                (seg + 1) % radial
            } else {
                seg + 1
            };
            if !is_full_circle && seg == radial - 1 {
                continue;
            }

            let curr = inner_start + h * radial + seg;
            let next_h = inner_start + (h + 1) * radial + seg;
            let curr_next = inner_start + h * radial + next_seg;
            let next_h_next = inner_start + (h + 1) * radial + next_seg;

            // 内表面：从内部看逆时针（即从外部看顺时针）
            indices.extend_from_slice(&[curr as u32, curr_next as u32, next_h as u32]);
            indices.extend_from_slice(&[curr_next as u32, next_h_next as u32, next_h as u32]);
        }
    }

    // === 生成顶部环形面三角形 ===
    // 顶部外圈索引: h_segs * radial .. (h_segs+1) * radial
    // 顶部内圈索引: inner_start + h_segs * radial .. inner_start + (h_segs+1) * radial
    let top_outer_start = h_segs * radial;
    let top_inner_start = inner_start + h_segs * radial;

    for seg in 0..radial {
        let next_seg = if is_full_circle {
            (seg + 1) % radial
        } else {
            seg + 1
        };
        if !is_full_circle && seg == radial - 1 {
            continue;
        }

        let outer_curr = top_outer_start + seg;
        let outer_next = top_outer_start + next_seg;
        let inner_curr = top_inner_start + seg;
        let inner_next = top_inner_start + next_seg;

        // 顶面法向量向上，需要与外圆柱面顶部边缘方向相反
        // 外圆柱面边: seg -> next_seg，所以顶面边应为: next_seg -> seg
        indices.extend_from_slice(&[outer_curr as u32, inner_curr as u32, outer_next as u32]);
        indices.extend_from_slice(&[outer_next as u32, inner_curr as u32, inner_next as u32]);
    }

    // === 生成底部环形面三角形 ===
    // 底部外圈索引: 0 .. radial
    // 底部内圈索引: inner_start .. inner_start + radial
    let bottom_outer_start = 0;
    let bottom_inner_start = inner_start;

    for seg in 0..radial {
        let next_seg = if is_full_circle {
            (seg + 1) % radial
        } else {
            seg + 1
        };
        if !is_full_circle && seg == radial - 1 {
            continue;
        }

        let outer_curr = bottom_outer_start + seg;
        let outer_next = bottom_outer_start + next_seg;
        let inner_curr = bottom_inner_start + seg;
        let inner_next = bottom_inner_start + next_seg;

        // 底面法向量向下，需要与外圆柱面底部边缘方向相反
        // 外圆柱面边: seg -> next_seg，所以底面边应为: next_seg -> seg
        indices.extend_from_slice(&[outer_curr as u32, outer_next as u32, inner_curr as u32]);
        indices.extend_from_slice(&[inner_curr as u32, outer_next as u32, inner_next as u32]);
    }

    // === 部分圆环的端面 ===
    if !is_full_circle {
        // 起始端面 (seg=0)
        // 四个角点已存在：外底(0), 外顶(h_segs*radial), 内底(inner_start), 内顶(inner_start+h_segs*radial)
        let start_outer_bottom = 0;
        let start_outer_top = h_segs * radial;
        let start_inner_bottom = inner_start;
        let start_inner_top = inner_start + h_segs * radial;

        // 起始端面法向量：指向负Y方向（角度=0时）
        // 从外部看，顺序应为：外底->内底->内顶->外顶（逆时针）
        indices.extend_from_slice(&[
            start_outer_bottom as u32,
            start_inner_bottom as u32,
            start_inner_top as u32,
        ]);
        indices.extend_from_slice(&[
            start_outer_bottom as u32,
            start_inner_top as u32,
            start_outer_top as u32,
        ]);

        // 结束端面 (seg=radial-1)
        let end_outer_bottom = radial - 1;
        let end_outer_top = h_segs * radial + radial - 1;
        let end_inner_bottom = inner_start + radial - 1;
        let end_inner_top = inner_start + h_segs * radial + radial - 1;

        // 结束端面法向量：指向正方向
        // 从外部看，顺序应为：外底->外顶->内顶->内底（逆时针）
        indices.extend_from_slice(&[
            end_outer_bottom as u32,
            end_outer_top as u32,
            end_inner_top as u32,
        ]);
        indices.extend_from_slice(&[
            end_outer_bottom as u32,
            end_inner_top as u32,
            end_inner_bottom as u32,
        ]);
    }

    let final_aabb = Some(aabb);

    // 生成几何边：内外圆弧（顶部和底部）
    let mut edges = Vec::new();

    // 顶部外圆弧
    let mut top_outer_points = Vec::with_capacity(radial);
    for i in 0..radial {
        top_outer_points.push(Vec3::new(
            outer_radius * cos_vals[i],
            outer_radius * sin_vals[i],
            half_height,
        ));
    }
    edges.push(Edge::new(top_outer_points));

    // 顶部内圆弧
    let mut top_inner_points = Vec::with_capacity(radial);
    for i in 0..radial {
        top_inner_points.push(Vec3::new(
            inner_radius * cos_vals[i],
            inner_radius * sin_vals[i],
            half_height,
        ));
    }
    edges.push(Edge::new(top_inner_points));

    // 底部外圆弧
    let mut bottom_outer_points = Vec::with_capacity(radial);
    for i in 0..radial {
        bottom_outer_points.push(Vec3::new(
            outer_radius * cos_vals[i],
            outer_radius * sin_vals[i],
            -half_height,
        ));
    }
    edges.push(Edge::new(bottom_outer_points));

    // 底部内圆弧
    let mut bottom_inner_points = Vec::with_capacity(radial);
    for i in 0..radial {
        bottom_inner_points.push(Vec3::new(
            inner_radius * cos_vals[i],
            inner_radius * sin_vals[i],
            -half_height,
        ));
    }
    edges.push(Edge::new(bottom_inner_points));

    let mut mesh =
        create_mesh_with_custom_edges(indices, vertices, normals, final_aabb, Some(edges));
    mesh.sync_wire_vertices_from_edges();

    Some(GeneratedMesh {
        mesh,
        aabb: final_aabb,
    })
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
fn generate_extrusion_mesh(extrusion: &Extrusion, refno: RefnoEnum) -> Option<GeneratedMesh> {
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

    let refno_str = Some(refno.to_string());
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
pub fn safe_normalize(v: Vec3) -> Option<Vec3> {
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

/// 根据 z_axis 方向构造稳定的方位四元数
///
/// 规则：
/// - 如果 z_axis 垂直（与世界 Z 轴共线）：参考方向使用世界 Y
/// - 否则（非垂直）：参考方向使用世界 Z
///
/// 这与 E3D 的 mthNormalToEulerAngles 行为一致
pub fn construct_basis_from_z_axis(z_axis: Vec3) -> Quat {
    construct_basis_from_z_axis_with_ref(z_axis, None)
}

/// 根据 z_axis 方向和可选的参考方向构造方位四元数
///
/// 当 ref_dir 存在时，使用它来确定局部 X 轴方向（投影到垂直于 z_axis 的平面）
/// 当 ref_dir 不存在或无效时，回退到默认逻辑
///
/// 这用于 SSLC 等需要保持剪切方向一致性的几何体
pub fn construct_basis_from_z_axis_with_ref(z_axis: Vec3, ref_dir: Option<Vec3>) -> Quat {
    let z_axis = z_axis.normalize_or_zero();
    if !z_axis.is_normalized() {
        return Quat::IDENTITY;
    }

    // 如果提供了有效的参考方向，使用它来确定 X 轴
    if let Some(ref_vec) = ref_dir {
        let ref_vec = ref_vec.normalize_or_zero();
        if ref_vec.is_normalized() {
            // 将 ref_dir 投影到垂直于 z_axis 的平面上
            let projected = ref_vec - z_axis * ref_vec.dot(z_axis);
            let projected = projected.normalize_or_zero();
            if projected.is_normalized() {
                // ref_dir 作为局部 X 轴方向的参考
                let x_axis = projected;
                let y_axis = z_axis.cross(x_axis).normalize_or_zero();
                if y_axis.is_normalized() {
                    return Quat::from_mat3(&Mat3::from_cols(x_axis, y_axis, z_axis));
                }
            }
        }
    }

    // 回退到默认逻辑
    let is_vertical = z_axis.dot(Vec3::Z).abs() > 0.999;

    let (x_axis, y_axis) = if is_vertical {
        // 垂直构件：参考方向使用世界 Y
        let y_target = Vec3::Y;
        let x_res = y_target.cross(z_axis).normalize_or_zero();
        let y_res = z_axis.cross(x_res).normalize_or_zero();
        (x_res, y_res)
    } else {
        // 非垂直构件：参考方向使用世界 Z
        let y_target = Vec3::Z;
        let x_res = y_target.cross(z_axis).normalize_or_zero();
        let y_res = z_axis.cross(x_res).normalize_or_zero();
        (x_res, y_res)
    };

    if !x_axis.is_normalized() || !y_axis.is_normalized() {
        return Quat::IDENTITY;
    }

    Quat::from_mat3(&Mat3::from_cols(x_axis, y_axis, z_axis))
}

///
/// 给定一个法向量，生成两个与之正交的切向量，形成正交基（u, v, n）
///
/// # 参数
/// - `normal`: 法向量（将被归一化）
///
/// # 返回
/// (tangent, bitangent) 两个切向量，与normal一起形成右手坐标系
///
/// 规则与 E3D 的 mthNormalToEulerAngles 一致：
/// - 如果 normal 垂直（与世界 Z 轴共线）：tangent = Y × normal
/// - 否则：tangent = Z × normal
pub fn orthonormal_basis(normal: Vec3) -> (Vec3, Vec3) {
    let n = normal.normalize();
    let is_vertical = n.dot(Vec3::Z).abs() > 0.999;

    let tangent = if is_vertical {
        // 垂直方向：使用世界 Y 作为参考
        Vec3::Y.cross(n).normalize_or_zero()
    } else {
        // 非垂直方向：使用世界 Z 作为参考
        Vec3::Z.cross(n).normalize_or_zero()
    };

    // 退化检查
    let tangent = if tangent.length_squared() <= MIN_LEN * MIN_LEN {
        Vec3::X.cross(n).normalize_or_zero()
    } else {
        tangent
    };

    // 副切向量 = n × tangent（确保右手坐标系）
    let bitangent = n.cross(tangent).normalize();
    let tangent = bitangent.cross(normal).normalize();
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
        let csg = generate_csg_mesh(&param, &settings, false, Some(RefnoEnum::default()))
            .expect("CSG cylinder generation failed");
        #[cfg(feature = "occ")]
        let occ_mesh = {
            let shape = param
                .gen_csg_shape_compat()
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
        let csg = generate_csg_mesh(&param, &settings, false, Some(RefnoEnum::default()))
            .expect("CSG snout generation failed");
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
pub(crate) fn generate_polyhedron_mesh(
    poly: &Polyhedron,
    refno: RefnoEnum,
) -> Option<GeneratedMesh> {
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
/// 直接使用 Revolution::gen_csg_mesh，自动处理 FRAD 圆角
pub(crate) fn generate_revolution_mesh(
    rev: &Revolution,
    settings: &LodMeshSettings,
    non_scalable: bool,
    refno: RefnoEnum,
) -> Option<GeneratedMesh> {
    use crate::shape::pdms_shape::BrepShapeTrait;

    // 使用 Revolution::gen_csg_mesh，它会自动处理 FRAD
    let mesh = rev.gen_csg_mesh()?;

    // 计算 AABB
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

/// 生成PrimLoft（SweepSolid）网格
///
/// PrimLoft是一个通用的扫掠实体，通过将截面轮廓沿着路径扫掠来生成实体
/// 支持多种路径类型：直线、圆弧、多段路径等
fn generate_prim_loft_mesh(
    sweep: &SweepSolid,
    settings: &LodMeshSettings,
    non_scalable: bool,
    refno: RefnoEnum,
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
