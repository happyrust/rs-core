use std::collections::hash_map::DefaultHasher;
use std::f32::consts::{FRAC_PI_2, PI};
use std::f32::EPSILON;
use std::hash::{Hash, Hasher};
use bevy::prelude::*;
use truck_modeling::{builder, Face, Shell, Surface, Wire};
use truck_meshalgo::prelude::*;
use bevy::reflect::Reflect;
use bevy::ecs::reflect::ReflectComponent;
use glam::{TransformRT, TransformSRT, Vec3};


use truck_modeling::builder::try_attach_plane;
use crate::parsed_data::CateProfileParam;
use crate::prim_geo::helper::cal_ref_axis;
use crate::shape::pdms_shape::{BrepMathTrait, BrepShapeTrait, PdmsMesh, VerifiedShape};
use crate::tool::hash_tool::hash_vec3;

#[derive(Component, Debug, Clone)]
pub struct SctnSolid {
    pub profile: CateProfileParam,
    pub drns: Vec3,
    pub drne: Vec3,
    pub height: f32,
    pub arc_path: Option<(Vec3, Vec3, Vec3)>,  //p1, p2, center  弧形的路径
}

impl SctnSolid {

    //is_btm 是否是底部的face
    fn cal_sann_face(&self, is_btm: bool, start_dir: Vec3, angle: f32, r1: f32, r2: f32) -> Option<Face>{
        use truck_base::cgmath64::*;
        let mut n = if is_btm { self.drns.normalize() } else { self.drne.normalize() };
        //dbg!(&n);
        let h = if is_btm { 0.0 } else { self.height };
        let a = angle;
        // let rot = Quat::IDENTITY;
        let mut z_axis = Vec3::Z;
        let z_angle: f32 = z_axis.angle_between(n);
        if z_angle == FRAC_PI_2 { return None; }
        //dbg!(z_angle);
        let mut y_axis_scale = (1.0 / z_angle.cos()) as f64;
        //dbg!(y_axis_scale);
        // let long_axis_len_2 = r2 / z_angle.cos();
        let mut rot_face = Quat::from_rotation_arc(Vec3::Z, n.normalize());
        let rot = Quat::from_rotation_arc(Vec3::X, start_dir);

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
        ]);

        let mat0 = Matrix4::from_translation(-center_pt.to_vec());
        // y_axis_scale = 5.0;
        let mat1 = Matrix4::from_nonuniform_scale(1.0, y_axis_scale, 1.0);
        let mat2 = Matrix4::from_angle_z(Rad(z_angle as f64));
        let mat3 = Matrix4::from_translation(center_pt.to_vec());
        // let new_wire = builder::transformed(&wire, mat3 * mat2 * /*mat1 **/ mat0);

        // let wire = builder::scaled(&wire, center_pt, Vector3::new(1.0, 1.0, z_axis_scale));
        // let (axis, angle) = rots.to_axis_angle();
        // let wire = builder::rotated(&wire, Point3::new(0.0, 0.0, h as f64), Vector3::new(1.0, 0.0, 0.0),
        //                             Rad(rot_angle as f64));
        // try_attach_plane(&[wire.clone()]).unwrap();
        try_attach_plane(&[wire.inverse()]).ok()
    }

    fn cal_spro_face(&self, is_btm: bool, verts: &Vec<[f32; 2]>) -> Option<Face>{
        let n = if is_btm { self.drns } else { self.drne };
        let h = if is_btm { 0.0 } else { self.height };
        let len = verts.len();
        let mut v0 = builder::vertex(Point3::new(verts[0][0] as f64, verts[0][1] as f64,h as f64));
        let mut prev_v0 = v0.clone();
        let mut edges = vec![];
        for i in 1..len {
            let next_v = builder::vertex(Point3::new(verts[i][0] as f64, verts[i][1] as f64,h as f64));
            edges.push(builder::line(&prev_v0, &next_v));
            prev_v0 = next_v.clone();
        }
        let last_v = edges.last().unwrap().back();
        edges.push(builder::line(last_v, &v0));

        let wire = edges.into();
        try_attach_plane(&[wire]).ok()
    }

}

impl Default for SctnSolid {
    fn default() -> Self {
        Self {
            profile: CateProfileParam::None,
            drns: Default::default(),
            drne: Default::default(),
            // axis_dir: Default::default(),
            height: 0.0,
            arc_path: None
        }
    }
}

impl VerifiedShape for SctnSolid {
    fn check_valid(&self) -> bool { self.height > f32::EPSILON }
}


impl BrepShapeTrait for SctnSolid {
    //涵盖的情况，需要考虑，上边只有一条边，和退化成点的情况
    fn gen_brep_shell(&self) -> Option<Shell> {
        use truck_modeling::*;
        use truck_base::cgmath64::{Point3, Vector3};
        let mut face_s = None;
        let mut face_e = None;
        match &self.profile {
            //需要用切面去切出相交的face
            CateProfileParam::SANN(p) =>{
                let w = p.pwidth;
                let r = p.pradius;
                let r1 = r - w;
                let r2 = r;
                let d = &p.ptaxis.as_ref().unwrap().dir;
                let dir = Vec3::new(d[0] as f32, d[1] as f32, d[2] as f32).normalize();
                let angle = p.pangle.to_radians();
                face_s = self.cal_sann_face(true, dir, angle, r1, r2);
                let w = p.pwidth + p.dwid;
                let r = p.pradius + p.drad;
                let r1 = r - w;
                let r2 = r;
                face_e = self.cal_sann_face(false, dir, angle, r1, r2).map(|x| x.inverse());
            }
            CateProfileParam::SPRO(p) =>{
                face_s = self.cal_spro_face(true, p);
                face_e = self.cal_spro_face(false, p).map(|x| x.inverse());
            }
            _ => {}
        }

        if let Some(face_s) = face_s{
            if let Some(face_e) = face_e {
                let mut faces = vec![];
                return if let Some((p1, p2, c)) = self.arc_path {
                    let angle = (p2 - c).angle_between(p1 - c);
                    let solid = builder::rsweep(&face_s, c.point3(), Vector3::new(0.0, 0.0, 1.0), Rad(angle as f64));
                    Some(solid.into_boundaries().remove(0))
                } else {
                    let edges_cnt = face_s.boundaries()[0].len();
                    for i in 0..edges_cnt {
                        let c1 = &face_s.boundaries()[0][i];
                        let c2 = &face_e.boundaries()[0][edges_cnt - i - 1];
                        faces.push(builder::homotopy(&c1.inverse(), c2));
                    }
                    faces.push(face_s);
                    faces.push(face_e);
                    Some(faces.into())
                }
            }
        }
        None
    }

    fn hash_mesh_params(&self) -> u64 {
        //截面暂时用这个最省力的方法
        let mut hasher = DefaultHasher::default();
        let bytes = bincode::serialize(&self.profile).unwrap();
        bytes.hash(&mut hasher);

        hash_vec3::<DefaultHasher>(&self.drns, &mut hasher);
        hash_vec3::<DefaultHasher>(&self.drne, &mut hasher);

        if let Some((p1, p2, c)) = self.arc_path {
            hash_vec3::<DefaultHasher>(&p1, &mut hasher);
            hash_vec3::<DefaultHasher>(&p2, &mut hasher);
            hash_vec3::<DefaultHasher>(&c, &mut hasher);
        }

        hasher.finish()
    }


    //拉伸为height方向
    fn gen_unit_shape(&self) -> PdmsMesh {
        let mut unit = self.clone();
        unit.height = 1.0;
        unit.gen_mesh(None)
    }

    #[inline]
    fn get_scaled_vec3(&self) -> Vec3 {
        Vec3::new(1.0, 1.0, self.height)
    }

    #[inline]
    fn get_trans(&self) -> TransformSRT {
        let mut vec = Vec3::Y;

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
                return TransformSRT {
                    rotation: Quat::IDENTITY,//Quat::from_rotation_arc(Vec3::Y, vec),
                    scale: self.get_scaled_vec3(),
                    translation: Vec3::ZERO,
                };
            }
            _ => {}
        }

        TransformSRT::IDENTITY
    }
}

// impl From<&AttrMap> for Loft {
//     fn from(m: &AttrMap) -> Self {
//         let xbot = m.get_val("XBOT").unwrap().f32_value().unwrap_or_default();
//         let ybot = m.get_val("YBOT").unwrap().f32_value().unwrap_or_default();
//
//         let xtop = m.get_val("XTOP").unwrap().f32_value().unwrap_or_default();
//         let ytop = m.get_val("YTOP").unwrap().f32_value().unwrap_or_default();
//
//         let xoff = m.get_val("XOFF").unwrap().f32_value().unwrap_or_default();
//         let yoff = m.get_val("YOFF").unwrap().f32_value().unwrap_or_default();
//
//         let height = m.get_val("HEIG").unwrap().f32_value().unwrap_or_default();
//
//
//         Loft {
//             pbax_expr: "X".to_string(),
//             pbax_pt: Default::default(),
//             pbax_dir: Vec3::X,
//             pcax_expr: "Y".to_string(),
//             pcax_pt: Default::default(),
//             pcax_dir: Vec3::Y,
//             paax_expr: "Z".to_string(),
//             paax_pt: Default::default(),
//             paax_dir: Vec3::Z,
//             pbtp: xtop,
//             pctp: ytop,
//             pbbt: xbot,
//             pcbt: ybot,
//             ptdi: height/2.0,
//             pbdi: -height/2.0,
//             pbof: xoff,
//             pcof: yoff,
//         }
//     }
// }
//
// impl From<AttrMap> for Loft {
//     fn from(m: AttrMap) -> Self {
//         (&m).into()
//     }
// }
