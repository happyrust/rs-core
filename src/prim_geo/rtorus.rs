use std::collections::hash_map::DefaultHasher;
use std::default::default;
use std::f32::EPSILON;
use std::hash::Hasher;
use std::hash::Hash;
use anyhow::anyhow;
use glam::{Mat3, Quat, Vec3};
use bevy_ecs::prelude::*;
use truck_modeling::{builder, Shell};
use crate::tool::hash_tool::*;
use crate::shape::pdms_shape::VerifiedShape;
use bevy_ecs::reflect::ReflectComponent;
use crate::pdms_types::AttrMap;
use serde::{Serialize, Deserialize};
use crate::parsed_data::geo_params_data::PdmsGeoParam;
use bevy_ecs::prelude::*;
use crate::prim_geo::helper::*;
use crate::shape::pdms_shape::*;
use crate::tool::float_tool::hash_f32;
use bevy_ecs::prelude::*;

#[cfg(feature = "opencascade")]
use opencascade::{OCCShape, Edge, Wire, Axis, Vertex};
use bevy_transform::prelude::Transform;
#[derive(Component, Debug, Clone, Serialize, Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, )]

pub struct SRTorus {
    pub paax_expr: String,
    pub paax_pt: Vec3,
    //A Axis point
    pub paax_dir: Vec3,   //A Axis Direction

    pub pbax_expr: String,
    pub pbax_pt: Vec3,
    //B Axis point
    pub pbax_dir: Vec3,   //B Axis Direction

    pub pheig: f32,
    pub pdia: f32,

}


impl Default for SRTorus {
    fn default() -> Self {
        Self {
            paax_expr: "X".to_string(),
            paax_pt: Vec3::new(5.0, 0.0, 0.0),
            paax_dir: Vec3::X,

            pbax_expr: "Y".to_string(),
            pbax_pt: Vec3::new(0.0, 5.0, 0.0),
            pbax_dir: Vec3::Y,
            pheig: 1.0,
            pdia: 1.0,
        }
    }
}

#[derive(Default)]
struct TorusInfo {
    pub center: Vec3,
    pub angle: f32,
    pub rot_axis: Vec3,
    pub radius: f32,
}

impl SRTorus {
    pub fn convert_to_rtorus(&self) -> Option<(RTorus, Transform)> {
        if let Some(torus_info) = RotateInfo::cal_rotate_info(self.paax_dir,
                                                              self.paax_pt, self.pbax_dir, self.pbax_pt, self.pdia/2.0) {
            let mut rtorus = RTorus::default();
            rtorus.angle = torus_info.angle;
            rtorus.height = self.pheig;
            rtorus.rins = torus_info.radius - self.pdia / 2.0;
            rtorus.rout = torus_info.radius + self.pdia / 2.0;
            let z_axis = -torus_info.rot_axis.normalize();
            let x_axis = (self.pbax_pt - torus_info.center).normalize();
            let y_axis = z_axis.cross(x_axis).normalize();
            let translation = torus_info.center;
            let mat = Transform {
                rotation: Quat::from_mat3(&Mat3::from_cols(
                    x_axis, y_axis, z_axis,
                )),
                translation,
                ..default()
            };
            return Some((rtorus, mat));
        }

        None
    }
}

impl VerifiedShape for SRTorus {
    fn check_valid(&self) -> bool {
        self.pheig > 0.0 && self.pdia > 0.0
    }
}


impl From<AttrMap> for SRTorus {
    fn from(_: AttrMap) -> Self {
        Default::default()
    }
}

#[derive(Component, Debug, Clone, Serialize, Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, )]
pub struct RTorus {
    pub rins: f32,
    //内圆半径
    pub rout: f32,
    //外圆半径
    pub height: f32,
    pub angle: f32,  //旋转角度
}

impl Default for RTorus {
    fn default() -> Self {
        Self {
            rins: 0.5,
            rout: 1.0,
            height: 1.0,
            angle: 90.0,
        }
    }
}

impl VerifiedShape for RTorus {
    #[inline]
    fn check_valid(&self) -> bool {
        self.rout > 0.0 && self.angle.abs() > 0.0 && (self.rout - self.rins) > f32::EPSILON && self.height > f32::EPSILON
    }
}

impl From<&AttrMap> for RTorus {
    fn from(m: &AttrMap) -> Self {
        let rins = m.get_f32("RINS").unwrap();
        let rout = m.get_f32("ROUT").unwrap();
        let height = m.get_f32("HEIG").unwrap();
        let angle = m.get_f32("ANGL").unwrap();
        RTorus {
            rins,
            rout,
            height,
            angle,
        }
    }
}

impl From<AttrMap> for RTorus {
    fn from(m: AttrMap) -> Self {
        (&m).into()
    }
}

