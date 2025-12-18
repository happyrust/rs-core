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
        let mesh = Mesh::new(&m.vertices, &m.indices);
        Self {
            inner: mesh.to_manifold(),
        }
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

    pub fn convert_to_manifold_mesh(
        mut plant_mesh: PlantMesh,
        mat4: DMat4,
        ceil_or_trunc: bool,
    ) -> Self {
        let len = plant_mesh.vertices.len();
        let mut verts: Vec<f32> = Vec::with_capacity(len * 3);
        for v in mem::take(&mut plant_mesh.vertices) {
            let pt = mat4.transform_point3(glam::DVec3::from(v));
            if ceil_or_trunc {
                verts.push((num_traits::signum(pt[0]) * f64_round_2(pt[0].abs())) as f32);
                verts.push((num_traits::signum(pt[1]) * f64_round_2(pt[1].abs())) as f32);
                verts.push((num_traits::signum(pt[2]) * f64_round_2(pt[2].abs())) as f32);
            } else {
                verts.push(f64_trunc_1(pt[0]) as _);
                verts.push(f64_trunc_1(pt[1]) as _);
                verts.push(f64_trunc_1(pt[2]) as _);
            }
        }
        Self {
            vertices: verts,
            indices: plant_mesh.indices,
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
