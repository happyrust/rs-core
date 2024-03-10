use std::collections::hash_map::DefaultHasher;
use std::f32::consts::PI;

use std::hash::{Hash, Hasher};
use std::thread::sleep;
use glam::Vec3;
use anyhow::anyhow;

use nalgebra::ComplexField;
use serde::{Deserialize, Serialize};
use truck_meshalgo::prelude::*;
use truck_modeling::{Shell};
use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::types::attmap::AttrMap;
use crate::prim_geo::helper::cal_ref_axis;
use crate::shape::pdms_shape::{BrepMathTrait, BrepShapeTrait, LEN_TOL, RsVec3, VerifiedShape};
use crate::tool::float_tool::hash_f32;

use bevy_ecs::prelude::*;
#[cfg(feature = "opencascade_rs")]
use opencascade::primitives::*;
use crate::NamedAttrMap;

//可不可以用来表达 sphere
#[derive(Component, Debug, Clone, Serialize, Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize,)]
pub struct Dish {
    pub paax_expr: String,
    pub paax_pt: Vec3,
    //Axis point
    pub paax_dir: Vec3,  //Axis Direction

    pub pdis: f32,
    pub pheig: f32,
    pub pdia: f32, //diameter
    //r = √[(a^2 - b^2) / a^2]
    #[serde(default)]
    pub prad: f32,
}

impl Default for Dish {
    fn default() -> Self {
        Self {
            paax_expr: "Z".to_string(),
            paax_pt: Default::default(),
            paax_dir: Vec3::Z,
            pdis: 0.0,
            pheig: 1.0,
            pdia: 2.0,
            prad: 0.0,
        }
    }
}

impl VerifiedShape for Dish {
    fn check_valid(&self) -> bool {
        self.pdia > f32::EPSILON && self.pheig > f32::EPSILON
    }
}

/// dish的实现 shape trait
impl BrepShapeTrait for Dish {

    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    ///获得关键点
    fn key_points(&self) -> Vec<RsVec3>{
        let r = self.pdia / 2.0;
        let mut h = self.pheig;
        //是个椭圆, 先暂时按圆来处理，然后再拉伸
        if self.prad > 0.0 {
            h = r;
        }
        let radius = (r * r + h * h) / (2.0f32 * h);
        if radius < f32::EPSILON { return vec![]; }
        let sinval = (r / radius).max(-1.0f32).min(1.0f32);
        let mut theta = (sinval).asin();
        if r < h { theta = PI - theta; }

        let rot_axis = self.paax_dir.normalize();
        let c = rot_axis * self.pdis + self.paax_pt;
        let ref_axis = cal_ref_axis(&rot_axis);
        let p0 = rot_axis * h + c;
        let center = p0 - radius * rot_axis;
        vec![center.into()]
    }

    #[cfg(feature = "opencascade_rs")]
    fn gen_occ_shape(&self) -> anyhow::Result<Shape> {
        Ok(Shape::dish(self.pdia as f64 / 2.0, self.pheig as f64).ok_or(anyhow!("Dish 参数错误"))?)
    }

    fn tol(&self) -> f32 {
        0.001 * self.pdia.max(1.0)
    }

    fn gen_brep_shell(&self) -> Option<Shell> {
        use truck_modeling::*;
        let r = self.pdia / 2.0;
        let mut h = self.pheig;
        //是个椭圆, 先暂时按圆来处理，然后再拉伸
        if self.prad > 0.0 {
            h = r;
        }
        let radius = (r * r + h * h) / (2.0f32 * h);
        if radius < f32::EPSILON { return None; }
        let sinval = (r / radius).max(-1.0f32).min(1.0f32);
        let mut theta = (sinval).asin();
        if r < h { theta = PI - theta; }

        let rot_axis = self.paax_dir.normalize();
        let c = rot_axis * self.pdis + self.paax_pt;
        let ref_axis = cal_ref_axis(&rot_axis);
        let p0 = rot_axis * h + c;
        let center = p0 - radius * rot_axis;
        let p1 = ref_axis * r + c;

        let c = c.point3();
        let v0 = builder::vertex(c);
        let v1 = builder::vertex(p0.point3());
        let v2 = builder::vertex(p1.point3());

        let axis = -ref_axis.cross(rot_axis);
        let curve = builder::circle_arc_with_center(center.point3(), &v1, &v2, axis.vector3(), Rad(theta as f64));
        let wire: Wire = vec![curve, builder::line(&v2, &v0)].into();
        let up_axis = rot_axis.vector3();
        let mut s = builder::cone(&wire, up_axis, Rad(7.0));
        let btm = builder::rsweep(
            &v2,
            c,
            -up_axis,
            Rad(7.0),
        );
        if let Ok(disk) = builder::try_attach_plane(&vec![btm]) {
            s.push(disk);
        }
        Some(s)
    }


    fn hash_unit_mesh_params(&self) -> u64 {
        let r = self.pdia / 2.0;
        let h = self.pheig;
        let radius = (r * r + h * h) / (2.0f32 * h);
        let sinval = (r / radius).max(-1.0f32).min(1.0f32);
        let mut theta = (sinval).asin();
        if radius < f32::EPSILON { return 0; }
        let mut beta = (h / radius / 2.0).atan();
        if r < h {
            theta = PI - theta;
            beta = PI + beta;
        }
        let mut hasher = DefaultHasher::new();

        hash_f32(theta, &mut hasher);
        hash_f32(self.prad, &mut hasher);
        hash_f32(beta, &mut hasher);
        "dish".hash(&mut hasher);
        hasher.finish()
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
        //按比例跳到单位直径圆下
        let dia = self.pdia;
        let h = self.pheig / dia;
        let prad = self.prad / dia;
        Box::new(Self {
            pheig: h,
            pdia: 1.0,
            prad,
            ..Default::default()
        })
    }


    fn get_scaled_vec3(&self) -> Vec3 {
        if self.prad > 0.0 {
            Vec3::new(self.pdia, self.pdia, (self.pheig/(self.pdia/2.0)) * self.pdia)
        }else{
            Vec3::new(self.pdia, self.pdia, self.pdia)
        }
    }

    fn convert_to_geo_param(&self) -> Option<PdmsGeoParam> {
        Some(
            PdmsGeoParam::PrimDish(self.clone())
        )
    }
}

impl From<&AttrMap> for Dish {
    fn from(m: &AttrMap) -> Self {
        Self {
            paax_expr: "Z".to_string(),
            paax_pt: Default::default(),
            paax_dir: Vec3::Z,
            pdis: 0.0,
            pheig: m.get_f32_or_default("HEIG"),
            pdia: m.get_f32_or_default("DIAM"),
            prad: m.get_f32_or_default("RADI"),
        }
    }
}

impl From<AttrMap> for Dish {
    fn from(m: AttrMap) -> Self {
        (&m).into()
    }
}

impl From<&NamedAttrMap> for Dish {
    fn from(m: &NamedAttrMap) -> Self {
        Self {
            paax_expr: "Z".to_string(),
            paax_pt: Default::default(),
            paax_dir: Vec3::Z,
            pdis: 0.0,
            pheig: m.get_f32_or_default("HEIG"),
            pdia: m.get_f32_or_default("DIAM"),
            prad: m.get_f32_or_default("RADI"),
        }
    }
}

impl From<NamedAttrMap> for Dish {
    fn from(m: NamedAttrMap) -> Self {
        (&m).into()
    }
}
