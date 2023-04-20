use std::collections::hash_map::DefaultHasher;
use std::f32::consts::{PI, TAU};
use std::f32::EPSILON;
use std::hash::{Hash, Hasher};
use approx::abs_diff_eq;
use bevy::prelude::*;
use truck_modeling::{builder, Shell, Surface, Wire};
use crate::tool::hash_tool::*;
use truck_meshalgo::prelude::*;
use bevy::reflect::Reflect;
use bevy::ecs::reflect::ReflectComponent;
use glam::Vec3;
use crate::pdms_types::AttrMap;
use serde::{Serialize, Deserialize};
use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::prim_geo::extrusion::Extrusion;
use crate::prim_geo::wire::*;
use crate::prim_geo::helper::cal_ref_axis;
use crate::shape::pdms_shape::{BrepMathTrait, BrepShapeTrait, PdmsMesh, TRI_TOL, VerifiedShape};
use crate::tool::float_tool::{hash_f32, hash_vec3};

#[derive(Component, Debug,  Clone,  Reflect, Serialize, Deserialize)]
#[reflect(Component)]
pub struct Revolution {
    pub verts: Vec<Vec3>, //loop vertex
    pub fradius_vec: Vec<f32>,
    pub angle: f32,     //degrees
    pub rot_dir: Vec3,
    pub rot_pt: Vec3,
}


impl Default for Revolution {
    fn default() -> Self {
        Self {
            verts: vec![Vec3::ZERO, Vec3::new(2.0, 0.0, 0.0), Vec3::new(2.0, 1.0, 0.0),
                             Vec3::new(1.0, 1.0, 0.0), Vec3::new(1.0, 2.0, 0.0), Vec3::new(0.0, 2.0, 0.0)],
            fradius_vec: vec![0.0; 6],
            angle: 90.0,
            rot_dir: Vec3::X,   //默认绕X轴旋转
            rot_pt: Vec3::ZERO, //默认旋转点
        }
    }
}

impl VerifiedShape for Revolution {
    fn check_valid(&self) -> bool{
        self.angle.abs() > std::f32::EPSILON
    }
}

//#[typetag::serde]
impl BrepShapeTrait for Revolution {

    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    fn gen_brep_shell(&self) -> Option<Shell> {
        if !self.check_valid() { return None; }

        let wire = gen_wire( &self.verts, &self.fradius_vec).unwrap();
        if let Ok(mut face) = builder::try_attach_plane(&[wire]){
            if let Surface::Plane(plane) = face.surface(){
                let mut rot_dir = self.rot_dir.normalize().vector3();
                let rot_pt = self.rot_pt.point3();
                let mut angle = self.angle.to_radians() as f64;
                // dbg!(angle);
                if plane.normal().dot(Vector3::new(0.0, 0.0, 1.0)) < 0.0 {
                    face = face.inverse();
                }
                if angle < 0.0 {
                    angle = -angle;
                    rot_dir = -rot_dir;
                }
                if abs_diff_eq!(angle as f32, TAU) {
                    let mut s = builder::rsweep(&face, rot_pt, rot_dir, Rad(angle/2.0)).into_boundaries();
                    let mut shell = s.pop();
                    if shell.is_none() {
                        dbg!(&self);
                    }
                    let face = face.inverse();
                    let mut s = builder::rsweep(&face, rot_pt, -rot_dir, Rad(angle/2.0)).into_boundaries();
                    shell.as_mut().unwrap().append(&mut s[0]);
                    return shell;
                }else{
                    let mut s = builder::rsweep(&face, rot_pt, rot_dir, Rad(angle)).into_boundaries();
                    let shell = s.pop();
                    if shell.is_none() {
                        dbg!(&self);
                    }
                    return shell;
                }

            }
        }else{
            dbg!(&self);
        }
        None
    }

    fn hash_unit_mesh_params(&self) -> u64{
        let mut hasher = DefaultHasher::new();
        self.verts.iter().for_each(|v|  {
            hash_vec3::<DefaultHasher>(v, &mut hasher);
        });
        "Revolution".hash(&mut hasher);
        hash_f32(self.angle, &mut hasher);
        hasher.finish()
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    fn gen_unit_mesh(&self) -> Option<PdmsMesh>{
        self.gen_mesh(Some(TRI_TOL/10.0))
    }
    fn get_scaled_vec3(&self) -> Vec3{
        Vec3::ONE
    }

    fn convert_to_geo_param(&self) -> Option<PdmsGeoParam> {
        Some(
            PdmsGeoParam::PrimRevolution(self.clone())
        )
    }
}