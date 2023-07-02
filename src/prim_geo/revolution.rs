use std::collections::hash_map::DefaultHasher;
use std::f32::consts::{PI, TAU};
use std::f32::EPSILON;
use std::hash::{Hash, Hasher};
use anyhow::anyhow;
use approx::abs_diff_eq;
use crate::shape::pdms_shape::VerifiedShape;
use crate::tool::hash_tool::*;
use truck_meshalgo::prelude::*;

use bevy_ecs::reflect::ReflectComponent;
use glam::{Vec2, Vec3};
use crate::pdms_types::AttrMap;
use serde::{Serialize, Deserialize};
use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::prim_geo::extrusion::Extrusion;
use crate::prim_geo::wire::*;
use crate::prim_geo::helper::cal_ref_axis;

use crate::tool::float_tool::{hash_f32, hash_vec3};
use bevy_ecs::prelude::*;
#[cfg(feature = "opencascade")]
use opencascade::{OCCShape, Edge, Wire, Axis};
#[cfg(feature = "gen_model")]
use crate::csg::manifold::*;

#[derive(Component, Debug, Clone, Serialize, Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, )]
pub struct Revolution {
    pub verts: Vec<Vec3>,
    //loop vertex
    pub fradius_vec: Vec<f32>,
    pub angle: f32,
    //degrees
    pub rot_dir: Vec3,
    pub rot_pt: Vec3,
}


impl Default for Revolution {
    fn default() -> Self {
        Self {
            verts: vec![Vec3::ZERO, Vec3::new(2.0, 0.0, 0.0), Vec3::new(2.0, 1.0, 0.0),
                        Vec3::new(1.0, 1.0, 0.0), Vec3::new(1.0, 2.0, 0.0), Vec3::new(0.0, 2.0, 0.0)],
            fradius_vec: vec![0.0; 6],
            angle: 90.0,
            rot_dir: Vec3::X,   //默认绕X轴旋转
            rot_pt: Vec3::ZERO, //默认旋转点
        }
    }
}

impl VerifiedShape for Revolution {
    fn check_valid(&self) -> bool {
        self.angle.abs() > std::f32::EPSILON
    }
}