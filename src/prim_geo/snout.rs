use std::collections::hash_map::DefaultHasher;
use std::f32::EPSILON;
use std::hash::Hasher;
use bevy::prelude::*;
use truck_meshalgo::prelude::*;
use truck_modeling::Shell;
use bevy::reflect::Reflect;
use bevy::ecs::reflect::ReflectComponent;
use std::hash::Hash;
use glam::TransformSRT;
use serde::{Serialize,Deserialize};
use crate::pdms_types::AttrMap;
use crate::shape::pdms_shape::{BrepMathTrait, PdmsMesh, TRI_TOL};
use crate::shape::pdms_shape::{BrepShapeTrait, VerifiedShape};
use crate::tool::float_tool::hash_f32;
use crate::tool::hash_tool::*;

#[derive(Component, Debug, /*Inspectable,*/ Clone,  Reflect, Serialize, Deserialize)]
#[reflect(Component)]
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
        }
    }
}

impl VerifiedShape for LSnout {
    #[inline]
    fn check_valid(&self) -> bool {
        self.ptdm >= 0.0 && self.pbdm >= 0.0  && (self.ptdi - self.pbdi).abs() > f32::EPSILON
    }
}

impl BrepShapeTrait for LSnout {
    //todo 需要支持Cone 的情况
    fn gen_brep_shell(&self) -> Option<Shell> {
        use truck_modeling::*;
        let rt = (self.ptdm/2.0).max(0.01);
        let rb = (self.pbdm/2.0).max(0.01);
        let a_dir = self.paax_dir.normalize();
        let b_dir = self.pbax_dir.normalize();
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
            dbg!(is_cone);
        }

        //let cone = builder::cone(&wire, Vector3::unit_y(), Rad(2.0 * PI));
        let rot_axis = a_dir.vector3();
        let mut circle1 = builder::rsweep(&v3, p0.point3(), rot_axis, Rad(7.0));
        let c1 = circle1.clone();

        let mut circle2 = builder::rsweep(&v2, p1.point3(), rot_axis, Rad(7.0));
        let c2 = circle2.clone();

        let new_wire_1 = circle1.split_off((0.5 * circle1.len() as f32) as usize);
        let new_wire_2 = circle2.split_off((0.5 * circle2.len() as f32) as usize);

        let face1 = builder::homotopy(new_wire_1.front().unwrap(), &new_wire_2.front().unwrap());
        let face2 = builder::homotopy(circle1.front().unwrap(), &circle2.front().unwrap());

        if let Ok(disk1) = builder::try_attach_plane(&vec![c1.inverse()]){
            if let Ok(disk2) = builder::try_attach_plane(&vec![c2]){
                let shell = Shell::from(vec![face1, face2, disk1, disk2]);
                return Some(shell)
            }
        }
        None
    }

    fn hash_mesh_params(&self) -> u64{
        let mut hasher = DefaultHasher::new();
        //对于有偏移的，直接不复用，后面看情况再考虑复用
        if self.poff.abs() > f32::EPSILON {
            let bytes = bincode::serialize(self).unwrap();
            let mut hasher = DefaultHasher::default();
            bytes.hash(&mut hasher);
            return hasher.finish();
        }
        // let pheight = self.ptdi - self.pbdi;
        let alpha = if self.pbdm != 0.0 {
            self.ptdm / self.pbdm
        }else{
            0.0
        };
        hash_f32(alpha, &mut hasher);
        // hash_f32(pheight, &mut hasher);
        "snout".hash(&mut hasher);
        hasher.finish()
    }

    //参考圆点在中心位置
    fn gen_unit_shape(&self) -> PdmsMesh{
        let ptdm = self.ptdm / self.pbdm;
       if self.poff.abs() > f32::EPSILON {
            self.gen_mesh(Some(TRI_TOL))
        }else{
            Self{
                ptdi: 0.5 ,
                pbdi: -0.5,
                ptdm,
                pbdm: 1.0,
                poff: 0.0,
                ..Default::default()
            }.gen_mesh(Some(TRI_TOL))
        }

    }

    #[inline]
    fn get_scaled_vec3(&self) -> Vec3{
        let pheight = (self.ptdi - self.pbdi).abs();
        if self.poff.abs() > f32::EPSILON {
            Vec3::ONE
        }else{
            Vec3::new(self.pbdm, self.pbdm, pheight)
        }

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


