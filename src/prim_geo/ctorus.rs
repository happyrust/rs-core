use std::collections::hash_map::DefaultHasher;
use std::f32::EPSILON;
use std::hash::Hasher;
use std::hash::Hash;
use anyhow::anyhow;
use bevy::prelude::*;
use truck_modeling::{builder, Shell};
use bevy::reflect::Reflect;
use bevy::ecs::reflect::ReflectComponent;
use bevy::reflect::erased_serde::{Error, Serializer};
use crate::tool::hash_tool::*;
use nalgebra_glm::normalize;
use serde::{Serialize, Deserialize};
use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::pdms_types::AttrMap;
use crate::prim_geo::helper::{cal_ref_axis, rotate_from_vec3_to_vec3, RotateInfo};
use crate::shape::pdms_shape::{BrepMathTrait,  PlantMesh, TRI_TOL, VerifiedShape};
use crate::tool::float_tool::hash_f32;

#[cfg(feature = "opencascade")]
use opencascade::{OCCShape, Edge, Wire, Axis};

#[derive(Component, Debug, Clone, Reflect, Serialize, Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, )]
#[reflect(Component)]
pub struct SCTorus {
    pub paax_pt: Vec3,
    //A Axis point
    pub paax_dir: Vec3,   //A Axis Direction

    pub pbax_pt: Vec3,
    //B Axis point
    pub pbax_dir: Vec3,   //B Axis Direction

    pub pdia: f32,
}


impl SCTorus {
    pub fn convert_to_ctorus(&self) -> Option<(CTorus, Transform)> {
        if let Some(torus_info) = RotateInfo::cal_rotate_info(self.paax_dir, self.paax_pt, self.pbax_dir, self.pbax_pt) {
            let mut ctorus = CTorus::default();
            ctorus.angle = torus_info.angle;
            ctorus.rins = torus_info.radius - self.pdia / 2.0;
            ctorus.rout = torus_info.radius + self.pdia / 2.0;
            let z_axis = -torus_info.rot_axis.normalize();
            let x_axis = (self.pbax_pt - torus_info.center).normalize();
            let y_axis = z_axis.cross(x_axis).normalize();
            let mat = Transform {
                rotation: bevy::prelude::Quat::from_mat3(&bevy::prelude::Mat3::from_cols(
                    x_axis, y_axis, z_axis,
                )),
                translation: torus_info.center,
                ..default()
            };
            if mat.is_nan() {
                return None;
            }
            // dbg!(torus_info.radius);
            return Some((ctorus, mat));
        }
        None
    }
}


impl Default for SCTorus {
    fn default() -> Self {
        SCTorus {
            paax_pt: Vec3::new(5.0, 0.0, 0.0),
            paax_dir: Vec3::new(1.0, 0.0, 0.0),//Down

            pbax_pt: Vec3::new(0.0, 5.0, 0.0),
            pbax_dir: Vec3::new(0.0, 1.0, 0.0), //UP
            pdia: 2.0,

        }
    }
}

impl VerifiedShape for SCTorus {
    fn check_valid(&self) -> bool {
        true
    }
}

impl From<AttrMap> for SCTorus {
    fn from(m: AttrMap) -> Self {
        Default::default()
    }
}

#[derive(Component, Debug, Clone, Reflect, Serialize, Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, )]
pub struct CTorus {
    pub rins: f32,
    //内圆半径
    pub rout: f32,
    //外圆半径
    pub angle: f32,  //旋转角度
}

impl Default for CTorus {
    fn default() -> Self {
        Self {
            rins: 0.5,
            rout: 1.0,
            angle: 90.0,
        }
    }
}

impl VerifiedShape for CTorus {
    fn check_valid(&self) -> bool {
        self.rout > 0.0 && self.rins >= 0.0 && self.angle.abs() > 0.0 && (self.rout - self.rins) > f32::EPSILON
    }
}

impl From<&AttrMap> for CTorus {
    fn from(m: &AttrMap) -> Self {
        let r_i = m.get_f64("RINS").unwrap_or_default() as f32;
        let r_o = m.get_f64("ROUT").unwrap_or_default() as f32;
        let angle = m.get_f64("ANGL").unwrap_or_default() as f32;
        CTorus {
            rins: r_i,
            rout: r_o,
            angle,
        }
    }
}

impl From<AttrMap> for CTorus {
    fn from(m: AttrMap) -> Self {
        (&m).into()
    }
}