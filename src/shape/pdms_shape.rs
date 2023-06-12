use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::default::default;
use std::fmt::Debug;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use anyhow::anyhow;
// #[cfg(feature = "opencascade")]
// use opencascade::OCCShape;
use bevy::ecs::component::Component;
use bevy::prelude::{FromWorld, Mesh, Transform};
use bevy::render::mesh::Indices;
use bevy::render::render_resource::PrimitiveTopology::{LineList, TriangleList};
use bevy::utils::HashSet;
use dashmap::DashMap;
use dashmap::mapref::one::Ref;
use glam::{DVec3, Mat4, Vec3, vec3, Vec4};
use lyon::path::polygon;
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;
use truck_base::bounding_box::BoundingBox;
use truck_base::cgmath64::{Point3, Vector3, Vector4};
use truck_meshalgo::prelude::{MeshableShape, MeshedShape};
use truck_modeling::{Curve, Shell};
#[cfg(not(target_arch = "wasm32"))]
use csg::{Mesh as CsgMesh, Pt3 as CsgPt3};
use parry3d::bounding_volume::Aabb;
use parry3d::math::{Matrix, Point, Vector};
use parry3d::shape::{TriMesh, TriMeshFlags};
use dyn_clone::DynClone;
use nalgebra::Matrix4;
use crate::pdms_types::*;
use crate::prim_geo::category::CateBrepShape;
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
use crate::tool::float_tool::f32_round_2;


pub const TRIANGLE_TOL: f64 = 0.01;

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
#[derive(Serialize, Deserialize, Component, Debug, Default, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, )]
pub struct PlantMesh {
    pub indices: Vec<u32>,
    pub vertices: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,

    pub wire_vertices: Vec<Vec<[f32; 3]>>,
    pub aabb: Option<Aabb>,
}
unsafe impl Sync for PlantMesh {}
unsafe impl Send for PlantMesh {}



#[test]
fn test_project_to_plane() {
    // Define a triangle's vertices in 3D space
    let v1 = vec3(0.0, 0.0, 0.0);
    let v2 = vec3(1.0, 0.0, 0.0);
    let v3 = vec3(0.0, 1.0, 0.0);

    // Define a projection plane in 3D space
    let plane_origin = vec3(0.0, 0.0, 1.0);
    let plane_normal = vec3(0.0, 0.0, 1.0); // the plane normal faces in the positive Z direction
    let projection_matrix = Mat4::from_scale_rotation_translation(Vec3::ONE, glam::Quat::from_rotation_z(0.0),
                                                                  -plane_origin);

    // Project the triangle onto the 2D plane
    let projected_v1 = projection_matrix.transform_point3(v1);
    let projected_v2 = projection_matrix.transform_point3(v2);
    let projected_v3 = projection_matrix.transform_point3(v3);

    // Check if the triangle is valid, i.e., if its area is positive
    let edge1 = projected_v2 - projected_v1;
    let edge2 = projected_v3 - projected_v1;
    let triangle_area = edge1.cross(edge2).length() * 0.5;
    assert!(triangle_area > 0.0);
}


impl PlantMesh {
    //集成lod的功能
    #[inline]
    pub fn get_tri_mesh(&self, trans: Mat4) -> TriMesh {
        let mut points: Vec<Point<f32>> = vec![];
        let mut indices: Vec<[u32; 3]> = vec![];
        //如果 数量太大，需要使用LOD的模型去做碰撞检测
        self.vertices.iter().for_each(|p| {
            let new_pt = trans.transform_point3(Vec3::new(p[0], p[1], p[2]));
            points.push(Point::new(new_pt[0], new_pt[1], new_pt[2]))
        });
        self.indices.chunks(3).for_each(|i| {
            indices.push([i[0] as u32, i[1] as u32, i[2] as u32]);
        });
        // TriMesh::with_flags(points, indices, TriMeshFlags::ORIENTED)
        TriMesh::new(points, indices)
    }

    ///todo 后面需要把uv使用上
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

    ///返回三角模型 （tri_mesh, AABB）
    pub fn gen_bevy_mesh_with_aabb(&self) -> (Mesh, Option<Aabb>) {
        let mut mesh = Mesh::new(TriangleList);
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.vertices.clone());
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals.clone());
        let n = self.vertices.len();
        let mut uvs = vec![];
        for i in 0..n {
            uvs.push([0.0f32, 0.0]);
        }
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        //todo 是否需要优化索引
        mesh.set_indices(Some(Indices::U32(
            self.indices.clone()
        )));

        // let mut wire_mesh = Mesh::new(LineList);
        // wire_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.wf_vertices.clone());
        // wire_mesh.set_indices(Some(Indices::U32(
        //     self.wf_indices.clone()
        // )));
        (mesh, self.aabb.clone())
    }

    // ///变成压缩的模型数据
    // #[inline]
    // pub fn into_compress_bytes(&self) -> Vec<u8> {
    //     use flate2::Compression;
    //     use flate2::write::DeflateEncoder;
    //     let mut e = DeflateEncoder::new(Vec::new(), Compression::default());
    //     let serialized = rkyv::to_bytes::<_, 2048>(self).unwrap().to_vec();
    //     e.write_all(&serialized);
    //     e.finish().unwrap_or_default()
    // }

    // ///根据反序列化的数据还原成mesh
    // #[inline]
    // pub fn from_compress_bytes(bytes: &[u8]) -> Option<Self> {
    //     use flate2::write::DeflateDecoder;
    //     let mut writer = Vec::new();
    //     let mut deflater = DeflateDecoder::new(writer);
    //     deflater.write_all(bytes).ok()?;
    //     let buf = deflater.finish().ok()?;
    //     use rkyv::{archived_root, Deserialize};
    //     let archived = unsafe { rkyv::archived_root::<Self>(buf.as_slice()) };
    //     archived.deserialize(&mut rkyv::Infallible).ok()
    // }

    #[inline]
    pub fn into_compress_bytes(&self) -> Vec<u8> {
        use flate2::Compression;
        use flate2::write::DeflateEncoder;
        let mut e = DeflateEncoder::new(Vec::new(), Compression::default());
        e.write_all(&bincode::serialize(&self).unwrap());
        e.finish().unwrap_or_default()
    }

    #[inline]
    pub fn from_compress_bytes(bytes: &[u8]) -> Option<Self> {
        use flate2::write::DeflateDecoder;
        let mut writer = Vec::new();
        let mut deflater = DeflateDecoder::new(writer);
        deflater.write_all(bytes).ok()?;
        bincode::deserialize(&deflater.finish().ok()?).ok()
    }

    ///转变成csg模型
    #[cfg(not(target_arch = "wasm32"))]
    pub fn into_csg_mesh(&self, transform: &Mat4) -> CsgMesh {
        let mut triangles = Vec::new();
        for chuck in self.indices.chunks(3) {
            let vertices_a: Option<&[f32; 3]> = self.vertices.get(chuck[0] as usize);
            let vertices_b: Option<&[f32; 3]> = self.vertices.get(chuck[1] as usize);
            let vertices_c: Option<&[f32; 3]> = self.vertices.get(chuck[2] as usize);
            if vertices_a.is_none() || vertices_b.is_none() || vertices_c.is_none() { continue; }

            let vertices_a = Vec3::from_array(*vertices_a.unwrap());
            let vertices_b = Vec3::from_array(*vertices_b.unwrap());
            let vertices_c = Vec3::from_array(*vertices_c.unwrap());

            let pt_a = transform.transform_point3(vertices_a);
            let pt_b = transform.transform_point3(vertices_b);
            let pt_c = transform.transform_point3(vertices_c);

            triangles.push(csg::Triangle {
                a: CsgPt3 { x: pt_a[0] as f64, y: pt_a[1] as f64, z: pt_a[2] as f64 },
                b: CsgPt3 { x: pt_b[0] as f64, y: pt_b[1] as f64, z: pt_b[2] as f64 },
                c: CsgPt3 { x: pt_c[0] as f64, y: pt_c[1] as f64, z: pt_c[2] as f64 },
            })
        }
        csg::Mesh::from_triangles(triangles)
    }

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

#[cfg(not(target_arch = "wasm32"))]
impl From<CsgMesh> for PlantMesh {
    fn from(o: CsgMesh) -> Self {
        let vertex_count = o.triangles.len() * 3;
        let mut aabb = Aabb::new_invalid();

        let mut vertices = Vec::with_capacity(vertex_count);
        let mut normals = Vec::with_capacity(vertex_count);
        let mut indices = Vec::with_capacity(vertex_count);

        for (i, t) in o.triangles.iter().enumerate() {
            //顶点重排，保证normal是正确的
            aabb.take_point(nalgebra::Point3::new(t.a.x as _, t.a.y as _, t.a.z as _));
            vertices.push(t.a.into());
            vertices.push(t.b.into());
            vertices.push(t.c.into());
            indices.push((i * 3) as u32);
            indices.push((i * 3 + 1) as u32);
            indices.push((i * 3 + 2) as u32);
            normals.push(t.normal().into());
            normals.push(t.normal().into());
            normals.push(t.normal().into());
        }

        Self{
            indices,
            vertices,
            normals,
            wire_vertices: vec![],
            aabb: Some(aabb),
        }
    }
}




pub const TRI_TOL: f32 = 0.05;
dyn_clone::clone_trait_object!(BrepShapeTrait);

///brep形状trait
pub trait BrepShapeTrait: VerifiedShape + Debug + Send + Sync + DynClone {
    //拷贝函数
    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait>;

    ///生成shell
    fn gen_brep_shell(&self) -> Option<Shell> {
        return None;
    }

    ///限制参数大小，主要是对负实体的不合理进行限制
    fn apply_limit_by_size(&mut self, limit_size: f32){

    }


    //计算单元模型的参数hash值，也就是做成被可以复用的模型后的hash
    fn hash_unit_mesh_params(&self) -> u64 {
        0
    }


    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait>;

    ///生成对应的单位长度的模型，比如Dish，就是以R为1的情况生成模型
    /// box
    /// cylinder
    /// sphere
    fn gen_unit_mesh(&self) -> Option<PlantMesh> {
        self.gen_unit_shape().gen_mesh()
    }

    ///获得缩放向量
    #[inline]
    fn get_scaled_vec3(&self) -> Vec3 {
        Vec3::ONE
    }

    ///获得变换矩阵
    #[inline]
    fn get_trans(&self) -> Transform {
        Transform {
            rotation: Default::default(),
            translation: Default::default(),
            scale: self.get_scaled_vec3(),
        }
    }

    #[inline]
    fn tol(&self) -> f32{
        TRI_TOL
    }


    #[cfg(not(feature = "opencascade"))]
    ///生成mesh
    fn gen_mesh(&self) -> Option<PlantMesh> {
        let mut aabb = Aabb::new_invalid();
        if let Some(brep) = self.gen_brep_shell() {
            let brep_bbox = gen_bounding_box(&brep);
            let d = brep_bbox.diagonal();
            if d.x < 0.01 || d.y < 0.01 || d.z < 0.01 {
                return None;
            }
            let vv = DVec3::new(d.x, d.y, d.z);
            if vv.max_element() / vv.min_element() > 10000.0 {
                return None;
            }
            let (size, c) = (brep_bbox.diameter(), brep_bbox.center());
            let d = brep_bbox.diagonal() / 2.0;
            aabb = Aabb::from_half_extents(
                Point::<f32>::new(c[0] as f32, c[1] as f32, c[2] as f32),
                Vector::<f32>::new(d[0] as f32, d[1] as f32, d[2] as f32),
            );
            // dbg!(&aabb);
            // dbg!(size);
            if size <= f64::EPSILON {
                return None;
            }
            let tolerance = self.tol() as f64 * size;
            let meshed_shape = brep.triangulation(tolerance);
            let polygon = meshed_shape.to_polygon();
            if polygon.positions().is_empty() { return None; }
            let vertices = polygon.positions().iter().map(|&x| x.array()).collect::<Vec<_>>();
            let normals = polygon.normals().iter().map(|&x| x.array()).collect::<Vec<_>>();
            let uvs = polygon.uv_coords().iter().map(|x| [x[0] as f32, x[1] as f32]).collect::<Vec<_>>();
            let indices = polygon
                .faces()
                .triangle_iter()
                .flatten()
                .map(|x| x.pos as u32)
                .collect::<Vec<_>>();

            let curves = meshed_shape
                .edge_iter()
                .map(|edge| edge.curve())
                .collect::<Vec<_>>();
            let wire_vertices: Vec<Vec<[f32; 3]>> = curves
                .iter()
                .map(|poly| poly.iter().map(|x| x.array()).collect::<Vec<_>>())
                .collect();
            return Some(PlantMesh {
                indices,
                vertices,
                normals,
                wire_vertices,
                aabb: Some(aabb),
            });
        }
        None
    }

    fn convert_to_geo_param(&self) -> Option<PdmsGeoParam> {
        None
    }
}


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
    //point3_without_z
    #[inline]
    fn point3_without_z(&self) -> Point3 {
        Point3::new(f32_round_2(self[0]) as f64, f32_round_2(self[1]) as f64, 0.0 as f64)
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
