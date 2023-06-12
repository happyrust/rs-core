use std::f32::EPSILON;
use bevy::prelude::*;
use truck_base::cgmath64::Vector3;
use truck_meshalgo::prelude::{MeshableShape, MeshedShape};
use truck_modeling::{builder, Shell, Solid};
use bevy::reflect::Reflect;
use bevy::ecs::reflect::ReflectComponent;
use serde::{Serialize, Deserialize};
use crate::consts::BOX_HASH;
use crate::parsed_data::CateBoxParam;
use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::pdms_types::AttrMap;
use crate::prim_geo::CUBE_GEO_HASH;

use crate::shape::pdms_shape::{BrepMathTrait, BrepShapeTrait, PlantMesh, VerifiedShape};

#[derive(Component, Debug, Clone, Serialize, Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, )]
pub struct SBox {
    pub center: Vec3,
    pub size: Vec3,
}

impl Default for SBox {
    fn default() -> Self {
        SBox {
            center: Default::default(),
            size: Vec3::new(1.0, 1.0, 1.0),
        }
    }
}

impl VerifiedShape for SBox {
    #[inline]
    fn check_valid(&self) -> bool {
        self.size.x > f32::EPSILON && self.size.y > f32::EPSILON && self.size.z > f32::EPSILON
    }
}

//#[typetag::serde]
impl BrepShapeTrait for SBox {
    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    fn apply_limit_by_size(&mut self, l: f32) {
        self.size.x = self.size.x.min(l);
        self.size.y = self.size.y.min(l);
        self.size.z = self.size.z.min(l);
    }

    fn gen_brep_shell(&self) -> Option<Shell> {
        if !self.check_valid() { return None; }
        let v = builder::vertex((self.center - self.size / 2.0).point3());
        let e = builder::tsweep(&v, Vector3::unit_x() * self.size.x as f64);
        let f = builder::tsweep(&e, Vector3::unit_y() * self.size.y as f64);
        let mut s = builder::tsweep(&f, Vector3::unit_z() * self.size.z as f64).into_boundaries();
        s.pop()
    }

    fn hash_unit_mesh_params(&self) -> u64 {
        CUBE_GEO_HASH
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(Self::default())
    }

    #[inline]
    fn get_scaled_vec3(&self) -> Vec3 {
        self.size
    }

    fn convert_to_geo_param(&self) -> Option<PdmsGeoParam> {
        Some(PdmsGeoParam::PrimBox(self.clone()))
    }
}


impl From<&AttrMap> for SBox {
    fn from(m: &AttrMap) -> Self {
        SBox {
            center: Default::default(),
            size: Vec3::new(m.get_f32("XLEN").unwrap_or_default(),
                            m.get_f32("YLEN").unwrap_or_default(),
                            m.get_f32("ZLEN").unwrap_or_default(), ),
        }
    }
}

impl From<AttrMap> for SBox {
    fn from(m: AttrMap) -> Self {
        (&m).into()
    }
}



