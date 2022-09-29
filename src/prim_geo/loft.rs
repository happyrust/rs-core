use std::collections::hash_map::DefaultHasher;
use std::f32::consts::{FRAC_PI_2, PI, TAU};
use std::f32::EPSILON;
use std::hash::{Hash, Hasher};

use approx::{abs_diff_eq, abs_diff_ne};
use bevy::ecs::reflect::ReflectComponent;
use bevy::pbr::LightEntity::Point;
use bevy::prelude::*;
use bevy::reflect::Reflect;
use glam::{TransformRT, TransformSRT, Vec3};
use serde::{Deserialize, Serialize};
use truck_meshalgo::prelude::*;
use truck_modeling::{builder, Face, Shell, Surface, Wire};
use truck_modeling::builder::try_attach_plane;

use crate::parsed_data::{CateProfileParam, SProfileData};
use crate::prim_geo::circle::Circle2D;
use crate::prim_geo::helper::cal_ref_axis;
use crate::prim_geo::wire;
use crate::shape::pdms_shape::{BrepMathTrait, BrepShapeTrait, PdmsMesh, TRI_TOL, VerifiedShape};
use crate::tool::float_tool::{hash_f32, hash_vec3};

//todo 针对确实只是extrusion的处理，可以转换成extrusion去处理，而不是占用

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct LoftSolid {
    pub profile: CateProfileParam,
    pub drns: Vec3,
    pub drne: Vec3,
    pub plane_normal: Vec3,
    pub extrude_dir: Vec3,
    pub height: f32,
    pub arc_path: Option<(Vec3, Vec3, Vec3)>,  //p1, p2, p3  弧形的路径
}

impl LoftSolid {
    pub fn is_sloped(&self) -> bool {
        if abs_diff_eq!(self.drns.z, 1.0, epsilon = 0.01) && abs_diff_eq!(self.drne.z, -1.0, epsilon = 0.01) {
            return false;
        }
        (abs_diff_ne!(self.drns.length(), 0.0) || abs_diff_ne!(self.drne.length(), 0.0))
    }

    fn cal_sann_face(&self, is_btm: bool, dir: Vec3, angle: f32, r1: f32, r2: f32, circle: Option<Circle2D>) -> Option<Face> {
        return None;
        // let rot = Quat::from_rotation_arc(Vec3::X, dir);
        let a = angle;
        let c = circle.unwrap_or_default().r;
        let c = 0.0;
        let center_pt = Point3::new(0.0 - c as f64, 0.0, 0.0 as f64);
        let mut rot = Quat::from_rotation_arc(Vec3::Z, self.plane_normal);
        let p1 = rot.mul_vec3(Vec3::new(r1 - c, 0.0, 0.0));
        let p2 = rot.mul_vec3(Vec3::new(r2 - c, 0.0, 0.0));
        let p3 = rot.mul_vec3(Vec3::new(r2 * a.cos() - c, r2 * a.sin(), 0.0));
        let p4 = rot.mul_vec3(Vec3::new(r1 * a.cos() - c, r1 * a.sin(), 0.0));

        let v1 = builder::vertex(p1.point3());
        let v2 = builder::vertex(p2.point3());
        let v3 = builder::vertex(p3.point3());
        let v4 = builder::vertex(p4.point3());

        let mut z_axis = self.plane_normal;
        let mut wire = Wire::from(vec![
            builder::line(&v1, &v2),
            builder::circle_arc_with_center(center_pt,
                                            &v2, &v3, z_axis.vector3(), Rad(a as f64)),
            builder::line(&v3, &v4),
            builder::circle_arc_with_center(center_pt,
                                            &v4, &v1, z_axis.vector3(), Rad(-a as f64)),
        ]);

        if let Ok(mut f) = try_attach_plane(&[wire]) {
            if let Surface::Plane(plane) = f.get_surface() {
                if plane.normal().dot(self.plane_normal.vector3()) < 0.0 {
                    f = f.inverse();
                }
            }
            return Some(f);
        }

        None
    }

    //is_btm 是否是底部的face
    fn cal_sann_face_1(&self, is_btm: bool, dir: Vec3, angle: f32, r1: f32, r2: f32) -> Option<Face> {
        use truck_base::cgmath64::*;
        let mut n = if is_btm { self.drns.normalize() } else { self.drne.normalize() };
        //dbg!(&n);
        let h = if is_btm { 0.0 } else { self.height };
        let extrude_dir = self.extrude_dir.normalize();
        let a = angle;
        let mut z_axis = Vec3::Z;
        let z_angle: f32 = z_axis.angle_between(n);
        if z_angle == FRAC_PI_2 { return None; }
        let mut y_axis_scale = (1.0 / z_angle.cos()) as f64;
        let mut rot_face = Quat::from_rotation_arc(Vec3::Z, n.normalize());
        let rot = Quat::from_rotation_arc(Vec3::X, dir);

        let p1 = rot.mul_vec3(Vec3::new(r1, 0.0, h));
        let p2 = rot.mul_vec3(Vec3::new(r2, 0.0, h));
        let p3 = rot.mul_vec3(Vec3::new(r2 * a.cos(), r2 * a.sin(), h));
        let p4 = rot.mul_vec3(Vec3::new(r1 * a.cos(), r1 * a.sin(), h));

        let v1 = builder::vertex(p1.point3());
        let v2 = builder::vertex(p2.point3());
        let v3 = builder::vertex(p3.point3());
        let v4 = builder::vertex(p4.point3());
        //try to make it as ellipse wire
        let center_pt = Point3::new(0.0, 0.0, h as f64);
        let mut wire = Wire::from(vec![
            builder::line(&v1, &v2),
            builder::circle_arc_with_center(center_pt,
                                            &v2, &v3, z_axis.vector3(), Rad(a as f64)),
            builder::line(&v3, &v4),
            builder::circle_arc_with_center(center_pt,
                                            &v4, &v1, z_axis.vector3(), Rad(-a as f64)),
        ]).inverse();

        let mat0 = Matrix4::from_translation(-center_pt.to_vec());
        let mat1 = Matrix4::from_nonuniform_scale(1.0, y_axis_scale, 1.0);
        let mat2 = Matrix4::from_angle_z(Rad(z_angle as f64));
        let mat3 = Matrix4::from_translation(center_pt.to_vec());

        if let Ok(mut f) = try_attach_plane(&[wire]) {
            if let Surface::Plane(plane) = f.get_surface() {
                if plane.normal().dot(self.plane_normal.vector3()) < 0.0 {
                    f = f.inverse();
                }
            }
            return Some(f);
        }

        None
    }

    ///计算SPRO的face
    /// start_vec 为起始方向
    fn cal_spro_wire(&self, is_btm: bool, profile: &SProfileData, start_vec: Vec3, circle: Option<Circle2D>) -> Option<Wire> {
        let n = if is_btm { self.drns } else { self.drne };
        let h = if is_btm { 0.0 } else { self.height };
        let verts = &profile.verts;
        let len = verts.len();

        let mut extrude = Vec3::Z;
        let mut offset_pt = Vec3::ZERO;
        let mut rot = Quat::IDENTITY;
        let mut local_rot = Quat::IDENTITY;
        let mut angle = 0.0f32;
        if circle.is_some() {
            let circle = circle.as_ref().unwrap();
            //todo 需要确定哪个边是x轴的
            let mut delta_vec = Vec2::new(verts[1][0], verts[1][1]) - Vec2::new(verts[0][0], verts[0][1]);
            if abs_diff_eq!(delta_vec.dot(Vec2::X), 0.0) {
                delta_vec = Vec2::new(verts[2][0], verts[2][1]) - Vec2::new(verts[1][0], verts[1][1])
            }
            dbg!(circle.clock_wise);
            if circle.clock_wise {
                offset_pt.x = circle.r - profile.plin_pos.x;
            } else {
                offset_pt.x = circle.r - delta_vec.length() + profile.plin_pos.x;
            }
            angle = start_vec.angle_between(Vec3::X);
            // dbg!(angle);
            extrude = self.plane_normal;
            rot = Quat::from_rotation_arc(self.plane_normal, Vec3::Z);
            local_rot = Quat::from_rotation_z(angle);
        } else {
            offset_pt.x = -profile.plin_pos.x;
        }
        offset_pt.y = -profile.plin_pos.y;
        dbg!(&offset_pt);
        // let p0 = local_rot.mul_vec3(rot.mul_vec3(Vec3::new(verts[0][0], verts[0][1], h) + offset_pt));

        // let mut v0 = builder::vertex(p0.point3());
        // let mut prev_v0 = v0.clone();
        let mut points = vec![];

        for i in 0..len {
            let p = local_rot.mul_vec3(rot.mul_vec3(Vec3::new(verts[i][0], verts[i][1], h) + offset_pt));
            points.push(p);
            // let next_v = builder::vertex(p.point3());
            // edges.push(builder::line(&prev_v0, &next_v));
            // prev_v0 = next_v.clone();
        }
        // let last_v = edges.last().unwrap().back();
        // edges.push(builder::line(last_v, &v0));

        wire::gen_wire(&points, &profile.frads).ok()
    }
}

impl Default for LoftSolid {
    fn default() -> Self {
        Self {
            profile: CateProfileParam::None,
            drns: Default::default(),
            drne: Default::default(),
            // axis_dir: Default::default(),
            plane_normal: Vec3::Z,
            extrude_dir: Vec3::Z,
            height: 0.0,
            arc_path: None,
        }
    }
}

impl VerifiedShape for LoftSolid {
    fn check_valid(&self) -> bool { /*self.height > f32::EPSILON*/ !self.extrude_dir.is_nan() && self.extrude_dir.length() > 0.0 }
}


//#[typetag::serde]
impl BrepShapeTrait for LoftSolid {
    fn clone_dyn(&self) -> Box<dyn BrepShapeTrait> {
        Box::new(self.clone())
    }

    //涵盖的情况，需要考虑，上边只有一条边，和退化成点的情况
    fn gen_brep_shell(&self) -> Option<Shell> {
        use truck_modeling::*;
        use truck_base::cgmath64::{Point3, Vector3};
        let mut profile_wire = None;
        // let mut face_e = None;
        let mut circle = None;
        let mut start_vec = Vec3::X;
        // dbg!(&self.arc_path);
        if let Some((p1, p2, p3)) = self.arc_path {
            let c = Circle2D::from_three_points(&Vec2::new(p1.x, p1.y), &Vec2::new(p2.x, p2.y), &Vec2::new(p3.x, p3.y));
            circle = Some(c);
            if p1.length() > EPSILON {
                start_vec = p1.normalize();
            }
        }
        // dbg!(&circle);
        let mut is_sann = false;
        match &self.profile {
            //需要用切面去切出相交的face
            CateProfileParam::SANN(p) => {
                let w = p.pwidth;
                let r = p.pradius;
                let r1 = r - w;
                let r2 = r;
                let d = &p.ptaxis.as_ref().unwrap().dir;
                let dir = Vec3::new(d[0] as f32, d[1] as f32, d[2] as f32).normalize();
                let angle = p.pangle.to_radians();
                // face_s = self.cal_sann_face(true, dir, angle, r1, r2, circle.clone());
                // let w = p.pwidth + p.dwid;
                // let r = p.pradius + p.drad;
                // let r1 = r - w;
                // let r2 = r;
                // face_e = self.cal_sann_face(false, dir, angle, r1, r2).map(|x| x.inverse());
                is_sann = true;
            }
            CateProfileParam::SPRO(p) => {
                profile_wire = self.cal_spro_wire(true, p, start_vec, circle.clone());
            }
            _ => {}
        }

        if let Some(mut wire) = profile_wire {
            //先生成start 和 end face
            let mut drns = self.drns;
            let mut drne = self.drne;
            dbg!(self.plane_normal);
            dbg!(&drns);
            dbg!(&drne);
            let mut transform_btm = Matrix4::one();
            let mut transform_top = Matrix4::one();
            let mut rotation = Matrix4::one();
            let mut scale_mat = Matrix4::one();


            if let Some((p1, p2, p3)) = self.arc_path {
                let c = circle.unwrap_or_default();
                let v1 = Vec2::new(p1.x, p1.y) - c.center;
                let v3 = Vec2::new(p3.x, p3.y) - c.center;
                let mut angle = v1.angle_between(v3);
                let mut rot_z = Vec3::Z;
                if c.clock_wise {
                    rot_z = -Vec3::Z;
                }
                if self.is_sloped() {
                    let a = Vec3::X.angle_between(self.drns);
                    let b = Vec3::Z.angle_between(self.drns);
                    dbg!((a, b));
                    //slope对应的斜面必须要缩放
                    if abs_diff_ne!(drns.y, 0.0) {
                        if !a.is_nan() {
                            let mut scale_x = 1.0 / a.sin().abs() as f64;
                            let mut found_err = false;
                            if scale_x > 100.0 {
                                scale_x = 1.0;
                                found_err = true;
                            }
                            let mut scale_z = 1.0 / b.sin().abs() as f64;
                            if scale_z > 100.0 {
                                scale_z = 1.0;
                                found_err = true;
                            }
                            dbg!((scale_x, scale_z));
                            if found_err {
                                println!("Sloped ele wrong caculate scale: {:?}", (scale_x, scale_z));
                            }
                            // scale_mat = Matrix4::from_nonuniform_scale(scale_x, 1.0, scale_z);
                            // let m = Mat3::from_quat(glam::Quat::from_rotation_arc(Vec3::Y, drns));
                            // rotation = Matrix4::from_cols(
                            //     m.x_axis.vector4(),
                            //     m.y_axis.vector4(),
                            //     m.z_axis.vector4(),
                            //     Vector4::new(0.0, 0.0, 0.0, 1.0),
                            // );
                        }
                    }
                    transform_btm = rotation * scale_mat;
                    let a = Vec3::X.angle_between(drne);
                    let b = Vec3::Z.angle_between(drne);
                    dbg!((a, b));
                    if abs_diff_ne!(drne.y, 0.0) {
                        if !a.is_nan() {
                            let mut scale_x = 1.0 / a.sin().abs() as f64;
                            let mut found_err = false;
                            if scale_x > 100.0 {
                                scale_x = 1.0;
                                found_err = true;
                            }
                            let mut scale_z = 1.0 / b.sin().abs() as f64;
                            if scale_z > 100.0 {
                                scale_z = 1.0;
                                found_err = true;
                            }
                            dbg!((scale_x, scale_z));
                            if found_err {
                                println!("Sloped ele wrong caculate scale: {:?}", (scale_x, scale_z));
                            }
                            // scale_mat = Matrix4::from_nonuniform_scale(scale_x, 1.0, scale_z);
                            // let m = Mat3::from_quat(glam::Quat::from_rotation_arc(-Vec3::Y, self.drne));
                            // rotation = Matrix4::from_cols(
                            //     m.x_axis.vector4(),
                            //     m.y_axis.vector4(),
                            //     m.z_axis.vector4(),
                            //     Vector4::new(0.0, 0.0, 0.0, 1.0),
                            // );
                        }
                    }

                    transform_top = transform_top * rotation * scale_mat;
                }
                // dbg!(c.clock_wise);
                // dbg!(rot_z);
                // dbg!(angle);
                // dbg!(self.plane_normal);
                let mut faces = vec![];
                let start_angle = Vec2::X.angle_between(v1);
                let rot = Matrix4::from_angle_z(Rad(angle as f64));
                let wire_s = builder::transformed(&wire, transform_btm);
                let wire_e = builder::transformed(&wire, rot * transform_top);
                let edges_cnt = wire_s.len();
                for i in 0..edges_cnt {
                    let edge0 = &wire_s[i];
                    let edge1 = &wire_e[i];
                    let arc_0 = builder::circle_arc_with_center(Point3::new(0.0, 0.0, 0.0),
                                                                edge0.back(), edge1.back(), rot_z.vector3(), Rad(angle.abs() as f64));
                    let arc_1 = builder::circle_arc_with_center(Point3::new(0.0, 0.0, 0.0),
                                                                edge0.front(), edge1.front(), rot_z.vector3(), Rad(angle.abs() as f64));

                    let curve0 = arc_0.oriented_curve().lift_up();
                    let curve1 = arc_1.oriented_curve().lift_up();
                    let surface = BSplineSurface::homotopy(curve0, curve1);

                    let wire: Wire = vec![
                        edge0.clone(),
                        arc_0,
                        edge1.inverse(),
                        arc_1.inverse(),
                    ].into();

                    faces.push(Face::new(
                        vec![wire],
                        Surface::NURBSSurface(NURBSSurface::new(surface)),
                    ).inverse());
                }
                let mut face_s = builder::try_attach_plane(&[wire_s]).unwrap();
                //需要判断faces_s的方向，来决定是否需要inverse
                let mut face_e = builder::try_attach_plane(&[wire_e]).unwrap();
                faces.push(face_s.clone());
                faces.push(face_e.inverse());

                if let Surface::Plane(plane) = face_s.get_surface() {
                    dbg!(plane.normal());
                    let is_neg = start_angle.abs() >= PI;
                    let is_rev_face = (plane.normal().dot(self.plane_normal.vector3())) * rot_z.z as f64 > 0.0;
                    if is_neg ^ is_rev_face {
                        dbg!("invert");
                        for mut f in &mut faces {
                            f.invert();
                        }
                    }
                }

                return Some(faces.into());
            } else {

                if self.is_sloped() {
                    let a = Vec3::X.angle_between(self.drns);
                    let b = Vec3::Y.angle_between(self.drns);
                    // dbg!((a, b));
                    //slope对应的斜面必须要缩放
                    if abs_diff_ne!(drns.z, 0.0) {
                        if !a.is_nan() {
                            let mut scale_x = 1.0 / a.sin().abs() as f64;
                            let mut found_err = false;
                            if scale_x > 100.0 {
                                scale_x = 1.0;
                                found_err = true;
                            }
                            let mut scale_y = 1.0 / b.sin().abs() as f64;
                            if scale_y > 100.0 {
                                scale_y = 1.0;
                                found_err = true;
                            }
                            dbg!((scale_x, scale_y));
                            if found_err {
                                println!("Sloped ele wrong caculate scale: {:?}", (scale_x, scale_y));
                            }
                            scale_mat = Matrix4::from_nonuniform_scale(scale_x, scale_y, 1.0);
                            let m = Mat3::from_quat(glam::Quat::from_rotation_arc(Vec3::Z, drns));
                            rotation = Matrix4::from_cols(
                                m.x_axis.vector4(),
                                m.y_axis.vector4(),
                                m.z_axis.vector4(),
                                Vector4::new(0.0, 0.0, 0.0, 1.0),
                            );
                        }
                    }
                    transform_btm = rotation * scale_mat;
                    let a = Vec3::X.angle_between(drne);
                    let b = Vec3::Y.angle_between(drne);
                    // dbg!((a, b));
                    if abs_diff_ne!(drne.z, 0.0) {
                        if !a.is_nan() {
                            let mut scale_x = 1.0 / a.sin().abs() as f64;
                            let mut found_err = false;
                            if scale_x > 100.0 {
                                scale_x = 1.0;
                                found_err = true;
                            }
                            let mut scale_y = 1.0 / b.sin().abs() as f64;
                            if scale_y > 100.0 {
                                scale_y = 1.0;
                                found_err = true;
                            }
                            dbg!((scale_x, scale_y));
                            if found_err {
                                println!("Sloped ele wrong caculate scale: {:?}", (scale_x, scale_y));
                            }
                            scale_mat = Matrix4::from_nonuniform_scale(scale_x, scale_y, 1.0);
                            let m = Mat3::from_quat(glam::Quat::from_rotation_arc(-Vec3::Z, self.drne));
                            rotation = Matrix4::from_cols(
                                m.x_axis.vector4(),
                                m.y_axis.vector4(),
                                m.z_axis.vector4(),
                                Vector4::new(0.0, 0.0, 0.0, 1.0),
                            );
                        }
                    }

                    transform_top = transform_top * rotation * scale_mat;
                }

                let mut faces = vec![];
                let translation = Matrix4::from_translation(Vector3::new(0.0 as f64, 0.0 as f64, self.height as f64));
                let wire_s = builder::transformed(&wire, transform_btm);
                let wire_e = builder::transformed(&wire, translation * transform_top);
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
                    dbg!(plane.normal());
                    if plane.normal().z > 0.0 {
                        dbg!("invert");
                        for mut f in &mut faces {
                            f.invert();
                        }
                    }
                }

                return Some(faces.into());
            };
        }
        None
    }

    fn hash_unit_mesh_params(&self) -> u64 {
        //截面暂时用这个最省力的方法
        let mut hasher = DefaultHasher::default();
        let bytes = bincode::serialize(&self.profile).unwrap();
        bytes.hash(&mut hasher);

        if self.is_sloped() {
            hash_vec3::<DefaultHasher>(&self.drns, &mut hasher);
            hash_vec3::<DefaultHasher>(&self.drne, &mut hasher);
            hash_f32(self.height, &mut hasher);
        }

        if let Some((p1, p2, p3)) = self.arc_path {
            hash_vec3::<DefaultHasher>(&p1, &mut hasher);
            hash_vec3::<DefaultHasher>(&p2, &mut hasher);
            hash_vec3::<DefaultHasher>(&p3, &mut hasher);
        }
        "loft".hash(&mut hasher);

        hasher.finish()
    }

    fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
        let mut unit = self.clone();
        //sloped 不允许拉伸
        if unit.arc_path.is_none() && !self.is_sloped() {
            unit.extrude_dir = Vec3::Z;
            unit.height = 1.0;
        }
        Box::new(unit)
    }


    //拉伸为height方向
    fn gen_unit_mesh(&self) -> Option<PdmsMesh> {
        self.gen_unit_shape().gen_mesh(Some(TRI_TOL / 10.0))
    }

    #[inline]
    fn get_scaled_vec3(&self) -> Vec3 {
        if self.arc_path.is_some() || self.is_sloped() {
            Vec3::ONE
        } else {
            Vec3::new(1.0, 1.0, self.height)
        }
    }

    #[inline]
    fn get_trans(&self) -> TransformSRT {
        let mut vec = self.extrude_dir.normalize();

        match &self.profile {
            CateProfileParam::SANN(p) => {
                // if let Some(s) = &p.ptaxis {
                //     vec = Vec3::new(s.dir[0] as f32, s.dir[1] as f32, s.dir[2] as f32);
                // }
                return TransformSRT {
                    rotation: Quat::IDENTITY,//Quat::from_rotation_arc(Vec3::Y, vec),
                    scale: self.get_scaled_vec3(),
                    translation: Vec3::ZERO,
                    // translation: Vec3::new(p.xy[0] + p.dxy[0], p.xy[1] + p.dxy[1], 0.0),
                };
            }
            CateProfileParam::SPRO(_) => {
                if self.arc_path.is_some() {
                    return TransformSRT {
                        rotation: Quat::IDENTITY,
                        translation: self.get_scaled_vec3(),
                        scale: Vec3::ONE,
                    };
                } else {
                    return TransformSRT {
                        rotation: Quat::IDENTITY,
                        scale: self.get_scaled_vec3(),
                        translation: Vec3::ZERO,
                    };
                }
            }
            _ => {}
        }

        TransformSRT::IDENTITY
    }
}
