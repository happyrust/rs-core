use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::types::refno::RefnoEnum;
#[cfg(feature = "truck")]
use crate::shape::pdms_shape::BrepMathTrait;
use crate::shape::pdms_shape::{BrepShapeTrait, PlantMesh, RsVec3, TRI_TOL, VerifiedShape};
use anyhow::anyhow;
use bevy_ecs::prelude::*;
use glam::Vec3;
use itertools::Itertools;
use nalgebra::Point;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
#[cfg(feature = "truck")]
use truck_meshalgo::prelude::*;
#[cfg(feature = "truck")]
use truck_modeling::Face;
#[cfg(feature = "truck")]
use truck_modeling::builder::*;

#[derive(
    Component,
    Debug,
    Clone,
    Serialize,
    Deserialize,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub struct Polyhedron {
    pub polygons: Vec<Polygon>,
    pub mesh: Option<PlantMesh>,
    pub is_polyhe: bool,
}

#[derive(
    Component,
    Debug,
    Clone,
    Serialize,
    Deserialize,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub struct Polygon {
    pub loops: Vec<Vec<Vec3>>,
}

impl Polygon {
    #[cfg(feature = "truck")]
    pub fn gen_face(&self) -> anyhow::Result<Face> {
        #[cfg(feature = "truck")]
        use truck_meshalgo::prelude::*;
        #[cfg(feature = "truck")]
        use truck_modeling::{Wire, builder};
        if self.verts.len() < 3 {
            return Err(anyhow!("Polygon must have at least 3 vertices"));
        }
        let mut wire = Wire::new();
        let mut verts = self
            .verts
            .iter()
            .map(|x| builder::vertex(x.point3()))
            .collect::<Vec<_>>();
        verts.pop();
        for (v0, v1) in verts.iter().tuple_windows() {
            wire.push_back(builder::line(v0, v1));
        }
        wire.push_back(builder::line(verts.last().unwrap(), verts.first().unwrap()));
        Ok(builder::try_attach_plane(&[wire])?)
    }
}

impl VerifiedShape for Polyhedron {
    fn check_valid(&self) -> bool {
        !self.polygons.is_empty()
    }
}

impl BrepShapeTrait for Polyhedron {
    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    #[inline]
    fn tol(&self) -> f32 {
        // let pts: Vec<parry3d::math::Point<f32>> = self
        //     .polygons
        //     .iter()
        //     .map(|x| x.verts.iter().map(|y| y.clone().into()))
        //     .flatten()
        //     .collect();
        // let profile_aabb = parry3d::bounding_volume::Aabb::from_points(&pts);
        // 0.01 * profile_aabb.bounding_sphere().radius.max(1.0)
        0.01
    }

    #[cfg(feature = "truck")]
    fn gen_brep_shell(&self) -> Option<truck_modeling::Shell> {
        let mut faces = vec![];
        for poly in &self.polygons {
            if let Ok(face) = poly.gen_face() {
                faces.push(face);
            }
        }
        let shell: Shell = faces.into();
        Some(shell)
    }

    fn hash_unit_mesh_params(&self) -> u64 {
        let bytes = bincode::serialize(self).unwrap();
        let mut hasher = DefaultHasher::default();
        bytes.hash(&mut hasher);
        "Polyhedron".hash(&mut hasher);
        hasher.finish()
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    fn convert_to_geo_param(&self) -> Option<PdmsGeoParam> {
        Some(PdmsGeoParam::PrimPolyhedron(self.clone()))
    }

    ///直接通过基本体的参数，生成模型
    fn gen_csg_mesh(&self) -> Option<PlantMesh> {
        use crate::geometry::csg::generate_polyhedron_mesh;
        generate_polyhedron_mesh(self, RefnoEnum::default()).map(|g| g.mesh)
    }

    fn gen_csg_shape(&self) -> anyhow::Result<crate::prim_geo::basic::CsgSharedMesh> {
        if let Some(mesh) = self.gen_csg_mesh() {
            Ok(crate::prim_geo::basic::CsgSharedMesh::new(mesh))
        } else {
            Err(anyhow::anyhow!(
                "Failed to generate CSG mesh for Polyhedron"
            ))
        }
    }

    fn enhanced_key_points(
        &self,
        transform: &bevy_transform::prelude::Transform,
    ) -> Vec<(Vec3, String, u8)> {
        // Polyhedron 是复杂的 Mesh 类型，只返回中心点
        // 计算所有顶点的平均位置作为中心点
        let mut center = Vec3::ZERO;
        let mut count = 0;

        for polygon in &self.polygons {
            for loop_verts in &polygon.loops {
                for vert in loop_verts {
                    center += *vert;
                    count += 1;
                }
            }
        }

        if count > 0 {
            center /= count as f32;
        }

        vec![(transform.transform_point(center), "Center".to_string(), 100)]
    }
}
