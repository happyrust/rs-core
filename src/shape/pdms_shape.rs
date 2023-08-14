


use std::fmt::Debug;
use std::fs::File;
use std::hash::{Hasher};
use std::io::{Write};


// #[cfg(feature = "opencascade")]
// use opencascade::OCCShape;
use bevy_ecs::component::Component;
#[cfg(feature = "bevy_render")]
use bevy_render::prelude::*;


use glam::{Mat4, Vec3, Vec4};

use serde::{Deserialize, Serialize};

use truck_base::bounding_box::BoundingBox;
use truck_base::cgmath64::{Point3, Vector3, Vector4, Matrix4};
use truck_meshalgo::prelude::{MeshableShape, MeshedShape};
use truck_modeling::{Curve, Shell};
use parry3d::bounding_volume::Aabb;
use parry3d::math::{Point, Vector};
use parry3d::shape::{TriMesh};
use dyn_clone::DynClone;
use crate::pdms_types::*;










use std::io::BufWriter;
use std::{slice, vec};
use bevy_transform::prelude::Transform;


use tobj::{export_faces_multi_index};

use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::tool::float_tool::f32_round_3;

#[cfg(feature = "opencascade_rs")]
use opencascade::primitives::Shape;

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

    ///计算aabb
    pub fn cal_aabb(&self) -> Option<Aabb>{
        let mut aabb = Aabb::new_invalid();
        self.vertices.iter().for_each(|v|{
            aabb.take_point(nalgebra::Point3::new(v.x, v.y, v.z));
        });
        if Vec3::from(aabb.mins).is_nan() || Vec3::from(aabb.maxs).is_nan()  {
            return None;
        }
        Some(aabb)
    }

    pub fn cal_normals(&mut self) {
        for (_i, c) in self.indices.chunks(3).enumerate() {
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

    pub fn merge_without_normal(&self, merge_iden: bool) -> anyhow::Result<Self> {
        let pos = unsafe {
            slice::from_raw_parts(self.vertices.as_ptr() as *mut Vec3 as *mut f32, self.vertices.len() * 3)
        };
        let faces = self.indices.chunks(3).map(|c|
            tobj::Face::Triangle(tobj::VertexIndices {
                v: c[0] as usize,
                ..Default::default()
            }, tobj::VertexIndices {
                v: c[1] as usize,
                ..Default::default()
            }, tobj::VertexIndices {
                v: c[2] as usize,
                ..Default::default()
            },
            )).collect::<Vec<_>>();
        //try to use custom implementation
        let options = tobj::LoadOptions {
            merge_identical_points: merge_iden,
            ..Default::default()
        };
        let t_mesh = export_faces_multi_index(pos, &[], &[], &[], &faces, None, &options)?;
        Ok(Self {
            indices: t_mesh.indices,
            vertices: t_mesh.positions.chunks(3).map(|c| Vec3::new(c[0], c[1], c[2])).collect(),
            normals: vec![],
            wire_vertices: vec![],
        })
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
    pub fn from_compress_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        use flate2::write::DeflateDecoder;
        let writer = Vec::new();
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

    pub fn export_obj(&self, reverse: bool, file_path: &str) -> std::io::Result<()> {
        let mut buffer = BufWriter::new(File::create(file_path)?);
        buffer.write_all(b"# List of geometric vertices, with (x, y, z [,w]) coordinates, w is optional and defaults to 1.0.\n")?;
        for vd in &self.vertices {
            buffer.write_all(
                format!(
                    "v {:.3} {:.3} {:.3}\n",
                    vd[0] as f32, vd[1] as f32, vd[2] as f32
                )
                    .as_ref(),
            )?;
        }
        buffer.write_all(b"# Polygonal face element\n")?;
        for id in self.indices.chunks(3) {
            if reverse {
                buffer.write_all(
                    format!(
                        "f {} {} {}\n",
                        id[2] + 1,
                        id[1] + 1,
                        id[0] + 1,
                    ).as_ref(),
                )?;
            } else {
                buffer.write_all(
                    format!(
                        "f {} {} {}\n",
                        id[0] + 1,
                        id[1] + 1,
                        id[2] + 1,
                    ).as_ref(),
                )?;
            }
        }

        buffer.flush()?;
        Ok(())
    }
}

#[cfg(feature = "opencascade")]
impl From<OCCMesh> for PlantGeoData {
    fn from(o: OCCMesh) -> Self {
        let vertex_count = o.triangles.len() * 3;
        let mut aabb = Aabb::new_invalid();
        o.vertices.iter().for_each(|v| {
            aabb.take_point(nalgebra::Point3::new(v.x, v.y, v.z));
        });

        let mut vertices = Vec::with_capacity(vertex_count);
        let mut normals = Vec::with_capacity(vertex_count);
        let mut indices = Vec::with_capacity(vertex_count);

        for (i, (t, normal)) in o.triangles_with_normals().enumerate() {
            //顶点重排，保证normal是正确的
            vertices.push(o.vertices[t[0]].into());
            vertices.push(o.vertices[t[1]].into());
            vertices.push(o.vertices[t[2]].into());
            indices.push((i * 3) as u32);
            indices.push((i * 3 + 1) as u32);
            indices.push((i * 3 + 2) as u32);
            normals.push(normal.into());
            normals.push(normal.into());
            normals.push(normal.into());
        }

        Self {
            geo_hash: 0,
            mesh: Some(PlantMesh {
                indices,
                vertices,
                normals,
                wire_vertices: vec![],
            }),
            aabb: Some(aabb),
            occ_shape: None,
        }
    }
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
//             #[cfg(feature = "opencascade")]
//             occ_shape: None,
//         }
//     }
// }


pub const TRI_TOL: f32 = 0.001;
pub const LEN_TOL: f32 = 0.001;
pub const ANGLE_RAD_TOL: f32 = 0.001;
pub const MIN_SIZE_TOL: f32 = 0.01;
pub const MAX_SIZE_TOL: f32 = 1.0e5;
dyn_clone::clone_trait_object!(BrepShapeTrait);

///brep形状trait
pub trait BrepShapeTrait: VerifiedShape + Debug + Send + Sync + DynClone {
    fn is_reuse_unit(&self) -> bool {
        false
    }

    //拷贝函数
    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait>;

    ///生成shell
    fn gen_brep_shell(&self) -> Option<Shell> {
        return None;
    }

    ///获得关键点
    fn key_points(&self) -> Vec<Vec3>{  return vec![Vec3::ZERO];}

    ///限制参数大小，主要是对负实体的不合理进行限制
    fn apply_limit_by_size(&mut self, _limit_size: f32) {}

    #[cfg(feature = "opencascade_rs")]
    fn gen_occ_shape(&self) -> anyhow::Result<Shape> {
        return Err(anyhow!("不存在该occ shape"));
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
    fn gen_unit(&self, tol_ratio: Option<f32>) -> Option<PlantGeoData> {
        self.gen_unit_shape().gen_plant_geo_data(tol_ratio)
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
    fn tol(&self) -> f32 {
        TRI_TOL
    }

    ///生成mesh
    #[cfg(feature = "truck")]
    fn gen_plant_geo_data(&self, tol_ratio: Option<f32>) -> Option<PlantGeoData> {
        let mut aabb = Aabb::new_invalid();
        let geo_hash = self.hash_unit_mesh_params();
        if self.need_use_csg() {
            if let Some(csg_mesh) = self.gen_csg_mesh() {
                for vertex in &csg_mesh.vertices {
                    aabb.take_point((*vertex).into());
                }
                return Some(PlantGeoData {
                    geo_hash,
                    mesh: Some(csg_mesh),
                    aabb: Some(aabb),
                });
            }
            return None;
        }
        if let Some(brep) = self.gen_brep_shell() {
            let brep_bbox = gen_bounding_box(&brep);
            let d = brep_bbox.diameter() as f32;
            if d < MIN_SIZE_TOL || d > MAX_SIZE_TOL{
                return None;
            }
            let (_size, c) = (brep_bbox.diameter(), brep_bbox.center());
            let d = brep_bbox.diagonal() / 2.0;
            aabb = Aabb::from_half_extents(
                Point::<f32>::new(c[0] as f32, c[1] as f32, c[2] as f32),
                Vector::<f32>::new(d[0] as f32, d[1] as f32, d[2] as f32),
            );
            let tolerance = self.tol() * tol_ratio.unwrap_or(2.0);
            // #[cfg(debug_assertions)]
            // dbg!(tolerance);
            let meshed_shape = brep.triangulation(tolerance as f64);
            let polygon = meshed_shape.to_polygon();
            if polygon.positions().is_empty() { return None; }
            let vertices = polygon.positions().iter().map(|&x| x.vec3()).collect::<Vec<_>>();
            let normals = polygon.normals().iter().map(|&x| x.vec3()).collect::<Vec<_>>();
            let _uvs = polygon.uv_coords().iter().map(|x| [x[0] as f32, x[1] as f32]).collect::<Vec<_>>();
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
            let wire_vertices: Vec<Vec<Vec3>> = curves
                .iter()
                .map(|poly| poly.iter().map(|x| x.vec3()).collect::<Vec<_>>())
                .collect();

            return Some(PlantGeoData {
                geo_hash,
                mesh: Some(PlantMesh {
                    indices,
                    vertices,
                    normals,
                    wire_vertices,
                }),
                aabb: Some(aabb),
                // occ_shape: None,
            });
            // return
        }
        None
    }

    fn convert_to_geo_param(&self) -> Option<PdmsGeoParam> {
        None
    }

    fn gen_csg_mesh(&self) -> Option<PlantMesh> {
        None
    }

    fn need_use_csg(&self) -> bool {
        false
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

