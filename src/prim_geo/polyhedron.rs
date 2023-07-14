use std::collections::hash_map::DefaultHasher;
use std::f32::consts::PI;
use std::f32::EPSILON;
use std::hash::{Hash, Hasher};
use approx::abs_diff_eq;
use bevy_ecs::reflect::ReflectComponent;
use glam::Vec3;
use serde::{Deserialize, Serialize};
use truck_meshalgo::prelude::*;
use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::tool::hash_tool::*;
use crate::pdms_types::AttrMap;
use crate::prim_geo::helper::cal_ref_axis;
use crate::shape::pdms_shape::{BevyMathTrait, BrepMathTrait, BrepShapeTrait, PlantMesh, VerifiedShape};
#[cfg(feature = "opencascade")]
use opencascade::{OCCShape, Edge, Wire, Axis, Vertex};
use bevy_ecs::prelude::*;
use itertools::Itertools;
use truck_modeling::Face;
use truck_topology::Shell;
use crate::prim_geo::wire::gen_wire;


#[derive(Component, Debug, Clone, Serialize, Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, )]
pub struct Polyhedron {
    pub polygons: Vec<Polygon>,
}

#[derive(Component, Debug, Clone, Serialize, Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, )]
pub struct Polygon {
    pub verts: Vec<Vec3>,
}

impl Polygon {
    pub fn gen_face(&self) -> anyhow::Result<Face>{
        use truck_modeling::{builder, Shell, Surface, Wire};
        use truck_meshalgo::prelude::*;
        let mut fradius_vec = vec![];
        fradius_vec.resize(self.verts.len(), 0.0);
        let wire = gen_wire(&self.verts, &fradius_vec)?;
        Ok(builder::try_attach_plane(&[wire])?)
    }
}

impl VerifiedShape for Polyhedron {
    fn check_valid(&self) -> bool {
        !self.polygons.is_empty()
    }
}

impl BrepShapeTrait for Polyhedron  {
    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    fn hash_unit_mesh_params(&self) -> u64 {
        let bytes = bincode::serialize(self).unwrap();
        let mut hasher = DefaultHasher::default();
        bytes.hash(&mut hasher);
        "Polyhedron".hash(&mut hasher);
        hasher.finish()
    }


    fn gen_brep_shell(&self) -> Option<truck_modeling::Shell> {
        use truck_modeling::*;
        use truck_modeling::builder::*;

        let mut faces = vec![];
        for poly  in &self.polygons {
            if let Ok(face) = poly.gen_face() {
                faces.push(face);
            }
        }
        let shell: Shell = faces.into();
        Some(shell)
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    fn convert_to_geo_param(&self) -> Option<PdmsGeoParam> {
        Some(
            PdmsGeoParam::PrimPolyhedron(self.clone())
        )
    }


}