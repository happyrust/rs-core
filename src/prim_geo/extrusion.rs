use std::collections::hash_map::DefaultHasher;
use std::f32::consts::PI;
use std::f32::EPSILON;
use std::hash::{Hash, Hasher};

use anyhow::anyhow;
use approx::abs_diff_eq;
use bevy::ecs::reflect::ReflectComponent;
use bevy::prelude::*;
use bevy::reflect::Reflect;
use nalgebra_glm::sin;
use truck_meshalgo::prelude::*;
use truck_modeling::{builder, Shell, Surface, Wire};


use crate::pdms_types::AttrMap;
use crate::prim_geo::helper::{cal_ref_axis, RotateInfo};
use crate::shape::pdms_shape::{BevyMathTrait, BrepMathTrait, BrepShapeTrait, PdmsMesh, TRI_TOL, VerifiedShape};
use crate::tool::hash_tool::{hash_f32, hash_vec3};
use serde::{Serialize, Deserialize};

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub enum CurveType {
    Fill,
    Spline(f32),  //thick
}

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Extrusion {
    pub verts: Vec<Vec3>,
    pub fradius_vec: Vec<f32>,
    pub height: f32,
    pub cur_type: CurveType,
}

fn circus_center(pt0: Point3, pt1: Point3, pt2: Point3) -> Point3 {
    let vec0 = pt1 - pt0;
    let vec1 = pt2 - pt0;
    let a2 = vec0.dot(vec0);
    let ab = vec0.dot(vec1);
    let b2 = vec1.dot(vec1);
    let det = a2 * b2 - ab * ab;
    let u = (b2 * a2 - ab * b2) / (2.0 * det);
    let v = (-ab * a2 + b2 * a2) / (2.0 * det);
    pt0 + u * vec0 + v * vec1
}

impl Extrusion {
    //todo 实现Justline
    pub fn gen_spline_wire(&self, mut new_verts: Vec<Vec3>, thick: f32) -> anyhow::Result<Wire> {
        if new_verts.len() != 3 {
            return Err(anyhow!("SPINE number is not 3".to_string()));   //先假定必须有三个
        }

        let pt0 = new_verts[0].point3();
        let transit = new_verts[1].point3();
        let pt1 = new_verts[2].point3();

        let vec0 = (pt0 - transit).normalize();
        let vec1 = (pt1 - transit).normalize();
        let origin = circus_center(pt0, pt1, transit);
        let mut angle = Rad(PI as f64) - vec0.angle(vec1);
        let mut rot_axis = Vec3::Z;
        if (vec0.cross(vec1)).dot(Vector3::unit_z()) > 0.0 {
            rot_axis = -Vec3::Z;
        }
        let radius = origin.distance(pt0);

        let v0 = (pt0 - origin).normalize();
        let v1 = (pt1 - origin).normalize();

        let half_thick = thick as f64 / 2.0;
        let p0 = pt0 - v0 * half_thick;
        let p1 = pt1 - v1 * half_thick;
        let p2 = pt1 + v1 * half_thick;
        let p3 = pt0 + v0 * half_thick;

        let ver0 = builder::vertex(p0);
        let ver1 = builder::vertex(p1);
        let ver2 = builder::vertex(p2);
        let ver3 = builder::vertex(p3);

        let t_v = (transit - origin).normalize();
        let t0 = transit - (half_thick * t_v);
        let t1 = transit + (half_thick * t_v);

        let wire = Wire::from([
            builder::circle_arc(&ver0, &ver1, t0),
            // builder::circle_arc_with_center(origin, &ver0, &ver1, rot_axis.vector3(), angle),
            builder::line(&ver1, &ver2),
            // builder::circle_arc_with_center(origin, &ver2, &ver3, -rot_axis.vector3(), angle),
            builder::circle_arc(&ver2, &ver3, t1),
            builder::line(&ver3, &ver0),
        ]);

        Ok(wire)
    }

    pub fn gen_wire(&self, mut new_verts: Vec<Vec3>) -> anyhow::Result<Wire> {
        let mut wire = Wire::new();
        let ll = new_verts.len();
        let mut pre_radius = 0.0;
        let mut i = 1;
        let r = self.fradius_vec[0];
        let origin_vert = if abs_diff_eq!(r, 0.0) {
            builder::vertex(new_verts[0].point3())
        } else {
            let v = &new_verts;
            let b_dir = (v[1] - v[0]).normalize();
            let a_dir = (v[ll - 1] - v[0]).normalize();
            let angle = a_dir.angle_between(b_dir) / 2.0;
            if abs_diff_eq!(angle, 0.0) { return Err(anyhow!("fill的两个方向角度不能为0".to_string())); }
            let b_len = r / angle.tan();
            //dbg!(b_len);
            let pbax_pt = v[0] + b_dir * b_len;
            builder::vertex(pbax_pt.point3())
        };
        let mut pre_vert = origin_vert.clone();
        //从下一个点开始
        for i in 1..=ll {
            let cur_pt = &new_verts[i % ll];
            //如果点重合了，需要跳过
            if pre_vert.get_point().vec3().distance(*cur_pt) <= 0.01 {
                continue;
            }
            let fradius = self.fradius_vec[i % ll];
            if abs_diff_eq!(fradius, 0.0) {
                let cur_vert = if i != ll { builder::vertex(cur_pt.point3()) } else { origin_vert.clone() };
                if pre_vert.get_point().distance(cur_vert.get_point()) > 0.01 {
                    wire.push_back(builder::line(&pre_vert, &cur_vert));
                    pre_vert = cur_vert.clone();
                }
            } else {
                let r = fradius;
                let pre_i = i - 1;
                let n_i = (i + 1) % ll;
                let pre_pt = new_verts[pre_i];
                let cur_pt = new_verts[i % ll];
                let next_pt = new_verts[n_i];
                let pa_dist = pre_pt.distance(cur_pt);
                let pb_dist = next_pt.distance(cur_pt);
                let a_dir = (pre_pt - cur_pt).normalize();
                let b_dir = (next_pt - cur_pt).normalize();
                let angle = a_dir.angle_between(b_dir) / 2.0;
                let b_len = r / angle.tan();

                if b_len - pa_dist.min(pb_dist) > 0.01 {
                    let cur_vert = if i != ll { builder::vertex(cur_pt.point3()) } else { origin_vert.clone() };
                    wire.push_back(builder::line(&pre_vert, &cur_vert));
                    pre_vert = cur_vert.clone();
                    continue;
                }
                let paax_pt = cur_pt + a_dir * b_len;
                let pbax_pt = cur_pt + b_dir * b_len;

                let mut t_va = pre_vert.clone();
                let mut va = builder::vertex(paax_pt.point3());
                let mut t_vb = builder::vertex(pbax_pt.point3());

                if paax_pt.distance(pre_vert.get_point().vec3()) >= 0.01 {
                    t_va = va.clone();
                    wire.push_back(builder::line(&pre_vert, &t_va));
                }

                let origin_dist = pbax_pt.distance(origin_vert.get_point().vec3());
                if origin_dist < 0.01 {
                    t_vb = origin_vert.clone();
                }

                let h = r * angle.sin();
                let d = r - h;
                let mid_pt = (pbax_pt + paax_pt) / 2.0;
                let mid_dir = (cur_pt - mid_pt).normalize();
                let transit_pt = mid_pt + mid_dir * d;

                wire.push_back(builder::circle_arc(&t_va, &t_vb, transit_pt.point3()));
                //提前结束
                if origin_dist < 0.01 {
                    break;
                }
                if i == ll {
                    if origin_dist >= 0.01 {
                        wire.push_back(builder::line(&t_vb, &origin_vert));
                    }
                }
                pre_vert = t_vb.clone();
            }
        }

        Ok(wire)
    }
}


fn get_vec3_hash(v: &Vec3) -> u64 {
    let mut hasher = DefaultHasher::new();
    hash_vec3::<DefaultHasher>(v, &mut hasher);
    hasher.finish()
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

impl BrepShapeTrait for Extrusion {
    fn gen_brep_shell(&self) -> Option<Shell> {
        if !self.check_valid() { return None; }
        if self.verts.len() < 3 {
            return None;
        }
        let mut new_verts = self.verts.iter().map(|v| {
            Vec3::new(v.x, v.y, 0.0)
        }).collect::<Vec<_>>();
        let mut pre_hash = 0;
        if get_vec3_hash(&new_verts[0]) == get_vec3_hash(new_verts.last().unwrap()) {
            new_verts.remove(new_verts.len() - 1);
        }
        new_verts.retain(|x| {
            let hash = get_vec3_hash(x);
            let retain = pre_hash != hash;
            pre_hash = hash;
            retain
        });
        let ll = new_verts.len();
        if ll < 3 {
            return None;
        }

        let mut wire = Wire::new();
        if let CurveType::Spline(thick) = self.cur_type {
            wire = self.gen_spline_wire(new_verts, thick).ok()?;
        } else {
            wire = self.gen_wire(new_verts).ok()?;
        };

        // dbg!(&wire);
        if let Ok(mut face) = builder::try_attach_plane(&[wire.clone()]) {
            if let Surface::Plane(plane) = face.get_surface() {
                let extrude_dir = Vector3::new(0.0, 0.0, 1.0);
                // dbg!(&plane.normal());
                if plane.normal().dot(extrude_dir) < 0.0 {
                    face = face.inverse();
                }
                let mut s = builder::tsweep(&face, extrude_dir * self.height as f64).into_boundaries();
                return s.pop();
            }
        } else {
            dbg!(self);
        }
        None
    }


    fn hash_mesh_params(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.verts.iter().for_each(|v| {
            hash_vec3::<DefaultHasher>(v, &mut hasher);
        });
        self.fradius_vec.iter().for_each(|v| {
            hash_f32::<DefaultHasher>(v, &mut hasher);
        });
        "Extrusion".hash(&mut hasher);
        hasher.finish()
    }

    fn gen_unit_shape(&self) -> PdmsMesh {
        let unit = Self {
            verts: self.verts.clone(),
            height: 100.0,   //开放一点大小，不然三角化出来的不对
            fradius_vec: self.fradius_vec.clone(),
            cur_type: self.cur_type.clone(),
            ..default()
        };
        unit.gen_mesh(Some(TRI_TOL))
    }


    //沿着指定方向拉伸 pbax_dir
    fn get_scaled_vec3(&self) -> Vec3 {
        // let e = self.paax_dir.normalize()
        //     .cross(self.pbax_dir.normalize()).normalize();
        // let new_dir = Vec3::new(e.x.abs(), e.y.abs(), e.z.abs());
        // //(self.height as f32 / 100.0) * new_dir
        // Vec3::new(1.0, (self.height as f32 / 100.0), 1.0)
        Vec3::new(1.0, 1.0, (self.height as f32 / 100.0))
    }
}
