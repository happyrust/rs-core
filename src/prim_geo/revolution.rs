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

#[derive(Component, Debug, /*Inspectable,*/ Clone,  Reflect)]
#[reflect(Component)]
pub struct Revolution {
    pub paax_expr: String,
    pub paax_pt: Vec3,   //A Axis point
    pub paax_dir: Vec3,   //A Axis Direction

    pub pbax_expr: String,
    pub pbax_pt: Vec3,   //B Axis point
    pub pbax_dir: Vec3,   //B Axis Direction, with paax make the plane to draft

    pub loop_verts: Vec<Vec3>, //loop vertex
    pub angle: f32,
    pub rot_dir: Vec3,
    pub rot_pt: Vec3,
}


impl Default for Revolution {
    fn default() -> Self {
        Self {
            paax_expr: "X".to_string(),
            paax_pt: Default::default(),
            paax_dir: Vec3::X,

            pbax_expr: "Y".to_string(),
            pbax_pt: Default::default(),
            pbax_dir: Vec3::Y,

            loop_verts: vec![Vec3::ZERO, Vec3::new(2.0, 0.0, 0.0), Vec3::new(2.0, 1.0, 0.0),
                             Vec3::new(1.0, 1.0, 0.0), Vec3::new(1.0, 2.0, 0.0), Vec3::new(0.0, 2.0, 0.0)],
            angle: 90.0,
            rot_dir: Vec3::X,   //默认绕Z轴旋转
            rot_pt: Vec3::ZERO, //默认旋转点
        }
    }
}

impl VerifiedShape for Revolution {
    fn check_valid(&self) -> bool{
        self.angle > std::f32::EPSILON
    }
}

impl BrepShapeTrait for Revolution {
    fn gen_brep_shell(& self) -> Option<Shell> {
        if !self.check_valid() { return None; }

        let mut wire = Wire::new();
        let ll = self.loop_verts.len();
        let mut verts: Vec<_> = self.loop_verts.iter().map(|x| builder::vertex(x.point3())).collect();
        for i in 0..ll {
            let cur_v = &verts[i];
            let next_v = &verts[(i+1)%ll];
            wire.push_back(builder::line(&cur_v, &next_v));
        }
        if let Ok(mut face) = builder::try_attach_plane(&[wire]){
            if let Surface::Plane(plane) = face.get_surface(){
                let rot_dir = self.rot_dir.normalize().vector3();
                let rot_pt = self.rot_pt.point3();
                let angle = self.angle.to_radians() as f64;
                let mut s = builder::rsweep(&face, rot_pt, rot_dir, Rad(angle)).into_boundaries();
                let shell = s.pop();
                return shell;
            }
        }
        None
    }

    fn hash_mesh_params(&self) -> u64{
        let mut hasher = DefaultHasher::new();
        self.loop_verts.iter().for_each(|v|  {
            hash_vec3::<DefaultHasher>(v, &mut hasher);
        });
        hasher.finish()
    }

    //暂时不做可拉伸
    fn gen_unit_shape(&self) -> PdmsMesh{
        self.gen_mesh(Some(TRI_TOL))
        // self.gen_mesh(Some(0.002))
    }
    fn get_scaled_vec3(&self) -> Vec3{
        Vec3::ONE
    }
}

impl From<AttrMap> for Revolution {
    fn from(m: AttrMap) -> Self {
        Default::default()
    }
}