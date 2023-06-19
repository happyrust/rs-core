use std::collections::hash_map::DefaultHasher;
use std::f32::consts::{FRAC_PI_2, PI, TAU};
use std::f32::EPSILON;
use std::hash::{Hash, Hasher};
use anyhow::anyhow;

use approx::{abs_diff_eq, abs_diff_ne};
use bevy::ecs::reflect::ReflectComponent;
use bevy::prelude::*;
use bevy::reflect::Reflect;
use glam::{Vec3};
use serde::{Deserialize, Serialize};

use crate::parsed_data::{CateProfileParam, SannData, SProfileData};
use crate::prim_geo::helper::cal_ref_axis;
use crate::prim_geo::spine::*;
use crate::prim_geo::wire;
use crate::shape::pdms_shape::{BrepMathTrait,  convert_to_cg_matrix4, PlantMesh, TRI_TOL, VerifiedShape};
use crate::tool::float_tool::{hash_f32, hash_vec3};


///含有两边方向的，扫描体
#[derive(Component, Debug, Clone, Serialize, Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, )]
pub struct SweepSolid {
    pub profile: CateProfileParam,
    pub drns: Vec3,
    pub drne: Vec3,
    pub bangle: f32,
    pub plane_normal: Vec3,
    pub extrude_dir: Vec3,
    pub height: f32,
    pub path: SweepPath3D,
}


impl SweepSolid {
    #[inline]
    pub fn is_sloped(&self) -> bool {
        self.is_drns_sloped() || self.is_drne_sloped()
    }

    #[inline]
    pub fn is_drns_sloped(&self) -> bool {
        let dot_s = self.drns.dot(self.extrude_dir);
        abs_diff_ne!(dot_s.abs(), 1.0, epsilon = 0.01) && abs_diff_ne!(dot_s.abs(), 0.0, epsilon = 0.01)
    }

    #[inline]
    pub fn is_drne_sloped(&self) -> bool {
        let dot_e = self.drne.dot(self.extrude_dir);
        abs_diff_ne!(dot_e.abs(), 1.0, epsilon = 0.01) && abs_diff_ne!(dot_e.abs(), 0.0, epsilon = 0.01)
    }

}

impl Default for SweepSolid {
    fn default() -> Self {
        Self {
            profile: CateProfileParam::UNKOWN,
            drns: Default::default(),
            drne: Default::default(),
            bangle: 0.0,
            plane_normal: Vec3::Z,
            extrude_dir: Vec3::Z,
            ..default()
        }
    }
}

impl VerifiedShape for SweepSolid {
    fn check_valid(&self) -> bool { !self.extrude_dir.is_nan() && self.extrude_dir.length() > 0.0 }
}


