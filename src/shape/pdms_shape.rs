use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::default::default;
use std::fmt::Debug;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use anyhow::anyhow;
use dashmap::DashMap;
use dashmap::mapref::one::Ref;
use glam::{DVec3, Mat4, Vec3, vec3, Vec4};
use lyon::path::polygon;
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;
use truck_base::bounding_box::BoundingBox;
use truck_base::cgmath64::{Point3, Vector3, Vector4, Matrix4};
use truck_meshalgo::prelude::{MeshableShape, MeshedShape};
use truck_modeling::{Curve, Shell};
use bevy_ecs::prelude::Component;
// #[cfg(not(target_arch = "wasm32"))]
// use csg::{Mesh as CsgMesh, Pt3 as CsgPt3};
use parry3d::bounding_volume::Aabb;
use parry3d::math::{Matrix, Point, Vector};
use parry3d::shape::{TriMesh, TriMeshFlags};
use dyn_clone::DynClone;
use crate::pdms_types::*;
use crate::prim_geo::ctorus::{CTorus, SCTorus};
use crate::prim_geo::cylinder::{LCylinder, SCylinder};
use crate::prim_geo::dish::Dish;
use crate::prim_geo::extrusion::Extrusion;
use crate::prim_geo::facet::Facet;
use crate::prim_geo::pyramid::Pyramid;
use crate::prim_geo::rtorus::SRTorus;
use crate::prim_geo::sbox::SBox;
use crate::prim_geo::snout::LSnout;

use rkyv::with::Skip;

use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::tool::float_tool::f32_round_3;


pub const TRIANGLE_TOL: f64 = 0.01;
pub const ANGLE_RAD_TOL: f32 = 0.01;
pub const LEN_TOL: f32 = 0.001;

pub trait VerifiedShape {
    fn check_valid(&self) -> bool {
        true
    }
}

#[inline]
pub fn gen_bounding_box(shell: &Shell) -> BoundingBox<Point3> {
    let mut bdd_box = BoundingBox::new();
    shell
        .iter()
        .flat_map(truck_modeling::Face::boundaries)
        .flatten()
        .for_each(|edge| {
            let curve = edge.oriented_curve();
            bdd_box += match curve {
                Curve::Line(line) => vec![line.0, line.1].into_iter().collect(),
                Curve::BSplineCurve(curve) => {
                    let bdb = curve.roughly_bounding_box();
                    vec![*bdb.max(), *bdb.min()].into_iter().collect()
                }
                Curve::NurbsCurve(curve) => curve.roughly_bounding_box(),
                Curve::IntersectionCurve(_) => BoundingBox::new(),
            };
        });
    bdd_box
}


//todo 增加LOD的实现
#[derive(Serialize, Deserialize, Component, Debug, Default, Clone, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, )]
pub struct PlantMesh {
    pub indices: Vec<u32>,
    pub vertices: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub wire_vertices: Vec<Vec<Vec3>>,
}

impl PlantMesh {
    //集成lod的功能
    #[inline]
    pub fn get_tri_mesh(&self, trans: Mat4) -> TriMesh {
        let mut points: Vec<Point<f32>> = vec![];
        let mut indices: Vec<[u32; 3]> = vec![];
        //如果 数量太大，需要使用LOD的模型去做碰撞检测
        self.vertices.iter().for_each(|p| {
            let new_pt = trans.transform_point3(*p);
            points.push(new_pt.into())
        });
        self.indices.chunks(3).for_each(|i| {
            indices.push([i[0] as u32, i[1] as u32, i[2] as u32]);
        });
        // TriMesh::with_flags(points, indices, TriMeshFlags::ORIENTED)
        TriMesh::new(points, indices)
    }

    pub fn cal_normals(&mut self) {
        for (i, c) in self.indices.chunks(3).enumerate() {
            let a: Vec3 = self.vertices[c[0] as usize];
            let b: Vec3 = self.vertices[c[1] as usize];
            let c: Vec3 = self.vertices[c[2] as usize];

            let normal = ((b - a).cross(c - a)).normalize();
            self.normals.push(normal);
            self.normals.push(normal);
            self.normals.push(normal);
        }
    }

    ///todo 后面需要把uv使用上
    #[cfg(feature = "bevy_render")]
    pub fn gen_bevy_mesh(&self) -> Mesh {
        let mut mesh = Mesh::new(TriangleList);
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.vertices.clone());
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals.clone());
        // mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.set_indices(Some(Indices::U32(
            self.indices.clone()
        )));
        mesh
    }

    pub fn transform_by(&self, t: &Mat4) -> Self {
        let mut vertices = Vec::with_capacity(self.vertices.len());
        let mut normals = Vec::with_capacity(self.vertices.len());
        let len = self.vertices.len();
        for i in 0..len {
            vertices.push(t.transform_point3(self.vertices[i]));
            if i < self.normals.len() {
                normals.push(t.transform_vector3(self.normals[i]).normalize());
            }
        }
        Self {
            indices: self.indices.clone(),
            vertices,
            normals,
            wire_vertices: vec![],
        }
    }

    #[inline]
    pub fn into_compress_bytes(&self) -> Vec<u8> {
        use flate2::Compression;
        use flate2::write::DeflateEncoder;
        let mut e = DeflateEncoder::new(Vec::new(), Compression::default());
        e.write_all(&bincode::serialize(&self).unwrap());
        e.finish().unwrap_or_default()
    }

    #[inline]
    pub fn from_compress_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        use flate2::write::DeflateDecoder;
        let mut writer = Vec::new();
        let mut deflater = DeflateDecoder::new(writer);
        deflater.write_all(bytes)?;
        Ok(bincode::deserialize(&deflater.finish()?)?)
    }

    //转变成csg模型
    // #[cfg(not(target_arch = "wasm32"))]
    // pub fn into_csg_mesh(&self, transform: Option<&Mat4>) -> CsgMesh {
    //     let mut triangles = Vec::new();
    //     for chuck in self.indices.chunks(3) {
    //         let mut vertices_a: Vec3 = self.vertices[chuck[0] as usize];
    //         let mut vertices_b: Vec3 = self.vertices[chuck[1] as usize];
    //         let mut vertices_c: Vec3 = self.vertices[chuck[2] as usize];
    //
    //         if let Some(transform) = transform {
    //             vertices_a = transform.transform_point3(vertices_a);
    //             vertices_b = transform.transform_point3(vertices_b);
    //             vertices_c = transform.transform_point3(vertices_c);
    //         }
    //         triangles.push(csg::Triangle {
    //             a: CsgPt3 { x: vertices_a[0] as f64, y: vertices_a[1] as f64, z: vertices_a[2] as f64 },
    //             b: CsgPt3 { x: vertices_b[0] as f64, y: vertices_b[1] as f64, z: vertices_b[2] as f64 },
    //             c: CsgPt3 { x: vertices_c[0] as f64, y: vertices_c[1] as f64, z: vertices_c[2] as f64 },
    //         })
    //     }
    //     csg::Mesh::from_triangles(triangles)
    // }

    // #[cfg(not(target_arch = "wasm32"))]
    // pub fn from_scg_mesh(&self, csg_mesh: &CsgMesh, world_transform: &Transform) -> Self {
    //     let rev_mat = world_transform.compute_matrix().inverse();
    //     let mut mesh = PlantMesh {
    //         aabb: self.aabb.clone(),
    //         ..default()
    //     };
    //     let mut i = 0;
    //     for tri in &csg_mesh.triangles {
    //         mesh.indices.push(i);
    //         mesh.indices.push(i + 1);
    //         mesh.indices.push(i + 2);
    //         let normal = tri.normal();
    //         let normal = Vec3::from_array([normal.x as f32, normal.y as f32, normal.z as f32]);
    //         let local_normal = rev_mat.transform_vector3(normal);
    //         let normal = [local_normal.x, local_normal.y, local_normal.z];
    //         mesh.normals.push(normal);
    //         mesh.normals.push(normal);
    //         mesh.normals.push(normal);
    // 
    //         let pta = Vec3::from_array([tri.a.x as f32, tri.a.y as f32, tri.a.z as f32]);
    //         let pta = rev_mat.transform_point3(pta);
    // 
    //         let ptb = Vec3::from_array([tri.b.x as f32, tri.b.y as f32, tri.b.z as f32]);
    //         let ptb = rev_mat.transform_point3(ptb);
    // 
    //         let ptc = Vec3::from_array([tri.c.x as f32, tri.c.y as f32, tri.c.z as f32]);
    //         let ptc = rev_mat.transform_point3(ptc);
    // 
    //         mesh.vertices.push(pta.into());
    //         mesh.vertices.push(ptb.into());
    //         mesh.vertices.push(ptc.into());
    //         i += 3;
    //     }
    //     mesh
    // }
}

// #[cfg(not(target_arch = "wasm32"))]
// impl From<CsgMesh> for PlantGeoData {
//     fn from(o: CsgMesh) -> Self {
//         (&o).into()
//     }
// }
//
// #[cfg(not(target_arch = "wasm32"))]
// impl From<&CsgMesh> for PlantGeoData {
//     fn from(o: &CsgMesh) -> Self {
//         let vertex_count = o.triangles.len() * 3;
//         let mut aabb = Aabb::new_invalid();
//
//         let mut vertices = Vec::with_capacity(vertex_count);
//         let mut normals = Vec::with_capacity(vertex_count);
//         let mut indices = Vec::with_capacity(vertex_count);
//
//         for (i, t) in o.triangles.iter().enumerate() {
//             //顶点重排，保证normal是正确的
//             aabb.take_point(nalgebra::Point3::new(t.a.x as _, t.a.y as _, t.a.z as _));
//             vertices.push(t.a.into());
//             vertices.push(t.b.into());
//             vertices.push(t.c.into());
//             indices.push((i * 3) as u32);
//             indices.push((i * 3 + 1) as u32);
//             indices.push((i * 3 + 2) as u32);
//             normals.push(t.normal().into());
//             normals.push(t.normal().into());
//             normals.push(t.normal().into());
//         }
//
//         Self {
//             geo_hash: 0,
//             mesh: Some(PlantMesh {
//                 indices,
//                 vertices,
//                 normals,
//                 wire_vertices: vec![],
//             }),
//             aabb: Some(aabb),
//         }
//     }
// }


pub const TRI_TOL: f32 = 0.05;


pub trait BrepMathTrait {
    fn vector3(&self) -> Vector3;
    fn vector4(&self) -> Vector4;
    fn point3(&self) -> Point3;
    fn point3_without_z(&self) -> Point3;
}


impl BrepMathTrait for Vec3 {
    #[inline]
    fn vector3(&self) -> Vector3 {
        Vector3::new(self[0] as f64, self[1] as f64, self[2] as f64)
    }

    #[inline]
    fn vector4(&self) -> Vector4 {
        Vector4::new(self[0] as f64, self[1] as f64, self[2] as f64, 0.0f64)
    }

    #[inline]
    fn point3(&self) -> Point3 {
        Point3::new(self[0] as f64, self[1] as f64, self[2] as f64)
    }

    #[inline]
    fn point3_without_z(&self) -> Point3 {
        Point3::new(f32_round_3(self[0]) as f64, f32_round_3(self[1]) as f64, 0.0 as f64)
    }
}

impl BrepMathTrait for Vec4 {
    #[inline]
    fn vector3(&self) -> Vector3 {
        Vector3::new(self[0] as f64, self[1] as f64, self[2] as f64)
    }

    #[inline]
    fn vector4(&self) -> Vector4 {
        Vector4::new(self[0] as f64, self[1] as f64, self[2] as f64, self[3] as f64)
    }

    #[inline]
    fn point3(&self) -> Point3 {
        Point3::new(self[0] as f64, self[1] as f64, self[2] as f64)
    }

    #[inline]
    fn point3_without_z(&self) -> Point3 {
        Point3::new(self[0] as f64, self[1] as f64, 0.0 as f64)
    }
}

pub trait BevyMathTrait {
    fn vec3(&self) -> Vec3;
    fn array(&self) -> [f32; 3];
}

impl BevyMathTrait for Vector3 {
    #[inline]
    fn vec3(&self) -> Vec3 {
        Vec3::new(self[0] as f32, self[1] as f32, self[2] as f32)
    }

    #[inline]
    fn array(&self) -> [f32; 3] {
        [self[0] as f32, self[1] as f32, self[2] as f32]
    }
}

impl BevyMathTrait for Point3 {
    #[inline]
    fn vec3(&self) -> Vec3 {
        Vec3::new(self[0] as f32, self[1] as f32, self[2] as f32)
    }

    #[inline]
    fn array(&self) -> [f32; 3] {
        [self[0] as f32, self[1] as f32, self[2] as f32]
    }
}

#[inline]
pub fn convert_to_cg_matrix4(m: &Mat4) -> Matrix4 {
    Matrix4::from_cols(m.x_axis.vector4(), m.y_axis.vector4(), m.z_axis.vector4(), m.w_axis.vector4())
}

