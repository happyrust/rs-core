use glam::{TransformSRT, Vec3};
use crate::prim_geo::cylinder::SCylinder;
use bevy::math::Quat;
use std::default::default;
use approx::abs_diff_eq;
use crate::pdms_types::RefU64;
use crate::prim_geo::category::CateBrepShape;

#[derive(Default, Debug, Clone)]
pub struct PdmsTubing{
    pub start_pt: Vec3,
    pub end_pt: Vec3,
    pub desire_leave_dir: Vec3,
    pub desire_arrive_dir: Vec3,
    pub from: RefU64,
    pub to: RefU64,
    pub bore: f32,
    pub finished: bool,  //完整的一个tubing信息
}


impl PdmsTubing{

    #[inline]
    pub fn get_dir(&self) -> Vec3{
        (self.end_pt - self.start_pt).normalize_or_zero()
    }

    #[inline]
    pub fn is_dir_ok(&self) -> bool{
        let a = self.desire_leave_dir.normalize_or_zero();
        let b = -self.desire_arrive_dir.normalize_or_zero();
        let c = self.get_dir();
        abs_diff_eq!(a.dot(c), 1.0, epsilon=0.01) && abs_diff_eq!(b.dot(c), 1.0, epsilon=0.01)
    }

    pub fn convert_to_shape(&self) -> CateBrepShape{
        let dir = (self.end_pt - self.start_pt).normalize();
        let mut cylinder = SCylinder{
            phei: self.start_pt.distance(self.end_pt),
            pdia: self.bore,
            center_in_mid: false,
            ..default()
        };

        CateBrepShape{
            refno: Default::default(),
            brep_shape: Box::new(cylinder),
            transform: TransformSRT{
                rotation: Quat::from_rotation_arc(Vec3::Z, dir),
                translation: self.start_pt,
                scale: Vec3::ONE,
            },
            visible: true,
            is_tubi: true,
            shape_err: None,
            pts: Default::default()
        }
    }
}