use std::collections::hash_map::DefaultHasher;
use std::f32::consts::PI;
use std::f32::EPSILON;
use std::hash::{Hash, Hasher};

use crate::tool::hash_tool::*;
use truck_meshalgo::prelude::*;
use crate::shape::pdms_shape::VerifiedShape;
use bevy_ecs::reflect::ReflectComponent;
use glam::Vec3;

use truck_modeling::builder::try_attach_plane;
use serde::{Serialize, Deserialize};
use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::pdms_types::AttrMap;
use crate::prim_geo::helper::cal_ref_axis;

use bevy_ecs::prelude::*;
#[cfg(feature = "opencascade")]
use opencascade::{OCCShape, Edge, Wire, Axis, Vertex};

#[derive(Component, Debug, Clone, Serialize, Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, )]

pub struct Pyramid {
    pub pbax_pt: Vec3,
    pub pbax_dir: Vec3,   //B Axis Direction

    pub pcax_pt: Vec3,
    pub pcax_dir: Vec3,   //C Axis Direction

    pub paax_pt: Vec3,
    pub paax_dir: Vec3,   //A Axis Direction

    pub pbtp: f32,
    //x top
    pub pctp: f32,  //y top

    pub pbbt: f32,
    // x bottom
    pub pcbt: f32,  // y bottom

    pub ptdi: f32,
    //dist to top
    pub pbdi: f32,  //dist to bottom

    pub pbof: f32,
    // x offset
    pub pcof: f32,  // y offset
}


impl Default for Pyramid {
    fn default() -> Self {
        Self {
            pbax_pt: Default::default(),
            pbax_dir: Vec3::X,

            pcax_pt: Default::default(),
            pcax_dir: Vec3::Y,

            paax_pt: Default::default(),
            paax_dir: Vec3::Z,

            pbtp: 1.0,
            pctp: 1.0,
            pbbt: 1.0,
            pcbt: 1.0,
            ptdi: 1.0,
            pbdi: 0.0,
            pbof: 0.0,
            pcof: 0.0,
        }
    }
}

impl VerifiedShape for Pyramid {
    fn check_valid(&self) -> bool {
        (self.pbtp + self.pctp) > f32::EPSILON || (self.pbbt + self.pcbt) > f32::EPSILON
    }
}

impl From<&AttrMap> for Pyramid {
    fn from(m: &AttrMap) -> Self {
        let xbot = m.get_val("XBOT").unwrap().f32_value().unwrap_or_default();
        let ybot = m.get_val("YBOT").unwrap().f32_value().unwrap_or_default();

        let xtop = m.get_val("XTOP").unwrap().f32_value().unwrap_or_default();
        let ytop = m.get_val("YTOP").unwrap().f32_value().unwrap_or_default();

        let xoff = m.get_val("XOFF").unwrap().f32_value().unwrap_or_default();
        let yoff = m.get_val("YOFF").unwrap().f32_value().unwrap_or_default();

        let height = m.get_val("HEIG").unwrap().f32_value().unwrap_or_default();


        Pyramid {
            pbax_pt: Default::default(),
            pbax_dir: Vec3::X,
            pcax_pt: Default::default(),
            pcax_dir: Vec3::Y,
            paax_pt: Default::default(),
            paax_dir: Vec3::Z,
            pbtp: xtop,
            pctp: ytop,
            pbbt: xbot,
            pcbt: ybot,
            ptdi: height / 2.0,
            pbdi: -height / 2.0,
            pbof: xoff,
            pcof: yoff,
        }
    }
}

impl From<AttrMap> for Pyramid {
    fn from(m: AttrMap) -> Self {
        (&m).into()
    }
}
