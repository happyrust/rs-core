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
use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::prim_geo::SPHERE_GEO_HASH;
use crate::shape::pdms_shape::{BrepMathTrait, BrepShapeTrait, PlantMesh, VerifiedShape};
#[cfg(feature = "opencascade")]
use opencascade::OCCShape;
use crate::pdms_types::AttrMap;

#[derive(Component, Debug, Clone, Reflect, Serialize, Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize,)]
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

    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    //OCC 的生成
    #[cfg(feature = "opencascade")]
    fn gen_occ_shape(&self) -> anyhow::Result<OCCShape> {
        Ok(OCCShape::sphere(self.radius as f64)?)
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

    // #[cfg(feature = "truck")]
    // fn gen_mesh(&self) -> Option<PlantMesh> {
    //     let generated = IcoSphere::new(32, |point| {
    //         let inclination = point.y.acos();
    //         let azimuth = point.z.atan2(point.x);
    //
    //         let norm_inclination = inclination / std::f32::consts::PI;
    //         let norm_azimuth = 0.5 - (azimuth / std::f32::consts::TAU);
    //
    //         [norm_azimuth, norm_inclination]
    //     });
    //
    //     let raw_points = generated.raw_points();
    //
    //     let points = raw_points
    //         .iter()
    //         .map(|&p| (p * self.radius).into())
    //         .collect::<Vec<[f32; 3]>>();
    //
    //     let normals = raw_points
    //         .iter()
    //         .copied()
    //         .map(Into::into)
    //         .collect::<Vec<[f32; 3]>>();
    //
    //     let mut indices = Vec::with_capacity(generated.indices_per_main_triangle() * 20);
    //     for i in 0..20 {
    //         generated.get_indices(i, &mut indices);
    //     }
    //
    //     //球也需要提供wireframe的绘制
    //     return Some(PlantMesh{
    //         indices,
    //         vertices: points,
    //         normals,
    //         wire_vertices: vec![],
    //         aabb: None,
    //         #[cfg(feature = "opencascade")]
    //         occ_shape: None,
    //     });
    // }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(Sphere::default())
    }

    fn hash_unit_mesh_params(&self) -> u64{
        SPHERE_GEO_HASH            //代表SPHERE
    }

    #[inline]
    fn get_scaled_vec3(&self) -> Vec3 {
        Vec3::splat(self.radius)
    }

    fn convert_to_geo_param(&self) -> Option<PdmsGeoParam> {
        Some(
            PdmsGeoParam::PrimSphere(self.clone())
        )
    }
}

impl From<&AttrMap> for Sphere {
    fn from(m: &AttrMap) -> Self {
        Self {
            center: Default::default(),
            
            // size: Vec3::new(m.get_f32("XLEN").unwrap_or_default(),
            //                 m.get_f32("YLEN").unwrap_or_default(),
            //                 m.get_f32("ZLEN").unwrap_or_default(),),
            radius: m.get_f32("RADI").unwrap_or_default(),
        }
    }
}





