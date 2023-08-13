use std::collections::hash_map::DefaultHasher;
use std::f32::EPSILON;
use std::hash::Hasher;

use truck_meshalgo::prelude::*;
use truck_modeling::Shell;
use std::hash::Hash;
use glam::Vec3;
use serde::{Serialize,Deserialize};
use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::pdms_types::AttrMap;
use crate::shape::pdms_shape::{BrepMathTrait, PlantMesh};
use crate::shape::pdms_shape::{BrepShapeTrait, VerifiedShape};
use crate::tool::float_tool::hash_f32;
use crate::tool::hash_tool::*;
use bevy_ecs::prelude::*;
#[cfg(feature = "opencascade_rs")]
use opencascade::primitives::{Vertex, Shape, Solid, Wire};
#[cfg(feature = "opencascade_rs")]
use opencascade::workplane::Workplane;

#[derive(Component, Debug, Clone, Serialize, Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize,)]
pub struct LSnout {
    pub paax_expr: String,
    pub paax_pt: Vec3,   //A Axis point
    pub paax_dir: Vec3,   //A Axis Direction

    pub pbax_expr: String,
    pub pbax_pt: Vec3,   //B Axis point
    pub pbax_dir: Vec3,   //B Axis Direction

    pub ptdi: f32,      //dist to top
    pub pbdi: f32,      //dist to bottom
    pub ptdm: f32,      //top diameter
    pub pbdm: f32,      //bottom diameter
    pub poff: f32,      //offset

    pub btm_on_top: bool,
}

impl Default for LSnout {
    fn default() -> Self {
        Self {
            paax_expr: "Z".to_string(),
            paax_pt: Default::default(),
            paax_dir: Vec3::Z,

            pbax_expr: "X".to_string(),
            pbax_pt: Default::default(),
            pbax_dir: Vec3::X,

            ptdi: 0.5,
            pbdi: -0.5,
            ptdm: 1.0,
            pbdm: 1.0,
            poff: 0.0,
            btm_on_top: false,
        }
    }
}

impl VerifiedShape for LSnout {
    #[inline]
    fn check_valid(&self) -> bool {
        //height 必须 >0， 小于0 的情况直接用变换矩阵
        self.ptdm >= 0.0 && self.pbdm >= 0.0  && (self.ptdi - self.pbdi) > f32::EPSILON
    }
}

//#[typetag::serde]
impl BrepShapeTrait for LSnout {

    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    #[inline]
    fn tol(&self) -> f32{
        //以最小的圆精度为准
        0.005 * (( self.pbdm + self.ptdm) / 2.0).max(1.0)
    }

    #[cfg(feature = "opencascade_rs")]
    fn gen_occ_shape(&self) -> anyhow::Result<Shape> {
        let rt = (self.ptdm/2.0);
        let rb = (self.pbdm/2.0);

        let mut a_dir = self.paax_dir.normalize();
        let mut b_dir = self.pbax_dir.normalize();
        let p0 = a_dir * self.pbdi + self.paax_pt;
        let p1 = a_dir * self.ptdi + self.paax_pt + self.poff * b_dir;

        let mut circles = vec![];
        let mut verts = vec![];
        if self.pbdm < f32::EPSILON {
            verts.push(Vertex::new(p0.as_dvec3()));
        }else{
            // circles.push(Wire::circle(rb, p0, a_dir)?);
            // let mut circle =  Workplane::xy().translated(p0.as_dvec3()).circle(0.0, 0.0, rb as f64);
            let circle = Wire::circle(rb as _, p0.as_dvec3(), a_dir.as_dvec3());
            circles.push(circle);
        }

        if self.ptdm < f32::EPSILON {
            verts.push(Vertex::new(p1.as_dvec3()));
        }else{
            // circles.push(Wire::circle(rt, p1, a_dir)?);
            // let mut circle = Workplane::xy().translated(p1.as_dvec3()).circle(0.0, 0.0, rt as f64);
            let circle = Wire::circle(rt as _, p1.as_dvec3(), a_dir.as_dvec3());
            circles.push(circle);
        }

        Ok(Solid::loft_with_points(circles.iter(), verts.iter()).to_shape())
    }

    fn gen_brep_shell(&self) -> Option<Shell> {
        use truck_modeling::*;
        let rt = (self.ptdm/2.0).max(0.01);
        let rb = (self.pbdm/2.0).max(0.01);

        let mut a_dir = self.paax_dir.normalize();
        let mut b_dir = self.pbax_dir.normalize();
        let p0 = a_dir * self.pbdi + self.paax_pt;
        let p1 = a_dir * self.ptdi + self.paax_pt + self.poff * b_dir;
        let p2 = b_dir * rt + p1;
        let p3 = b_dir * rb + p0;
        let v2 = builder::vertex(p2.point3());
        let v3 = builder::vertex(p3.point3());

        //todo 表达cone的情况
        let mut is_cone = false;
        if self.ptdm * self.pbdm < EPSILON {
            is_cone = true;
        }

        let rot_axis = a_dir.vector3();
        let mut circle1 = builder::rsweep(&v3, p0.point3(), rot_axis, Rad(7.0));
        let mut c1 = circle1.clone();

        let mut circle2 = builder::rsweep(&v2, p1.point3(), rot_axis, Rad(7.0));
        let mut c2 = circle2.clone();


        let new_wire_1 = circle1.split_off((0.5 * circle1.len() as f32) as usize);
        let new_wire_2 = circle2.split_off((0.5 * circle2.len() as f32) as usize);
        let mut face1 = builder::homotopy(new_wire_1.front().unwrap(), &new_wire_2.front().unwrap());
        let mut face2 = builder::homotopy(circle1.front().unwrap(), &circle2.front().unwrap());

        if let Ok(disk1) = builder::try_attach_plane(&vec![c1.inverse()]){
            if let Ok(disk2) = builder::try_attach_plane(&vec![c2]){
                let mut shell = Shell::from(vec![face1, face2, disk1, disk2]);
                return Some(shell)
            }
        }
        None
    }

    fn hash_unit_mesh_params(&self) -> u64{
        let mut hasher = DefaultHasher::new();
        //对于有偏移的，直接不复用，后面看情况再考虑复用
        if self.poff.abs() > f32::EPSILON {
            let bytes = bincode::serialize(self).unwrap();
            let mut hasher = DefaultHasher::default();
            bytes.hash(&mut hasher);
            return hasher.finish();
        }
        let pheight = (self.ptdi - self.pbdi) > 0.0;
        let alpha = if self.pbdm != 0.0 {
            self.ptdm / self.pbdm
        }else{
            0.0
        };
        hash_f32(alpha, &mut hasher);
        pheight.hash(&mut hasher);
        "snout".hash(&mut hasher);
        hasher.finish()
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
        let ptdm = self.ptdm / self.pbdm;
        if self.poff.abs() > f32::EPSILON {
            Box::new(self.clone())
        }else{
            Box::new(Self{
                ptdi: 0.5 ,
                pbdi: -0.5,
                ptdm,
                pbdm: 1.0,
                ..Default::default()
            })
        }
    }


    #[inline]
    fn get_scaled_vec3(&self) -> Vec3{
        let pheight = (self.ptdi - self.pbdi).abs();
        //有偏心的时候，不缩放
        if self.poff.abs() > f32::EPSILON {
            Vec3::ONE
        }else{
            Vec3::new(self.pbdm, self.pbdm, pheight)
        }
    }

    fn convert_to_geo_param(&self) -> Option<PdmsGeoParam> {
        Some(
            PdmsGeoParam::PrimLSnout(self.clone())
        )
    }
}

impl From<&AttrMap> for LSnout {
    fn from(m: &AttrMap) -> Self {
        let h = m.get_val("HEIG").unwrap().double_value().unwrap() as f32 ;
        LSnout {
            ptdi: h / 2.0,
            pbdi: -h / 2.0,
            ptdm: m.get_val("DTOP").unwrap().double_value().unwrap() as f32 ,
            pbdm: m.get_val("DBOT").unwrap().double_value().unwrap() as f32 ,
            ..Default::default()
        }
    }
}

impl From<AttrMap> for LSnout {
    fn from(m: AttrMap) -> Self {
        (&m).into()
    }
}


