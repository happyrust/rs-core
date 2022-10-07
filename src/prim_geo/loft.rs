use std::collections::hash_map::DefaultHasher;
use std::f32::consts::{FRAC_PI_2, PI, TAU};
use std::f32::EPSILON;
use std::hash::{Hash, Hasher};

use approx::{abs_diff_eq, abs_diff_ne};
use bevy::ecs::reflect::ReflectComponent;
use bevy::pbr::LightEntity::Point;
use bevy::prelude::*;
use bevy::reflect::Reflect;
use bevy_egui::egui::Shape::Vec;
use glam::{TransformRT, TransformSRT, Vec3};
use serde::{Deserialize, Serialize};
use truck_meshalgo::prelude::*;
use truck_modeling::{builder, Face, Shell, Surface, Wire};
use truck_modeling::builder::try_attach_plane;

use crate::parsed_data::{CateProfileParam, SannData, SProfileData};
use crate::prim_geo::helper::cal_ref_axis;
use crate::prim_geo::spine::*;
use crate::prim_geo::wire;
use crate::shape::pdms_shape::{BrepMathTrait, BrepShapeTrait, PdmsMesh, TRI_TOL, VerifiedShape};
use crate::tool::float_tool::{hash_f32, hash_vec3};

//todo 针对确实只是extrusion的处理，可以转换成extrusion去处理，而不是占用

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct SweepSolid {
    pub profile: CateProfileParam,
    pub drns: Vec3,
    pub drne: Vec3,
    pub bangle: f32,
    pub plane_normal: Vec3,
    pub extrude_dir: Vec3,
    pub height: f32,
    pub path: SweepPath3D,
}

impl SweepSolid {
    pub fn is_sloped(&self) -> bool {
        if abs_diff_eq!(self.drns.z, 1.0, epsilon = 0.01) && abs_diff_eq!(self.drne.z, -1.0, epsilon = 0.01) {
            return false;
        }
        (abs_diff_ne!(self.drns.length(), 0.0) || abs_diff_ne!(self.drne.length(), 0.0))
    }

    fn gen_sann_wire(&self, origin: Vec2, sann: &SannData, is_btm: bool, r1: f32, r2: f32) -> Option<Wire> {
        let (r1, r2) = if is_btm {
            (r1, r2)
        }else { (r2 + sann.drad - sann.dwid - sann.pwidth,  r2 + sann.drad)};
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
                if d.clock_wise {
                    z_axis = -z_axis;
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

    ///计算SPRO的face
    /// start_vec 为起始方向
    fn cal_spro_wire(&self, profile: &SProfileData) -> Option<Wire> {
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
                let x_axis = y_axis.cross(z_axis).normalize();
                dbg!((x_axis, y_axis, z_axis));
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
            profile: CateProfileParam::None,
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

//获得斜切面的变换矩阵
pub fn get_sloped_transform(drn_axis: Vec3, axis: Vec3) -> Matrix4 {
    let mut mat = Matrix4::one();
    let axis = if drn_axis.z * axis.z < 0.0{
        -axis
    }else{
        axis
    };
    if abs_diff_ne!(drn_axis.z, 0.0, epsilon = 0.001) {
        let a = Vec3::X.angle_between(drn_axis);
        let b = Vec3::Y.angle_between(drn_axis);
        // dbg!((a, b));
        if !a.is_nan() {
            let mut scale_x = 1.0 / a.sin().abs() as f64;
            let mut found_err = false;
            if scale_x > 100.0 {
                scale_x = 1.0;
                found_err = true;
            }
            let mut scale_y = 1.0 / b.sin().abs() as f64;
            // dbg!((scale_x, scale_y));
            if scale_y > 100.0 {
                scale_y = 1.0;
                found_err = true;
            }
            if found_err {
                println!("Sloped ele wrong caculate scale: {:?}", (scale_x, scale_y));
            }
            let scale_mat = Matrix4::from_nonuniform_scale(scale_x, scale_y, 1.0);
            // dbg!(&axis);
            let m = Mat3::from_quat(glam::Quat::from_rotation_arc(axis, drn_axis));
            let rotation = Matrix4::from_cols(
                m.x_axis.vector4(),
                m.y_axis.vector4(),
                m.z_axis.vector4(),
                Vector4::new(0.0, 0.0, 0.0, 1.0),
            );
            mat = rotation * scale_mat;
        }
    }

    mat
}


//#[typetag::serde]
impl BrepShapeTrait for SweepSolid {
    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }


    fn gen_brep_shell(&self) -> Option<Shell> {
        use truck_modeling::*;
        use truck_base::cgmath64::{Point3, Vector3};
        let mut profile_wire = None;
        let mut top_profile_wire = None;
        let mut is_sann = false;
        match &self.profile {
            CateProfileParam::SANN(p) => {
                let w = p.pwidth;
                let r = p.pradius;
                let r1 = r - w;
                let r2 = r;
                let d = p.paxis.as_ref().unwrap().dir.normalize();
                let mut angle = p.pangle.to_radians();
                let origin = p.xy + p.dxy;
                profile_wire = self.gen_sann_wire(origin, p, true, r1, r2);
                top_profile_wire = self.gen_sann_wire(origin, p,false, r1, r2);
                is_sann = true;
            }
            CateProfileParam::SPRO(p) => {
                profile_wire = self.cal_spro_wire(p);
                top_profile_wire = self.cal_spro_wire(p);
            }
            _ => {}
        }
        if let Some(mut wire) = profile_wire  && let Some(mut top_wire) = top_profile_wire{
            //先生成start 和 end face
            let mut drns = self.drns;
            let mut drne = self.drne;
            let mut transform_btm = Matrix4::one();
            let mut transform_top = Matrix4::one();
            let mut rotation = Matrix4::one();
            let mut scale_mat = Matrix4::one();

            match &self.path {
                SweepPath3D::SpineArc(arc) => {
                    let mut face_s = builder::try_attach_plane(&[wire]).unwrap();
                    if let Surface::Plane(plane) = face_s.get_surface() {
                        let is_rev_face = (plane.normal().y * arc.axis.z as f64) < 0.0;
                        if is_rev_face {
                            dbg!("Face inveted");
                            face_s.invert();
                        }
                    }
                    let rot_angle = arc.angle;
                    dbg!(rot_angle);
                    let rot_axis = if arc.clock_wise {
                        -Vec3::Z
                    } else {
                        Vec3::Z
                    };
                    let solid = builder::rsweep(&face_s, Point3::origin(),
                                                rot_axis.vector3(), Rad(rot_angle as f64));
                    let shell: Shell = solid.into_boundaries().pop()?;
                    return Some(shell);
                }
                SweepPath3D::Line(l) => {
                    if self.is_sloped() {
                        transform_btm = get_sloped_transform(drns, Vec3::Z);
                        transform_top = get_sloped_transform(drne, -Vec3::Z);
                    }
                    let mut faces = vec![];
                    let translation = Matrix4::from_translation(Vector3::new(0.0 as f64, 0.0 as f64, l.len() as f64));
                    let wire_s = builder::transformed(&wire, transform_btm);
                    let wire_e = builder::transformed(&top_wire, translation * transform_top);
                    let edges_cnt = wire_s.len();
                    for i in 0..edges_cnt {
                        let c1 = &wire_s[i];
                        let c2 = &wire_e[i];
                        faces.push(builder::homotopy(c1, c2).inverse());
                    }
                    let mut face_s = builder::try_attach_plane(&[wire_s]).unwrap();
                    //需要判断faces_s的方向，来决定是否需要inverse
                    let mut face_e = builder::try_attach_plane(&[wire_e]).unwrap();
                    faces.push(face_s.clone());
                    faces.push(face_e.inverse());

                    if let Surface::Plane(plane) = face_s.get_surface() {
                        // dbg!(plane.normal());
                        if plane.normal().z > 0.0 {
                            // dbg!("invert");
                            for mut f in &mut faces {
                                f.invert();
                            }
                        }
                    }
                    let shell: Shell = faces.into();
                    return Some(shell);
                }
            }
        }
        None
    }


    fn hash_unit_mesh_params(&self) -> u64 {
        //截面暂时用这个最省力的方法
        let mut hasher = DefaultHasher::default();
        let bytes = if self.is_sloped() {
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
        //sloped 不允许拉伸
        if let SweepPath3D::Line(_) = unit.path && !self.is_sloped() {
            unit.extrude_dir = Vec3::Z;
            unit.path = SweepPath3D::Line(Line3D::default());
        }
        // dbg!(&unit);
        Box::new(unit)
    }

    //拉伸为height方向
    fn gen_unit_mesh(&self) -> Option<PdmsMesh> {
        self.gen_unit_shape().gen_mesh(Some(TRI_TOL / 10.0))
    }

    #[inline]
    fn get_scaled_vec3(&self) -> Vec3 {
        // dbg!(self.is_sloped());
        if self.is_sloped() { return Vec3::ONE; }
        match &self.path {
            SweepPath3D::Line(l) => Vec3::new(1.0, 1.0, self.height),
            _ => Vec3::ONE,
        }
    }

    #[inline]
    fn get_trans(&self) -> TransformSRT {
        let mut vec = self.extrude_dir.normalize();
        // dbg!(self.get_scaled_vec3());
        match &self.profile {
            CateProfileParam::SANN(p) => {
                let mut translation = Vec3::ZERO;
                // match &self.path {
                //     SweepPath3D::Arc(d) => {
                //         translation = d.center;
                //     }
                //     SweepPath3D::Line(d) => {
                //         translation = d.start;
                //     }
                // }

                return TransformSRT {
                    rotation: Quat::IDENTITY,
                    scale: self.get_scaled_vec3(),
                    translation,
                };
            }
            CateProfileParam::SPRO(_) => {
                return TransformSRT {
                    rotation: Quat::IDENTITY,
                    scale: self.get_scaled_vec3(),
                    translation: Vec3::ZERO,
                };
            }
            _ => {}
        }

        TransformSRT::IDENTITY
    }
}
