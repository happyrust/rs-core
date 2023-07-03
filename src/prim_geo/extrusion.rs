use std::collections::hash_map::DefaultHasher;
use std::f32::consts::PI;
use std::f32::EPSILON;
use std::hash::{Hash, Hasher};
use std::default::default;
use anyhow::anyhow;
use approx::abs_diff_eq;
use bevy_ecs::reflect::ReflectComponent;
use crate::shape::pdms_shape::VerifiedShape;
use glam::{DVec3, Vec2, Vec3};
use serde::{Deserialize, Serialize};
use truck_meshalgo::prelude::*;
use truck_modeling::{builder, Shell, Surface, Wire};
use crate::parsed_data::geo_params_data::PdmsGeoParam;

use crate::pdms_types::AttrMap;
use crate::prim_geo::helper::{cal_ref_axis, RotateInfo};
use crate::prim_geo::wire::*;
use crate::shape::pdms_shape::*;
use crate::tool::float_tool::{hash_f32, hash_vec3};
use bevy_ecs::prelude::*;
#[cfg(feature = "gen_model")]
use crate::csg::manifold::*;

#[derive(Component, Debug, Clone, Serialize, Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, )]
pub struct Extrusion {
    pub verts: Vec<Vec3>,
    pub fradius_vec: Vec<f32>,
    pub height: f32,
    pub cur_type: CurveType,
}

impl Default for Extrusion {
    fn default() -> Self {
        Self {
            verts: vec![],
            fradius_vec: vec![],
            height: 100.0,
            cur_type: CurveType::Fill,
        }
    }
}

impl VerifiedShape for Extrusion {
    fn check_valid(&self) -> bool {
        self.height > std::f32::EPSILON
    }
}