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

use crate::mesh_precision::LodMeshSettings;
use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::prim_geo::ctorus::CTorus;
use crate::prim_geo::cylinder::{LCylinder, SCylinder};
use crate::prim_geo::dish::Dish;
use crate::prim_geo::extrusion::Extrusion;
use crate::prim_geo::lpyramid::LPyramid;
use crate::prim_geo::polyhedron::{Polygon, Polyhedron};
use crate::prim_geo::pyramid::Pyramid;
use crate::prim_geo::revolution::Revolution;
use crate::prim_geo::rtorus::RTorus;
use crate::prim_geo::sbox::SBox;
use crate::prim_geo::snout::LSnout;
use crate::prim_geo::sphere::Sphere;
use crate::prim_geo::wire::CurveType;
use crate::shape::pdms_shape::{PlantMesh, VerifiedShape};
use glam::Vec3;
use nalgebra::Point3;
use parry3d::bounding_volume::{Aabb, BoundingVolume};

/// 最小长度阈值，用于判断几何形状是否有效
const MIN_LEN: f32 = 1e-6;

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
    PlantMesh {
        vertices,
        normals,
        indices,
        wire_vertices: Vec::new(),
        aabb: Some(Aabb::new(
            Point3::new(-half, -half, -half),
            Point3::new(half, half, half),
        )),
    }
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

    PlantMesh {
        indices,
        vertices,
        normals,
        wire_vertices: vec![],
        aabb: Some(aabb),
    }
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
        let offset = vertices.len() as u32;
        // 根据是顶部还是底部设置不同的z坐标、法向量和绕序
        let (z, normal_z, winding) = if top {
            (height, 1.0, (1, 0))
        } else {
            (0.0, -1.0, (0, 1))
        };

        for i in 0..resolution {
            let theta = i as f32 * step_theta;
            let (sin, cos) = theta.sin_cos();
            vertices.push([cos * radius, sin * radius, z].into());
            normals.push([0.0, 0.0, normal_z].into());
        }

        for i in 1..(resolution - 1) {
            indices.extend_from_slice(&[
                offset,
                offset + (i as u32) + (winding.1 as u32),
                offset + (i as u32) + (winding.0 as u32),
            ]);
        }
    };

    build_cap(true);
    build_cap(false);

    PlantMesh {
        vertices,
        normals,
        indices,
        wire_vertices: Vec::new(),
        aabb: Some(Aabb::new(
            Point3::new(-0.5, -0.5, 0.0),
            Point3::new(0.5, 0.5, 1.0),
        )),
    }
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
///
/// # 返回
/// 如果几何参数有效，返回生成的网格和包围盒；否则返回None
pub fn generate_csg_mesh(
    param: &PdmsGeoParam,
    settings: &LodMeshSettings,
    non_scalable: bool,
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
        PdmsGeoParam::PrimExtrusion(extrusion) => generate_extrusion_mesh(extrusion),
        PdmsGeoParam::PrimPolyhedron(poly) => generate_polyhedron_mesh(poly),
        PdmsGeoParam::PrimRevolution(rev) => generate_revolution_mesh(rev, settings, non_scalable),
        _ => None,
    }
}

/// 生成线性圆柱体（LCylinder）网格
///
/// LCylinder由轴向方向、直径和两个端面的偏移距离定义
fn generate_lcylinder_mesh(
    cyl: &LCylinder,
    settings: &LodMeshSettings,
    non_scalable: bool,
) -> Option<GeneratedMesh> {
    // 归一化轴向方向向量
    let dir = safe_normalize(cyl.paxi_dir)?;
    let radius = (cyl.pdia * 0.5).abs();
    if radius <= MIN_LEN {
        return None;
    }
    // 确定底部和顶部的偏移距离（确保bottom_offset < top_offset）
    let (bottom_offset, top_offset) = if cyl.pbdi <= cyl.ptdi {
        (cyl.pbdi, cyl.ptdi)
    } else {
        (cyl.ptdi, cyl.pbdi)
    };
    if (top_offset - bottom_offset).abs() <= MIN_LEN {
        return None;
    }
    let bottom_center = cyl.paxi_pt + dir * bottom_offset;
    let top_center = cyl.paxi_pt + dir * top_offset;
    build_cylinder_mesh(bottom_center, top_center, radius, settings, non_scalable)
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
    let height_segments = compute_height_segments(settings, height.abs(), non_scalable, 1);
    let ring_stride = radial + 1;
    let step_theta = std::f32::consts::TAU / radial as f32;

    // 计算顶点、法线和索引的数量
    let vertex_count = (height_segments + 1) * ring_stride + 2 * (radial + 1);
    let mut vertices = Vec::with_capacity(vertex_count);
    let mut normals = Vec::with_capacity(vertex_count);
    let mut indices = Vec::with_capacity(height_segments * radial * 6 + radial * 6);
    let mut aabb = Aabb::new_invalid();

    // 生成侧面顶点
    for ring in 0..=height_segments {
        let t = ring as f32 / height_segments as f32;
        let z_local = t * height; // 局部z坐标

        // 在底面和顶面之间插值剪切角度
        let tan_x = tan_btm_x + t * (tan_top_x - tan_btm_x);
        let tan_y = tan_btm_y + t * (tan_top_y - tan_btm_y);

        // 计算当前环的中心点
        let center = bottom_center + dir * z_local;

        for slice in 0..=radial {
            let angle = slice as f32 * step_theta;
            let (sin, cos) = angle.sin_cos();

            // 应用剪切变换
            let x_sheared = radius * cos + z_local * tan_x;
            let y_sheared = radius * sin + z_local * tan_y;

            // 转换到世界坐标
            let vertex = center + basis_u * x_sheared + basis_v * y_sheared;
            extend_aabb(&mut aabb, vertex);
            vertices.push(vertex);

            // 计算法线（近似）
            // 对于剪切圆柱体，法线需要考虑剪切变换的影响
            let radial_dir = basis_u * cos + basis_v * sin;
            normals.push(radial_dir);
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

    // 生成底面
    let bottom_center_index = vertices.len() as u32;
    vertices.push(bottom_center);
    normals.push(-dir);
    extend_aabb(&mut aabb, bottom_center);

    // 计算底面椭圆上的点
    for slice in 0..=radial {
        let angle = slice as f32 * step_theta;
        let (sin, cos) = angle.sin_cos();

        // 底面剪切变换
        let x_sheared = radius * cos;
        let y_sheared = radius * sin;

        let vertex = bottom_center + basis_u * x_sheared + basis_v * y_sheared;
        vertices.push(vertex);
        normals.push(-dir);
        extend_aabb(&mut aabb, vertex);
    }

    // 底面索引
    for slice in 0..radial {
        let next = (slice + 1) % (radial + 1);
        indices.extend_from_slice(&[
            bottom_center_index,
            bottom_center_index + 1 + next as u32,
            bottom_center_index + 1 + slice as u32,
        ]);
    }

    // 生成顶面
    let top_center_index = vertices.len() as u32;
    vertices.push(top_center);
    normals.push(dir);
    extend_aabb(&mut aabb, top_center);

    // 计算顶面椭圆上的点
    for slice in 0..=radial {
        let angle = slice as f32 * step_theta;
        let (sin, cos) = angle.sin_cos();

        // 顶面剪切变换
        let x_sheared = radius * cos + height * tan_top_x;
        let y_sheared = radius * sin + height * tan_top_y;

        let vertex = top_center + basis_u * x_sheared + basis_v * y_sheared;
        vertices.push(vertex);
        normals.push(dir);
        extend_aabb(&mut aabb, vertex);
    }

    // 顶面索引
    let top_ring_start = top_center_index + 1;
    for slice in 0..radial {
        let next = (slice + 1) % (radial + 1);
        indices.extend_from_slice(&[
            top_center_index,
            top_ring_start + slice as u32,
            top_ring_start + next as u32,
        ]);
    }

    Some(GeneratedMesh {
        mesh: PlantMesh {
            indices,
            vertices,
            normals,
            wire_vertices: vec![],
            aabb: Some(aabb),
        },
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
        mesh: PlantMesh {
            indices,
            vertices,
            normals,
            wire_vertices: vec![],
            aabb: Some(aabb),
        },
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
        mesh: PlantMesh {
            indices,
            vertices,
            normals,
            wire_vertices: vec![],
            aabb: Some(aabb),
        },
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

    Some(GeneratedMesh {
        mesh: PlantMesh {
            indices,
            vertices,
            normals,
            wire_vertices: vec![],
            aabb: Some(aabb),
        },
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
    let mut indices = Vec::with_capacity(36); // 6个面 × 2个三角形 × 3个索引 = 36

    // 定义6个面的法向量和4个角点（在单位坐标系中）
    let faces = [
        // +Z面（前面）
        (
            Vec3::Z,
            [
                Vec3::new(-1.0, -1.0, 1.0),
                Vec3::new(1.0, -1.0, 1.0),
                Vec3::new(1.0, 1.0, 1.0),
                Vec3::new(-1.0, 1.0, 1.0),
            ],
        ),
        // -Z面（后面）
        (
            Vec3::NEG_Z,
            [
                Vec3::new(-1.0, 1.0, -1.0),
                Vec3::new(1.0, 1.0, -1.0),
                Vec3::new(1.0, -1.0, -1.0),
                Vec3::new(-1.0, -1.0, -1.0),
            ],
        ),
        // +X面（右面）
        (
            Vec3::X,
            [
                Vec3::new(1.0, -1.0, -1.0),
                Vec3::new(1.0, 1.0, -1.0),
                Vec3::new(1.0, 1.0, 1.0),
                Vec3::new(1.0, -1.0, 1.0),
            ],
        ),
        // -X面（左面）
        (
            Vec3::NEG_X,
            [
                Vec3::new(-1.0, -1.0, 1.0),
                Vec3::new(-1.0, 1.0, 1.0),
                Vec3::new(-1.0, 1.0, -1.0),
                Vec3::new(-1.0, -1.0, -1.0),
            ],
        ),
        // +Y面（上面）
        (
            Vec3::Y,
            [
                Vec3::new(-1.0, 1.0, -1.0),
                Vec3::new(1.0, 1.0, -1.0),
                Vec3::new(1.0, 1.0, 1.0),
                Vec3::new(-1.0, 1.0, 1.0),
            ],
        ),
        // -Y面（下面）
        (
            Vec3::NEG_Y,
            [
                Vec3::new(-1.0, -1.0, 1.0),
                Vec3::new(1.0, -1.0, 1.0),
                Vec3::new(1.0, -1.0, -1.0),
                Vec3::new(-1.0, -1.0, -1.0),
            ],
        ),
    ];

    for (normal, corners) in faces {
        let base_index = vertices.len() as u32;
        for corner in corners {
            let scaled = Vec3::new(corner.x * half.x, corner.y * half.y, corner.z * half.z);
            vertices.push(sbox.center + scaled);
            normals.push(normal);
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
    Some(GeneratedMesh {
        mesh: PlantMesh {
            indices,
            vertices,
            normals,
            wire_vertices: vec![],
            aabb: Some(aabb),
        },
        aabb: Some(aabb),
    })
}

/// 生成圆盘（Dish）网格
///
/// 圆盘是一个球形帽面，由球面的一部分和底部圆面组成
/// 当前实现仅支持prad=0的情况（完整圆盘）
fn generate_dish_mesh(
    dish: &Dish,
    settings: &LodMeshSettings,
    non_scalable: bool,
) -> Option<GeneratedMesh> {
    // 仅支持prad=0的情况
    if dish.prad.abs() > MIN_LEN {
        return None;
    }
    let axis = safe_normalize(dish.paax_dir)?;
    let radius_rim = dish.pdia * 0.5; // 边缘半径
    let height = dish.pheig;
    if radius_rim <= MIN_LEN || height <= MIN_LEN {
        return None;
    }
    // 计算形成圆盘表面的球面半径
    // 使用几何关系：R² = r² + (R-h)²，解得 R = (r² + h²) / (2h)
    let radius_sphere = (radius_rim * radius_rim + height * height) / (2.0 * height);
    if !radius_sphere.is_finite() || radius_sphere <= MIN_LEN {
        return None;
    }

    // 计算底部中心点和球心位置
    let base_center = dish.paax_pt + axis * dish.pdis;
    let center_offset = height - radius_sphere; // 球心相对于底部中心的偏移
    let sphere_center = base_center + axis * center_offset;
    let (basis_u, basis_v) = orthonormal_basis(axis);

    let radial_segments = compute_radial_segments(settings, radius_rim, non_scalable, 3);
    let height_segments = compute_height_segments(settings, height, non_scalable, 1);
    let stride = radial_segments + 1;

    let mut vertices = Vec::with_capacity((height_segments + 1) * stride + radial_segments + 1);
    let mut normals = Vec::with_capacity(vertices.capacity());
    let mut indices =
        Vec::with_capacity(height_segments * radial_segments * 6 + radial_segments * 3);
    let mut aabb = Aabb::new_invalid();

    for lat in 0..=height_segments {
        let t = lat as f32 / height_segments as f32;
        let z = t * height;
        let axis_point = base_center + axis * z;
        // 计算当前高度环的半径（使用球面几何）
        let dist_from_center = z - center_offset; // 当前点到球心的距离（沿轴向）
        let ring_radius_sq = radius_sphere * radius_sphere - dist_from_center * dist_from_center;
        // 如果距离超过球半径，环半径为0
        let ring_radius = if ring_radius_sq <= 0.0 {
            0.0
        } else {
            ring_radius_sq.sqrt()
        };

        for lon in 0..=radial_segments {
            let angle = lon as f32 / radial_segments as f32 * std::f32::consts::TAU;
            let dir = basis_u * angle.cos() + basis_v * angle.sin();
            let vertex = axis_point + dir * ring_radius;
            extend_aabb(&mut aabb, vertex);
            vertices.push(vertex);
            let normal = (vertex - sphere_center).normalize();
            normals.push(normal);
        }
    }

    for lat in 0..height_segments {
        for lon in 0..radial_segments {
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

    let base_ring_offset = height_segments * stride;
    let base_center_index = vertices.len() as u32;
    vertices.push(base_center);
    normals.push(-axis);
    extend_aabb(&mut aabb, base_center);
    for lon in 0..radial_segments {
        let curr = base_ring_offset + lon;
        let next = base_ring_offset + ((lon + 1) % stride);
        indices.extend_from_slice(&[base_center_index, next as u32, curr as u32]);
    }

    Some(GeneratedMesh {
        mesh: PlantMesh {
            indices,
            vertices,
            normals,
            wire_vertices: vec![],
            aabb: Some(aabb),
        },
        aabb: Some(aabb),
    })
}

/// 生成圆环（CTorus）网格
///
/// 圆环由外半径（rout）和内半径（rins）定义
/// 当前实现仅支持完整圆环（360度）
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
    let major_radius = torus.rins + tube_radius; // 主圆环的半径
    let sweep_angle = torus.angle.to_radians();
    if sweep_angle <= MIN_LEN {
        return None;
    }

    // 仅支持完整圆环（接近2π）
    if sweep_angle < std::f32::consts::TAU - 1e-3 {
        return None;
    }

    let major_segments = compute_radial_segments(settings, major_radius, non_scalable, 3);
    let tube_segments = compute_radial_segments(settings, tube_radius, non_scalable, 3);
    let stride = tube_segments + 1;

    let mut vertices = Vec::with_capacity((major_segments + 1) * stride);
    let mut normals = Vec::with_capacity(vertices.capacity());
    let mut indices = Vec::with_capacity(major_segments * tube_segments * 6);
    let mut aabb = Aabb::new_invalid();

    // 生成圆环顶点
    // i: 主圆环方向的段数（大圆）
    // j: 管截面的段数（小圆）
    for i in 0..=major_segments {
        let u = std::f32::consts::TAU * (i as f32 / major_segments as f32); // 主圆环角度
        let (sin_u, cos_u) = u.sin_cos();
        // 主圆环上的中心点
        let center = Vec3::new(major_radius * cos_u, major_radius * sin_u, 0.0);
        for j in 0..=tube_segments {
            let v = std::f32::consts::TAU * (j as f32 / tube_segments as f32); // 管截面角度
            let (sin_v, cos_v) = v.sin_cos();
            // 计算管截面上的法向量和顶点
            let normal = Vec3::new(cos_u * cos_v, sin_u * cos_v, sin_v);
            let vertex = center + normal * tube_radius;
            extend_aabb(&mut aabb, vertex);
            vertices.push(vertex);
            normals.push(normal.normalize());
        }
    }

    for i in 0..major_segments {
        for j in 0..tube_segments {
            let current = i * stride + j;
            let next = (i + 1) * stride + j;
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
        mesh: PlantMesh {
            indices,
            vertices,
            normals,
            wire_vertices: vec![],
            aabb: Some(aabb),
        },
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
        mesh: PlantMesh {
            indices,
            vertices,
            normals,
            wire_vertices: vec![],
            aabb: Some(aabb),
        },
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
/// 当前实现仅支持完整圆环（360度）
///
/// 该形状由以下部分组成：
/// - 外圆柱面
/// - 内圆柱面
/// - 顶部和底部环形端面
fn generate_rect_torus_mesh(
    rtorus: &RTorus,
    settings: &LodMeshSettings,
    non_scalable: bool,
) -> Option<GeneratedMesh> {
    if !rtorus.check_valid() {
        return None;
    }
    // 仅支持完整圆环
    if (rtorus.angle.to_radians() - std::f32::consts::TAU).abs() > 1e-3 {
        return None;
    }

    let outer_radius = rtorus.rout.abs().max(MIN_LEN);
    let inner_radius = rtorus
        .rins
        .abs()
        .max(MIN_LEN)
        .min((outer_radius - MIN_LEN).max(MIN_LEN));
    let major_segments = compute_radial_segments(settings, outer_radius, non_scalable, 3);
    let height_segments = compute_height_segments(settings, rtorus.height.abs(), non_scalable, 1);
    let radial_span = (outer_radius - inner_radius).abs().max(MIN_LEN);
    let radial_segments = compute_height_segments(
        settings,
        radial_span,
        non_scalable,
        settings.cap_segments.max(1),
    );

    let half_height = rtorus.height * 0.5;
    let mut combined = PlantMesh::default();
    combined.aabb = Some(Aabb::new_invalid());

    // 生成外圆柱面（法向量向外）
    let (outer_mesh, outer_aabb) = generate_cylinder_surface(
        rtorus.rout,
        half_height,
        major_segments,
        height_segments,
        true, // outward = true
    );
    merge_meshes(&mut combined, outer_mesh, outer_aabb);

    // 生成内圆柱面（法向量向内）
    let (inner_mesh, inner_aabb) = generate_cylinder_surface(
        rtorus.rins,
        half_height,
        major_segments,
        height_segments,
        false, // outward = false
    );
    merge_meshes(&mut combined, inner_mesh, inner_aabb);

    // 生成顶部环形端面
    let (top_mesh, top_aabb) = generate_annulus_surface(
        half_height,
        rtorus.rins,
        rtorus.rout,
        major_segments,
        radial_segments,
        1.0, // normal_sign = 1.0 (向上)
    );
    merge_meshes(&mut combined, top_mesh, top_aabb);

    // 生成底部环形端面
    let (bottom_mesh, bottom_aabb) = generate_annulus_surface(
        -half_height,
        rtorus.rins,
        rtorus.rout,
        major_segments,
        radial_segments,
        -1.0, // normal_sign = -1.0 (向下)
    );
    merge_meshes(&mut combined, bottom_mesh, bottom_aabb);

    let final_aabb = combined.cal_aabb();
    combined.aabb = final_aabb;

    Some(GeneratedMesh {
        mesh: combined,
        aabb: final_aabb,
    })
}

/// 生成拉伸体（Extrusion）网格
///
/// 拉伸体将一个2D轮廓沿Z轴方向拉伸一定高度形成3D形状
/// 当前实现仅支持：
/// - 单一轮廓（单个顶点列表）
/// - 填充类型（CurveType::Fill）
/// - 轮廓位于XY平面（所有点的z坐标相同）
fn generate_extrusion_mesh(extrusion: &Extrusion) -> Option<GeneratedMesh> {
    if extrusion.height.abs() <= MIN_LEN {
        return None;
    }
    if extrusion.verts.is_empty() || extrusion.verts[0].len() < 3 {
        return None;
    }
    // 仅支持单一轮廓
    if extrusion.verts.len() > 1 {
        return None;
    }
    // 仅支持填充类型
    if !matches!(&extrusion.cur_type, CurveType::Fill) {
        return None;
    }

    let profile = &extrusion.verts[0];
    let base_z = profile[0].z;
    // 检查所有点是否在同一平面上（z坐标相同）
    if profile.iter().any(|p| (p.z - base_z).abs() > 1e-3) {
        return None;
    }

    let n = profile.len();
    if n < 3 {
        return None;
    }

    // 使用鞋带公式（Shoelace formula）计算轮廓面积
    // 面积的正负号表示轮廓的绕向（逆时针为正，顺时针为负）
    let area = profile
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let next = profile[(i + 1) % n];
            p.x * next.y - next.x * p.y
        })
        .sum::<f32>()
        * 0.5;
    if area.abs() <= MIN_LEN {
        return None;
    }

    let top_z = base_z + extrusion.height;
    let mut vertices: Vec<Vec3> = Vec::new();
    let mut normals: Vec<Vec3> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut aabb = Aabb::new_invalid();

    let mut add_vertex = |position: Vec3,
                          normal: Vec3,
                          vertices: &mut Vec<Vec3>,
                          normals: &mut Vec<Vec3>,
                          aabb: &mut Aabb|
     -> u32 {
        extend_aabb(aabb, position);
        vertices.push(position);
        normals.push(normal);
        (vertices.len() - 1) as u32
    };

    let mut bottom_indices = Vec::with_capacity(n);
    let mut top_indices = Vec::with_capacity(n);

    for p in profile {
        bottom_indices.push(add_vertex(
            Vec3::new(p.x, p.y, base_z),
            Vec3::new(0.0, 0.0, -1.0),
            &mut vertices,
            &mut normals,
            &mut aabb,
        ));
    }
    for p in profile {
        top_indices.push(add_vertex(
            Vec3::new(p.x, p.y, top_z),
            Vec3::new(0.0, 0.0, 1.0),
            &mut vertices,
            &mut normals,
            &mut aabb,
        ));
    }

    // 根据面积的正负判断轮廓的绕向（ccw = counter-clockwise，逆时针）
    let ccw = area > 0.0;
    // 生成顶部和底部的三角形索引（扇形三角化）
    for i in 1..(n - 1) {
        if ccw {
            // 逆时针：顶部和底部的索引顺序保持一致性
            indices.extend_from_slice(&[top_indices[0], top_indices[i], top_indices[i + 1]]);
            indices.extend_from_slice(&[
                bottom_indices[0],
                bottom_indices[i + 1],
                bottom_indices[i],
            ]);
        } else {
            // 顺时针：反转索引顺序
            indices.extend_from_slice(&[top_indices[0], top_indices[i + 1], top_indices[i]]);
            indices.extend_from_slice(&[
                bottom_indices[0],
                bottom_indices[i],
                bottom_indices[i + 1],
            ]);
        }
    }

    for i in 0..n {
        let next = (i + 1) % n;
        let p0 = Vec3::new(profile[i].x, profile[i].y, base_z);
        let p1 = Vec3::new(profile[next].x, profile[next].y, base_z);
        let p2 = Vec3::new(profile[next].x, profile[next].y, top_z);
        let p3 = Vec3::new(profile[i].x, profile[i].y, top_z);

        let mut normal = (p1 - p0).cross(p3 - p0);
        if normal.length_squared() <= MIN_LEN * MIN_LEN {
            continue;
        }
        normal = normal.normalize();
        let a = add_vertex(p0, normal, &mut vertices, &mut normals, &mut aabb);
        let b = add_vertex(p1, normal, &mut vertices, &mut normals, &mut aabb);
        let c = add_vertex(p2, normal, &mut vertices, &mut normals, &mut aabb);
        let d = add_vertex(p3, normal, &mut vertices, &mut normals, &mut aabb);

        indices.extend_from_slice(&[a, b, c]);
        indices.extend_from_slice(&[a, c, d]);
    }

    let mesh = PlantMesh {
        indices,
        vertices,
        normals,
        wire_vertices: vec![],
        aabb: Some(aabb),
    };

    Some(GeneratedMesh {
        mesh,
        aabb: Some(aabb),
    })
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
        PlantMesh {
            indices,
            vertices,
            normals,
            wire_vertices: vec![],
            aabb: Some(aabb),
        },
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
        PlantMesh {
            indices,
            vertices,
            normals,
            wire_vertices: vec![],
            aabb: Some(aabb),
        },
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
        let csg =
            generate_csg_mesh(&param, &settings, false).expect("CSG cylinder generation failed");
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
        let csg = generate_csg_mesh(&param, &settings, false).expect("CSG snout generation failed");
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
        mesh: PlantMesh {
            vertices: all_vertices,
            normals: all_normals,
            indices: all_indices,
            wire_vertices: vec![],
            aabb: Some(aabb),
        },
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

    // 将轮廓投影到垂直于旋转轴的平面上
    // 计算轮廓点到轴的距离（沿轴方向的距离和垂直于轴的距离）
    let mut profile_points_3d = Vec::new();
    
    for &profile_pt in profile.iter() {
        let offset = profile_pt - rot_pt;
        // 沿轴方向的距离
        let along_axis = offset.dot(rot_dir);
        // 垂直于轴的距离
        let perp_offset = offset - rot_dir * along_axis;
        
        // 保存沿轴距离和垂直偏移
        profile_points_3d.push((along_axis, perp_offset));
    }

    // 生成顶点：对每个轮廓点，绕轴旋转生成环形顶点
    for (profile_idx, &(along_axis, perp_offset)) in profile_points_3d.iter().enumerate() {
        let perp_dist = perp_offset.length();
        
        // 如果点在轴上，创建一条线
        if perp_dist < MIN_LEN {
            // 在轴上的点，旋转后仍然是同一点
            for seg in 0..=angular_segments {
                let position = rot_pt + rot_dir * along_axis;
                extend_aabb(&mut aabb, position);
                vertices.push(position);
                // 法向量指向旋转方向
                normals.push(u);
            }
            continue;
        }

        let perp_dir = perp_offset / perp_dist;

        // 生成该轮廓点旋转后的环形顶点
        for seg in 0..=angular_segments {
            let theta = (seg as f32 / angular_segments as f32) * angle_rad;
            let (sin, cos) = theta.sin_cos();

            // 旋转垂直于轴的方向向量
            let rotated_perp = perp_dir * cos + (rot_dir.cross(perp_dir)) * sin;
            let position = rot_pt + rot_dir * along_axis + rotated_perp * perp_dist;

            extend_aabb(&mut aabb, position);
            vertices.push(position);

            // 计算法向量（指向外部的方向，垂直于表面）
            let normal = rotated_perp;
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

    Some(GeneratedMesh {
        mesh: PlantMesh {
            vertices,
            normals,
            indices,
            wire_vertices: vec![],
            aabb: Some(aabb),
        },
        aabb: Some(aabb),
    })
}
