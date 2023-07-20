use glam::{Vec3};
use crate::prim_geo::cylinder::SCylinder;
use bevy_math::prelude::Quat;
use std::default::default;
use approx::abs_diff_eq;
use bevy_transform::prelude::Transform;
use crate::pdms_types::RefU64;
use crate::prim_geo::category::CateBrepShape;
use serde::{Serialize, Deserialize};
use crate::parsed_data::CateSCylinderParam;
use crate::parsed_data::geo_params_data::PdmsGeoParam;
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use crate::prim_geo::sbox::SBox;
use crate::shape::pdms_shape::{BrepShapeTrait, TRI_TOL};
use glam::Mat3;
use crate::tool::math_tool::{quat_to_pdms_ori_str, to_pdms_vec_str};
use crate::shape::pdms_shape::ANGLE_RAD_TOL;

#[serde_as]
#[derive(Debug, Clone,  Serialize, Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize,)]
pub struct PdmsTubing {
    #[serde(rename="_key")]
    #[serde_as(as = "DisplayFromStr")]
    pub leave_refno: RefU64,
    #[serde_as(as = "DisplayFromStr")]
    pub arrive_refno: RefU64,
    pub start_pt: Vec3,
    pub end_pt: Vec3,
    pub desire_leave_dir: Vec3,
    pub leave_ref_dir: Option<Vec3>,
    pub desire_arrive_dir: Vec3,
    pub tubi_size: TubiSize,
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
    pub tubi_size: TubiSize,
    pub bran_name: String,
}

impl TubiEdge {
    pub fn new_from_edge() -> Self{
        Self{
            ..default()
        }
    }
}

#[serde_as]
#[derive(PartialEq, Default, Debug, Clone, Copy, Serialize, Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize,)]
pub enum TubiSize{
    #[default]
    None,
    BoreSize(f32),
    BoxSize((f32, f32)),
}


impl PdmsTubing {

    ///获得方向
    #[inline]
    pub fn get_dir(&self) -> Vec3 {
        (self.end_pt - self.start_pt).normalize_or_zero()
    }

    /// 是否方向是ok的
    #[inline]
    pub fn is_dir_ok(&self) -> bool {
        // return true;
        let a = self.desire_leave_dir.normalize_or_zero();
        let b = -self.desire_arrive_dir.normalize_or_zero();
        let c = self.get_dir();
        abs_diff_eq!(a.dot(c).abs(), 1.0, epsilon=0.01) && abs_diff_eq!(b.dot(c).abs(), 1.0, epsilon=0.01)
    }

    /// 获得tubi的transform
    pub fn get_transform(&self) -> Option<Transform>{
        let v = self.end_pt - self.start_pt;
        let len = v.length();
        let is_bore = matches!(self.tubi_size, TubiSize::BoreSize(_));
        let z_dir = if is_bore {
            v.normalize_or_zero()
        }else {
            self.desire_leave_dir.normalize_or_zero()
        };

        if self.tubi_size == TubiSize::None || z_dir.length().abs() < f32::EPSILON {
            return  None;
        }
        let scale = match self.tubi_size {
            TubiSize::BoreSize(bore) => Vec3::new(bore, bore, len),
            TubiSize::BoxSize((w, h)) => Vec3::new(w, h, len),
            _ => Vec3::ONE,
        };
        let rotation = if is_bore {
            Quat::from_rotation_arc(Vec3::Z, z_dir)
        }else if let Some(y_dir) = self.leave_ref_dir{
                // dbg!(to_pdms_vec_str(&y_dir));
                let x_dir = y_dir.cross(z_dir).normalize_or_zero();
                //考虑平行的情况
                if x_dir.length() < ANGLE_RAD_TOL {
                    Quat::from_rotation_arc(Vec3::Z, z_dir)
                } else {
                    Quat::from_mat3(&Mat3::from_cols(x_dir, y_dir, z_dir))
                }
        }else{
            Quat::from_rotation_arc(Vec3::Z, z_dir)
        };

        let translation = match self.tubi_size {
            TubiSize::BoreSize(_) => self.start_pt,
            TubiSize::BoxSize(_) => self.start_pt + rotation * (v * 0.5),
            _ => self.start_pt,
        };

        Some(Transform {
            rotation,
            translation,
            scale,
        })
    }

    pub fn convert_to_shape(&self) -> Option<CateBrepShape> {
        let dir = (self.end_pt - self.start_pt).normalize();
        let brep_shape: Option<Box<dyn BrepShapeTrait>> = match &self.tubi_size {
            TubiSize::BoreSize(d) => {
                let mut cylinder = SCylinder {
                    phei: self.start_pt.distance(self.end_pt),
                    pdia: *d,
                    center_in_mid: false,
                    ..default()
                };
                Some(Box::new(cylinder))
            },
            TubiSize::BoxSize((w, h)) =>{
                let len = self.start_pt.distance(self.end_pt);
                let size = Vec3::new(*w, *h, len);
                let mut cube = SBox {
                    center: Default::default(),
                    size,
                };
                Some(Box::new(cube))
            },
            _ => {
                None
            }
        };
        if brep_shape.is_none() { return None; }

        Some(CateBrepShape {
            refno: self.leave_refno,
            brep_shape: brep_shape.unwrap(),
            transform: Transform {
                rotation: Quat::from_rotation_arc(Vec3::Z, dir),
                translation: self.start_pt,
                scale: Vec3::ONE,
            },
            visible: true,
            is_tubi: true,
            shape_err: None,
            pts: Default::default(),
            is_ngmr: false,
        })
    }
}