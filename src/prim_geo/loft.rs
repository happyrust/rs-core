use std::collections::hash_map::DefaultHasher;
use std::f32::consts::{FRAC_PI_2, PI, TAU};
use std::f32::EPSILON;
use std::hash::{Hash, Hasher};

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
use crate::shape::pdms_shape::{BrepMathTrait, BrepShapeTrait, PdmsMesh, TRI_TOL, VerifiedShape};
use crate::tool::hash_tool::hash_vec3;

//todo 针对确实只是extrusion的处理，可以转换成extrusion去处理，而不是占用

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct SctnSolid {
    pub profile: CateProfileParam,
    pub drns: Vec3,
    pub drne: Vec3,
    pub plane_normal: Vec3,
    pub extrude_dir: Vec3,
    pub height: f32,
    pub arc_path: Option<(Vec3, Vec3, Vec3)>,  //p1, p2, p3  弧形的路径
}

impl SctnSolid {
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
        //try to make it as ellipse wire

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
        // y_axis_scale = 5.0;
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

    fn cal_spro_face(&self, is_btm: bool, profile: &SProfileData, circle: Option<Circle2D>) -> Option<Face> {
        let n = if is_btm { self.drns } else { self.drne };
        let h = if is_btm { 0.0 } else { self.height };
        let verts = &profile.verts;
        let len = verts.len();

        let mut edges = vec![];
        let mut extrude = Vec3::Z;
        let mut offset_pt = Vec3::ZERO;
        let mut rot = Quat::from_rotation_arc(Vec3::Z, self.plane_normal);
        if circle.is_some() {
            offset_pt.x = circle.unwrap().r;
            extrude = self.plane_normal;
            // let rot = Quat::from_rotation_arc(Vec3::Z, self.plane_normal);
            // let p0 = rot.mul_vec3(Vec3::new(verts[0][0], verts[0][1],h) + offset_pt);
            // extrude = self.plane_normal;
            // let mut v0 = builder::vertex(p0.point3());
            // let mut prev_v0 = v0.clone();
            //
            // for i in 1..len {
            //     let p = rot.mul_vec3(Vec3::new(verts[i][0], verts[i][1],h) + offset_pt);
            //     let next_v = builder::vertex(p.point3());
            //     edges.push(builder::line(&prev_v0, &next_v));
            //     prev_v0 = next_v.clone();
            // }
            // let last_v = edges.last().unwrap().back();
            // edges.push(builder::line(last_v, &v0));
        } else {
            rot = Quat::IDENTITY;
        }
        // else{
        //     let mut v0 = builder::vertex(Point3::new(verts[0][0] as f64, verts[0][1] as f64,h as f64));
        //     let mut prev_v0 = v0.clone();
        //     for i in 1..len {
        //         let next_v = builder::vertex(Point3::new(verts[i][0] as f64, verts[i][1] as f64,h as f64));
        //         edges.push(builder::line(&prev_v0, &next_v));
        //         prev_v0 = next_v.clone();
        //     }
        //     let last_v = edges.last().unwrap().back();
        //     edges.push(builder::line(last_v, &v0));
        // }


        let p0 = rot.mul_vec3(Vec3::new(verts[0][0], verts[0][1], h) + offset_pt);

        let mut v0 = builder::vertex(p0.point3());
        let mut prev_v0 = v0.clone();

        for i in 1..len {
            let p = rot.mul_vec3(Vec3::new(verts[i][0], verts[i][1], h) + offset_pt);
            let next_v = builder::vertex(p.point3());
            edges.push(builder::line(&prev_v0, &next_v));
            prev_v0 = next_v.clone();
        }
        let last_v = edges.last().unwrap().back();
        edges.push(builder::line(last_v, &v0));

        let wire = edges.into();
        // dbg!(&extrude);
        // dbg!(&wire);
        // dbg!(extrude);
        if let Ok(mut f) = try_attach_plane(&[wire]) {
            if let Surface::Plane(plane) = f.get_surface() {
                // dbg!(plane.normal());
                if self.arc_path.is_none() {
                    if plane.normal().dot(extrude.vector3()) < 0.0 {
                        f = f.inverse();
                    }
                }
            }
            return Some(f);
        }

        None
    }
}

impl Default for SctnSolid {
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

impl VerifiedShape for SctnSolid {
    fn check_valid(&self) -> bool { /*self.height > f32::EPSILON*/ !self.extrude_dir.is_nan() && self.extrude_dir.length() > 0.0 }
}


impl BrepShapeTrait for SctnSolid {
    //涵盖的情况，需要考虑，上边只有一条边，和退化成点的情况
    fn gen_brep_shell(&self) -> Option<Shell> {
        use truck_modeling::*;
        use truck_base::cgmath64::{Point3, Vector3};
        let mut face_s = None;
        // let mut face_e = None;
        let mut circle = None;
        if let Some((p1, p2, p3)) = self.arc_path {
            let c = Circle2D::from_three_points(&Vec2::new(p1.x, p1.y), &Vec2::new(p2.x, p2.y), &Vec2::new(p3.x, p3.y));
            circle = Some(c);
        }
        // dbg!(&circle);
        let mut is_sann = false;
        match &self.profile {
            //需要用切面去切出相交的face
            CateProfileParam::SANN(p) => {

                // dbg!(&p);
                let w = p.pwidth;
                let r = p.pradius;
                let r1 = r - w;
                let r2 = r;
                let d = &p.ptaxis.as_ref().unwrap().dir;
                let dir = Vec3::new(d[0] as f32, d[1] as f32, d[2] as f32).normalize();
                let angle = p.pangle.to_radians();
                face_s = self.cal_sann_face(true, dir, angle, r1, r2, circle.clone());
                // let w = p.pwidth + p.dwid;
                // let r = p.pradius + p.drad;
                // let r1 = r - w;
                // let r2 = r;
                // face_e = self.cal_sann_face(false, dir, angle, r1, r2).map(|x| x.inverse());
                is_sann = true;
            }
            CateProfileParam::SPRO(p) => {
                face_s = self.cal_spro_face(true, p, circle.clone());
                // if self.arc_path.is_none() {
                //     face_e = self.cal_spro_face(false, p).map(|x| x.inverse());
                // }
            }
            _ => {}
        }

        if let Some(face_s) = face_s {
            {
                // let mut faces = vec![];
                return if let Some((p1, p2, p3)) = self.arc_path {
                    let c = circle.unwrap_or_default();
                    // dbg!(&c);
                    let v1 = Vec2::new(p1.x, p1.y) - c.center;
                    let v3 = Vec2::new(p3.x, p3.y) - c.center;
                    let mut angle = v3.angle_between(v1);
                    let mut rot_z = Vec3::Z;
                    let tmp_axis = Vec3::new(v1.x, v1.y, 0.0).cross(Vec3::new(v3.x, v3.y, 0.0));
                    if tmp_axis.dot(rot_z) > 0.0 {
                        rot_z = Vec3::Z;
                    }
                    if angle < 0.0 {
                        // rot_z = -Vec3::Z;
                        angle = angle.abs();
                    }
                    // dbg!(angle.to_degrees());
                    let solid = builder::rsweep(&face_s, Point3::new(c.center.x as f64, c.center.y as f64, 0.0),
                                                rot_z.vector3(), Rad(angle as f64));
                    Some(solid.into_boundaries().remove(0))
                } else {

                    //self.plane_normal.vector3()
                    let solid = builder::tsweep(&face_s, Vec3::Z.vector3() * self.height as f64);
                    Some(solid.into_boundaries().remove(0))
                    // if let Some(face_e) = face_e {
                    //     let edges_cnt = face_s.boundaries()[0].len();
                    //
                    //     //need to check if need inverse
                    //     for i in 0..edges_cnt {
                    //         let c1 = &face_s.boundaries()[0][i];
                    //         let c2 = &face_e.boundaries()[0][edges_cnt - i - 1];
                    //         faces.push(builder::homotopy(&c1.inverse(), c2));
                    //     }
                    //     faces.push(face_s);
                    //     faces.push(face_e);
                    //     Some(faces.into())
                    // }else{
                    //     None
                    // }
                };
            }
        }
        None
    }

    fn hash_mesh_params(&self) -> u64 {
        //截面暂时用这个最省力的方法
        let mut hasher = DefaultHasher::default();
        let bytes = bincode::serialize(&self.profile).unwrap();
        // let bytes = bincode::serialize(&self/*.profile*/).unwrap();
        bytes.hash(&mut hasher);

        hash_vec3::<DefaultHasher>(&self.drns, &mut hasher);
        hash_vec3::<DefaultHasher>(&self.drne, &mut hasher);

        if let Some((p1, p2, p3)) = self.arc_path {
            hash_vec3::<DefaultHasher>(&p1, &mut hasher);
            hash_vec3::<DefaultHasher>(&p2, &mut hasher);
            hash_vec3::<DefaultHasher>(&p3, &mut hasher);
        }

        hasher.finish()
    }


    //拉伸为height方向
    fn gen_unit_shape(&self) -> PdmsMesh {
        let mut unit = self.clone();
        if unit.arc_path.is_none() {
            unit.extrude_dir = Vec3::Z;
            unit.height = 1.0;
        }
        unit.gen_mesh(Some(TRI_TOL / 10.0))
    }

    #[inline]
    fn get_scaled_vec3(&self) -> Vec3 {
        if self.arc_path.is_some() {
            Vec3::ONE
        } else {
            // Vec3::ONE
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
                        // rotation: Quat::from_rotation_arc(Vec3::Z,  self.plane_normal),

                        // rotation: Quat::from_rotation_arc(Vec3::X, self.extrude_dir),
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
