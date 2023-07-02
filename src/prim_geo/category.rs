use std::default::default;
use std::f32::consts::PI;
use std::f32::EPSILON;
use std::ops::Range;

use bevy_math::prelude::*;
use bevy_transform::prelude::Transform;
use id_tree::NodeId;
use smallvec::SmallVec;

use crate::parsed_data::geo_params_data::{CateGeoParam, PdmsGeoParam};
use crate::pdms_types::RefU64;
use crate::prim_geo::ctorus::{CTorus, SCTorus};
use crate::prim_geo::cylinder::SCylinder;
use crate::prim_geo::dish::Dish;
use crate::prim_geo::extrusion::Extrusion;
use crate::prim_geo::lpyramid::LPyramid;
use crate::prim_geo::pyramid::Pyramid;
use crate::prim_geo::revolution::Revolution;
use crate::prim_geo::rtorus::{RTorus, SRTorus};
use crate::prim_geo::sbox::SBox;
use crate::prim_geo::snout::LSnout;
use crate::prim_geo::sphere::Sphere;
use crate::prim_geo::tubing::PdmsTubing;

#[derive(Debug, Clone)]
pub enum ShapeErr {
    //tubi的方向不一致
    TubiDirErr,
    Unknown,
}