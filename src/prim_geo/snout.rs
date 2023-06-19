use std::collections::hash_map::DefaultHasher;
use std::f32::EPSILON;
use std::hash::Hasher;
use bevy::prelude::*;
use truck_meshalgo::prelude::*;
use truck_modeling::Shell;
use std::hash::Hash;
use serde::{Serialize,Deserialize};
use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::pdms_types::AttrMap;
use crate::shape::pdms_shape::{BrepMathTrait, PlantMesh};
use crate::shape::pdms_shape::{ VerifiedShape};
use crate::tool::float_tool::hash_f32;
use crate::tool::hash_tool::*;



#[derive(Component, Debug, Clone, Reflect, Serialize, Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize,)]
pub struct LSnout {
    pub paax_expr: String,
    pub paax_pt: Vec3,   //A Axis point
    pub paax_dir: Vec3,   //A Axis Direction

    pub pbax_expr: String,
    pub pbax_pt: Vec3,   //B Axis point
    pub pbax_dir: Vec3,   //B Axis Direction

    pub ptdi: f32,      //dist to top
    pub pbdi: f32,      //dist to bottom
    pub ptdm: f32,      //top diameter
    pub pbdm: f32,      //bottom diameter
    pub poff: f32,      //offset

    pub btm_on_top: bool,
}

impl Default for LSnout {
    fn default() -> Self {
        Self {
            paax_expr: "Z".to_string(),
            paax_pt: Default::default(),
            paax_dir: Vec3::Z,

            pbax_expr: "X".to_string(),
            pbax_pt: Default::default(),
            pbax_dir: Vec3::X,

            ptdi: 0.5,
            pbdi: -0.5,
            ptdm: 1.0,
            pbdm: 1.0,
            poff: 0.0,
            btm_on_top: false,
        }
    }
}

impl VerifiedShape for LSnout {
    #[inline]
    fn check_valid(&self) -> bool {
        //height 必须 >0， 小于0 的情况直接用变换矩阵
        self.ptdm >= 0.0 && self.pbdm >= 0.0  && (self.ptdi - self.pbdi) > f32::EPSILON
    }
}

impl From<&AttrMap> for LSnout {
    fn from(m: &AttrMap) -> Self {
        let h = m.get_val("HEIG").unwrap().double_value().unwrap() as f32 ;
        LSnout {
            ptdi: h / 2.0,
            pbdi: -h / 2.0,
            ptdm: m.get_val("DTOP").unwrap().double_value().unwrap() as f32 ,
            pbdm: m.get_val("DBOT").unwrap().double_value().unwrap() as f32 ,
            ..Default::default()
        }
    }
}

impl From<AttrMap> for LSnout {
    fn from(m: AttrMap) -> Self {
        (&m).into()
    }
}


