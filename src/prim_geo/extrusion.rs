use std::collections::hash_map::DefaultHasher;
use std::f32::consts::PI;
use std::f32::EPSILON;
use std::hash::{Hash, Hasher};

use anyhow::anyhow;
use approx::abs_diff_eq;
use bevy::ecs::reflect::ReflectComponent;
use bevy::prelude::*;
use bevy::reflect::Reflect;
use glam::DVec3;
use nalgebra_glm::sin;
use serde::{Deserialize, Serialize};
use truck_meshalgo::prelude::*;
use truck_modeling::{builder, Shell, Surface, Wire};
use crate::parsed_data::geo_params_data::PdmsGeoParam;

use crate::pdms_types::AttrMap;
use crate::prim_geo::helper::{cal_ref_axis, RotateInfo};
use crate::prim_geo::wire::*;
use crate::shape::pdms_shape::*;
use crate::tool::float_tool::{hash_f32, hash_vec3};

#[derive(Component, Debug, Clone, Serialize, Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, )]
pub struct Extrusion {
    pub verts: Vec<Vec3>,
    pub fradius_vec: Vec<f32>,
    pub height: f32,
    pub cur_type: CurveType,
}

impl Default for Extrusion {
    fn default() -> Self {
        Self {
            verts: vec![],
            fradius_vec: vec![],
            height: 100.0,
            cur_type: CurveType::Fill,
        }
    }
}

impl VerifiedShape for Extrusion {
    fn check_valid(&self) -> bool {
        self.height > std::f32::EPSILON
    }
}

//#[typetag::serde]
impl BrepShapeTrait for Extrusion {
    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    ///限制参数大小，主要是对负实体的不合理进行限制
    fn apply_limit_by_size(&mut self, l: f32) {
        self.height = self.height.min(l);
        dbg!(&self.height);
    }

    #[cfg(feature = "opencascade")]
    fn gen_occ_shape(&self) -> anyhow::Result<opencascade::OCCShape> {
        if !self.check_valid() || self.verts.len() < 3 { return Err(anyhow!("Extrusion params not valid.")); }
        let mut wire = if let CurveType::Spline(thick) = self.cur_type {
            gen_occ_spline_wire(&self.verts, thick)?
        } else {
            gen_occ_wire(&self.verts, &self.fradius_vec)?
        };
        // dbg!(self);
        Ok(wire.extrude(DVec3::new(0., 0.0, self.height as _))?)
    }

    fn gen_brep_shell(&self) -> Option<Shell> {
        if !self.check_valid() { return None; }
        if self.verts.len() < 3 {
            return None;
        }
        let mut wire = Wire::new();
        if let CurveType::Spline(thick) = self.cur_type {
            wire = gen_spline_wire(&self.verts, thick).ok()?;
        } else {
            wire = gen_wire(&self.verts, &self.fradius_vec).ok()?;
        };
        if let Ok(mut face) = builder::try_attach_plane(&[wire.clone()]) {
            if let Surface::Plane(plane) = face.surface() {
                let extrude_dir = Vector3::new(0.0, 0.0, 1.0);
                if plane.normal().dot(extrude_dir) < 0.0 {
                    face = face.inverse();
                }
                let mut s = builder::tsweep(&face, extrude_dir * self.height as f64).into_boundaries();
                return s.pop();
            }
        } else {
            dbg!(self);
            error!("生成的wire有问题，数据：{:?}", self);
        }
        None
    }


    fn hash_unit_mesh_params(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.verts.iter().for_each(|v| {
            hash_vec3::<DefaultHasher>(v, &mut hasher);
        });
        self.fradius_vec.iter().for_each(|v| {
            hash_f32::<DefaultHasher>(*v, &mut hasher);
        });
        "Extrusion".hash(&mut hasher);
        hasher.finish()
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
        let unit = Self {
            verts: self.verts.clone(),
            height: 1000.0,   //开放一点大小,不然三角化出来的不对
            fradius_vec: self.fradius_vec.clone(),
            cur_type: self.cur_type.clone(),
            ..default()
        };
        Box::new(unit)
    }

    #[inline]
    fn tol(&self) -> f32 {
        use parry2d::bounding_volume::Aabb;
        let pts = self.verts.iter().map(|x|
            nalgebra::Point2::from(nalgebra::Vector2::from(x.truncate()))
        ).collect::<Vec<_>>();
        let profile_aabb = Aabb::from_points(&pts);
        0.002 * profile_aabb.bounding_sphere().radius.max(1.0)
    }

    //沿着指定方向拉伸 pbax_dir
    fn get_scaled_vec3(&self) -> Vec3 {
        Vec3::new(1.0, 1.0, (self.height as f32 / 1000.0))
    }

    fn convert_to_geo_param(&self) -> Option<PdmsGeoParam> {
        Some(
            PdmsGeoParam::PrimExtrusion(self.clone())
        )
    }
}


#[test]
fn test_circle_fradius() {
    let ext = Extrusion {
        verts: vec![
            Vec3::new(
                125.0, 125.0, 227.0,
            ),
            Vec3::new(
                125.0, -125.0, 227.0, ),
            Vec3::new(
                -125.0, -125.0, 227.0, ),
            Vec3::new(
                -125.0, 125.0, 227.0,
            ),
        ],
        fradius_vec: vec![125.0; 4],
        height: 100.0,
        ..default()
    };
    let r = ext.gen_brep_shell();
    dbg!(r);
}