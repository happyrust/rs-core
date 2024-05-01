use crate::parsed_data::{CateProfileParam, SProfileData, SannData};
use crate::prim_geo::spine::*;
use crate::prim_geo::wire;
#[cfg(feature = "truck")]
use crate::shape::pdms_shape::{convert_to_cg_matrix4, BrepMathTrait};
use crate::shape::pdms_shape::{BrepShapeTrait, VerifiedShape, ANGLE_RAD_F64_TOL};
use crate::tool::math_tool::{quat_to_pdms_ori_str, to_pdms_ori_str};
use anyhow::anyhow;
use approx::abs_diff_ne;
use bevy_ecs::prelude::*;
use bevy_math::prelude::*;
use cavalier_contours::core::math::bulge_from_angle;
use cavalier_contours::polyline::{seg_midpoint, PlineSourceMut, Polyline, PlineSource};
use glam::*;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use crate::parsed_data::geo_params_data::PdmsGeoParam;
#[cfg(feature = "occ")]
use crate::prim_geo::basic::OccSharedShape;
#[cfg(feature = "occ")]
use opencascade::angle::ToAngle;
#[cfg(feature = "occ")]
use opencascade::primitives::*;
#[cfg(feature = "truck")]
use truck_base::cgmath64::*;
use crate::prim_geo::wire::polyline_to_debug_json_str;

///含有两边方向的，扫描体
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
pub struct SweepSolid {
    pub profile: CateProfileParam,
    #[serde(default)]
    pub drns_rot: DQuat,
    #[serde(default)]
    pub drne_rot: DQuat,
    pub bangle: f32,
    pub plax: Vec3,
    pub extrude_dir: DVec3,
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
        self.drns_rot.to_axis_angle().1.abs() > ANGLE_RAD_F64_TOL
    }

    #[inline]
    pub fn is_drne_sloped(&self) -> bool {
        self.drne_rot.to_axis_angle().1.abs() > ANGLE_RAD_F64_TOL
    }

    #[cfg(feature = "occ")]
    ///生成OCC的SANN 线框
    fn gen_occ_sann_wire(
        &self,
        origin: DVec2,
        sann: &SannData,
        is_btm: bool,
        mut r1: f64,
        mut r2: f64,
    ) -> anyhow::Result<Wire> {
        if !is_btm {
            r1 = r2 + (sann.drad - sann.dwid - sann.pwidth) as f64;
            r2 = r2 + sann.drad as f64;
        };
        // dbg!((r1, r2));
        // dbg!(&self);
        let angle = sann.pangle.to_radians() as f64;
        let mut offset_pt = DVec3::ZERO;
        let mut rot_mat = DMat3::IDENTITY;
        let mut beta_rot = DQuat::IDENTITY;
        let mut r_translation = DVec3::ZERO;
        offset_pt.x = -sann.plin_pos.x as f64;
        offset_pt.y = -sann.plin_pos.y as f64;
        match &self.path {
            SweepPath3D::SpineArc(d) => {
                let y_axis = d.pref_axis.as_dvec3();
                let mut z_axis = self.plax.as_dvec3();
                r_translation.x = d.radius as f64;
                if d.clock_wise {
                    z_axis = -z_axis;
                }
                if self.lmirror {
                    z_axis = -z_axis;
                }
                let x_axis = y_axis.cross(z_axis).normalize();
                dbg!((x_axis, y_axis, z_axis));
                rot_mat = DMat3::from_cols(x_axis, y_axis, z_axis);
                beta_rot = DQuat::from_axis_angle(z_axis, self.bangle.to_radians() as _);
                rot_mat =
                    DMat3::from_quat(DQuat::from_rotation_arc(self.plax.as_dvec3(), DVec3::Z));
            }

            SweepPath3D::Line(d) => {
                rot_mat =
                    DMat3::from_quat(DQuat::from_rotation_arc(self.plax.as_dvec3(), DVec3::Y));
                if d.is_spine {
                    dbg!(self.bangle.to_radians());
                    beta_rot = DQuat::from_axis_angle(DVec3::Z, self.bangle.to_radians() as _);
                }
            }
        }
        let p1 = DVec3::new(r1, 0.0, 0.0);
        let p2 = DVec3::new(r2, 0.0, 0.0);
        let p3 = DVec3::new(r2 * angle.cos(), r2 * angle.sin(), 0.0);
        let p4 = DVec3::new(r1 * angle.cos(), r1 * angle.sin(), 0.0);

        // dbg!((p1, p2, p3, p4));
        let mut polyline = Polyline::new_closed();
        let bulge = bulge_from_angle(angle);
        if r1 < 0.0001 {
            polyline.add(p2.x, p2.y, bulge);
            polyline.add(p3.x, p3.y, 0.0);
            // Wire::from_edges(&vec![
            //     Edge::segment(p1, p3),
            //     Edge::arc(p2, DVec3::new(0.0, r2, 0.0), p1),
            // ])
        } else {
            polyline.add(p1.x, p1.y, 0.0);
            polyline.add(p2.x, p2.y, bulge);
            polyline.add(p3.x, p3.y, -bulge);
            polyline.add(p4.x, p4.y, 0.0);
        };

        println!("Sweep polyline is {}", polyline_to_debug_json_str(&polyline));

        let mut edges = vec![];
        for (p, q) in polyline.iter_segments() {
            if p.bulge.abs() < 0.0001 {
                edges.push(Edge::segment(
                    DVec3::new(p.x, p.y, 0.0),
                    DVec3::new(q.x, q.y, 0.0),
                ));
            } else {
                let m = seg_midpoint(p, q);
                edges.push(Edge::arc(
                    DVec3::new(p.x, p.y, 0.0),
                    DVec3::new(m.x, m.y, 0.0),
                    DVec3::new(q.x, q.y, 0.0),
                ));
            }
        }
        let mut wire = Wire::from_edges(&edges);

        let offset = offset_pt + DVec3::new(origin.x, origin.y, 0.0);
        let translation = DMat4::from_translation(offset);
        let r_trans_mat = DMat4::from_translation(r_translation);
        let local_mat = DMat4::from_mat3(rot_mat);
        let m = DMat3::from_quat(beta_rot);
        let beta_mat = DMat4::from_mat3(m);
        let final_mat = r_trans_mat * beta_mat * local_mat * translation;
        // dbg!(final_mat);

        Ok(wire.transformed_by_gmat(&final_mat)?)
    }

    /// 生成sann的线框
    #[cfg(feature = "truck")]
    fn gen_sann_wire(
        &self,
        origin: Vec2,
        sann: &SannData,
        is_btm: bool,
        r1: f32,
        r2: f32,
    ) -> Option<truck_modeling::Wire> {
        #[cfg(feature = "truck")]
        use truck_modeling::{builder, Surface, Wire};

        let (r1, r2) = if is_btm {
            (r1, r2)
        } else {
            (r2 + sann.drad - sann.dwid - sann.pwidth, r2 + sann.drad)
        };
        // dbg!((r1, r2));
        let z_axis = Vec3::Z;
        let angle = sann.pangle.to_radians();
        // dbg!(angle);
        let mut offset_pt = Vec3::ZERO;
        let mut rot_mat = Mat3::IDENTITY;
        let mut beta_rot = Quat::IDENTITY;
        let mut r_translation = Vector3::new(0.0, 0.0, 0.0);
        offset_pt.x = -sann.plin_pos.x;
        offset_pt.y = -sann.plin_pos.y;
        match &self.path {
            SweepPath3D::SpineArc(d) => {
                let y_axis = d.pref_axis;
                let mut z_axis = self.plax;
                r_translation.x = d.radius as f64;
                if d.clock_wise {
                    z_axis = -z_axis;
                }
                if self.lmirror {
                    z_axis = -z_axis;
                }
                let x_axis = y_axis.cross(z_axis).normalize();
                rot_mat = Mat3::from_cols(x_axis, y_axis, z_axis);
                beta_rot = Quat::from_axis_angle(z_axis, self.bangle.to_radians());
                rot_mat = Mat3::from_quat(Quat::from_rotation_arc(self.plax, Vec3::Z));
            }

            SweepPath3D::Line(d) => {
                rot_mat = Mat3::from_quat(Quat::from_rotation_arc(self.plax, Vec3::Y));
                if d.is_spine {
                    dbg!(self.bangle.to_radians());
                    beta_rot = Quat::from_axis_angle(Vec3::Z, self.bangle.to_radians());
                }
            }
        }
        let p1 = Vec3::new(r1, 0.0, 0.0);
        let p2 = Vec3::new(r2, 0.0, 0.0);
        let p3 = Vec3::new(r2 * angle.cos(), r2 * angle.sin(), 0.0);
        let p4 = Vec3::new(r1 * angle.cos(), r1 * angle.sin(), 0.0);

        let v1 = builder::vertex(p1.point3());
        let v2 = builder::vertex(p2.point3());
        let v3 = builder::vertex(p3.point3());
        let v4 = builder::vertex(p4.point3());
        let center_pt = Point3::new(0.0, 0.0, 0.0);
        let wire = Wire::from(vec![
            builder::line(&v1, &v2),
            builder::circle_arc_with_center(
                center_pt,
                &v2,
                &v3,
                z_axis.vector3(),
                Rad(angle as f64),
            ),
            builder::line(&v3, &v4),
            builder::circle_arc_with_center(
                center_pt,
                &v4,
                &v1,
                -z_axis.vector3(),
                Rad(angle as f64),
            ),
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
        let mut result_wire =
            builder::transformed(&wire, r_trans_mat * beta_mat * local_mat * translation);
        let face = builder::try_attach_plane(&[result_wire.clone()]).ok()?;
        if let Surface::Plane(plane) = face.surface() {
            let _s = self.plax.y as f64;
            // if is_btm && plane.normal().dot(self.extrude_dir.vector3()) > 0.0 {
            //     result_wire.invert();
            // }
        }
        Some(result_wire)
    }

    #[cfg(feature = "occ")]
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
                let y_axis = d.pref_axis;
                let mut z_axis = self.plax;
                r_translation.x = d.radius;
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
                rot_mat = Mat3::from_quat(Quat::from_rotation_arc(self.plax, Vec3::Y));
                if d.is_spine {
                    beta_rot = Quat::from_axis_angle(Vec3::Z, self.bangle.to_radians());
                }
            }
        }
        let mut points = vec![];
        for i in 0..len {
            let p = Vec3::new(verts[i][0], verts[i][1], profile.frads[i]);
            points.push(p);
        }
        let mut wire = wire::gen_occ_wires(&vec![points])?
            .pop()
            .ok_or(anyhow!("无法生成wire。"))?;
        let translation = Mat4::from_translation(offset_pt);
        let r_trans_mat = Mat4::from_translation(r_translation);
        let beta_mat = Mat4::from_mat3(Mat3::from_quat(beta_rot));
        let local_mat = Mat4::from_mat3(rot_mat);
        let final_mat = r_trans_mat * beta_mat * local_mat * translation;

        Ok(wire.transformed_by_gmat(&final_mat.as_dmat4())?)
    }

    ///计算SPRO的face
    /// start_vec 为起始方向
    #[cfg(feature = "truck")]
    fn cal_spro_wire(&self, profile: &SProfileData) -> Option<truck_modeling::Wire> {
        #[cfg(feature = "truck")]
        use truck_meshalgo::prelude::*;
        #[cfg(feature = "truck")]
        use truck_modeling::{builder, Surface};

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
                let y_axis = d.pref_axis;
                let mut z_axis = self.plax;
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
            }
            SweepPath3D::Line(d) => {
                rot_mat = Mat3::from_quat(Quat::from_rotation_arc(self.plax, Vec3::Y));
                // dbg!(rot_mat);
                // dbg!(to_pdms_ori_str(&rot_mat));
            }
        }

        // dbg!(&offset_pt);
        let mut points = vec![];
        for i in 0..len {
            // let p = Vec3::new(verts[i][0], verts[i][1], 0.0);
            let p = verts[i].extend(0.0);
            points.push(p);
        }
        let wire = wire::gen_wire(&points, &profile.frads).ok()?;
        // dbg!(self.bangle);
        let translation = Matrix4::from_translation(offset_pt.vector3());
        // dbg!(translation);
        let r_trans_mat = Matrix4::from_translation(r_translation);
        let m = &rot_mat;
        let local_mat = Matrix4::from_cols(
            m.x_axis.vector4(),
            m.y_axis.vector4(),
            m.z_axis.vector4(),
            Vector4::new(0.0, 0.0, 0.0, 1.0),
        );
        let m = Mat3::from_quat(beta_rot);
        let final_mat = r_trans_mat * local_mat * translation;
        // dbg!(&wire);
        let mut result_wire = builder::transformed(&wire, final_mat);
        // dbg!(result_wire.vertex_iter().collect::<Vec<_>>());
        let face = builder::try_attach_plane(&[result_wire.clone()]).ok()?;
        if let Surface::Plane(plane) = face.surface() {
            // let _s = self.plax.y as f64;
            // if plane.normal().dot(self.extrude_dir.vector3()) > 0.0 {
            //     result_wire.invert();
            // }
        }
        Some(result_wire)
    }
}

impl Default for SweepSolid {
    fn default() -> Self {
        Self {
            profile: CateProfileParam::UNKOWN,
            bangle: 0.0,
            plax: Vec3::Z,
            extrude_dir: DVec3::Z,
            ..Default::default()
        }
    }
}

impl VerifiedShape for SweepSolid {
    fn check_valid(&self) -> bool {
        !self.extrude_dir.is_nan() && self.extrude_dir.length() > 0.0
    }
}

impl BrepShapeTrait for SweepSolid {
    fn is_reuse_unit(&self) -> bool {
        matches!(&self.path, SweepPath3D::Line(_)) && !self.is_sloped()
    }

    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    #[cfg(feature = "truck")]
    fn gen_brep_shell(&self) -> Option<truck_modeling::Shell> {
        #[cfg(feature = "truck")]
        use truck_base::cgmath64::Point3;
        #[cfg(feature = "truck")]
        use truck_modeling::*;
        let mut profile_wire = None;
        let mut top_profile_wire = None;
        let mut is_sann = false;
        let (profile_wire, _top_profile_wire) = match &self.profile {
            CateProfileParam::SANN(p) => {
                let w = p.pwidth;
                let r = p.pradius;
                let r1 = r - w;
                let r2 = r;
                let origin = p.xy + p.dxy;
                profile_wire = self.gen_sann_wire(origin, p, true, r1, r2);
                top_profile_wire = self.gen_sann_wire(origin, p, false, r1, r2);
                is_sann = true;
                (profile_wire, top_profile_wire)
            }
            CateProfileParam::SPRO(p) => {
                let wire = self.cal_spro_wire(p);
                (wire, None)
            }
            CateProfileParam::SREC(p) => {
                let profile = p.convert_to_spro();
                let wire = self.cal_spro_wire(&profile);
                (wire, None)
            }
            _ => (None, None),
        };
        // if let Some(mut wire) = profile_wire && let Some(mut top_wire) = top_profile_wire {
        if let Some(wire) = profile_wire {
            //check if valid
            if self.drns.is_nan() || self.drne.is_nan() {
                // return Err(anyhow!("drns or drne is nan"));
                println!("drns or drne is nan");
                return None;
            }
            match &self.path {
                SweepPath3D::SpineArc(arc) => {
                    let mut face_s = builder::try_attach_plane(&[wire]).unwrap();
                    if let Surface::Plane(plane) = face_s.surface() {
                        let is_rev_face = (plane.normal().y * arc.axis.z as f64) < 0.0;
                        if is_rev_face {
                            dbg!("Face inveted");
                            face_s.invert();
                        }
                    }
                    let rot_angle = arc.angle;
                    let rot_axis = if arc.clock_wise { -Vec3::Z } else { Vec3::Z };
                    let solid = builder::rsweep(
                        &face_s,
                        Point3::origin(),
                        rot_axis.vector3(),
                        Rad(rot_angle as f64),
                    );
                    let shell: Shell = solid.into_boundaries().pop()?;
                    return Some(shell);
                }
                SweepPath3D::Line(l) => {
                    let mut transform_btm = Mat4::IDENTITY;
                    let mut transform_top = Mat4::IDENTITY;
                    if self.drns.is_normalized() && self.is_drns_sloped() {
                        let x_angle = self.drns.angle_between(Vec3::X).abs();
                        let scale_x = if x_angle < ANGLE_RAD_F64_TOL {
                            1.0
                        } else {
                            1.0 / (x_angle.sin())
                        };
                        let y_angle = self.drns.angle_between(Vec3::Y).abs();
                        let scale_y = if y_angle < ANGLE_RAD_F64_TOL {
                            1.0
                        } else {
                            1.0 / (y_angle.sin())
                        };
                        transform_btm =
                            Mat4::from_quat(glam::Quat::from_rotation_arc(Vec3::Z, self.drns))
                                * Mat4::from_scale(Vec3::new(scale_x, scale_y, 1.0));
                    }
                    if self.drne.is_normalized() && self.is_drne_sloped() {
                        let x_angle = (-self.drne).angle_between(Vec3::X).abs();
                        let scale_x = if x_angle < ANGLE_RAD_F64_TOL {
                            1.0
                        } else {
                            1.0 / (x_angle.sin())
                        };
                        let y_angle = (-self.drne).angle_between(DVec3::Y).abs();
                        let scale_y = if y_angle < ANGLE_RAD_F64_TOL {
                            1.0
                        } else {
                            1.0 / (y_angle.sin())
                        };
                        transform_top =
                            Mat4::from_quat(glam::Quat::from_rotation_arc(Vec3::Z, -self.drne))
                                * Mat4::from_scale(Vec3::new(scale_x, scale_y, 1.0));
                    }
                    transform_top =
                        Mat4::from_translation(Vec3::new(0.0, 0.0, l.length())) * transform_top;

                    let mut faces = vec![];
                    let wire_s = builder::transformed(&wire, convert_to_cg_matrix4(&transform_btm));
                    let wire_e = builder::transformed(&wire, convert_to_cg_matrix4(&transform_top));
                    let edges_cnt = wire_s.len();
                    for i in 0..edges_cnt {
                        let c1 = &wire_s[i];
                        let c2 = &wire_e[i];
                        faces.push(builder::homotopy(c1, c2).inverse());
                    }
                    let face_s = builder::try_attach_plane(&[wire_s]).ok()?;
                    let face_e = builder::try_attach_plane(&[wire_e]).ok()?;
                    faces.push(face_s);
                    faces.push(face_e.inverse());
                    let shell: Shell = faces.into();
                    return Some(shell);
                }
            }
        }
        None
    }

    #[cfg(feature = "occ")]
    fn gen_occ_shape(&self) -> anyhow::Result<OccSharedShape> {
        let mut is_sann = false;
        let (profile_wire, top_profile_wire) = match &self.profile {
            CateProfileParam::SANN(p) => {
                let w = p.pwidth as f64;
                let r = p.pradius as f64;
                let r1 = r - w;
                let r2 = r;
                let origin = (p.xy + p.dxy).as_dvec2();
                let wire_btm = self.gen_occ_sann_wire(origin, p, true, r1, r2).ok();
                let wire_top = self.gen_occ_sann_wire(origin, p, false, r1, r2).ok();
                is_sann = true;
                (wire_btm, wire_top)
            }
            CateProfileParam::SPRO(p) => {
                let wire = self.gen_occ_spro_wire(p).ok();
                (wire, None)
            }
            CateProfileParam::SREC(p) => {
                let profile = p.convert_to_spro();
                let wire = self.gen_occ_spro_wire(&profile).ok();
                (wire, None)
            }
            _ => (None, None),
        };

        if let Some(mut wire) = profile_wire {
            //check if valid
            // if self.drns.is_nan() || self.drne.is_nan() {
            //     return Err(anyhow!("drns or drne is nan"));
            // }

            match &self.path {
                SweepPath3D::SpineArc(arc) => {
                    let rot_angle = arc.angle;
                    let rot_axis = if arc.clock_wise { -DVec3::Z } else { DVec3::Z };
                    let r =
                        wire.to_face()
                            .revolve(DVec3::ZERO, rot_axis, Some(rot_angle.radians()));
                    return Ok(r.into_shape().into());
                }
                SweepPath3D::Line(l) => {
                    let mut wires = vec![];
                    // let mut transform_btm = DMat4::from_quat(self.drns_rot);
                    // let mut transform_top = DMat4::from_quat(self.drne_rot);

                    let mut transform_btm = DMat4::default();
                    let mut transform_top = DMat4::default();

                    //在实际中，是以X轴为主轴
                    // if self.is_drns_sloped() {
                    //     let drns = self.drns.unwrap();
                    //     // println!("drns {:?}  is sloped", self.drns);
                    //     let mut x_angle = drns.angle_between(DVec3::X);
                    //     let rot_x_angle = FRAC_PI_2 - x_angle;
                    //     let x = x_angle.sin().abs();
                    //     if x < ANGLE_RAD_F64_TOL {
                    //         return Err(anyhow!("drns is aligned to x"));
                    //     }
                    //     let scale_x = 1.0 / x;
                    //
                    //     let mut y_angle = drns.angle_between(DVec3::Y);
                    //     let y = y_angle.sin().abs();
                    //     if y < ANGLE_RAD_F64_TOL {
                    //         return Err(anyhow!("drns is aligned to y"));
                    //     }
                    //     let scale_y = 1.0 / y;
                    //
                    //     let rot_y_angle = FRAC_PI_2 - y_angle;
                    //
                    //     // dbg!((x_angle, rot_x_angle, rot_y_angle, scale_x, scale_y));
                    //
                    //     let rotation =
                    //         DMat4::from_rotation_y(rot_x_angle) * DMat4::from_rotation_x(rot_y_angle);
                    //     // transform_btm =
                    //     //     rotation * DMat4::from_scale(DVec3::new(scale_x, scale_y, 1.0));
                    // }
                    // if self.is_drne_sloped() {
                    //     // println!("drne {:?}  is sloped", self.drne);
                    //     let drne = self.drne.unwrap();
                    //     let mut x_angle = (-drne).angle_between(DVec3::X);
                    //     let rot_x = (FRAC_PI_2 - x_angle);
                    //     let x = x_angle.sin().abs();
                    //     if x < ANGLE_RAD_F64_TOL {
                    //         return Err(anyhow!("drne is aligned to x"));
                    //     }
                    //     let scale_x = 1.0 / x;
                    //
                    //     let mut y_angle = (-drne).angle_between(DVec3::Y);
                    //     let rot_y = (FRAC_PI_2 - y_angle);
                    //     let y = y_angle.sin().abs();
                    //     if y < ANGLE_RAD_F64_TOL {
                    //         return Err(anyhow!("drne is aligned to y"));
                    //     }
                    //     let scale_y = 1.0 / y;
                    //     //todo need to fix, why above -0.7 is not correct
                    //     // dbg!((x_angle, rot_x, rot_y, scale_x, scale_y));
                    //
                    //     let rotation =
                    //         DMat4::from_rotation_y(rot_x) * DMat4::from_rotation_x(rot_y);
                    //     // transform_top =
                    //     //     rotation * DMat4::from_scale(DVec3::new(scale_x, scale_y, 1.0));
                    // }

                    transform_top.w_axis = DVec4::new(0.0, 0.0, l.length() as f64, 1.0);
                    wires.push(wire.transformed_by_gmat(&transform_btm)?);
                    if let Some(mut top_wire) = top_profile_wire {
                        wires.push(top_wire.transformed_by_gmat(&transform_top)?);
                    } else {
                        wires.push(wire.transformed_by_gmat(&transform_top)?);
                    }

                    return Ok(Solid::loft(wires.iter()).into_shape().into());
                }
            }
        }

        return Err(anyhow!("SweepSolid 生成错误"));
    }

    fn hash_unit_mesh_params(&self) -> u64 {
        //截面暂时用这个最省力的方法
        let mut hasher = DefaultHasher::default();
        let bytes = if self.is_drns_sloped() || self.is_drne_sloped() {
            bincode::serialize(&self).unwrap()
        } else if let SweepPath3D::SpineArc(_) = self.path {
            bincode::serialize(&self).unwrap()
        } else {
            bincode::serialize(&self.profile).unwrap()
        };
        bytes.hash(&mut hasher);
        "loft".hash(&mut hasher);

        hasher.finish()
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
        let mut unit = self.clone();
        if let SweepPath3D::Line(_) = unit.path
            && !self.is_sloped()
        {
            unit.extrude_dir = DVec3::Z;
            unit.path = SweepPath3D::Line(Line3D {
                start: Default::default(),
                end: Vec3::Z * 10.0,
                is_spine: false,
            });
        }
        Box::new(unit)
    }

    #[inline]
    fn get_scaled_vec3(&self) -> Vec3 {
        if self.is_sloped() {
            return Vec3::ONE;
        }
        match &self.path {
            SweepPath3D::Line(l) => Vec3::new(1.0, 1.0, l.length() / 10.0),
            _ => Vec3::ONE,
        }
    }

    #[inline]
    fn get_trans(&self) -> bevy_transform::prelude::Transform {
        match &self.profile {
            CateProfileParam::SANN(_p) => {
                return bevy_transform::prelude::Transform {
                    rotation: Quat::IDENTITY,
                    scale: self.get_scaled_vec3(),
                    translation: Vec3::ZERO,
                };
            }
            CateProfileParam::SPRO(_) | CateProfileParam::SREC(_) => {
                return bevy_transform::prelude::Transform {
                    rotation: Quat::IDENTITY,
                    scale: self.get_scaled_vec3(),
                    translation: Vec3::ZERO,
                };
            }
            _ => {}
        }

        bevy_transform::prelude::Transform::IDENTITY
    }

    fn tol(&self) -> f32 {
        if let Some(aabb) = self.profile.get_bbox() {
            return 0.01 * aabb.bounding_sphere().radius.max(1.0);
        }
        0.01
    }

    fn convert_to_geo_param(&self) -> Option<PdmsGeoParam> {
        Some(PdmsGeoParam::PrimLoft(self.clone()))
    }
}
