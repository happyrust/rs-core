use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
#[cfg(feature = "truck")]
use truck_modeling::builder::*;

use crate::parsed_data::geo_params_data::PdmsGeoParam;
#[cfg(feature = "occ")]
use crate::prim_geo::basic::OccSharedShape;
use crate::shape::pdms_shape::{BrepShapeTrait, VerifiedShape};
use bevy_ecs::prelude::*;
use glam::{DVec3, Vec3};
#[cfg(feature = "occ")]
use opencascade::primitives::*;
use serde::{Deserialize, Serialize};
#[cfg(feature = "truck")]
use truck_meshalgo::prelude::*;

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
pub struct LPyramid {
    pub pbax_pt: Vec3,
    pub pbax_dir: Vec3, //B Axis Direction

    pub pcax_pt: Vec3,
    pub pcax_dir: Vec3, //C Axis Direction

    pub paax_pt: Vec3,
    pub paax_dir: Vec3, //A Axis Direction

    pub pbtp: f32,
    pub pctp: f32,
    //y top
    pub pbbt: f32,
    pub pcbt: f32, // y bottom

    pub ptdi: f32,
    pub pbdi: f32,
    pub pbof: f32,
    // x offset
    pub pcof: f32, // y offset
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
    fn check_valid(&self) -> bool {
        let size_flag =
            self.pbtp * self.pctp >= f32::EPSILON || self.pbbt * self.pcbt >= f32::EPSILON;
        if !size_flag {
            return false;
        }
        (self.pbtp >= 0.0 && self.pctp >= 0.0 && self.pbbt >= 0.0 && self.pcbt >= 0.0)
            && ((self.pbtp + self.pctp) > f32::EPSILON || (self.pbbt + self.pcbt) > f32::EPSILON)
    }
}

//#[typetag::serde]
impl BrepShapeTrait for LPyramid {
    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    // truck 实现在局部坐标系中生成形状（X=B, Y=C, Z=A）
    // 需要考虑顶面退化为边或点的情况
    #[cfg(feature = "truck")]
    fn gen_brep_shell(&self) -> Option<truck_modeling::Shell> {
        let tx = (self.pbtp as f64 / 2.0).max(0.001);
        let ty = (self.pctp as f64 / 2.0).max(0.001);
        let bx = (self.pbbt as f64 / 2.0).max(0.001);
        let by = (self.pcbt as f64 / 2.0).max(0.001);
        
        // 偏移在局部坐标系中：PBOF -> X方向，PCOF -> Y方向
        let offset_x = self.pbof as f64;
        let offset_y = self.pcof as f64;
        let h2 = 0.5 * (self.ptdi - self.pbdi) as f64;

        // 顶面顶点（带偏移）
        let pts = vec![
            builder::vertex(Point3::new(-tx + offset_x, -ty + offset_y, h2)),
            builder::vertex(Point3::new(tx + offset_x, -ty + offset_y, h2)),
            builder::vertex(Point3::new(tx + offset_x, ty + offset_y, h2)),
            builder::vertex(Point3::new(-tx + offset_x, ty + offset_y, h2)),
        ];
        let ets = vec![
            builder::line(&pts[0], &pts[1]),
            builder::line(&pts[1], &pts[2]),
            builder::line(&pts[2], &pts[3]),
            builder::line(&pts[3], &pts[0]),
        ];

        // 底面顶点（无偏移）
        let pts = vec![
            builder::vertex(Point3::new(-bx, -by, -h2)),
            builder::vertex(Point3::new(bx, -by, -h2)),
            builder::vertex(Point3::new(bx, by, -h2)),
            builder::vertex(Point3::new(-bx, by, -h2)),
        ];
        let ebs = vec![
            builder::line(&pts[0], &pts[1]),
            builder::line(&pts[1], &pts[2]),
            builder::line(&pts[2], &pts[3]),
            builder::line(&pts[3], &pts[0]),
        ];

        let mut faces = vec![];
        if let Ok(f) = try_attach_plane(&[Wire::from_iter(&ebs)]) {
            faces.push(f.inverse());
        }
        if let Ok(f) = try_attach_plane(&[Wire::from_iter(&ets)]) {
            faces.push(f);
        }
        let mut shell: Shell = Shell::from(faces);
        shell.push(builder::homotopy(&ebs[0], &ets[0]));
        shell.push(builder::homotopy(&ebs[1], &ets[1]));
        shell.push(builder::homotopy(&ebs[2], &ets[2]));
        shell.push(builder::homotopy(&ebs[3], &ets[3]));

        Some(shell)
    }

    #[cfg(feature = "occ")]
    fn gen_occ_shape(&self) -> anyhow::Result<OccSharedShape> {
        // OCC 实现在局部坐标系中生成形状（X=B, Y=C, Z=A）
        // 外部 transform 负责坐标系旋转和定位
        let tx = (self.pbtp / 2.0).max(0.001) as f64;
        let ty = (self.pctp / 2.0).max(0.001) as f64;
        let bx = (self.pbbt / 2.0).max(0.001) as f64;
        let by = (self.pcbt / 2.0).max(0.001) as f64;
        
        // 偏移在局部坐标系中：PBOF -> X方向，PCOF -> Y方向
        let offset_x = self.pbof as f64;
        let offset_y = self.pcof as f64;
        
        let h2 = 0.5 * (self.ptdi - self.pbdi) as f64;

        let mut polys = vec![];
        let mut verts = vec![];

        // 顶面：带偏移
        let pts = vec![
            DVec3::new(-tx + offset_x, -ty + offset_y, h2),
            DVec3::new(tx + offset_x, -ty + offset_y, h2),
            DVec3::new(tx + offset_x, ty + offset_y, h2),
            DVec3::new(-tx + offset_x, ty + offset_y, h2),
        ];
        if tx + ty < f64::EPSILON {
            // 顶面退化为点
            verts.push(Vertex::new(DVec3::new(offset_x, offset_y, h2)));
        } else {
            polys.push(Wire::from_ordered_points(pts)?);
        }

        // 底面：无偏移
        let pts = vec![
            DVec3::new(-bx, -by, -h2),
            DVec3::new(bx, -by, -h2),
            DVec3::new(bx, by, -h2),
            DVec3::new(-bx, by, -h2),
        ];
        if bx + by < f64::EPSILON {
            // 底面退化为点
            verts.push(Vertex::new(DVec3::new(0.0, 0.0, -h2)));
        } else {
            polys.push(Wire::from_ordered_points(pts)?);
        }

        Ok(OccSharedShape::new(
            Solid::loft_with_points(polys.iter(), verts.iter())?.into_shape(),
        ))
    }

    fn hash_unit_mesh_params(&self) -> u64 {
        let bytes = bincode::serialize(self).unwrap();
        let mut hasher = DefaultHasher::default();
        bytes.hash(&mut hasher);
        "LPyramid".hash(&mut hasher);
        hasher.finish()
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    fn convert_to_geo_param(&self) -> Option<PdmsGeoParam> {
        Some(PdmsGeoParam::PrimLPyramid(self.clone()))
    }

    fn enhanced_key_points(
        &self,
        transform: &bevy_transform::prelude::Transform,
    ) -> Vec<(Vec3, String, u8)> {
        let mut points = Vec::new();

        let a_dir = self.paax_dir.normalize();
        let b_dir = self.pbax_dir.normalize();
        let c_dir = self.pcax_dir.normalize();

        // 顶面和底面的中心
        let top_center = self.paax_pt + a_dir * self.ptdi + b_dir * self.pbof + c_dir * self.pcof;
        let bottom_center = self.paax_pt + a_dir * self.pbdi;

        // 1. 顶面和底面中心（优先级100）
        points.push((
            transform.transform_point(top_center),
            "Center".to_string(),
            100,
        ));
        points.push((
            transform.transform_point(bottom_center),
            "Center".to_string(),
            100,
        ));

        // 2. 顶面的4个顶点（如果不是退化为点）
        if self.pbtp > 0.001 && self.pctp > 0.001 {
            let top_corners = [
                top_center + b_dir * self.pbtp / 2.0 + c_dir * self.pctp / 2.0,
                top_center + b_dir * self.pbtp / 2.0 - c_dir * self.pctp / 2.0,
                top_center - b_dir * self.pbtp / 2.0 + c_dir * self.pctp / 2.0,
                top_center - b_dir * self.pbtp / 2.0 - c_dir * self.pctp / 2.0,
            ];
            for corner in &top_corners {
                points.push((
                    transform.transform_point(*corner),
                    "Endpoint".to_string(),
                    90,
                ));
            }
        }

        // 3. 底面的4个顶点
        if self.pbbt > 0.001 && self.pcbt > 0.001 {
            let bottom_corners = [
                bottom_center + b_dir * self.pbbt / 2.0 + c_dir * self.pcbt / 2.0,
                bottom_center + b_dir * self.pbbt / 2.0 - c_dir * self.pcbt / 2.0,
                bottom_center - b_dir * self.pbbt / 2.0 + c_dir * self.pcbt / 2.0,
                bottom_center - b_dir * self.pbbt / 2.0 - c_dir * self.pcbt / 2.0,
            ];
            for corner in &bottom_corners {
                points.push((
                    transform.transform_point(*corner),
                    "Endpoint".to_string(),
                    90,
                ));
            }
        }

        points
    }
}
