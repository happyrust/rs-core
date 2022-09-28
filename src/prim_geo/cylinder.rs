use std::collections::hash_map::DefaultHasher;
use std::f32::EPSILON;
use std::f64::consts::PI;
use std::hash::Hash;
use std::hash::Hasher;

use approx::{abs_diff_eq, abs_diff_ne};
use bevy::ecs::reflect::ReflectComponent;
use bevy::prelude::*;
use bevy::reflect::Reflect;
use nom::Parser;
use serde::{Deserialize, Serialize};
use truck_meshalgo::prelude::*;
use truck_modeling::{builder, Shell};
use truck_topology::Face;

use crate::pdms_types::AttrMap;
use crate::prim_geo::helper::cal_ref_axis;
use crate::shape::pdms_shape::{BrepMathTrait, BrepShapeTrait, PdmsMesh, TRI_TOL, VerifiedShape};
use crate::tool::float_tool::hash_f32;

#[derive(Component, Debug, /*Inspectable,*/ Clone, Reflect, Serialize, Deserialize)]
// #[reflect(Component)]
pub struct LCylinder {
    pub paxi_expr: String,
    pub paxi_pt: Vec3,
    //A Axis point
    pub paxi_dir: Vec3,   //A Axis Direction

    pub pbdi: f32,
    //dist to bottom
    pub ptdi: f32,
    //dist to top
    pub pdia: f32,
    //diameter
    pub negative: bool,
}


impl Default for LCylinder {
    fn default() -> Self {
        LCylinder {
            paxi_expr: "Z".to_string(),
            paxi_pt: Default::default(),
            paxi_dir: Vec3::new(0.0, 0.0, 1.0),
            pbdi: -0.5,
            ptdi: 0.5,
            pdia: 1.0,
            negative: false,
        }
    }
}

impl VerifiedShape for LCylinder {
    fn check_valid(&self) -> bool {
        true
    }
}

//#[typetag::serde]
impl BrepShapeTrait for LCylinder {
    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    fn gen_brep_shell(&self) -> Option<Shell> {
        use truck_modeling::*;
        if !self.check_valid() { return None; }

        let dir = self.paxi_dir.normalize();
        let r = self.pdia / 2.0;
        let c_pt = dir * self.pbdi + self.paxi_pt;
        let center = c_pt.point3();
        let ref_axis = cal_ref_axis(&dir);
        let pt0 = c_pt + ref_axis * r;
        let mut ext_len = self.ptdi - self.pbdi;
        let mut ext_dir = dir.vector3();
        if ext_len < 0.0 {
            ext_dir = -ext_dir;
            ext_len = -ext_len;
        }
        let v = builder::vertex(pt0.point3());
        let w = builder::rsweep(&v, center, ext_dir, Rad(7.0));
        let f = builder::try_attach_plane(&[w]).unwrap();
        let mut s = builder::tsweep(&f, ext_dir * ext_len as f64).into_boundaries();
        s.pop()
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(Self::default())
    }

    #[inline]
    fn get_scaled_vec3(&self) -> Vec3 {
        Vec3::new(self.pdia, self.pdia, (self.pbdi - self.ptdi))
    }
}

impl From<&AttrMap> for LCylinder {
    fn from(m: &AttrMap) -> Self {
        let pdia = m.get_val("DIAM").unwrap().double_value().unwrap() as f32;
        let pbdi = m.get_val("PBDI").unwrap().double_value().unwrap() as f32;
        let ptdi = m.get_val("PTDI").unwrap().double_value().unwrap() as f32;
        LCylinder {
            paxi_expr: "Z".to_string(),
            paxi_pt: Default::default(),
            paxi_dir: Vec3::Z,
            pbdi,
            ptdi,
            negative: false,
            pdia,
        }
    }
}

impl From<AttrMap> for LCylinder {
    fn from(m: AttrMap) -> Self {
        (&m).into()
    }
}


#[derive(Component, Debug, /*Inspectable,*/ Reflect, Clone, Serialize, Deserialize)]
// #[reflect(Component)]
pub struct SCylinder {
    pub paxi_expr: String,
    pub paxi_pt: Vec3,
    //A Axis point
    pub paxi_dir: Vec3,   //A Axis Direction

    // pub pdis: f32,
    //dist to bottom
    pub phei: f32,
    // height
    pub pdia: f32,
    //diameter
    pub btm_shear_angles: [f32; 2],
    // x shear
    pub top_shear_angles: [f32; 2],
    // y shear
    pub negative: bool,
}

impl Default for SCylinder {
    fn default() -> Self {
        Self {
            paxi_expr: "Z".to_string(),
            paxi_dir: Vec3::Z,
            paxi_pt: Default::default(),
            phei: 1.0,
            pdia: 1.0,
            btm_shear_angles: [0.0f32; 2],
            top_shear_angles: [0.0f32; 2],
            negative: false,
        }
    }
}

impl SCylinder {
    #[inline]
    pub fn is_sscl(&self) -> bool {
        abs_diff_ne!(self.btm_shear_angles[0],  0.0) ||
            abs_diff_ne!(self.btm_shear_angles[1],  0.0) ||
            abs_diff_ne!(self.top_shear_angles[0],  0.0) ||
            abs_diff_ne!(self.top_shear_angles[1],  0.0)
    }
}

impl VerifiedShape for SCylinder {
    #[inline]
    fn check_valid(&self) -> bool {
        self.pdia > f32::EPSILON && self.phei.abs() > f32::EPSILON
    }
}

//#[typetag::serde]
impl BrepShapeTrait for SCylinder {
    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    #[inline]
    fn get_trans(&self) -> glam::TransformSRT {
        glam::TransformSRT {
            rotation: Default::default(),
            translation: Vec3::new(0.0, 0.0, -self.phei/2.0),
            scale: self.get_scaled_vec3(),
        }
    }

    fn gen_brep_shell(&self) -> Option<Shell> {
        use truck_modeling::*;
        let dir = self.paxi_dir.normalize();
        let r = self.pdia / 2.0;
        // let c_pt = dir * self.pdis + self.paxi_pt;
        let c_pt = Vec3::ZERO;
        let center = c_pt.point3();
        let ref_axis = cal_ref_axis(&dir);
        let pt0 = c_pt + ref_axis * r;
        let mut ext_len = self.phei as f64;
        let mut ext_dir = dir.vector3();
        let mut reverse_dir = false;
        if ext_len < 0.0 {
            reverse_dir = true;
        }
        // dbg!(ext_len);
        let v = builder::vertex(pt0.point3());
        let origin_w = builder::rsweep(&v, center, ext_dir, Rad(7.0));
        // dbg!(&self.btm_shear_angles);

        //还是要和extrude 区分出来
        // dbg!(&self.top_shear_angles);
        let scale_x = 1.0 / self.btm_shear_angles[0].to_radians().cos() as f64;
        let scale_y = 1.0 / self.btm_shear_angles[1].to_radians().cos() as f64;
        let transform_btm = Matrix4::from_angle_y(Rad(self.btm_shear_angles[0].to_radians() as f64))
            * Matrix4::from_angle_y(Rad(self.btm_shear_angles[1].to_radians() as f64))
            * Matrix4::from_nonuniform_scale(scale_x, scale_y, 1.0);
        let scale_x = 1.0 / self.top_shear_angles[0].to_radians().cos() as f64;
        let scale_y = 1.0 / self.top_shear_angles[1].to_radians().cos() as f64;
        // dbg!((scale_x, scale_y));
        let transform_top = Matrix4::from_translation(ext_dir * ext_len as f64)
            * Matrix4::from_angle_y(Rad(self.top_shear_angles[0].to_radians() as f64))
            * Matrix4::from_angle_y(Rad(self.top_shear_angles[1].to_radians() as f64))
            * Matrix4::from_nonuniform_scale(scale_x, scale_y, 1.0);
        // let angle = Rad(PI/16.0);
        // let transform_btm = Matrix4::from_angle_y(Rad(PI/16.0));
        // let transform_top =  Matrix4::from_translation(Vector3::new(0.0, 0.0, ext_len as f64))
        //     * Matrix4::from_angle_y(Rad(-PI/16.0));
        //
        // let transform_btm = Matrix4::identity() * Matrix4::from_angle_y(angle);
        // let transform_top = Matrix4::from_translation(ext_dir * ext_len as f64) * Matrix4::from_angle_y(-angle);
        let mut w_s = builder::transformed(&origin_w, transform_btm);
        let mut w_e = builder::transformed(&origin_w, transform_top);
        if let Ok(mut f) = builder::try_attach_plane(&[w_s.clone()])
        {
            let mut f_e = builder::try_attach_plane(&[w_e.clone()]).unwrap().inverse();
            if !reverse_dir {
                f = f.inverse();
                f_e = f_e.inverse();
            }
            let h_w_s = w_s.split_off(w_s.len() / 2);
            let h_w_e = w_e.split_off(w_e.len() / 2);
            let mut face1 = builder::homotopy(w_s.front().unwrap(), &w_e.front().unwrap());
            let mut face2 = builder::homotopy(h_w_s.front().unwrap(), &h_w_e.front().unwrap());
            let mut shell = vec![f, f_e, face1, face2].into();
            return Some(shell);
            // let mut s = builder::tsweep(&f, ext_dir * ext_len as f64).into_boundaries();
            // return s.pop();
        }
        None
    }

    fn hash_unit_mesh_params(&self) -> u64 {
        if self.is_sscl() {
            let mut hasher = DefaultHasher::new();
            hash_f32(self.btm_shear_angles[0], &mut hasher);
            "btm_x".hash(&mut hasher);
            hash_f32(self.btm_shear_angles[1], &mut hasher);
            "btm_y".hash(&mut hasher);
            hash_f32(self.top_shear_angles[0], &mut hasher);
            "top_x".hash(&mut hasher);
            hash_f32(self.top_shear_angles[1], &mut hasher);
            "top_y".hash(&mut hasher);
            hash_f32(self.pdia, &mut hasher);
            "sslc".hash(&mut hasher);
            hasher.finish()
        } else {
            if self.phei < 0.0 {
                102u64
            } else {
                2u64 //代表cylinder
            }
        }
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
        if self.is_sscl(){
            // let mut s = self.clone();
            // s.phei = 1.0;
            let mut s =  SCylinder {
                paxi_expr: "Z".to_string(),
                paxi_pt: Default::default(),
                paxi_dir: Vec3::Z,
                phei: self.phei,
                pdia: self.pdia,
                btm_shear_angles: self.btm_shear_angles.clone(),
                top_shear_angles: self.top_shear_angles.clone(),
                negative: false,
            };
            dbg!(&s);
            return Box::new(s);
        }
        Box::new(Self::default())
    }

    fn gen_unit_mesh(&self) -> Option<PdmsMesh> {
        self.gen_unit_shape().gen_mesh(Some(TRI_TOL / 10.0))
    }

    #[inline]
    fn get_scaled_vec3(&self) -> Vec3 {
        if self.is_sscl(){
            // Vec3::new(1.0, 1.0, self.phei.abs())
            Vec3::new(1.0, 1.0, 1.0)
        } else {
            Vec3::new(self.pdia, self.pdia, self.phei.abs())
        }
    }
}

impl From<&AttrMap> for SCylinder {
    fn from(m: &AttrMap) -> Self {
        let phei = m.get_val("HEIG").unwrap().double_value().unwrap_or_default() as f32;
        let pdia = m.get_val("DIAM").unwrap().double_value().unwrap_or_default() as f32;
        SCylinder {
            paxi_expr: "Z".to_string(),
            paxi_pt: Default::default(),
            paxi_dir: Vec3::Z,
            phei,
            pdia,
            btm_shear_angles: [0.0; 2],
            top_shear_angles: [0.0; 2],
            negative: false,
        }
    }
}

impl From<AttrMap> for SCylinder {
    fn from(m: AttrMap) -> Self {
        (&m).into()
    }
}