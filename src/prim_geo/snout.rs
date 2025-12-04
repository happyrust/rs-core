use crate::parsed_data::geo_params_data::PdmsGeoParam;
#[cfg(feature = "truck")]
use crate::shape::pdms_shape::BrepMathTrait;
use crate::shape::pdms_shape::{BrepShapeTrait, VerifiedShape};
use crate::tool::float_tool::hash_f32;
use crate::types::attmap::AttrMap;
use glam::Vec3;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::f32::EPSILON;
use std::hash::Hash;
use std::hash::Hasher;
#[cfg(feature = "truck")]
use truck_meshalgo::prelude::*;
#[cfg(feature = "truck")]
use truck_modeling::Shell;
#[cfg(feature = "truck")]
use truck_modeling::builder::*;

use crate::NamedAttrMap;
#[cfg(feature = "occ")]
use crate::prim_geo::basic::OccSharedShape;
use bevy_ecs::prelude::*;
#[cfg(feature = "occ")]
use opencascade::primitives::*;

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
pub struct LSnout {
    pub paax_expr: String,
    pub paax_pt: Vec3,  //A Axis point
    pub paax_dir: Vec3, //A Axis Direction

    pub pbax_expr: String,
    pub pbax_pt: Vec3,  //B Axis point
    pub pbax_dir: Vec3, //B Axis Direction

    pub ptdi: f32, //dist to top
    pub pbdi: f32, //dist to bottom
    pub ptdm: f32, //top diameter
    pub pbdm: f32, //bottom diameter
    pub poff: f32, //offset

    pub btm_on_top: bool,
}

impl Default for LSnout {
    fn default() -> Self {
        Self {
            paax_expr: "Z".to_string(),
            paax_pt: Default::default(),
            paax_dir: Vec3::Z,

            pbax_expr: "X".to_string(),
            pbax_pt: Default::default(),
            pbax_dir: Vec3::X,

            ptdi: 0.5,
            pbdi: -0.5,
            ptdm: 1.0,
            pbdm: 1.0,
            poff: 0.0,
            btm_on_top: false,
        }
    }
}

impl VerifiedShape for LSnout {
    #[inline]
    fn check_valid(&self) -> bool {
        //height 必须 >0， 小于0 的情况直接用变换矩阵
        (self.ptdm >= 0.0 && self.pbdm >= 0.0 && (self.ptdm + self.pbdm) > 0.0)
            && (self.ptdi - self.pbdi) > f32::EPSILON
    }
}

//#[typetag::serde]
impl BrepShapeTrait for LSnout {
    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    #[inline]
    fn tol(&self) -> f32 {
        //以最小的圆精度为准
        0.005 * ((self.pbdm + self.ptdm) / 2.0).max(1.0)
    }

    #[cfg(feature = "occ")]
    fn gen_occ_shape(&self) -> anyhow::Result<OccSharedShape> {
        let rt = self.ptdm / 2.0;
        let rb = self.pbdm / 2.0;

        let a_dir = self.paax_dir.normalize();
        let b_dir = self.pbax_dir.normalize();
        let p0 = a_dir * self.pbdi + self.paax_pt;
        let p1 = a_dir * self.ptdi + self.paax_pt + self.poff * b_dir;

        let mut circles = vec![];
        let mut verts = vec![];
        if self.pbdm < f32::EPSILON {
            verts.push(Vertex::new(p0.as_dvec3()));
        } else {
            let circle = Wire::circle(rb as _, p0.as_dvec3(), a_dir.as_dvec3());
            circles.push(circle);
        }

        if self.ptdm < f32::EPSILON {
            verts.push(Vertex::new(p1.as_dvec3()));
        } else {
            let circle = Wire::circle(rt as _, p1.as_dvec3(), a_dir.as_dvec3());
            circles.push(circle);
        }

        Ok(OccSharedShape::new(
            Solid::loft_with_points(circles.iter(), verts.iter())?.into(),
        ))
    }

    #[cfg(feature = "truck")]
    fn gen_brep_shell(&self) -> Option<Shell> {
        let rt = (self.ptdm / 2.0).max(0.01);
        let rb = (self.pbdm / 2.0).max(0.01);

        let a_dir = self.paax_dir.normalize();
        let b_dir = self.pbax_dir.normalize();
        let p0 = a_dir * self.pbdi + self.paax_pt;
        let p1 = a_dir * self.ptdi + self.paax_pt + self.poff * b_dir;
        let p2 = b_dir * rt + p1;
        let p3 = b_dir * rb + p0;
        let v2 = builder::vertex(p2.point3());
        let v3 = builder::vertex(p3.point3());

        //todo 表达cone的情况
        let mut is_cone = false;
        if self.ptdm * self.pbdm < 0.001 {
            is_cone = true;
        }
        // dbg!(is_cone);

        let rot_axis = a_dir.vector3();
        let mut circle1 = builder::rsweep(&v3, p0.point3(), rot_axis, Rad(7.0));
        let c1 = circle1.clone();

        let mut circle2 = builder::rsweep(&v2, p1.point3(), rot_axis, Rad(7.0));
        let c2 = circle2.clone();

        // dbg!((circle1.len(), circle2.len()));

        let new_wire_1 = circle1.split_off((0.5 * circle1.len() as f32) as usize);
        let new_wire_2 = circle2.split_off((0.5 * circle2.len() as f32) as usize);
        // dbg!((circle1.len(), circle2.len()));
        // dbg!((new_wire_1.len(), new_wire_2.len()));
        let shell1 = builder::try_wire_homotopy(&new_wire_1, &new_wire_2).ok()?;
        let shell2 = builder::try_wire_homotopy(&circle1, &circle2).ok()?;

        if let Ok(disk1) = builder::try_attach_plane(&vec![c1.inverse()]) {
            if let Ok(disk2) = builder::try_attach_plane(&vec![c2]) {
                let mut shell = Shell::from(vec![disk1, disk2]);
                shell.extend(shell1);
                shell.extend(shell2);
                return Some(shell);
            }
        }
        None
    }

    fn hash_unit_mesh_params(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        //对于有偏移的，直接不复用，后面看情况再考虑复用
        if self.poff.abs() > f32::EPSILON {
            let bytes = bincode::serialize(self).unwrap();
            let mut hasher = DefaultHasher::default();
            bytes.hash(&mut hasher);
            return hasher.finish();
        }
        let pheight = (self.ptdi - self.pbdi) > 0.0;
        let alpha = if self.pbdm != 0.0 {
            self.ptdm / self.pbdm
        } else {
            0.0
        };
        hash_f32(alpha, &mut hasher);
        pheight.hash(&mut hasher);
        "snout".hash(&mut hasher);
        hasher.finish()
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
        if self.poff.abs() > f32::EPSILON {
            Box::new(self.clone())
        } else {
            if self.ptdm < 0.001 {
                Box::new(Self {
                    ptdi: 0.5,
                    pbdi: -0.5,
                    ptdm: 0.0,
                    pbdm: 1.0,
                    ..Default::default()
                })
            } else if self.pbdm < 0.001 {
                Box::new(Self {
                    ptdi: 0.5,
                    pbdi: -0.5,
                    ptdm: 1.0,
                    pbdm: 0.0,
                    ..Default::default()
                })
            } else {
                let ptdm = self.ptdm / self.pbdm;
                Box::new(Self {
                    ptdi: 0.5,
                    pbdi: -0.5,
                    ptdm,
                    pbdm: 1.0,
                    ..Default::default()
                })
            }
        }
    }

    #[inline]
    fn get_scaled_vec3(&self) -> Vec3 {
        let pheight = (self.ptdi - self.pbdi).abs();
        //有偏心的时候，不缩放
        if self.poff.abs() > f32::EPSILON {
            Vec3::ONE
        } else {
            if self.pbdm < 0.001 {
                Vec3::new(self.ptdm, self.ptdm, pheight)
            } else {
                Vec3::new(self.pbdm, self.pbdm, pheight)
            }
        }
    }

    fn convert_to_geo_param(&self) -> Option<PdmsGeoParam> {
        Some(PdmsGeoParam::PrimLSnout(self.clone()))
    }

    fn enhanced_key_points(
        &self,
        transform: &bevy_transform::prelude::Transform,
    ) -> Vec<(Vec3, String, u8)> {
        use crate::prim_geo::helper::cal_ref_axis;

        let mut points = Vec::new();

        let a_dir = self.paax_dir.normalize();
        let b_dir = self.pbax_dir.normalize();

        // 底面中心和顶面中心
        let bottom_center = a_dir * self.pbdi + self.paax_pt;
        let top_center = a_dir * self.ptdi + self.paax_pt + self.poff * b_dir;

        let rb = self.pbdm / 2.0; // 底面半径
        let rt = self.ptdm / 2.0; // 顶面半径

        // 1. 底面中心和顶面中心（优先级100）
        points.push((
            transform.transform_point(bottom_center),
            "Center".to_string(),
            100,
        ));
        points.push((
            transform.transform_point(top_center),
            "Center".to_string(),
            100,
        ));

        // 计算垂直于轴的两个正交向量
        let ref_axis = cal_ref_axis(&a_dir);
        let perp1 = ref_axis.normalize();
        let perp2 = a_dir.cross(perp1).normalize();

        // 2. 底面圆周的8个点（优先级80）
        if rb > EPSILON {
            for i in 0..8 {
                let angle = i as f32 * std::f32::consts::PI / 4.0;
                let offset = perp1 * angle.cos() * rb + perp2 * angle.sin() * rb;
                points.push((
                    transform.transform_point(bottom_center + offset),
                    "Endpoint".to_string(),
                    80,
                ));
            }
        }

        // 3. 顶面圆周的8个点（优先级80）
        if rt > EPSILON {
            for i in 0..8 {
                let angle = i as f32 * std::f32::consts::PI / 4.0;
                let offset = perp1 * angle.cos() * rt + perp2 * angle.sin() * rt;
                points.push((
                    transform.transform_point(top_center + offset),
                    "Endpoint".to_string(),
                    80,
                ));
            }
        }

        // 4. 侧面中线点（优先级70）
        let mid_center = (bottom_center + top_center) / 2.0;
        let mid_radius = (rb + rt) / 2.0;
        if mid_radius > EPSILON {
            for i in 0..4 {
                let angle = i as f32 * std::f32::consts::PI / 2.0;
                let offset = perp1 * angle.cos() * mid_radius + perp2 * angle.sin() * mid_radius;
                points.push((
                    transform.transform_point(mid_center + offset),
                    "Midpoint".to_string(),
                    70,
                ));
            }
        }

        points
    }
}

impl From<&AttrMap> for LSnout {
    fn from(m: &AttrMap) -> Self {
        let h = m.get_f32("HEIG").unwrap_or_default();
        LSnout {
            ptdi: h / 2.0,
            pbdi: -h / 2.0,
            ptdm: m.get_f32("DTOP").unwrap_or_default(),
            pbdm: m.get_f32("DBOT").unwrap_or_default(),
            ..Default::default()
        }
    }
}

impl From<AttrMap> for LSnout {
    fn from(m: AttrMap) -> Self {
        (&m).into()
    }
}

impl From<&NamedAttrMap> for LSnout {
    fn from(m: &NamedAttrMap) -> Self {
        let h = m.get_f32("HEIG").unwrap_or_default();
        LSnout {
            ptdi: h / 2.0,
            pbdi: -h / 2.0,
            ptdm: m.get_f32("DTOP").unwrap_or_default(),
            pbdm: m.get_f32("DBOT").unwrap_or_default(),
            ..Default::default()
        }
    }
}

impl From<NamedAttrMap> for LSnout {
    fn from(m: NamedAttrMap) -> Self {
        (&m).into()
    }
}
