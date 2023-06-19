use std::collections::hash_map::DefaultHasher;
use std::f32::EPSILON;
use std::f64::consts::PI;
use std::hash::Hash;
use std::hash::Hasher;

use approx::{abs_diff_eq, abs_diff_ne};
use bevy::prelude::*;
use nom::Parser;
use serde::{Deserialize, Serialize};
use truck_topology::Face;
use crate::parsed_data::geo_params_data::PdmsGeoParam;

use crate::pdms_types::AttrMap;
use crate::prim_geo::CYLINDER_GEO_HASH;
use crate::prim_geo::helper::cal_ref_axis;
use crate::shape::pdms_shape::{BrepMathTrait,  PlantMesh, TRI_TOL, VerifiedShape};
use crate::tool::float_tool::hash_f32;

#[cfg(feature = "opencascade")]
use opencascade::{OCCShape, Edge, Wire, Axis, Vertex, DsShape};


#[derive(Component, Debug, Clone, Reflect, Serialize, Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, )]
// #[reflect(Component)]
pub struct LCylinder {
    pub paxi_expr: String,
    pub paxi_pt: Vec3,
    //A Axis point
    pub paxi_dir: Vec3,   //A Axis Direction

    pub pbdi: f32,
    //dist to bottom
    pub ptdi: f32,
    //dist to top
    pub pdia: f32,
    //diameter
    pub negative: bool,
}


impl Default for LCylinder {
    fn default() -> Self {
        LCylinder {
            paxi_expr: "Z".to_string(),
            paxi_pt: Default::default(),
            paxi_dir: Vec3::new(0.0, 0.0, 1.0),
            pbdi: -0.5,
            ptdi: 0.5,
            pdia: 1.0,
            negative: false,
        }
    }
}

impl VerifiedShape for LCylinder {
    fn check_valid(&self) -> bool {
        self.pdia > f32::EPSILON && (self.pbdi - self.ptdi).abs() > f32::EPSILON
    }
}


impl From<&AttrMap> for LCylinder {
    fn from(m: &AttrMap) -> Self {
        let pdia = m.get_val("DIAM").unwrap().double_value().unwrap() as f32;
        let pbdi = m.get_val("PBDI").unwrap().double_value().unwrap() as f32;
        let ptdi = m.get_val("PTDI").unwrap().double_value().unwrap() as f32;
        LCylinder {
            paxi_expr: "Z".to_string(),
            paxi_pt: Default::default(),
            paxi_dir: Vec3::Z,
            pbdi,
            ptdi,
            negative: false,
            pdia,
        }
    }
}

impl From<AttrMap> for LCylinder {
    fn from(m: AttrMap) -> Self {
        (&m).into()
    }
}


#[derive(Component, Debug, Clone, Reflect, Serialize, Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, )]
// #[reflect(Component)]
pub struct SCylinder {
    pub paxi_expr: String,
    pub paxi_pt: Vec3,
    //A Axis point
    pub paxi_dir: Vec3,   //A Axis Direction

    // pub pdis: f32,
    //dist to bottom
    pub phei: f32,
    // height
    pub pdia: f32,
    //diameter
    pub btm_shear_angles: [f32; 2],
    // x shear
    pub top_shear_angles: [f32; 2],
    // y shear
    pub negative: bool,

    pub center_in_mid: bool,
}

impl Default for SCylinder {
    fn default() -> Self {
        Self {
            paxi_expr: "Z".to_string(),
            paxi_dir: Vec3::Z,
            paxi_pt: Default::default(),
            phei: 1.0,
            pdia: 1.0,
            btm_shear_angles: [0.0f32; 2],
            top_shear_angles: [0.0f32; 2],
            negative: false,
            center_in_mid: false,
        }
    }
}

impl SCylinder {
    #[inline]
    pub fn is_sscl(&self) -> bool {
        self.btm_shear_angles[0].abs() > f32::EPSILON ||
            self.btm_shear_angles[1].abs() > f32::EPSILON ||
            self.top_shear_angles[0].abs() > f32::EPSILON ||
            self.top_shear_angles[1].abs() > f32::EPSILON
    }
}

impl VerifiedShape for SCylinder {
    #[inline]
    fn check_valid(&self) -> bool {
        self.pdia > f32::EPSILON && self.phei.abs() > f32::EPSILON
    }
}


impl From<&AttrMap> for SCylinder {
    fn from(m: &AttrMap) -> Self {
        let mut phei = m.get_f64("HEIG").unwrap_or_default() as f32;
        let pdia = m.get_f64("DIAM").unwrap_or_default() as f32;
        // dbg!(m);
        SCylinder {
            paxi_expr: "Z".to_string(),
            paxi_pt: Default::default(),
            paxi_dir: Vec3::Z,
            phei,
            pdia,
            btm_shear_angles: [0.0; 2],
            top_shear_angles: [0.0; 2],
            negative: false,
            center_in_mid: true,
        }
    }
}

impl From<AttrMap> for SCylinder {
    fn from(m: AttrMap) -> Self {
        (&m).into()
    }
}