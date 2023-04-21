use serde_derive::{Deserialize, Serialize};
use truck_base::cgmath64::{InnerSpace, MetricSpace, Point3, Rad, Vector3};
use glam::Vec3;
use truck_modeling::{builder, Vertex, Wire};
use std::f32::consts::PI;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use crate::prim_geo::extrusion::Extrusion;
use crate::shape::pdms_shape::BrepMathTrait;
use crate::tool::float_tool::hash_vec3;
use anyhow::*;
use approx::abs_diff_eq;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CurveType {
    Fill,
    Spline(f32),  //thick
}

pub fn circus_center(pt0: Point3, pt1: Point3, pt2: Point3) -> Point3 {
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

//todo 实现Justline
pub fn gen_spline_wire(verts: &Vec<Vec3>, thick: f32) -> anyhow::Result<Wire> {
    if verts.len() != 3 {
        return Err(anyhow!("SPINE number is not 3".to_string()));   //先假定必须有三个
    }

    let pt0 = verts[0].point3();
    let transit = verts[1].point3();
    let pt1 = verts[2].point3();

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
        builder::line(&ver1, &ver2),
        builder::circle_arc(&ver2, &ver3, t1),
        builder::line(&ver3, &ver0),
    ]);

    Ok(wire)
}

/// 根据顶点信息和fradius半径，生成wire
pub fn gen_wire(pts: &Vec<Vec3>, fradius_vec: &Vec<f32>) -> anyhow::Result<Wire> {
    //todo 需要返回带变换矩阵的wire
    if pts.len() < 3 {
        return Err(anyhow!("Extrusion 的wire 顶点数量不够，小于3。"));
    }
    let mut wire = Wire::new();
    let ll = pts.len();
    let mut pre_radius = 0.0;
    let mut i = 1;
    let r = fradius_vec[0];
    let mut verts = vec![];
    let mut pre_pt = pts[0];
    let mut circle_indexs = vec![];
    for i in 0..ll {
        let fradius = fradius_vec[i];
        let pt = pts[i].point3_without_z();
        //跳过相同的点
        if let Some(last_pt) = verts.last().map(|x: &Vertex| x.point()) {
            if pt.distance(last_pt) < 0.1 {
                // dbg!(pt);
                continue;
            }
            if i == ll - 1 {
                if pt.distance(verts[0].point()) < 0.1 {
                    // dbg!(pt);
                    continue;
                }
            }
        }
        if abs_diff_eq!(fradius, 0.0) {
            verts.push(builder::vertex(pt));
            pre_pt = pts[i];
        } else {
            let r = fradius;
            let pre_i = (ll + i - 1) % ll;
            let n_i = (i + 1) % ll;
            let pre_pt = pts[pre_i];
            let cur_pt = pts[i % ll];
            let next_pt = pts[n_i];
            let pa_dist = pre_pt.distance(cur_pt);
            let pb_dist = next_pt.distance(cur_pt);
            // dbg!((pre_pt, cur_pt, next_pt));
            // dbg!(pb_dist);
            // dbg!(pb_dist);
            let a_dir = (pre_pt - cur_pt).normalize();
            let b_dir = (next_pt - cur_pt).normalize();
            let angle = a_dir.angle_between(b_dir) / 2.0;
            let b_len = r / angle.tan();

            let h = r * angle.sin();
            let d = r - h;
            let p0 = cur_pt + a_dir * b_len;
            let p1 = cur_pt + b_dir * b_len;
            let mid_pt = (p0 + p1) / 2.0;
            let mid_dir = (cur_pt - mid_pt).normalize();
            let transit_pt = mid_pt + mid_dir * d;
            if pa_dist - b_len > 0.01 {
                verts.push(builder::vertex(p0.point3_without_z()));
            }

            verts.push(builder::vertex(transit_pt.point3_without_z()));
            circle_indexs.push(verts.len() - 1);

            if pb_dist - b_len > 0.01 {
                verts.push(builder::vertex(p1.point3_without_z()));
            }
        }
    }
    let mut j = 0;
    if !verts.is_empty() {
        let s_vert = verts.first().unwrap();
        let e_vert = verts.last().unwrap();
        let l = s_vert.point().distance(e_vert.point());
        if l < 0.1 {
            verts.pop();
        }
        let v_len = verts.len();
        if v_len == 0 {
            dbg!(pts);
            dbg!(fradius_vec);
            return Err(anyhow!(" verts are empty"));
        }
        let mut pre_vert = verts[0].clone();
        j = 1;
        // dbg!(&verts);
        // dbg!(&circle_indexs);
        while j <= v_len {
            let cur_vert = &verts[j % v_len];
            if pre_vert.point().distance(cur_vert.point()) > 1.0 {
                if circle_indexs.len() > 0 && j == circle_indexs[0] {
                    let next_vert = &verts[(j + 1) % v_len];
                    wire.push_back(builder::circle_arc(&pre_vert, next_vert, cur_vert.point()));
                    pre_vert = next_vert.clone();
                    circle_indexs.remove(0);
                    j += 1;
                } else {
                    wire.push_back(builder::line(&pre_vert, cur_vert));
                    pre_vert = cur_vert.clone();
                    // dbg!(i);
                }
            }
            j += 1;
        }
    }
    // dbg!(&wire);

    Ok(wire)
}
