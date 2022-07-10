use std::f32::EPSILON;
use bevy::prelude::*;
use truck_modeling::{builder, Shell};
// use bevy_inspector_egui::Inspectable;
use truck_meshalgo::prelude::*;
use bevy::reflect::Reflect;
use bevy::ecs::reflect::ReflectComponent;
use serde::{Serialize,Deserialize};
use crate::pdms_types::AttrMap;

use crate::prim_geo::helper::cal_ref_axis;
use crate::shape::pdms_shape::{BrepMathTrait, BrepShapeTrait, PdmsMesh, VerifiedShape};

#[derive(Component, Debug, /*Inspectable,*/ Clone,  Reflect, Serialize, Deserialize)]
// #[reflect(Component)]
pub struct LCylinder {
    pub paxi_expr: String,
    pub paxi_pt: Vec3,   //A Axis point
    pub paxi_dir: Vec3,   //A Axis Direction

    pub pbdi: f32, //dist to bottom
    pub ptdi: f32, //dist to top
    pub pdia: f32, //diameter
    pub negative: bool,
}



impl Default for LCylinder {
    fn default() -> Self {
        LCylinder {
            paxi_expr: "Z".to_string(),
            paxi_pt:  Default::default(),
            paxi_dir: Vec3::new(0.0,0.0,1.0),
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

impl BrepShapeTrait for LCylinder {

    #[inline]
    fn get_scaled_vec3(&self) -> Vec3 {
        Vec3::new(self.pdia, self.pdia, (self.pbdi - self.ptdi))
    }

    fn gen_brep_shell(& self) -> Option<Shell> {
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
}

impl From<&AttrMap> for LCylinder {
    fn from(m: &AttrMap) -> Self {
        let pdia = m.get_val("DIAM").unwrap().double_value().unwrap() as f32 ;
        let pbdi = m.get_val("PBDI").unwrap().double_value().unwrap() as f32 ;
        let ptdi = m.get_val("PTDI").unwrap().double_value().unwrap() as f32 ;
        LCylinder {
            paxi_expr: "Z".to_string(),
            paxi_pt: Default::default() ,
            paxi_dir: Vec3::Z,
            pbdi,
            ptdi,
            negative: false,
            pdia
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
    pub paxi_pt: Vec3,   //A Axis point
    pub paxi_dir: Vec3,   //A Axis Direction

    pub pdis: f32, //dist to bottom
    pub phei: f32, // height
    pub pdia: f32, //diameter
    pub negative: bool,
}

impl Default for SCylinder {
    fn default() -> Self {
        Self {
            paxi_expr: "Z".to_string(),
            paxi_dir: Vec3::Z,
            paxi_pt: Default::default(),
            pdis: -0.5,
            phei: 1.0,
            pdia: 1.0,
            negative: false,
        }
    }
}

impl VerifiedShape for SCylinder {
    #[inline]
    fn check_valid(&self) -> bool {
       self.pdia > f32::EPSILON && self.phei.abs() > f32::EPSILON
    }
}

impl BrepShapeTrait for SCylinder {
    fn gen_brep_shell(&self) -> Option<Shell> {
        use truck_modeling::*;
        let dir = self.paxi_dir.normalize();
        let r = self.pdia / 2.0;
        let c_pt = dir * self.pdis + self.paxi_pt;
        let center = c_pt.point3();
        let ref_axis = cal_ref_axis(&dir);
        let pt0 = c_pt + ref_axis * r;
        let mut ext_len = self.phei;
        let mut ext_dir = dir.vector3();
        let mut  reverse_dir = false;
        if ext_len < 0.0 {
            reverse_dir = true;
        }
        let v = builder::vertex(pt0.point3());
        let w = builder::rsweep(&v, center, ext_dir, Rad(7.0));
        if  let Ok(mut f) = builder::try_attach_plane(&[w]){
            if reverse_dir { f = f.inverse(); }
            let mut s = builder::tsweep(&f, ext_dir * ext_len as f64).into_boundaries();
            return s.pop()
        }
        None
    }

    fn hash_mesh_params(&self) -> u64{
        if self.phei < 0.0 {
            102u64
        }else{
            2u64 //代表cylinder
        }
    }


    fn gen_unit_shape(&self) -> PdmsMesh{
        SCylinder::default().gen_mesh(Some(0.01))
    }

    #[inline]
    fn get_scaled_vec3(&self) -> Vec3 {
        Vec3::new(self.pdia, self.pdia, self.phei.abs())
    }
}

impl From<&AttrMap> for SCylinder {
    fn from(m: &AttrMap) -> Self {
        let phei = m.get_val("HEIG").unwrap().double_value().unwrap_or_default() as f32 ;
        let pdia = m.get_val("DIAM").unwrap().double_value().unwrap_or_default() as f32 ;
        SCylinder {
            paxi_expr: "Z".to_string(),
            paxi_pt: Default::default() ,
            paxi_dir: Vec3::Z,
            pdis: -phei / 2.0,
            phei,
            pdia,
            negative: false,
        }
    }
}

impl From<AttrMap> for SCylinder {
    fn from(m: AttrMap) -> Self {
        (&m).into()
    }
}