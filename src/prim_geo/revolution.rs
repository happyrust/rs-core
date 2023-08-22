use crate::shape::pdms_shape::PlantMesh;
use approx::abs_diff_eq;
use std::collections::hash_map::DefaultHasher;
use std::f32::consts::PI;
use std::hash::{Hash, Hasher};
use geo::{Intersects, LineString, Polygon};
use truck_meshalgo::prelude::*;

#[cfg(feature = "gen_model")]
use crate::csg::manifold::*;
use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::prim_geo::wire::*;
use crate::shape::pdms_shape::{BrepMathTrait, BrepShapeTrait, VerifiedShape};
use crate::tool::float_tool::{f32_round_3, hash_f32, hash_vec3};

use glam::{Vec2, Vec3};
#[cfg(feature = "opencascade_rs")]
use opencascade::angle::ToAngle;
#[cfg(feature = "opencascade_rs")]
use opencascade::primitives::Shape;
use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    rkyv::Archive,
    rkyv::Deserialize,
    rkyv::Serialize,
)]
pub struct Revolution {
    pub verts: Vec<Vec3>,
    //loop vertex
    pub fradius_vec: Vec<f32>,
    pub angle: f32,
    //degrees
    pub rot_dir: Vec3,
    pub rot_pt: Vec3,
}

impl Default for Revolution {
    fn default() -> Self {
        Self {
            verts: vec![
                Vec3::ZERO,
                Vec3::new(2.0, 0.0, 0.0),
                Vec3::new(2.0, 1.0, 0.0),
                Vec3::new(1.0, 1.0, 0.0),
                Vec3::new(1.0, 2.0, 0.0),
                Vec3::new(0.0, 2.0, 0.0),
            ],
            fradius_vec: vec![0.0; 6],
            angle: 90.0,
            rot_dir: Vec3::X,   //默认绕X轴旋转
            rot_pt: Vec3::ZERO, //默认旋转点
        }
    }
}

impl VerifiedShape for Revolution {
    fn check_valid(&self) -> bool {
        self.angle.abs() > std::f32::EPSILON
    }
}

impl BrepShapeTrait for Revolution {
    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    #[cfg(feature = "opencascade_rs")]
    fn gen_occ_shape(&self) -> anyhow::Result<Shape> {
        let wire = gen_occ_wire(&self.verts, &self.fradius_vec)?;
        let angle = if abs_diff_eq!(self.angle, 360.0, epsilon = 0.01) {
            core::f64::consts::TAU
        } else {
            self.angle.to_radians() as _
        };
        let r = wire.to_face().revolve(
            self.rot_pt.as_dvec3(),
            self.rot_dir.as_dvec3(),
            Some(angle.radians()),
        );
        return Ok(r.to_shape());
    }

    ///使用manifold生成拉身体的mesh
    #[cfg(feature = "gen_model")]
    fn gen_csg_mesh(&self) -> Option<PlantMesh> {
        use truck_meshalgo::prelude::*;
        use truck_modeling::{builder, Shell, Surface, Wire};
        if !self.check_valid() {
            return None;
        }
        let mut wire = gen_wire(&self.verts, &self.fradius_vec).unwrap();
        if let Ok(mut face) = builder::try_attach_plane(&[wire.clone()]) {
            if let Surface::Plane(plane) = face.surface() {
                let extrude_dir = Vector3::new(0.0, 0.0, 1.0);
                if plane.normal().dot(extrude_dir) < 0.0 {
                    wire = wire.inverse();
                }
                let e_len = wire.len();
                let pts = wire
                    .edge_iter()
                    .enumerate()
                    .map(|(i, e)| {
                        let curve = e.oriented_curve();
                        let polyline = PolylineCurve::from_curve(
                            &curve,
                            curve.parameter_range(),
                            self.tol() as _,
                        );
                        // dbg!(&polyline);
                        let mut v = polyline
                            .iter()
                            .map(|x| Vec2::new(x.x as _, x.y as _))
                            .collect::<Vec<_>>();
                        if !v.is_empty() && i != (e_len - 1) {
                            v.pop();
                        }
                        v
                    })
                    .flatten()
                    .collect::<Vec<Vec2>>();
                dbg!(&pts);
                unsafe {
                    let mut cross_section = ManifoldCrossSectionRust::from_points(&pts);
                    let manifold = cross_section.extrude_rotate(Vec3::ZERO);
                    return Some(PlantMesh::from(manifold));
                }
            }
        }
        None
    }

    #[inline]
    fn need_use_csg(&self) -> bool {
        // self.angle.abs()  >  (TAU - 0.01)
        false
    }

    #[inline]
    fn tol(&self) -> f32 {
        use parry2d::bounding_volume::Aabb;
        let pts = self
            .verts
            .iter()
            .map(|x| nalgebra::Point2::from(nalgebra::Vector2::from(x.truncate())))
            .collect::<Vec<_>>();
        let profile_aabb = Aabb::from_points(&pts);
        0.001 * profile_aabb.bounding_sphere().radius.max(1.0)
    }

    ///如果是沿自己的一条边旋转，需要弄清楚为啥三角化出来的不对
    fn gen_brep_shell(&self) -> Option<truck_modeling::Shell> {
        use truck_modeling::{builder, Surface};

        if !self.check_valid() {
            return None;
        }
        let wire = gen_wire(&self.verts, &self.fradius_vec).unwrap();
                let geo_2d: LineString<f64> = wire.vertex_iter().map(|x| (x.point().x, x.point().y)).collect();
        let polygon = Polygon::new(geo_2d,Vec::new());
        let geo_axis: LineString<f64> = vec![(0.0, 0.0), (100000.0, 0.0)].into();
        let intersect = polygon.intersects(&geo_axis);
        dbg!(intersect);
        if let Ok(mut face) = builder::try_attach_plane(&[wire]) {
            if let Surface::Plane(plane) = face.surface() {
                let mut rot_dir = self.rot_dir.normalize().vector3();
                let rot_pt = self.rot_pt.point3();
                //避免精度的误差
                let mut angle = (f32_round_3(self.angle) as f64).to_radians();
                let normal_flag = plane.normal().dot(Vector3::new(0.0, 0.0, 1.0)) < 0.0;
                let angle_flag = angle > 0.0;
                //如果两者一致，就不需要reverse
                let reverse_flag = !(normal_flag ^ angle_flag);
                if reverse_flag {
                    face = face.inverse();
                }
                if angle < 0.0 {
                    angle = -angle;
                    rot_dir = -rot_dir;
                }
                //check if exist any point on axis
                //允许有误差, 只有没有在旋转轴（X轴）坐标轴上时，需要这样处理
                if !intersect && angle.abs() >= (core::f64::consts::TAU - 0.01) {
                    let mut s =
                        builder::rsweep(&face, rot_pt, rot_dir, Rad(PI as f64)).into_boundaries();
                    let mut shell = s.pop();
                    if shell.is_none() {
                        dbg!(&self);
                    }
                    let face = face.inverse();
                    let mut s =
                        builder::rsweep(&face, rot_pt, -rot_dir, Rad(PI as f64)).into_boundaries();
                    shell.as_mut().unwrap().append(&mut s[0]);
                    return shell;
                }

                {
                    let s = builder::rsweep(&face, rot_pt, rot_dir, Rad(angle as f64));
                    let json = serde_json::to_vec_pretty(&s).unwrap();
                    // std::fs::write("revo.json", json).unwrap();
                    let shell = s.into_boundaries().pop();
                    if shell.is_none() {
                        dbg!(&self);
                    }

                    return shell;
                }
            }
        }
        None
    }

    fn hash_unit_mesh_params(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.verts.iter().for_each(|v| {
            hash_vec3::<DefaultHasher>(v, &mut hasher);
        });
        "Revolution".hash(&mut hasher);
        hash_f32(self.angle, &mut hasher);
        hasher.finish()
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    fn convert_to_geo_param(&self) -> Option<PdmsGeoParam> {
        Some(PdmsGeoParam::PrimRevolution(self.clone()))
    }
}
