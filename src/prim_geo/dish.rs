use std::collections::hash_map::DefaultHasher;
use std::f32::consts::PI;
use std::f32::EPSILON;
use std::hash::{Hash, Hasher};

use bevy::ecs::reflect::ReflectComponent;
use bevy::prelude::*;
use bevy::reflect::Reflect;
use serde::{Deserialize, Serialize};
use truck_meshalgo::prelude::*;
use truck_modeling::{builder, Shell};
use crate::parsed_data::geo_params_data::PdmsGeoParam;

use crate::pdms_types::AttrMap;
use crate::prim_geo::helper::cal_ref_axis;
use crate::shape::pdms_shape::{BrepMathTrait, BrepShapeTrait, PdmsMesh, TRI_TOL, VerifiedShape};
use crate::tool::float_tool::hash_f32;
use crate::tool::hash_tool::*;

//可不可以用来表达 sphere
#[derive(Component, Debug, Clone, Reflect, Serialize, Deserialize)]
#[reflect(Component)]
pub struct Dish {
    pub paax_expr: String,
    pub paax_pt: Vec3,
    //Axis point
    pub paax_dir: Vec3,  //Axis Direction

    pub pdis: f32,
    pub pheig: f32,
    pub pdia: f32, //diameter
}

impl Default for Dish {
    fn default() -> Self {
        Self {
            paax_expr: "Z".to_string(),
            paax_pt: Default::default(),
            paax_dir: Vec3::Z,
            pdis: 0.0,
            pheig: 1.0,
            pdia: 1.0,
        }
    }
}

impl VerifiedShape for Dish {
    fn check_valid(&self) -> bool {
        self.pdia > f32::EPSILON && self.pheig > f32::EPSILON
    }
}

//#[typetag::serde]
impl BrepShapeTrait for Dish {
    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }
    fn gen_brep_shell(&self) -> Option<Shell> {
        use truck_modeling::*;
        let r = self.pdia / 2.0;
        let h = self.pheig;
        let radius = (r * r + h * h) / (2.0f32 * h);
        if radius < f32::EPSILON { return None; }
        let sinval = (r / radius).max(-1.0f32).min(1.0f32);
        let mut theta = (sinval).asin();
        if r < h { theta = PI - theta; }

        let rot_axis = self.paax_dir.normalize();
        let c = rot_axis * self.pdis + self.paax_pt;
        let ref_axis = cal_ref_axis(&rot_axis);
        let p0 = rot_axis * self.pheig + c;
        let center = p0 - radius * rot_axis;
        let p1 = ref_axis * self.pdia / 2.0 + c;

        let c = c.point3();
        let v0 = builder::vertex(c);
        let v1 = builder::vertex(p0.point3());
        let v2 = builder::vertex(p1.point3());

        let axis = -ref_axis.cross(rot_axis);
        let curve = builder::circle_arc_with_center(center.point3(), &v1, &v2, axis.vector3(), Rad(theta as f64));
        let wire: Wire = vec![curve, builder::line(&v2, &v0),/* builder::line(&v0, &v1)*/].into();
        let up_axis = rot_axis.vector3();
        let mut s = builder::cone(&wire, up_axis, Rad(7.0));
        let btm = builder::rsweep(
            &v2,
            c,
            -up_axis,
            Rad(7.0),
        );
        if let Ok(disk) = builder::try_attach_plane(&vec![btm]) {
            // dbg!(&disk);
            s.push(disk);
        }

        // let json = serde_json::to_vec_pretty(&Solid::new(vec![s.clone()])).unwrap();
        // std::fs::write("/Users/dongpengcheng/Documents/week-work/crates/truck/cone.json", json).unwrap();
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
        hash_f32(beta, &mut hasher);
        "dish".hash(&mut hasher);
        hasher.finish()
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
        let r = self.pdia;
        let h = self.pheig / r;
        Box::new(Self {
            pheig: h,
            pdia: 1.0,
            ..Default::default()
        })
    }

    fn gen_unit_mesh(&self) -> Option<PdmsMesh> {
        self.gen_unit_shape().gen_mesh(Some(TRI_TOL))
    }

    fn get_scaled_vec3(&self) -> Vec3 {
        Vec3::new(self.pdia, self.pdia, self.pdia)
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
            pheig: m.get_val("HEIG").unwrap().f32_value().unwrap_or_default(),
            pdia: m.get_val("DIAM").unwrap().f32_value().unwrap_or_default(),
        }
    }
}

impl From<AttrMap> for Dish {
    fn from(m: AttrMap) -> Self {
        (&m).into()
    }
}
