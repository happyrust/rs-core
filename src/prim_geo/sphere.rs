use std::f64::consts::PI;
use std::f32::EPSILON;
use bevy::prelude::*;
use truck_base::cgmath64::Vector3;
use truck_meshalgo::prelude::{MeshableShape, MeshedShape};
use truck_modeling::{builder, Shell, Solid};
use crate::tool::hash_tool::*;
use bevy::reflect::Reflect;
use bevy::ecs::reflect::ReflectComponent;

use lyon::math::size;
use hexasphere::shapes::IcoSphere;
use nalgebra::Point3;
use parry3d::bounding_volume::Aabb;
use parry3d::math::{Point, Vector};
use serde::{Serialize,Deserialize};
use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::prim_geo::SPHERE_GEO_HASH;
use crate::shape::pdms_shape::{BrepMathTrait,  PlantMesh, VerifiedShape};
use crate::pdms_types::AttrMap;

#[derive(Component, Debug, Clone, Reflect, Serialize, Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize,)]
// #[reflect(Component)]
pub struct Sphere {
    pub center: Vec3,
    pub radius: f32,
}

impl Default for Sphere {
    fn default() -> Self {
        Sphere {
            center: Default::default(),
            radius: 1.0,
        }
    }
}



impl VerifiedShape for Sphere {
    #[inline]
    fn check_valid(&self) -> bool {
        self.radius > f32::EPSILON
    }
}

impl From<&AttrMap> for Sphere {
    fn from(m: &AttrMap) -> Self {
        Self {
            center: Default::default(),
            
            // size: Vec3::new(m.get_f32("XLEN").unwrap_or_default(),
            //                 m.get_f32("YLEN").unwrap_or_default(),
            //                 m.get_f32("ZLEN").unwrap_or_default(),),
            radius: m.get_f32("RADI").unwrap_or_default(),
        }
    }
}





