use std::collections::hash_map::DefaultHasher;
use std::f32::consts::PI;
use std::f32::EPSILON;
use std::hash::{Hash, Hasher};

use bevy::ecs::reflect::ReflectComponent;
use bevy::prelude::*;
use bevy::reflect::Reflect;
use serde::{Deserialize, Serialize};
use truck_meshalgo::prelude::*;
use truck_modeling::{builder, Shell};
use crate::parsed_data::geo_params_data::PdmsGeoParam;

use crate::pdms_types::AttrMap;
use crate::prim_geo::helper::cal_ref_axis;
use crate::shape::pdms_shape::{BrepMathTrait,  PlantMesh, TRI_TOL, VerifiedShape};
use crate::tool::float_tool::hash_f32;
use crate::tool::hash_tool::*;

//可不可以用来表达 sphere
#[derive(Component, Debug, Clone, Reflect, Serialize, Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize,)]
#[reflect(Component)]
pub struct Dish {
    pub paax_expr: String,
    pub paax_pt: Vec3,
    //Axis point
    pub paax_dir: Vec3,  //Axis Direction

    pub pdis: f32,
    pub pheig: f32,
    pub pdia: f32, //diameter
}

impl Default for Dish {
    fn default() -> Self {
        Self {
            paax_expr: "Z".to_string(),
            paax_pt: Default::default(),
            paax_dir: Vec3::Z,
            pdis: 0.0,
            pheig: 1.0,
            pdia: 2.0,
        }
    }
}

impl VerifiedShape for Dish {
    fn check_valid(&self) -> bool {
        self.pdia > f32::EPSILON && self.pheig > f32::EPSILON
    }
}


impl From<&AttrMap> for Dish {
    fn from(m: &AttrMap) -> Self {
        Self {
            paax_expr: "Z".to_string(),
            paax_pt: Default::default(),
            paax_dir: Vec3::Z,
            pdis: 0.0,
            pheig: m.get_val("HEIG").unwrap().f32_value().unwrap_or_default(),
            pdia: m.get_val("DIAM").unwrap().f32_value().unwrap_or_default(),
        }
    }
}

impl From<AttrMap> for Dish {
    fn from(m: AttrMap) -> Self {
        (&m).into()
    }
}
