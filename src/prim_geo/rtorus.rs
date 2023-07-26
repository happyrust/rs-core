use std::collections::hash_map::DefaultHasher;
use std::default;
use std::f32::EPSILON;
use std::hash::Hasher;
use std::hash::Hash;
use anyhow::anyhow;
use glam::{Mat3, Quat, Vec3};
use bevy_ecs::prelude::*;
use truck_modeling::{builder, Shell};
use crate::tool::hash_tool::*;

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
                ..Default::default()
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

//#[typetag::serde]
impl BrepShapeTrait for SRTorus {
    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    #[inline]
    fn tol(&self) -> f32{
        0.01 * self.pdia.min(self.pheig).max(1.0)
    }

    #[cfg(feature = "opencascade")]
    fn gen_occ_shape(&self) -> anyhow::Result<OCCShape> {
        if let Some(torus_info) = RotateInfo::cal_rotate_info(self.paax_dir, self.paax_pt,
                                                              self.pbax_dir, self.pbax_pt, self.pdia/2.0) {
            let z_axis = self.paax_dir.normalize();
            let y_axis = torus_info.rot_axis;
            let x_axis = z_axis.cross(y_axis);
            let h = self.pheig;
            let d = self.pdia;
            let p1 = (self.paax_pt - y_axis * h / 2.0 - x_axis * d / 2.0).into();
            let p2 = (self.paax_pt + y_axis * h / 2.0 - x_axis * d / 2.0).into();
            let p3 = (self.paax_pt + y_axis * h / 2.0 + x_axis * d / 2.0).into();
            let p4 = (self.paax_pt - y_axis * h / 2.0 + x_axis * d / 2.0).into();
            //创建四边形
            let top = Edge::new_line(&p1, &p2)?;
            let right = Edge::new_line(&p2, &p3)?;
            let bottom = Edge::new_line(&p3, &p4)?;
            let left = Edge::new_line(&p4, &p1)?;

            let wire = Wire::from_edges([&top, &right, &bottom, &left].into_iter())?;
            let center = torus_info.center;
            // dbg!(center);
            // dbg!(-y_axis);
            let axis = Axis::new(center, -y_axis);
            return Ok(wire.extrude_rotate(&axis, torus_info.angle.to_radians() as _)?);
        }

        Err(anyhow::anyhow!("Rect torus 参数有问题。"))
    }

    fn gen_brep_shell(&self) -> Option<Shell> {
        if let Some(torus_info) = RotateInfo::cal_rotate_info(self.paax_dir, self.paax_pt,
                                                              self.pbax_dir, self.pbax_pt, self.pdia/2.0) {
            use truck_modeling::*;
            let circle_origin = self.paax_pt.point3();
            let z_axis = self.paax_dir.normalize().vector3();
            let y_axis = torus_info.rot_axis.vector3();
            let x_axis = z_axis.cross(y_axis);
            let h = self.pheig as f64;
            let d = self.pdia as f64;
            let p0 = self.paax_pt.point3() - y_axis * h / 2.0 - x_axis * d / 2.0;
            let v = builder::vertex(p0);
            let e = builder::tsweep(&v, y_axis * h as f64);
            let f = builder::tsweep(&e, x_axis * d as f64);
            let center = torus_info.center.point3();
            let mut solid = builder::rsweep(&f, center, -y_axis,
                                            Rad(torus_info.angle.to_radians() as f64)).into_boundaries();
            return solid.pop();
        }
        None
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
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

//#[typetag::serde]
impl BrepShapeTrait for RTorus {
    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    fn hash_unit_mesh_params(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        hash_f32((self.rins / self.rout), &mut hasher);
        hash_f32(self.angle, &mut hasher);
        "rtorus".hash(&mut hasher);
        hasher.finish()
    }

    #[inline]
    fn get_scaled_vec3(&self) -> Vec3 {
        Vec3::new(self.rout, self.rout, self.height)
    }

    #[inline]
    fn tol(&self) -> f32{
        let d = ((self.rout - self.rins)/2.0 + self.height)/2.0;
        0.01 * d.max(1.0)
    }

    #[cfg(feature = "opencascade")]
    fn gen_occ_shape(&self) -> anyhow::Result<OCCShape> {
        let h = self.height;
        let d = (self.rout - self.rins);
        // dbg!(d);
        let c = (self.rins + self.rout) / 2.0;
        // dbg!(Vec3::new(c - d / 2.0, 0.0, -h / 2.0));
        // dbg!(Vec3::new(c - d / 2.0, 0.0, h / 2.0));
        // dbg!(Vec3::new(c + d / 2.0, 0.0, h / 2.0));
        // dbg!(Vec3::new(c + d / 2.0, 0.0, -h / 2.0));

        let p1 = Vec3::new(c - d / 2.0, 0.0, -h / 2.0).into();
        let p2 = Vec3::new(c - d / 2.0, 0.0, h / 2.0).into();
        let p3 = Vec3::new(c + d / 2.0, 0.0, h / 2.0).into();
        let p4 = Vec3::new(c + d / 2.0, 0.0, -h / 2.0).into();
        //创建四边形
        let top = Edge::new_line(&p1, &p2)?;
        let right = Edge::new_line(&p2, &p3)?;
        let bottom = Edge::new_line(&p3, &p4)?;
        let left = Edge::new_line(&p4, &p1)?;

        let wire = Wire::from_edges([&top, &right, &bottom, &left].into_iter())?;
        let axis = Axis::new(Vec3::ZERO, Vec3::Z);
        return Ok(wire.extrude_rotate(&axis, self.angle.to_radians() as _)?);
    }

    fn gen_brep_shell(&self) -> Option<Shell> {
        use truck_modeling::*;
        //旋转圆心在中间
        let h = self.height as f64;
        let d = (self.rout - self.rins) as f64;
        let p0 = Point3::new(self.rins as f64, 0.0, -h / 2.0);
        let v = builder::vertex(p0);
        let e = builder::tsweep(&v, Vector3::new(0.0, 0.0, h));
        let f = builder::tsweep(&e, Vector3::new(d, 0.0, 0.0));

        let mut solid = builder::rsweep(&f, Point3::new(0.0, 0.0, 0.0),
                                        Vector3::new(0.0, 0.0, 1.0), Rad(self.angle.to_radians() as f64)).into_boundaries();
        return solid.pop();
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
        let rins = self.rins / self.rout;
        let unit = Self {
            rins,
            rout: 1.0,
            height: 1.0,
            angle: self.angle,
        };
        Box::new(unit)
    }

    fn convert_to_geo_param(&self) -> Option<PdmsGeoParam> {
        Some(
            PdmsGeoParam::PrimRTorus(self.clone())
        )
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

