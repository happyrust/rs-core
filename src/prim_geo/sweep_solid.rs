use std::collections::hash_map::DefaultHasher;
use std::f32::consts::{FRAC_PI_2, PI, TAU};
use std::f32::EPSILON;
use std::hash::{Hash, Hasher};
use anyhow::anyhow;
use std::default::default;
use approx::{abs_diff_eq, abs_diff_ne};
use bevy_ecs::reflect::ReflectComponent;
use bevy_math::prelude::*;
use crate::shape::pdms_shape::VerifiedShape;
use bevy_ecs::prelude::*;
use glam::{Vec3};
use serde::{Deserialize, Serialize};
use crate::parsed_data::{CateProfileParam, SannData, SProfileData};
use crate::prim_geo::helper::cal_ref_axis;
use crate::prim_geo::spine::*;
use crate::prim_geo::wire;
use crate::shape::pdms_shape::*;
use crate::tool::float_tool::{hash_f32, hash_vec3};

#[cfg(feature = "opencascade")]
use opencascade::{OCCShape, Wire, Axis, Edge, Point, DsShape};


///含有两边方向的，扫描体
#[derive(Component, Debug, Clone, Serialize, Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, )]
pub struct SweepSolid {
    pub profile: CateProfileParam,
    pub drns: Vec3,
    pub drne: Vec3,
    pub bangle: f32,
    pub plane_normal: Vec3,
    pub extrude_dir: Vec3,
    pub height: f32,
    pub path: SweepPath3D,
    pub lmirror: bool,
}


impl SweepSolid {
    #[inline]
    pub fn is_sloped(&self) -> bool {
        self.is_drns_sloped() || self.is_drne_sloped()
    }

    #[inline]
    pub fn is_drns_sloped(&self) -> bool {
        let dot_s = self.drns.dot(self.extrude_dir);
        abs_diff_ne!(dot_s.abs(), 1.0, epsilon = 0.001) && abs_diff_ne!(dot_s.abs(), 0.0, epsilon = 0.001)
    }

    #[inline]
    pub fn is_drne_sloped(&self) -> bool {
        let dot_e = self.drne.dot(self.extrude_dir);
        abs_diff_ne!(dot_e.abs(), 1.0, epsilon = 0.001) && abs_diff_ne!(dot_e.abs(), 0.0, epsilon = 0.001)
    }


    #[cfg(feature = "opencascade")]
    ///生成OCC的SANN 线框
    fn gen_occ_sann_wire(&self, origin: Vec2, sann: &SannData, is_btm: bool, r1: f32, r2: f32) -> anyhow::Result<Wire> {
        let (r1, r2) = if is_btm {
            (r1, r2)
        } else { (r2 + sann.drad - sann.dwid - sann.pwidth, r2 + sann.drad) };
        let mut z_axis = Vec3::Z;
        let mut a = sann.pangle.abs();
        let mut offset_pt = Vec3::ZERO;
        let mut rot_mat = Mat3::IDENTITY;
        let mut beta_rot = Quat::IDENTITY;
        let mut r_translation = Vec3::ZERO;
        offset_pt.x = -sann.plin_pos.x;
        offset_pt.y = -sann.plin_pos.y;
        match &self.path {
            SweepPath3D::SpineArc(d) => {
                let mut y_axis = d.pref_axis;
                let mut z_axis = self.plane_normal;
                r_translation.x = d.radius;
                if d.clock_wise {
                    z_axis = -z_axis;
                }
                let x_axis = y_axis.cross(z_axis).normalize();
                // dbg!((x_axis, y_axis, z_axis));
                rot_mat = Mat3::from_cols(x_axis, y_axis, z_axis);
                beta_rot = Quat::from_axis_angle(z_axis, self.bangle.to_radians());
                rot_mat = Mat3::from_quat(Quat::from_rotation_arc(self.plane_normal, Vec3::Z));
            }

            SweepPath3D::Line(d) => {
                rot_mat = Mat3::from_quat(Quat::from_rotation_arc(self.plane_normal, Vec3::Y));
                if d.is_spine {
                    dbg!(self.bangle.to_radians());
                    beta_rot = Quat::from_axis_angle(Vec3::Z, self.bangle.to_radians());
                }
            }
        }
        let p1: Point = Vec3::new(r1, 0.0, 0.0).into();
        let p2: Point = Vec3::new(r2, 0.0, 0.0).into();
        let p3: Point = Vec3::new(r2 * a.cos(), r2 * a.sin(), 0.0).into();
        let p4: Point = Vec3::new(r1 * a.cos(), r1 * a.sin(), 0.0).into();
        let b = a / 2.0;
        let t1: Point = Vec3::new(r2 * b.cos(), r2 * b.sin(), 0.0).into();
        let t2: Point = Vec3::new(r1 * b.cos(), r1 * b.sin(), 0.0).into();

        let mut wire = Wire::from_edges(&vec![
            Edge::new_line(&p1, &p2)?,
            Edge::new_arc(&p2, &t1, &p3)?,
            Edge::new_line(&p3, &p4)?,
            Edge::new_arc(&p4, &t2, &p1)?,
        ])?;
        let offset = offset_pt + Vec3::new(origin.x, origin.y, 0.0);
        let trans_mat = Mat4::from_translation(offset);
        let r_trans_mat = Mat4::from_translation(r_translation);

        let local_mat = Mat4::from_mat3(Mat3::from_quat(beta_rot) * rot_mat);
        let final_mat = r_trans_mat * local_mat * trans_mat;

        Ok(wire.g_transform(&final_mat.as_dmat4())?)

    }

    /// 生成sann的线框
    fn gen_sann_wire(&self, origin: Vec2, sann: &SannData, is_btm: bool, r1: f32, r2: f32) -> Option<truck_modeling::Wire> {
        use truck_modeling::{builder, Face, Shell, Surface, Wire};
        use truck_modeling::builder::try_attach_plane;

        let (r1, r2) = if is_btm {
            (r1, r2)
        } else { (r2 + sann.drad - sann.dwid - sann.pwidth, r2 + sann.drad) };
        // dbg!((r1, r2));
        use truck_base::cgmath64::*;
        let mut z_axis = Vec3::Z;
        let mut a = sann.pangle.abs();
        let mut offset_pt = Vec3::ZERO;
        let mut rot_mat = Mat3::IDENTITY;
        let mut beta_rot = Quat::IDENTITY;
        let mut r_translation = Vector3::new(0.0, 0.0, 0.0);
        offset_pt.x = -sann.plin_pos.x;
        offset_pt.y = -sann.plin_pos.y;
        match &self.path {
            SweepPath3D::SpineArc(d) => {
                let mut y_axis = d.pref_axis;
                let mut z_axis = self.plane_normal;
                r_translation.x = d.radius as f64;
                if !self.lmirror {
                    z_axis = -z_axis;
                    dbg!("lmirror");
                }
                let x_axis = y_axis.cross(z_axis).normalize();
                dbg!((x_axis, y_axis, z_axis));
                rot_mat = Mat3::from_cols(x_axis, y_axis, z_axis);
                beta_rot = Quat::from_axis_angle(z_axis, self.bangle.to_radians());
                rot_mat = Mat3::from_quat(Quat::from_rotation_arc(self.plane_normal, Vec3::Z));
            }

            SweepPath3D::Line(d) => {
                rot_mat = Mat3::from_quat(Quat::from_rotation_arc(self.plane_normal, Vec3::Y));
                if d.is_spine {
                    dbg!(self.bangle.to_radians());
                    beta_rot = Quat::from_axis_angle(Vec3::Z, self.bangle.to_radians());
                }
            }
        }
        let p1 = (Vec3::new(r1, 0.0, 0.0));
        let p2 = (Vec3::new(r2, 0.0, 0.0));
        let p3 = (Vec3::new(r2 * a.cos(), r2 * a.sin(), 0.0));
        let p4 = (Vec3::new(r1 * a.cos(), r1 * a.sin(), 0.0));

        let v1 = builder::vertex(p1.point3());
        let v2 = builder::vertex(p2.point3());
        let v3 = builder::vertex(p3.point3());
        let v4 = builder::vertex(p4.point3());
        let center_pt = Point3::new(0.0, 0.0, 0.0);
        let mut wire = Wire::from(vec![
            builder::line(&v1, &v2),
            builder::circle_arc_with_center(center_pt,
                                            &v2, &v3, z_axis.vector3(), Rad(a as f64)),
            builder::line(&v3, &v4),
            builder::circle_arc_with_center(center_pt,
                                            &v4, &v1, -z_axis.vector3(), Rad(a as f64)),
        ]);
        let offset = offset_pt + Vec3::new(origin.x, origin.y, 0.0);
        let translation = Matrix4::from_translation(offset.vector3());
        let r_trans_mat = Matrix4::from_translation(r_translation);
        let m = &rot_mat;
        let local_mat = Matrix4::from_cols(
            m.x_axis.vector4(),
            m.y_axis.vector4(),
            m.z_axis.vector4(),
            Vector4::new(0.0, 0.0, 0.0, 1.0),
        );
        let m = Mat3::from_quat(beta_rot);
        let beta_mat = Matrix4::from_cols(
            m.x_axis.vector4(),
            m.y_axis.vector4(),
            m.z_axis.vector4(),
            Vector4::new(0.0, 0.0, 0.0, 1.0),
        );
        Some(builder::transformed(&wire, r_trans_mat * beta_mat * local_mat * translation))
    }

    #[cfg(feature = "opencascade")]
    ///计算SPRO的face
    /// start_vec 为起始方向
    fn gen_occ_spro_wire(&self, profile: &SProfileData) -> anyhow::Result<Wire> {
        let verts = &profile.verts;
        let len = verts.len();

        let mut offset_pt = Vec3::ZERO;
        let mut rot_mat = Mat3::IDENTITY;
        let mut beta_rot = Quat::IDENTITY;
        let mut r_translation = Vec3::ZERO;
        let plin_pos = profile.plin_pos;
        // dbg!(&profile);
        offset_pt.x = -plin_pos.x;
        offset_pt.y = -plin_pos.y;
        match &self.path {
            SweepPath3D::SpineArc(d) => {
                let mut y_axis = d.pref_axis;
                let mut z_axis = self.plane_normal;
                // dbg!(z_axis);
                r_translation.x = d.radius;
                if d.clock_wise {
                    z_axis = -z_axis;
                }
                let x_axis = y_axis.cross(z_axis).normalize();
                // dbg!((x_axis, y_axis, z_axis));
                //旋转到期望的平面
                rot_mat = Mat3::from_cols(x_axis, y_axis, z_axis);
                beta_rot = Quat::from_axis_angle(z_axis, self.bangle.to_radians());
            }
            SweepPath3D::Line(d) => {
                rot_mat = Mat3::from_quat(Quat::from_rotation_arc(self.plane_normal, Vec3::Y));
                if d.is_spine {
                    // dbg!(self.bangle.to_radians());
                    beta_rot = Quat::from_axis_angle(Vec3::Z, self.bangle.to_radians());
                }
            }
        }
        let mut points = vec![];
        for i in 0..len {
            let p = Vec3::new(verts[i][0], verts[i][1], 0.0);
            points.push(p);
        }
        let mut wire = wire::gen_occ_wire(&points, &profile.frads)?;
        let trans_mat = Mat4::from_translation(offset_pt);
        let r_trans_mat = Mat4::from_translation(r_translation);
        let local_mat = Mat4::from_mat3(Mat3::from_quat(beta_rot) * rot_mat);
        let final_mat = r_trans_mat * local_mat * trans_mat;

        Ok(wire.g_transform(&final_mat.as_dmat4())?)
    }


    ///计算SPRO的face
    /// start_vec 为起始方向
    fn cal_spro_wire(&self, profile: &SProfileData) -> Option<truck_modeling::Wire> {
        use truck_meshalgo::prelude::*;
        use truck_modeling::{builder, Face, Shell, Surface, Wire};
        use truck_modeling::builder::try_attach_plane;
        let verts = &profile.verts;
        let len = verts.len();

        let mut offset_pt = Vec3::ZERO;
        let mut rot_mat = Mat3::IDENTITY;
        let mut beta_rot = Quat::IDENTITY;
        let mut r_translation = Vector3::new(0.0, 0.0, 0.0);
        let plin_pos = profile.plin_pos;
        // dbg!(&profile);
        offset_pt.x = -plin_pos.x;
        offset_pt.y = -plin_pos.y;
        match &self.path {
            SweepPath3D::SpineArc(d) => {
                let mut y_axis = d.pref_axis;
                let mut z_axis = self.plane_normal;
                r_translation.x = d.radius as f64;
                if d.clock_wise {
                    z_axis = -z_axis;
                }
                if self.lmirror {
                    z_axis = -z_axis;
                }
                let x_axis = y_axis.cross(z_axis).normalize();
                //旋转到期望的平面
                rot_mat = Mat3::from_cols(x_axis, y_axis, z_axis);
                beta_rot = Quat::from_axis_angle(z_axis, self.bangle.to_radians());
            }
            SweepPath3D::Line(d) => {
                rot_mat = Mat3::from_quat(Quat::from_rotation_arc(self.plane_normal, Vec3::Y));
                if d.is_spine {
                    beta_rot = Quat::from_axis_angle(Vec3::Z, self.bangle.to_radians());
                }
            }
        }

        // dbg!(&offset_pt);
        let mut points = vec![];
        for i in 0..len {
            let p = Vec3::new(verts[i][0], verts[i][1], 0.0);
            points.push(p);
        }
        let mut wire = wire::gen_wire(&points, &profile.frads).ok()?;
        // dbg!(self.bangle);
        let translation = Matrix4::from_translation(offset_pt.vector3());
        // dbg!(r_translation);
        let r_trans_mat = Matrix4::from_translation(r_translation);
        let m = &rot_mat;
        let local_mat = Matrix4::from_cols(
            m.x_axis.vector4(),
            m.y_axis.vector4(),
            m.z_axis.vector4(),
            Vector4::new(0.0, 0.0, 0.0, 1.0),
        );
        let m = Mat3::from_quat(beta_rot);
        let beta_mat = Matrix4::from_cols(
            m.x_axis.vector4(),
            m.y_axis.vector4(),
            m.z_axis.vector4(),
            Vector4::new(0.0, 0.0, 0.0, 1.0),
        );
        Some(builder::transformed(&wire, r_trans_mat * beta_mat * local_mat * translation))
    }
}

impl Default for SweepSolid {
    fn default() -> Self {
        Self {
            profile: CateProfileParam::UNKOWN,
            drns: Default::default(),
            drne: Default::default(),
            bangle: 0.0,
            plane_normal: Vec3::Z,
            extrude_dir: Vec3::Z,
            ..default()
        }
    }
}

impl VerifiedShape for SweepSolid {
    fn check_valid(&self) -> bool { !self.extrude_dir.is_nan() && self.extrude_dir.length() > 0.0 }
}
