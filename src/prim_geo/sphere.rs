use std::f32::consts::PI;
use std::f32::EPSILON;
use bevy::prelude::*;
use truck_base::cgmath64::Vector3;
use truck_meshalgo::prelude::{MeshableShape, MeshedShape};
use truck_modeling::{builder, Shell, Solid};
use truck_polymesh::stl::IntoSTLIterator;
// use bevy_inspector_egui::Inspectable;
use bevy::reflect::Reflect;
use bevy::ecs::reflect::ReflectComponent;
use bevy::prelude::shape::Icosphere;

use lyon::math::size;
use hexasphere::shapes::IcoSphere;
use serde::{Serialize,Deserialize};
use crate::prim_geo::helper::quad_indices;
use crate::pdms_types::AiosAABB;

use crate::shape::pdms_shape::{BrepMathTrait, BrepShapeTrait, PdmsMesh, VerifiedShape};

#[derive(Component, Debug, /*Inspectable, Reflect,*/ Clone, Serialize, Deserialize)]
// #[reflect(Component)]
pub struct Sphere {
    pub center: Vec3,
    pub radius: f32,
}

impl Default for Sphere {
    fn default() -> Self {
        Sphere {
            center: Default::default(),
            radius: 1.0,
        }
    }
}



impl VerifiedShape for Sphere {
    #[inline]
    fn check_valid(&self) -> bool {
        self.radius > f32::EPSILON
    }
}

impl BrepShapeTrait for Sphere {

    fn gen_mesh(&self, tol: Option<f32>) -> PdmsMesh {
        let generated = IcoSphere::new(32, |point| {
            let inclination = point.y.acos();
            let azimuth = point.z.atan2(point.x);

            let norm_inclination = inclination / std::f32::consts::PI;
            let norm_azimuth = 0.5 - (azimuth / std::f32::consts::TAU);

            [norm_azimuth, norm_inclination]
        });

        let raw_points = generated.raw_points();

        let points = raw_points
            .iter()
            .map(|&p| (p * self.radius).into())
            .collect::<Vec<[f32; 3]>>();

        let normals = raw_points
            .iter()
            .copied()
            .map(Into::into)
            .collect::<Vec<[f32; 3]>>();

        // let uvs = generated.raw_data().to_owned();

        let mut indices = Vec::with_capacity(generated.indices_per_main_triangle() * 20);
        for i in 0..20 {
            generated.get_indices(i, &mut indices);
        }

        //球也需要提供wireframe的绘制
        return PdmsMesh{
            indices,
            vertices: points,
            normals,
            wf_indices: vec![],
            wf_vertices: vec![],
            aabb: AiosAABB{
                min: -Vec3::ONE,
                max: Vec3::ONE,
            }
        }
    }

    fn hash_mesh_params(&self) -> u64{
        3u64            //代表BOX
    }

    fn gen_unit_shape(&self) -> PdmsMesh{
        Sphere::default().gen_mesh(None)
    }

    #[inline]
    fn get_scaled_vec3(&self) -> Vec3 {
        Vec3::splat(self.radius)
    }
}


// impl From<&AttrMap> for Sphere {
//     fn from(m: &AttrMap) -> Self {
//         Sphere {
//             center: Default::default(),
//             size: Vec3::new(m.get_f32("XLEN").unwrap_or_default(),
//                             m.get_f32("YLEN").unwrap_or_default(),
//                             m.get_f32("ZLEN").unwrap_or_default(),),
//         }
//     }
// }

// impl From<AttrMap> for Sphere {
//     fn from(m: AttrMap) -> Self {
//         (&m).into()
//     }
// }



