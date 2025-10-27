use glam::Vec3;
use hexasphere::shapes::IcoSphere;
use std::f64::consts::PI;
use std::sync::Arc;
#[cfg(feature = "truck")]
use truck_base::cgmath64::{Point3, Rad, Vector3};
#[cfg(feature = "truck")]
use truck_modeling::Shell;
#[cfg(feature = "truck")]
use truck_modeling::*;

use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::prim_geo::basic::*;
use crate::shape::pdms_shape::{BrepShapeTrait, PlantMesh, RsVec3, VerifiedShape};
#[cfg(feature = "occ")]
use opencascade::primitives::*;
use serde::{Deserialize, Serialize};

use crate::NamedAttrMap;
use crate::types::attmap::AttrMap;
use bevy_ecs::prelude::*;

#[derive(
    Component,
    Debug,
    Clone,
    Serialize,
    Deserialize,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
//
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

    //由于geom kernel还不支持fixed point ，暂时不用这个shell去生成mesh
    #[cfg(feature = "truck")]
    fn gen_brep_shell(&self) -> Option<Shell> {
        let vertex = builder::vertex(Point3::new(0.0, 0.0, 1.0));
        let wire = builder::rsweep(&vertex, Point3::origin(), Vector3::unit_y(), Rad(PI));
        let shell = builder::rsweep(&wire, Point3::origin(), Vector3::unit_z(), Rad(PI * 2.0));
        Some(shell)
    }

    ///获得关键点
    fn key_points(&self) -> Vec<RsVec3> {
        vec![self.center.into()]
    }

    //OCC 的生成
    #[cfg(feature = "occ")]
    fn gen_occ_shape(&self) -> anyhow::Result<OccSharedShape> {
        Ok(OccSharedShape::new(
            Shape::sphere(self.radius as f64).build(),
        ))
    }

    fn hash_unit_mesh_params(&self) -> u64 {
        SPHERE_GEO_HASH //代表SPHERE
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(Sphere::default())
    }

    #[inline]
    fn get_scaled_vec3(&self) -> Vec3 {
        Vec3::splat(self.radius)
    }

    fn convert_to_geo_param(&self) -> Option<PdmsGeoParam> {
        Some(PdmsGeoParam::PrimSphere(self.clone()))
    }

    /// 为球体生成增强的关键点
    ///
    /// 包括：
    /// - 1个中心点（优先级100）
    /// - 6个主轴端点（优先级90）
    /// - 8个赤道圆周点（优先级70）
    fn enhanced_key_points(
        &self,
        transform: &bevy_transform::prelude::Transform,
    ) -> Vec<(Vec3, String, u8)> {
        let mut points = Vec::new();

        // 中心点（优先级最高：100）
        let world_center = transform.transform_point(self.center);
        points.push((world_center, "Center".to_string(), 100));

        // 6个主轴端点（优先级：90）
        let axis_points = [
            Vec3::new(self.radius, 0.0, 0.0),  // +X
            Vec3::new(-self.radius, 0.0, 0.0), // -X
            Vec3::new(0.0, self.radius, 0.0),  // +Y
            Vec3::new(0.0, -self.radius, 0.0), // -Y
            Vec3::new(0.0, 0.0, self.radius),  // +Z
            Vec3::new(0.0, 0.0, -self.radius), // -Z
        ];

        for axis_point in axis_points {
            let local_pos = self.center + axis_point;
            let world_pos = transform.transform_point(local_pos);
            points.push((world_pos, "Endpoint".to_string(), 90));
        }

        // 赤道圆周8个点（优先级：70）
        for i in 0..8 {
            let angle = (i as f32) * std::f32::consts::TAU / 8.0;
            let x = self.radius * angle.cos();
            let y = self.radius * angle.sin();
            let local_pos = self.center + Vec3::new(x, y, 0.0);
            let world_pos = transform.transform_point(local_pos);
            points.push((world_pos, "SurfacePoint".to_string(), 70));
        }

        points
    }

    ///直接通过基本体的参数，生成模型
    fn gen_csg_mesh(&self) -> Option<PlantMesh> {
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
            .map(|&p| Vec3::from((p * self.radius).to_array()))
            .collect::<Vec<Vec3>>();

        let normals = raw_points
            .iter()
            .map(|&p| Vec3::from(p.to_array()))
            .collect::<Vec<Vec3>>();

        let mut indices = Vec::with_capacity(generated.indices_per_main_triangle() * 20);
        for i in 0..20 {
            generated.get_indices(i, &mut indices);
        }

        //球也需要提供wireframe的绘制
        return Some(PlantMesh {
            indices,
            vertices: points,
            normals,
            wire_vertices: vec![],
            aabb: None,
        });
    }

    fn need_use_csg(&self) -> bool {
        true
    }
}

impl From<&AttrMap> for Sphere {
    fn from(m: &AttrMap) -> Self {
        Self {
            center: Default::default(),
            radius: m.get_f32("RADI").unwrap_or_default(),
        }
    }
}

impl From<&NamedAttrMap> for Sphere {
    fn from(m: &NamedAttrMap) -> Self {
        Self {
            center: Default::default(),
            radius: m.get_f32("RADI").unwrap_or_default(),
        }
    }
}
