use std::f64::consts::PI;
use std::f32::EPSILON;
use bevy::prelude::*;
use truck_base::cgmath64::Vector3;
use truck_meshalgo::prelude::{MeshableShape, MeshedShape};
use truck_modeling::{builder, Shell, Solid};
use crate::tool::hash_tool::*;
use bevy::reflect::Reflect;
use bevy::ecs::reflect::ReflectComponent;

use lyon::math::size;
use hexasphere::shapes::IcoSphere;
use nalgebra::Point3;
use parry3d::bounding_volume::Aabb;
use parry3d::math::{Point, Vector};
use serde::{Serialize,Deserialize};
use crate::prim_geo::SPHERE_GEO_HASH;
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

//#[typetag::serde]
impl BrepShapeTrait for Sphere {

    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    //由于geom kernel还不支持fixed point ，暂时不用这个shell去生成mesh
    fn gen_brep_shell(&self) -> Option<Shell> {
        use truck_base::cgmath64::{Point3, Vector3, Rad};
        use truck_modeling::*;
        let vertex = builder::vertex(Point3::new(0.0, 0.0, 1.0));
        let wire = builder::rsweep(&vertex, Point3::origin(), Vector3::unit_y(), Rad(PI));
        let shell = builder::rsweep(&wire, Point3::origin(), Vector3::unit_z(), Rad(PI * 2.0));
        Some(shell)
    }

    fn gen_mesh(&self, tol: Option<f32>) -> Option<PdmsMesh> {
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

        let mut indices = Vec::with_capacity(generated.indices_per_main_triangle() * 20);
        for i in 0..20 {
            generated.get_indices(i, &mut indices);
        }

        //球也需要提供wireframe的绘制
        return Some(PdmsMesh{
            indices,
            vertices: points,
            normals,
            wf_indices: vec![],
            wf_vertices: vec![],
            aabb: Some(Aabb::new(Point::new(-1.0, -1.0, -1.0), Point::new(1.0, 1.0, 1.0))),
            // AiosAABB{
            //     min: -Vec3::ONE,
            //     max: Vec3::ONE,
            // },
            unit_shape: Sphere::default().gen_brep_shell().unwrap(),
        });
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {

        Box::new(Sphere::default())
    }

    fn hash_unit_mesh_params(&self) -> u64{
        SPHERE_GEO_HASH            //代表SPHERE
    }

    fn gen_unit_mesh(&self) -> Option<PdmsMesh>{
        Sphere::default().gen_mesh(None)
    }

    #[inline]
    fn get_scaled_vec3(&self) -> Vec3 {
        Vec3::splat(self.radius)
    }
}





