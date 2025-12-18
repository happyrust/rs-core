use std::collections::HashMap;
use std::mem;

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
                    max_extent / min_extent, min_extent, max_extent
                );
            }
        }
        
        result
    }

    pub fn get_mesh(&self) -> ManifoldMeshRust {
        let mesh = self.inner.to_mesh();
        ManifoldMeshRust {
            vertices: mesh.vertices(),
            indices: mesh.indices(),
        }
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
    fn compute_adaptive_precision(vertices: &[Vec3], mat4: &DMat4) -> f64 {
        if vertices.is_empty() {
            return 1000.0; // 默认使用 3 位小数精度
        }
        
        // 计算变换后的 AABB
        let mut min = glam::DVec3::splat(f64::MAX);
        let mut max = glam::DVec3::splat(f64::MIN);
        
        for v in vertices {
            let pt = mat4.transform_point3(glam::DVec3::new(v.x as f64, v.y as f64, v.z as f64));
            min = min.min(pt);
            max = max.max(pt);
        }
        
        let extent = max - min;
        let min_extent = extent.x.min(extent.y).min(extent.z);
        
        // 根据最小维度选择精度
        // 确保每个维度至少有足够的离散点
        if min_extent < 1.0 {
            // 非常小的几何体，使用 4 位小数
            10000.0
        } else if min_extent < 10.0 {
            // 小几何体，使用 3 位小数
            1000.0
        } else if min_extent < 100.0 {
            // 中等几何体，使用 2 位小数
            100.0
        } else {
            // 大型几何体，使用 1 位小数
            10.0
        }
    }
    
    /// 将顶点坐标量化为整数键（用于顶点焊接）
    fn quantize_vertex(x: f64, y: f64, z: f64, precision: f64) -> (i64, i64, i64) {
        (
            (x * precision).round() as i64,
            (y * precision).round() as i64,
            (z * precision).round() as i64,
        )
    }

    pub fn convert_to_manifold_mesh(
        mut plant_mesh: PlantMesh,
        mat4: DMat4,
        ceil_or_trunc: bool,
    ) -> Self {
        use std::collections::HashMap;
        
        let vertices = mem::take(&mut plant_mesh.vertices);
        let old_indices = &plant_mesh.indices;
        
        if vertices.is_empty() || old_indices.is_empty() {
            return Self::new();
        }
        
        // 计算适应性精度
        let precision = Self::compute_adaptive_precision(&vertices, &mat4);
        
        // Step 1: 变换所有顶点，并执行顶点焊接（合并重合顶点）
        let mut vertex_map: HashMap<(i64, i64, i64), u32> = HashMap::new();
        let mut welded_vertices: Vec<f32> = Vec::new();
        let mut old_to_new: Vec<u32> = Vec::with_capacity(vertices.len());
        
        for v in &vertices {
            let pt = mat4.transform_point3(glam::DVec3::new(v.x as f64, v.y as f64, v.z as f64));
            
            // 量化顶点坐标用于焊接
            let key = Self::quantize_vertex(pt.x, pt.y, pt.z, precision);
            
            let new_idx = if let Some(&idx) = vertex_map.get(&key) {
                // 已存在相同位置的顶点，复用
                idx
            } else {
                // 新顶点
                let idx = (welded_vertices.len() / 3) as u32;
                
                // 根据精度量化顶点坐标
                let qx = key.0 as f64 / precision;
                let qy = key.1 as f64 / precision;
                let qz = key.2 as f64 / precision;
                
                if ceil_or_trunc {
                    // 负实体：向外扩展（使用 ceil）
                    welded_vertices.push((num_traits::signum(qx) * qx.abs().ceil()) as f32);
                    welded_vertices.push((num_traits::signum(qy) * qy.abs().ceil()) as f32);
                    welded_vertices.push((num_traits::signum(qz) * qz.abs().ceil()) as f32);
                } else {
                    // 正实体：直接使用量化坐标
                    welded_vertices.push(qx as f32);
                    welded_vertices.push(qy as f32);
                    welded_vertices.push(qz as f32);
                }
                
                vertex_map.insert(key, idx);
                idx
            };
            
            old_to_new.push(new_idx);
        }
        
        // Step 2: 重建索引，过滤退化三角形
        let mut new_indices: Vec<u32> = Vec::with_capacity(old_indices.len());
        
        for tri in old_indices.chunks_exact(3) {
            let i0 = old_to_new[tri[0] as usize];
            let i1 = old_to_new[tri[1] as usize];
            let i2 = old_to_new[tri[2] as usize];
            
            // 跳过退化三角形（顶点重合）
            if i0 == i1 || i1 == i2 || i2 == i0 {
                continue;
            }
            
            new_indices.push(i0);
            new_indices.push(i1);
            new_indices.push(i2);
        }
        
        Self {
            vertices: welded_vertices,
            indices: new_indices,
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
