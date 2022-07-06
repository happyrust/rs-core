use std::collections::HashMap;
use std::fmt::Debug;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{Read, Write};

use bevy::prelude::{FromWorld, Mesh};
use truck_modeling::{Curve, Shell};
// use bevy_inspector_egui::Inspectable;
use bevy::ecs::component::Component;
use truck_base::cgmath64::{Point3, Vector3};
use truck_meshalgo::prelude::{MeshableShape, MeshedShape};
use bevy::reflect::{Reflect, ReflectRef};
use bevy::ecs::reflect::ReflectComponent;
use bevy::render::mesh::Indices;
use bevy::render::primitives::Aabb;
use bevy::render::render_resource::PrimitiveTopology::{LineList, TriangleList};
use dashmap::DashMap;
use dashmap::mapref::one::Ref;
use glam::{TransformRT, TransformSRT, Vec3};
use ncollide3d::bounding_volume::AABB;
use ncollide3d::math::{Point, Vector};
use ncollide3d::na;
use ncollide3d::na::Point3 as NPoint3;
use ncollide3d::shape::TriMesh;
use truck_base::bounding_box::BoundingBox;
use serde::{Serialize,Deserialize};

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
                Curve::NURBSCurve(curve) => curve.roughly_bounding_box(),
                Curve::IntersectionCurve(_) => BoundingBox::new(),
            };
        });
    bdd_box
    // let (size, center) = (bdd_box.size(), bdd_box.center());
}

#[derive(Serialize, Deserialize, Component, Debug, Clone, Default)]
pub struct PdmsMesh {
    pub indices: Vec<u32>,
    pub vertices: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub wf_indices: Vec<u32>,  //wireframe indices
    pub wf_vertices: Vec<[f32; 3]>, //wireframe vertex
    pub aabb: AiosAABB,
}

impl PdmsMesh {
    pub fn get_tri_mesh(&self, trans: TransformSRT) -> TriMesh<f32> {
        let mut points: Vec<ncollide3d::na::Point3<f32>> = vec![];
        let mut indices: Vec<ncollide3d::na::Point3<usize>> = vec![];
        self.vertices.iter().for_each(|p| {
            let mew_pt = trans.transform_point3(Vec3::new(p[0], p[1], p[2]));
            points.push(ncollide3d::na::Point3::<f32>::new(mew_pt[0], mew_pt[1], mew_pt[2]))
        });
        self.indices.chunks(3).for_each(|i| {
            indices.push(ncollide3d::na::Point3::<usize>::new(i[0] as usize, i[1] as usize, i[2] as usize));
        });
        TriMesh::new(points, indices, None)
    }

    pub fn gen_bevy_mesh(&self) -> Mesh{
        let mut mesh = Mesh::new(TriangleList);
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.vertices.clone());
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals.clone());
        // mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.set_indices(Some(Indices::U32(
            self.indices.clone()
        )));
        mesh
    }

    ///返回三角模型和线框模型
    pub fn gen_bevy_mesh_with_aabb(&self) -> (Mesh, Mesh, Aabb){
        let mut mesh = Mesh::new(TriangleList);
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.vertices.clone());
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals.clone());
        let n = self.vertices.len();
        let mut uvs = vec![];
        for i in 0..n {
            uvs.push([0.0f32, 0.0]);
        }
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.set_indices(Some(Indices::U32(
            self.indices.clone()
        )));
        let AiosAABB{
            min,
            max,
        } = self.aabb;

        let mut wire_mesh = Mesh::new(LineList);
        wire_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.wf_vertices.clone());
        wire_mesh.set_indices(Some(Indices::U32(
            self.wf_indices.clone()
        )));
        (mesh, wire_mesh, Aabb::from_min_max(min, max))
    }
}

//bevy's meshs

impl PdmsMeshMgr {
    #[inline]
    pub fn get_instants_data(&self, refno: RefU64) -> DashMap<RefU64, Ref<RefU64, EleGeosInfo>> {
        let mut results = DashMap::new();
        let inst_map = &self.inst_mgr.inst_map;
        if self.level_shape_mgr.contains_key(&refno) {
            for v in (*self.level_shape_mgr.get(&refno).unwrap()).iter(){
                if inst_map.contains_key(v) {
                    results.insert(v.clone(), inst_map.get(v).unwrap());
                }
            }
        } else {
            if inst_map.contains_key(&refno) {
                results.insert(refno.clone(), inst_map.get(&refno).unwrap());
            }
        }
        results
    }

    pub fn serialize_to_bin_file(&self, mdb: &str) -> bool {
        let mut file = File::create(format!(r"PdmsMeshMgr_{}.bin", mdb)).unwrap();
        let serialized = bincode::serialize(&self).unwrap();
        file.write_all(serialized.as_slice()).unwrap();
        true
    }

    pub fn serialize_to_specify_file(&self, file_path: &str) -> bool {
        let mut file = File::create(file_path).unwrap();
        let serialized = bincode::serialize(&self).unwrap();
        file.write_all(serialized.as_slice()).unwrap();
        true
    }

    pub fn deserialize_from_bin_file(mdb: &str) -> anyhow::Result<Self> {
        let mut file = File::open(format!("PdmsMeshMgr_{}.bin", mdb))?;
        let mut buf: Vec<u8> = Vec::new();
        file.read_to_end(&mut buf).ok();
        let r = bincode::deserialize(buf.as_slice())?;
        Ok(r)
    }

    pub fn serialize_to_json_file(&self) -> bool {
        let mut file = File::create(format!("PdmsMeshMgr.json")).unwrap();
        let serialized = serde_json::to_string(&self).unwrap();
        file.write_all(serialized.as_bytes()).unwrap();
        true
    }

    pub fn deserialize_from_json_file() -> anyhow::Result<Self> {
        let mut file = File::open(format!("PdmsMeshMgr.json"))?;
        let mut buf: Vec<u8> = Vec::new();
        file.read_to_end(&mut buf).ok();
        let r = serde_json::from_slice::<Self>(&buf)?;
        Ok(r)
    }


}



pub trait BrepShapeTrait: VerifiedShape + Debug + Send + Sync{

    fn gen_brep_shell(&self) -> Option<Shell>{
        return  None;
    }

    //todo 实现模型的hash，主要是看比列
    //通过比例缩放可以更大的共享几何信息
    fn hash_mesh_params(&self) -> u64 {
        0
    }

    //生成对应的单位长度的模型，比如Dish，就是以R为1的情况生成模型
    fn gen_unit_shape(&self) -> PdmsMesh {
        PdmsMesh::default()
    }

    #[inline]
    fn get_scaled_vec3(&self) -> Vec3 {
        Vec3::ONE
    }

    #[inline]
    fn get_trans(&self) -> TransformSRT {
        TransformSRT{
            rotation: Default::default(),
            translation: Default::default(),
            scale: self.get_scaled_vec3(),
        }
    }

    //直接使用基本体的快速生成
    fn quick_gen_mesh(&self) -> Option<PdmsMesh> {
        None
    }


    fn gen_mesh(&self, tol: Option<f32>) -> PdmsMesh {
        let mut aabb = AABB::new_invalid();
        if let Some(brep) = self.gen_brep_shell() {
            let brep_bbox = gen_bounding_box(&brep);
            let (size, c) = (brep_bbox.diameter(), brep_bbox.center());
            let d = brep_bbox.diagonal() / 2.0;
            aabb = AABB::from_half_extents(
                Point::<f32>::new(c[0] as f32, c[1] as f32, c[2] as f32),
                Vector::<f32>::new(d[0] as f32, d[1] as f32, d[2] as f32),
            );
            if size <= f64::EPSILON {
                return PdmsMesh::default();
            }
            let tolerance = (tol.unwrap_or((TRIANGLE_TOL) as f32)) as f64 * size;
            if let Some(s) = brep.triangulation(tolerance) {

                let polygon = s.to_polygon();
                let vertices = polygon.positions().iter().map(|&x| x.array()).collect::<Vec<_>>();
                let normals = polygon.normals().iter().map(|&x| x.array()).collect::<Vec<_>>();
                let uvs = polygon.uv_coords().iter().map(|x| [x[0] as f32, x[1] as f32]).collect::<Vec<_>>();
                let mut indices = vec![];
                for i in polygon.tri_faces() {
                    indices.push(i[0].pos as u32);
                    indices.push(i[1].pos as u32);
                    indices.push(i[2].pos as u32);
                }
                let a = aabb.mins;
                let b = aabb.maxs;

                let curves = s
                    .edge_iter()
                    .map(|edge| edge.get_curve())
                    .collect::<Vec<_>>();
                let wf_vertices: Vec<[f32; 3]> = curves
                    .iter()
                    .flat_map(|poly| poly.iter())
                    .map(|p| p.cast().unwrap().into())
                    .collect();
                let mut counter = 0;
                let wf_indices: Vec<u32> = curves
                    .iter()
                    .flat_map(|poly| {
                        let len = counter as u32;
                        counter += poly.len();
                        (1..poly.len()).flat_map(move |i| vec![len + i as u32 - 1, len + i as u32])
                    })
                    .collect();

                return PdmsMesh {
                    indices,
                    vertices,
                    normals,
                    wf_indices,
                    wf_vertices,
                    aabb: AiosAABB::new(Vec3::new(a.x, a.y, a.z), Vec3::new(b.x, b.y, b.z)),
                };
            }
        }
        PdmsMesh::default()
    }
}


pub trait BrepMathTrait {
    fn vector3(&self) -> Vector3;
    fn point3(&self) -> Point3;
}

impl BrepMathTrait for Vec3 {
    #[inline]
    fn vector3(&self) -> Vector3 {
        Vector3::new(self[0] as f64, self[1] as f64, self[2] as f64)
    }

    #[inline]
    fn point3(&self) -> Point3 {
        Point3::new(self[0] as f64, self[1] as f64, self[2] as f64)
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
