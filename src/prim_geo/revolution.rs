use std::collections::hash_map::DefaultHasher;
use std::f32::consts::PI;
use std::f32::EPSILON;
use std::hash::{Hash, Hasher};
use bevy::prelude::*;
use truck_modeling::{builder, Shell, Surface, Wire};
use crate::tool::hash_tool::*;
use truck_meshalgo::prelude::*;
use bevy::reflect::Reflect;
use bevy::ecs::reflect::ReflectComponent;
use glam::Vec3;
use crate::pdms_types::AttrMap;

use crate::prim_geo::helper::cal_ref_axis;
use crate::shape::pdms_shape::{BrepMathTrait, BrepShapeTrait, PdmsMesh, TRI_TOL, VerifiedShape};

#[derive(Component, Debug,  Clone,  Reflect)]
#[reflect(Component)]
pub struct Revolution {
    pub verts: Vec<Vec3>, //loop vertex
    pub angle: f32,     //degrees
    pub rot_dir: Vec3,
    pub rot_pt: Vec3,
}


impl Default for Revolution {
    fn default() -> Self {
        Self {
            verts: vec![Vec3::ZERO, Vec3::new(2.0, 0.0, 0.0), Vec3::new(2.0, 1.0, 0.0),
                             Vec3::new(1.0, 1.0, 0.0), Vec3::new(1.0, 2.0, 0.0), Vec3::new(0.0, 2.0, 0.0)],
            angle: 90.0,
            rot_dir: Vec3::X,   //默认绕Z轴旋转
            rot_pt: Vec3::ZERO, //默认旋转点
        }
    }
}

impl VerifiedShape for Revolution {
    fn check_valid(&self) -> bool{
        self.angle.abs() > std::f32::EPSILON
    }
}

impl BrepShapeTrait for Revolution {
    fn gen_brep_shell(&self) -> Option<Shell> {
        if !self.check_valid() { return None; }

        let mut wire = Wire::new();
        let ll = self.verts.len();
        let mut verts: Vec<_> = self.verts.iter().map(|x| builder::vertex(x.point3())).collect();
        // dbg!(&verts);
        for i in 0..ll {
            let cur_v = &verts[i];
            let next_v = &verts[(i+1)%ll];
            wire.push_back(builder::line(&cur_v, &next_v));
        }
        if let Ok(mut face) = builder::try_attach_plane(&[wire]){
            if let Surface::Plane(plane) = face.get_surface(){
                let mut rot_dir = self.rot_dir.normalize().vector3();
                let rot_pt = self.rot_pt.point3();
                // dbg!(self.angle);
                // dbg!(self.rot_dir);
                // dbg!(self.rot_pt);
                let mut angle = self.angle.to_radians() as f64;
                if angle < 0.0 {
                    angle = angle.abs();
                    rot_dir -= rot_dir;
                }

                let mut s = builder::rsweep(&face, rot_pt, rot_dir, Rad(angle)).into_boundaries();
                let shell = s.pop();
                if shell.is_none() {
                    dbg!(&self);
                }
                return shell;
            }
        }else{
            // dbg!(&self);
        }
        None
    }

    fn hash_mesh_params(&self) -> u64{
        let mut hasher = DefaultHasher::new();
        self.verts.iter().for_each(|v|  {
            hash_vec3::<DefaultHasher>(v, &mut hasher);
        });
        "Revolution".hash(&mut hasher);
        hash_f32(&self.angle, &mut hasher);
        // hash_vec3::<DefaultHasher>(&self.rot_dir, &mut hasher);
        // hash_vec3::<DefaultHasher>(&self.rot_pt, &mut hasher);
        hasher.finish()
    }

    fn gen_unit_shape(&self) -> PdmsMesh{
        self.gen_mesh(Some(TRI_TOL/10.0))
    }
    fn get_scaled_vec3(&self) -> Vec3{
        Vec3::ONE
    }
}