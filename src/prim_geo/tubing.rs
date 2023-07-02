use glam::{Vec3};
use crate::prim_geo::cylinder::SCylinder;
use bevy_math::prelude::Quat;
use std::default::default;
use approx::abs_diff_eq;
use bevy_transform::prelude::Transform;
use crate::pdms_types::RefU64;
use serde::{Serialize, Deserialize};
use crate::parsed_data::CateSCylinderParam;
use crate::parsed_data::geo_params_data::PdmsGeoParam;
use serde_with::serde_as;
use serde_with::DisplayFromStr;

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, )]
pub struct PdmsTubing {
    #[serde(rename = "_key")]
    #[serde_as(as = "DisplayFromStr")]
    pub leave_refno: RefU64,
    #[serde_as(as = "DisplayFromStr")]
    pub arrive_refno: RefU64,
    pub start_pt: Vec3,
    pub end_pt: Vec3,
    pub desire_leave_dir: Vec3,
    pub desire_arrive_dir: Vec3,
    pub bore: f32,
}

// 存放在图数据库的 tubi 的数据
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct TubiEdge {
    pub _key: String,
    pub _from: String,
    pub _to: String,
    pub start_pt: Vec3,
    pub end_pt: Vec3,
    pub att_type: String,
    pub extra_type: String,
    pub bore: f32,
    pub bran_name: String,
}

impl TubiEdge {
    pub fn new_from_edge() -> Self {
        Self {
            ..default()
        }
    }
}


impl PdmsTubing {
    #[inline]
    pub fn get_dir(&self) -> Vec3 {
        (self.end_pt - self.start_pt).normalize_or_zero()
    }

    #[inline]
    pub fn is_dir_ok(&self) -> bool {
        // return true;
        let a = self.desire_leave_dir.normalize_or_zero();
        let b = -self.desire_arrive_dir.normalize_or_zero();
        let c = self.get_dir();
        abs_diff_eq!(a.dot(c).abs(), 1.0, epsilon=0.01) && abs_diff_eq!(b.dot(c).abs(), 1.0, epsilon=0.01)
    }

    pub fn get_transform(&self) -> Option<Transform> {
        let v = (self.end_pt - self.start_pt);
        let dir = v.normalize_or_zero();
        if self.bore.abs() < f32::EPSILON || dir.length().abs() < f32::EPSILON {
            return None;
        }

        Some(Transform {
            rotation: Quat::from_rotation_arc(Vec3::Z, dir),
            translation: self.start_pt,
            scale: Vec3::new(self.bore, self.bore, v.length()),
        })
    }
}