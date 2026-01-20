use std::collections::HashMap;
use std::mem;
use std::path::Path;

use crate::shape::pdms_shape::{Edge, Edges, PlantMesh};
use crate::tool::float_tool::*;
use glam::{DMat4, Vec2, Vec3};
use manifold_rs::{Manifold, Mesh};

/// 布尔操作类型
#[derive(Clone, Copy, Debug)]
pub enum ManifoldOpType {
    Union,
    Intersection,
    Difference,
}

/// 2D 截面，用于生成 3D 实体
pub struct ManifoldCrossSectionRust {
    /// 多边形数据，格式为 [x0, y0, x1, y1, ...]
    polygon_data: Vec<f64>,
}

impl ManifoldCrossSectionRust {
    /// 根据 2d 的点生成 ManifoldCrossSectionRust
    pub fn from_points(pts: &[Vec2]) -> Self {
        let mut polygon_data = Vec::with_capacity(pts.len() * 2);
        for p in pts {
            polygon_data.push(p.x as f64);
            polygon_data.push(p.y as f64);
        }
        Self { polygon_data }
    }

    /// 拉伸成 manifold
    pub fn extrude(&self, height: f32, _slices: u32) -> ManifoldRust {
        let polygon_slice: &[f64] = &self.polygon_data;
        let multi_polygon: &[&[f64]] = &[polygon_slice];
        let manifold = Manifold::extrude(multi_polygon, height as f64, 1, 0.0, 1.0, 1.0);
        ManifoldRust { inner: manifold }
    }

    /// 旋转成 manifold
    pub fn extrude_rotate(&self, segments: i32, angle: f32) -> ManifoldRust {
        let polygon_slice: &[f64] = &self.polygon_data;
        let multi_polygon: &[&[f64]] = &[polygon_slice];
        let manifold = Manifold::revolve(multi_polygon, segments as u32, angle as f64);
        ManifoldRust { inner: manifold }
    }
}

/// Manifold 的 Rust 封装
pub struct ManifoldRust {
    pub inner: Manifold,
}

unsafe impl Send for ManifoldRust {}

impl Clone for ManifoldRust {
    fn clone(&self) -> Self {
        // manifold-rs 的 Manifold 不支持 Clone，需要通过 mesh 转换
        let mesh = self.inner.to_mesh();
        let vertices = mesh.vertices();
        let indices = mesh.indices();
        let new_mesh = Mesh::new(&vertices, &indices);
        Self {
            inner: new_mesh.to_manifold(),
        }
    }
}

impl ManifoldRust {
    pub fn new() -> Self {
        // 使用极小的 cube 替代 empty()，因为 manifold-rs 没有 empty() 方法
        Self {
            inner: Manifold::cube(1e-10, 1e-10, 1e-10),
        }
    }

    pub fn convert_to_manifold(plant_mesh: PlantMesh, mat4: DMat4, more_precsion: bool) -> Self {
        Self::from_mesh(&ManifoldMeshRust::convert_to_manifold_mesh(
            plant_mesh,
            mat4,
            more_precsion,
        ))
    }

    /// 从 GLB 文件直接转换为 Manifold
    ///
    /// 注意：GLB 文件中的网格应该已经在 CSG 生成阶段通过 weld_vertices_for_manifold
    /// 保证了流形性，这里只需要应用变换矩阵，不再做顶点焊接。
    pub fn import_glb_to_manifold(
        path: &Path,
        mat4: DMat4,
        more_precision: bool,
    ) -> anyhow::Result<Self> {
        let (document, buffers, _) = gltf::import(path)?;

        let mut all_vertices: Vec<f32> = Vec::new();
        let mut all_indices: Vec<u32> = Vec::new();

        let mut vertex_offset = 0u32;
        for mesh in document.meshes() {
            for primitive in mesh.primitives() {
                let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

                // 读取顶点并应用变换矩阵
                if let Some(iter) = reader.read_positions() {
                    for v in iter {
                        let pt = mat4.transform_point3(glam::DVec3::new(
                            v[0] as f64,
                            v[1] as f64,
                            v[2] as f64,
                        ));
                        all_vertices.push(pt.x as f32);
                        all_vertices.push(pt.y as f32);
                        all_vertices.push(pt.z as f32);
                    }
                }

                // 读取索引并调整偏移量
                if let Some(iter) = reader.read_indices() {
                    let indices: Vec<u32> = iter.into_u32().collect();
                    let vertex_count = (all_vertices.len() / 3) as u32 - vertex_offset;

                    for &idx in &indices {
                        all_indices.push(vertex_offset + idx);
                    }

                    vertex_offset += vertex_count;
                }
            }
        }

        if all_vertices.is_empty() || all_indices.is_empty() {
            return Ok(Self::new());
        }

        // 关键：GLB 中的网格不保证是“共享顶点拓扑”，需要在这里做顶点焊接，
        // 否则 Manifold::to_manifold 可能输出 0 三角形（典型：BOX 类 24 顶点/12 三角形的 per-face mesh）。
        let build_welded = |precision: f64| -> (Vec<f32>, Vec<u32>) {
            let mut map: HashMap<(i64, i64, i64), u32> = HashMap::new();
            let mut remap: Vec<u32> = Vec::with_capacity(all_vertices.len() / 3);
            let mut welded_vertices: Vec<f32> = Vec::new();

            for i in (0..all_vertices.len()).step_by(3) {
                let x = all_vertices[i] as f64;
                let y = all_vertices[i + 1] as f64;
                let z = all_vertices[i + 2] as f64;
                let key = ManifoldMeshRust::quantize_vertex(x, y, z, precision);
                if let Some(&idx) = map.get(&key) {
                    remap.push(idx);
                    continue;
                }
                let idx = (welded_vertices.len() / 3) as u32;
                map.insert(key, idx);
                remap.push(idx);
                welded_vertices.push(x as f32);
                welded_vertices.push(y as f32);
                welded_vertices.push(z as f32);
            }

            let mut welded_indices: Vec<u32> = Vec::with_capacity(all_indices.len());
            for tri in all_indices.chunks(3) {
                if tri.len() != 3 {
                    continue;
                }
                let a = remap[tri[0] as usize];
                let b = remap[tri[1] as usize];
                let c = remap[tri[2] as usize];
                if a == b || b == c || a == c {
                    continue;
                }
                welded_indices.extend_from_slice(&[a, b, c]);
            }

            (welded_vertices, welded_indices)
        };

        // 估算自适应精度（基于当前已应用 mat4 的坐标）
        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;
        let mut min_z = f32::MAX;
        let mut max_z = f32::MIN;
        for i in (0..all_vertices.len()).step_by(3) {
            let x = all_vertices[i];
            let y = all_vertices[i + 1];
            let z = all_vertices[i + 2];
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
            min_z = min_z.min(z);
            max_z = max_z.max(z);
        }
        let extent_x = (max_x - min_x).abs();
        let extent_y = (max_y - min_y).abs();
        let extent_z = (max_z - min_z).abs();
        let min_extent = extent_x.min(extent_y).min(extent_z);

        let mut precision: f64 = if min_extent < 0.1 {
            100000.0
        } else if min_extent < 1.0 {
            10000.0
        } else if min_extent < 10.0 {
            1000.0
        } else if min_extent < 100.0 {
            100.0
        } else {
            10.0
        };
        if more_precision {
            precision = (precision * 1000.0).min(1_000_000_000.0);
        }

        let input_triangles = all_indices.len() / 3;
        let (mut vertices, mut indices) = build_welded(precision);
        if input_triangles > 0 && indices.is_empty() {
            let retry_precision = (precision * 1000.0).min(1_000_000_000.0);
            (vertices, indices) = build_welded(retry_precision);
        }

        Ok(Self::from_mesh(&ManifoldMeshRust { vertices, indices }))
    }

    /// 导出到 GLB 文件
    ///
    /// 注意：导出时会将 manifold mesh（共享顶点）转换为普通 mesh（重复顶点），
    /// 以保证渲染时边缘轮廓清晰（每个面有独立的法线）
    pub fn export_to_glb(&self, path: &Path) -> anyhow::Result<()> {
        // 使用 From<&ManifoldRust> for PlantMesh 转换，
        // 该转换会将共享顶点的 manifold mesh 转为每个三角形独立顶点的普通 mesh
        let plant_mesh: PlantMesh = self.into();

        if plant_mesh.vertices.is_empty() || plant_mesh.indices.is_empty() {
            return Err(anyhow::anyhow!("布尔运算结果为空，无法导出"));
        }

        // 确保父目录存在
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        crate::fast_model::export_model::export_glb::export_single_mesh_to_glb(&plant_mesh, path)?;
        Ok(())
    }

    /// 导出到 OBJ 文件（用于调试）
    ///
    /// 注意：导出时会将 manifold mesh（共享顶点）转换为普通 mesh（重复顶点），
    /// 以保证渲染时边缘轮廓清晰（每个面有独立的法线）
    pub fn export_to_obj(&self, path_str: &str) -> anyhow::Result<()> {
        // 使用 From<&ManifoldRust> for PlantMesh 转换
        let plant_mesh: PlantMesh = self.into();

        if plant_mesh.vertices.is_empty() || plant_mesh.indices.is_empty() {
            return Err(anyhow::anyhow!("布尔运算结果为空，无法导出"));
        }

        // 确保父目录存在
        let path = Path::new(path_str);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        plant_mesh
            .export_obj(false, path_str)
            .map_err(|e| anyhow::anyhow!(e))
    }

    pub fn from_mesh(m: &ManifoldMeshRust) -> Self {
        let input_tri_count = m.indices.len() / 3;
        let input_vert_count = m.vertices.len() / 3;

        // 如果输入为空，返回空 manifold
        if m.indices.is_empty() || m.vertices.is_empty() {
            eprintln!("[Manifold] 输入 mesh 为空，跳过转换");
            return Self::new();
        }

        let mesh = Mesh::new(&m.vertices, &m.indices);
        let result = Self {
            inner: mesh.to_manifold(),
        };

        // 检查转换结果
        let result_mesh = result.inner.to_mesh();
        let output_tri_count = result_mesh.indices().len() / 3;

        if output_tri_count == 0 && input_tri_count > 0 {
            // Manifold 转换失败，输出诊断信息
            eprintln!(
                "[Manifold] ⚠️ 转换后三角形丢失: 输入 {} 顶点 {} 三角形 -> 输出 0 三角形",
                input_vert_count, input_tri_count
            );

            // 检查 mesh 是否有问题（如退化三角形、非流形等）
            // 计算 AABB 来诊断几何范围
            let mut min_x = f32::MAX;
            let mut max_x = f32::MIN;
            let mut min_y = f32::MAX;
            let mut max_y = f32::MIN;
            let mut min_z = f32::MAX;
            let mut max_z = f32::MIN;

            for i in (0..m.vertices.len()).step_by(3) {
                let x = m.vertices[i];
                let y = m.vertices[i + 1];
                let z = m.vertices[i + 2];
                min_x = min_x.min(x);
                max_x = max_x.max(x);
                min_y = min_y.min(y);
                max_y = max_y.max(y);
                min_z = min_z.min(z);
                max_z = max_z.max(z);
            }

            let extent_x = max_x - min_x;
            let extent_y = max_y - min_y;
            let extent_z = max_z - min_z;

            eprintln!(
                "[Manifold] AABB: ({:.2}, {:.2}, {:.2}) -> ({:.2}, {:.2}, {:.2}), 范围: ({:.2}, {:.2}, {:.2})",
                min_x, min_y, min_z, max_x, max_y, max_z, extent_x, extent_y, extent_z
            );

            // 检查是否有极端的长宽比
            let min_extent = extent_x.min(extent_y).min(extent_z);
            let max_extent = extent_x.max(extent_y).max(extent_z);
            if min_extent > 0.0 && max_extent / min_extent > 100.0 {
                eprintln!(
                    "[Manifold] ⚠️ 极端长宽比: {:.1} (最小维度 {:.4}, 最大维度 {:.2})",
                    max_extent / min_extent,
                    min_extent,
                    max_extent
                );
            }
        }

        result
    }

    pub fn get_mesh(&self) -> ManifoldMeshRust {
        let mesh = self.inner.to_mesh();
        let prop_num = mesh.num_props() as usize;
        let raw_vertices = mesh.vertices();
        let indices = mesh.indices();

        // manifold-rs 的 mesh 顶点可能包含额外属性（num_props > 3），这里统一压缩为 xyz 三分量，
        // 避免下游（AABB/导出）误把属性当作位置坐标。
        let mut vertices: Vec<f32> = Vec::new();
        if prop_num >= 3 && !raw_vertices.is_empty() {
            let vert_num = raw_vertices.len() / prop_num;
            vertices.reserve(vert_num * 3);
            for i in 0..vert_num {
                let base = prop_num * i;
                vertices.push(raw_vertices[base + 0]);
                vertices.push(raw_vertices[base + 1]);
                vertices.push(raw_vertices[base + 2]);
            }
        }

        ManifoldMeshRust { vertices, indices }
    }

    /// 不支持 subtract
    pub fn batch_boolean(batch: &[Self], op: ManifoldOpType) -> Self {
        if batch.is_empty() {
            return Self::new();
        }

        let mut result = batch[0].clone();
        for b in batch.iter().skip(1) {
            result.inner = match op {
                ManifoldOpType::Union => result.inner.union(&b.inner),
                ManifoldOpType::Intersection => result.inner.intersection(&b.inner),
                ManifoldOpType::Difference => result.inner.difference(&b.inner),
            };
        }
        result
    }

    pub fn batch_boolean_subtract(&self, negs: &[Self]) -> Self {
        if negs.is_empty() {
            return self.clone();
        }

        let mut result = self.clone();
        for b in negs.iter() {
            result.inner = result.inner.difference(&b.inner);
        }
        result
    }

    pub fn destroy(&self) {
        // manifold-rs 使用 RAII，无需手动释放
    }
}

/// Mesh 的 Rust 封装
pub struct ManifoldMeshRust {
    pub vertices: Vec<f32>,
    pub indices: Vec<u32>,
}

impl ManifoldMeshRust {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    /// 计算适应性精度因子
    ///
    /// 根据几何体的尺寸范围选择合适的精度：
    /// - 小尺寸几何体（如单位化的扫掠体）使用更高精度
    /// - 大尺寸几何体使用较低精度以避免数值问题
    pub fn compute_adaptive_precision(vertices: &[glam::Vec3], mat4: &glam::DMat4) -> f32 {
        let mut min = glam::DVec3::new(f64::MAX, f64::MAX, f64::MAX);
        let mut max = glam::DVec3::new(f64::MIN, f64::MIN, f64::MIN);

        for v in vertices {
            let tv = mat4.transform_point3(v.as_dvec3());
            min = min.min(tv);
            max = max.max(tv);
        }

        let extent = max - min;
        let min_extent = extent.x.min(extent.y).min(extent.z);

        // Adjust precision based on min_extent
        if min_extent < 0.1 {
            100000.0 // High precision for very small geometries
        } else if min_extent < 1.0 {
            10000.0 // High precision for small geometries
        } else if min_extent < 10.0 {
            1000.0 // Medium precision
        } else if min_extent < 100.0 {
            100.0 // Lower precision
        } else {
            10.0
        }
    }

    /// 计算网格的 AABB
    pub fn cal_aabb(&self) -> Option<parry3d::bounding_volume::Aabb> {
        use parry3d::bounding_volume::BoundingVolume;
        if self.vertices.is_empty() {
            return None;
        }
        let mut aabb = parry3d::bounding_volume::Aabb::new_invalid();
        for chunk in self.vertices.chunks_exact(3) {
            aabb.take_point(parry3d::math::Point::new(chunk[0], chunk[1], chunk[2]));
        }

        let mins = glam::Vec3::from(aabb.mins);
        let maxs = glam::Vec3::from(aabb.maxs);
        if !mins.is_finite() || !maxs.is_finite() {
            return None;
        }
        let ext_mag = aabb.extents().magnitude();
        if !ext_mag.is_finite() || ext_mag <= 0.0 {
            return None;
        }
        Some(aabb)
    }

    /// 将顶点坐标量化为整数键（用于顶点焊接）
    fn quantize_vertex(x: f64, y: f64, z: f64, precision: f64) -> (i64, i64, i64) {
        (
            (x * precision).round() as i64,
            (y * precision).round() as i64,
            (z * precision).round() as i64,
        )
    }

    /// 将 PlantMesh 转换为 ManifoldMeshRust
    ///
    /// 注意：PlantMesh 应该已经在 CSG 生成阶段通过 weld_vertices_for_manifold
    /// 保证了流形性，这里只需要应用变换矩阵并转换数据格式。
    pub fn convert_to_manifold_mesh(
        mut plant_mesh: PlantMesh,
        mat4: DMat4,
        ceil_or_trunc: bool,
    ) -> Self {
        let vertices = mem::take(&mut plant_mesh.vertices);
        let indices = plant_mesh.indices;

        if vertices.is_empty() || indices.is_empty() {
            return Self::new();
        }

        let _ = ceil_or_trunc;

        // 关键：对“非共享顶点”网格做顶点焊接（Manifold 需要共享拓扑）。
        // 使用自适应量化精度，避免因坐标尺度不同导致过度/不足合并。
        let base_precision = Self::compute_adaptive_precision(&vertices, &mat4) as f64;

        let build = |precision: f64| -> (Vec<f32>, Vec<u32>) {
            let mut map: HashMap<(i64, i64, i64), u32> = HashMap::new();
            let mut remap: Vec<u32> = Vec::with_capacity(vertices.len());
            let mut welded_vertices: Vec<f32> = Vec::new();

            for v in &vertices {
                let pt =
                    mat4.transform_point3(glam::DVec3::new(v.x as f64, v.y as f64, v.z as f64));
                let key = Self::quantize_vertex(pt.x, pt.y, pt.z, precision);
                if let Some(&idx) = map.get(&key) {
                    remap.push(idx);
                    continue;
                }
                let idx = (welded_vertices.len() / 3) as u32;
                map.insert(key, idx);
                remap.push(idx);
                welded_vertices.push(pt.x as f32);
                welded_vertices.push(pt.y as f32);
                welded_vertices.push(pt.z as f32);
            }

            let mut welded_indices: Vec<u32> = Vec::with_capacity(indices.len());
            for tri in indices.chunks(3) {
                if tri.len() != 3 {
                    continue;
                }
                let a = remap[tri[0] as usize];
                let b = remap[tri[1] as usize];
                let c = remap[tri[2] as usize];
                if a == b || b == c || a == c {
                    continue;
                }
                welded_indices.extend_from_slice(&[a, b, c]);
            }

            (welded_vertices, welded_indices)
        };

        let input_triangles = indices.len() / 3;
        let (mut transformed_vertices, mut welded_indices) = build(base_precision);
        if input_triangles > 0 && welded_indices.is_empty() {
            // 退化保护：量化过粗时会把“薄壁/小三角形”合并塌陷成退化三角形，导致全被过滤。
            // 这里提高精度重试（更细的网格单位）。
            let retry_precision = (base_precision * 1000.0).min(1_000_000_000.0);
            (transformed_vertices, welded_indices) = build(retry_precision);
        }

        Self {
            vertices: transformed_vertices,
            indices: welded_indices,
        }
    }
}
//负实体的模型应该更大一些
//正实体的模型更小一些

// impl From<(&PlantMesh, &DMat4)> for ManifoldMeshRust {
//     fn from(c: (&PlantMesh, &DMat4)) -> Self {
//         let m = c.0;
//         let t = c.1;
//         unsafe {
//             let mesh = ManifoldMeshRust::new();
//             let mut verts: Vec<f32> = Vec::with_capacity(m.vertices.len() * 3);
//             for v in m.vertices.clone() {
//                 let pt = t.transform_point3(glam::DVec3::from(v));
//                 // verts.push(f64_round_3(pt[0]) as _);
//                 // verts.push(f64_round_3(pt[1]) as _);
//                 // verts.push(f64_round_3(pt[2]) as _);
//
//                 verts.push(f64_round_1(pt[0]) as _);
//                 verts.push(f64_round_1(pt[1]) as _);
//                 verts.push(f64_round_1(pt[2]) as _);
//             }
//             manifold_meshgl(mesh.ptr as _,
//                             verts.as_ptr() as _, m.vertices.len(), 3,
//                             m.indices.as_ptr() as _, m.indices.len() / 3);
//             mesh
//         }
//     }
// }

impl From<&PlantMesh> for ManifoldMeshRust {
    fn from(m: &PlantMesh) -> Self {
        let mut verts = Vec::with_capacity(m.vertices.len() * 3);
        //todo 是否要根据包围盒的大小来判断用哪个等级的round
        for v in m.vertices.clone() {
            verts.push(f32_round_1(v[0]));
            verts.push(f32_round_1(v[1]));
            verts.push(f32_round_1(v[2]));
        }
        Self {
            vertices: verts,
            indices: m.indices.clone(),
        }
    }
}

impl From<&PlantMesh> for ManifoldRust {
    fn from(m: &PlantMesh) -> Self {
        let mesh: ManifoldMeshRust = m.into();
        Self::from_mesh(&mesh)
    }
}

impl From<PlantMesh> for ManifoldRust {
    fn from(m: PlantMesh) -> Self {
        let mesh: ManifoldMeshRust = (&m).into();
        Self::from_mesh(&mesh)
    }
}

// impl From<(&PlantMesh, &DMat4)> for ManifoldRust {
//     fn from(m: (&PlantMesh, &DMat4)) -> Self {
//         unsafe {
//             let mesh: ManifoldMeshRust = m.into();
//             Self::from_mesh(&mesh)
//         }
//     }
// }
// impl From<ManifoldRust> for PlantMesh {
//     fn from(m: ManifoldRust) -> Self {
//         (&m).into()
//     }
// }

impl From<&ManifoldRust> for PlantMesh {
    fn from(m: &ManifoldRust) -> Self {
        use std::collections::HashSet;

        let rs_mesh = m.inner.to_mesh();
        let prop_num = rs_mesh.num_props() as usize;
        let raw_vertices = rs_mesh.vertices();
        let old_indices = rs_mesh.indices();

        if old_indices.is_empty() {
            return Self::default();
        }

        // 顶点数 = raw_vertices.len() / prop_num
        let vert_num = raw_vertices.len() / prop_num;
        let mut vert: Vec<[f32; 3]> = Vec::with_capacity(vert_num);
        for i in 0..vert_num {
            vert.push([
                raw_vertices[prop_num * i + 0],
                raw_vertices[prop_num * i + 1],
                raw_vertices[prop_num * i + 2],
            ]);
        }

        let tri_num = old_indices.len() / 3;
        let index_num = tri_num * 3;
        let mut indices = Vec::with_capacity(index_num);
        let mut normals = Vec::with_capacity(index_num);
        let mut vertices = Vec::with_capacity(index_num);

        for (i, c) in old_indices.chunks(3).enumerate() {
            let a: Vec3 = Vec3::from(vert[c[0] as usize]);
            let b: Vec3 = Vec3::from(vert[c[1] as usize]);
            let c: Vec3 = Vec3::from(vert[c[2] as usize]);

            let normal = ((b - a).cross(c - a)).normalize();

            vertices.push(a.into());
            vertices.push(b.into());
            vertices.push(c.into());

            normals.push(normal);
            normals.push(normal);
            normals.push(normal);
            let i = i as u32;
            indices.push(i * 3 + 0);
            indices.push(i * 3 + 1);
            indices.push(i * 3 + 2);
        }

        // 提取边
        let mut edge_set: HashSet<(u32, u32)> = HashSet::new();
        for triangle in indices.chunks_exact(3) {
            let v0 = triangle[0];
            let v1 = triangle[1];
            let v2 = triangle[2];
            let edges = [
                if v0 < v1 { (v0, v1) } else { (v1, v0) },
                if v1 < v2 { (v1, v2) } else { (v2, v1) },
                if v2 < v0 { (v2, v0) } else { (v0, v2) },
            ];
            for edge in edges {
                edge_set.insert(edge);
            }
        }
        let edges: Edges = edge_set
            .iter()
            .filter_map(|(idx0, idx1)| {
                if *idx0 < vertices.len() as u32 && *idx1 < vertices.len() as u32 {
                    Some(Edge::new(vec![
                        vertices[*idx0 as usize],
                        vertices[*idx1 as usize],
                    ]))
                } else {
                    None
                }
            })
            .collect();

        let mut mesh = Self {
            indices,
            vertices,
            normals,
            uvs: Vec::new(),
            wire_vertices: vec![],
            edges,
            aabb: None,
        };
        mesh.generate_auto_uvs();
        mesh.sync_wire_vertices_from_edges();
        mesh
    }
}
