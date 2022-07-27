use std::collections::hash_map::DefaultHasher;
use std::f32::consts::PI;
use std::f32::EPSILON;
use std::hash::{Hash, Hasher};

use bevy::ecs::reflect::ReflectComponent;
use bevy::prelude::*;
use bevy::reflect::Reflect;
use glam::Vec3;
use serde::{Deserialize, Serialize};
use truck_meshalgo::prelude::*;
use truck_modeling::{builder, Shell, Surface, Wire};
use truck_modeling::builder::try_attach_plane;
use crate::tool::hash_tool::*;
use crate::pdms_types::AttrMap;
use crate::prim_geo::helper::cal_ref_axis;
use crate::shape::pdms_shape::{BrepMathTrait, BrepShapeTrait, PdmsMesh, VerifiedShape};

#[derive(Component, Debug, Clone, Reflect, Serialize, Deserialize)]
#[reflect(Component)]
pub struct LPyramid {

    pub pbax_pt: Vec3,
    pub pbax_dir: Vec3,   //B Axis Direction

    pub pcax_pt: Vec3,
    pub pcax_dir: Vec3,   //C Axis Direction

    pub paax_pt: Vec3,
    pub paax_dir: Vec3,   //A Axis Direction


    pub pbtp: f32,
    pub pctp: f32,  //y top
    pub pbbt: f32,
    pub pcbt: f32,  // y bottom

    pub ptdi: f32,
    pub pbdi: f32,
    pub pbof: f32,  // x offset
    pub pcof: f32,  // y offset
}

impl Default for LPyramid {
    fn default() -> Self {
        Self {
            pbax_pt: Default::default(),
            pbax_dir: Vec3::X,
            pcax_pt: Default::default(),
            pcax_dir: Vec3::Y,
            paax_pt: Default::default(),
            paax_dir: Vec3::Z,
            pbtp: 1.0,
            pctp: 1.0,
            pbbt: 1.0,
            pcbt: 1.0,
            ptdi: 1.0,
            pbdi: 0.0,
            pbof: 0.0,
            pcof: 0.0,
        }
    }
}

impl VerifiedShape for LPyramid {
    fn check_valid(&self) -> bool { true }
}

impl BrepShapeTrait for LPyramid {
    fn hash_mesh_params(&self) -> u64 {
        let bytes = bincode::serialize(self).unwrap();
        let mut hasher = DefaultHasher::default();
        bytes.hash(&mut hasher);
        "LPyramid".hash(&mut hasher);
        hasher.finish()
    }

    //暂时不做可拉伸
    fn gen_unit_shape(&self) -> PdmsMesh {
        self.gen_mesh(None)
    }

    fn get_scaled_vec3(&self) -> Vec3 {
        Vec3::ONE
    }

    //涵盖的情况，需要考虑，上边只有一条边，和退化成点的情况
    fn gen_brep_shell(&self) -> Option<Shell> {
        use truck_modeling::*;
        let x_dir = self.pbax_dir.normalize().vector3();
        let x_pt = self.pbax_pt.point3();
        let y_dir = self.pcax_dir.normalize().vector3();
        let y_pt = self.pcax_pt.point3();
        let z_dir = self.paax_dir.normalize().vector3();
        let z_pt = self.paax_pt.point3();
        // dbg!(&z_dir);
        // dbg!(&z_pt);
        //todo 以防止出现有单个点的情况，暂时用这个模拟

        let tx = (self.pbtp as f64 / 2.0).max(0.001);
        let ty = (self.pctp as f64 / 2.0).max(0.001);
        let bx = (self.pbbt as f64 / 2.0).max(0.001);
        let by = (self.pcbt as f64 / 2.0).max(0.001);
        let ox = self.pbof as f64;
        let oy = self.pcof as f64;
        // dbg!(&oy);
        // dbg!(&x_dir);
        // dbg!(&y_dir);
        let h_vector = z_dir * (self.ptdi - self.pbdi) as f64;

        let t_pt = z_pt + x_dir * ox + y_dir * oy + h_vector;
        let b_pt = z_pt ;

        let pts = vec![
            builder::vertex(t_pt - tx * x_dir - ty * y_dir),
            builder::vertex(t_pt + tx * x_dir - ty * y_dir),
            builder::vertex(t_pt + tx * x_dir + ty * y_dir),
            builder::vertex(t_pt - tx * x_dir + ty * y_dir),
        ];

        let mut ets = vec![
            builder::line(&pts[0], &pts[1]),
            builder::line(&pts[1], &pts[2]),
            builder::line(&pts[2], &pts[3]),
            builder::line(&pts[3], &pts[0]),
        ];

        let pts = vec![
            builder::vertex(b_pt - bx * x_dir - by * y_dir),
            builder::vertex(b_pt + bx * x_dir - by * y_dir),
            builder::vertex(b_pt + bx * x_dir + by * y_dir),
            builder::vertex(b_pt - bx * x_dir + by * y_dir),
        ];
        let mut ebs = vec![
            builder::line(&pts[0], &pts[1]),
            builder::line(&pts[1], &pts[2]),
            builder::line(&pts[2], &pts[3]),
            builder::line(&pts[3], &pts[0]),
        ];

        let mut need_inverse = false;
        let mut faces = vec![];
        let n = h_vector.normalize();
        // dbg!(&n);
        if let Ok(f) = try_attach_plane(&[Wire::from_iter(&ets)]) {
            if let Surface::Plane(plane) = f.get_surface() {
                // dbg!(&plane.normal());
                if plane.normal().dot(h_vector.normalize()) < 0.0 {
                    need_inverse = true;
                }
            }
            // dbg!(need_inverse);
            if need_inverse {
                faces.push(f.inverse());
            }else{
                faces.push(f);
            }
        }

        if let Ok(f) = try_attach_plane(&[Wire::from_iter(&ebs)]) {

            if need_inverse {
                faces.push(f);
            }else{
                faces.push(f.inverse());
            }
        }

        let mut shell: Shell = Shell::from(faces);
        if need_inverse {
            shell.push(builder::homotopy(&ebs[0], &ets[0]).inverse());
            shell.push(builder::homotopy(&ebs[1], &ets[1]).inverse());
            shell.push(builder::homotopy(&ebs[2], &ets[2]).inverse());
            shell.push(builder::homotopy(&ebs[3], &ets[3]).inverse());
        }
        else{
            shell.push(builder::homotopy(&ebs[0], &ets[0]));
            shell.push(builder::homotopy(&ebs[1], &ets[1]));
            shell.push(builder::homotopy(&ebs[2], &ets[2]));
            shell.push(builder::homotopy(&ebs[3], &ets[3]));
        }

        Some(shell)
    }
}


