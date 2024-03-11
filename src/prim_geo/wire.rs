use cavalier_contours::core::traits::Real;
use cavalier_contours::polyline::internal::pline_intersects::visit_global_self_intersects;
use cavalier_contours::polyline::*;
use cavalier_contours::static_aabb2d_index::StaticAABB2DIndex;
use cgmath::Basis2;
use geo::convex_hull::{graham_hull, quick_hull};
use geo::{coord, IsConvex};
use glam::{DVec3, Quat, Vec3};
use nalgebra::ComplexField;
use serde_derive::{Deserialize, Serialize};
use truck_base::cgmath64::{InnerSpace, MetricSpace, Point3, Rad, Vector3};

use crate::shape::pdms_shape::{BrepMathTrait, LEN_TOL};
use crate::tool::float_tool::*;
use crate::tool::float_tool::{cal_vec2_hash_string, cal_xy_hash_string, vec3_round_2};
use anyhow::anyhow;
use approx::abs_diff_eq;

use std::collections::{BTreeSet, HashMap};
use std::f32::consts::PI;

#[cfg(feature = "occ")]
use opencascade::primitives::{Edge, Wire};

#[derive(
    Debug, Clone, Serialize, Deserialize, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize,
)]
pub enum CurveType {
    Fill,
    Spline(f32), //thick
}

pub fn cal_circus_center(pt0: Vec3, pt1: Vec3, pt2: Vec3) -> Vec3 {
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

#[cfg(feature = "occ")]
///生成occ的wire
pub fn gen_occ_spline_wire(verts: &Vec<Vec3>, thick: f32) -> anyhow::Result<Wire> {
    if verts.len() != 3 {
        return Err(anyhow!("SPINE number is not 3".to_string())); //先假定必须有三个
    }

    let pt0 = verts[0];
    let transit = verts[1];
    let pt1 = verts[2];

    let vec0 = (pt0 - transit).normalize();
    let vec1 = (pt1 - transit).normalize();
    let origin = cal_circus_center(pt0, pt1, transit);
    let _angle = PI - vec0.angle_between(vec1);
    let mut rot_axis = Vec3::Z;
    if (vec0.cross(vec1)).dot(Vec3::Z) > 0.0 {
        rot_axis = -Vec3::Z;
    }
    let _radius = origin.distance(pt0);

    let v0 = (pt0 - origin).normalize();
    let v1 = (pt1 - origin).normalize();

    let half_thick = thick / 2.0;
    let p0 = (pt0 - v0 * half_thick).as_dvec3();
    let p1 = (pt1 - v1 * half_thick).as_dvec3();
    let p2 = (pt1 + v1 * half_thick).as_dvec3();
    let p3 = (pt0 + v0 * half_thick).as_dvec3();

    let t_v = (transit - origin).normalize();
    let t0 = (transit - (half_thick * t_v)).as_dvec3();
    let t1 = (transit + (half_thick * t_v)).as_dvec3();

    let edges = vec![
        Edge::arc(p0, p1, t0),
        Edge::segment(p1, p2),
        Edge::arc(p2, p3, t1),
        Edge::segment(p3, p0),
    ];

    Ok(Wire::from_edges(&edges))
}

///生成truck的wire
pub fn gen_spline_wire(
    input_verts: &Vec<Vec3>,
    thick: f32,
) -> anyhow::Result<truck_modeling::Wire> {
    use truck_modeling::{builder, Wire};
    if input_verts.len() != 3 {
        return Err(anyhow!("SPINE number is not 3".to_string())); //先假定必须有三个
    }
    let verts = input_verts
        .into_iter()
        .map(|x| vec3_round_2(*x))
        .collect::<Vec<_>>();

    let pt0 = verts[0].point3();
    let transit = verts[1].point3();
    let pt1 = verts[2].point3();

    let vec0 = (pt0 - transit).normalize();
    let vec1 = (pt1 - transit).normalize();
    let origin = circus_center(pt0, pt1, transit);
    let _angle = Rad(PI as f64) - vec0.angle(vec1);
    let mut rot_axis = Vec3::Z;
    if (vec0.cross(vec1)).dot(Vector3::unit_z()) > 0.0 {
        rot_axis = -Vec3::Z;
    }
    let _radius = origin.distance(pt0);

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

///生成occ的wire
#[cfg(feature = "occ")]
pub fn gen_occ_wire(pts: &Vec<Vec3>, fradius_vec: &Vec<f32>) -> anyhow::Result<Wire> {
    if pts.len() < 3 {
        return Err(anyhow!("Extrusion 的wire 顶点数量不够，小于3。"));
    }
    let mut edges = vec![];
    let ll = pts.len();
    let _pre_radius = 0.0;
    let _i = 1;
    let _r = fradius_vec[0];
    let mut verts = vec![];
    let mut pre_pt = pts[0];
    let mut circle_indexs = vec![];
    for i in 0..ll {
        let fradius = fradius_vec[i];
        let pt = pts[i].truncate().extend(0.0);
        //跳过相同的点
        if let Some(&last_pt) = verts.last() {
            if pt.distance(last_pt) < 0.1 {
                continue;
            }
            if i == ll - 1 {
                if pt.distance(verts[0]) < 0.1 {
                    continue;
                }
            }
        }
        if abs_diff_eq!(fradius.abs(), 0.0) {
            verts.push(pt);
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
                verts.push(p0.truncate().extend(0.0));
            }

            verts.push(transit_pt.truncate().extend(0.0));
            circle_indexs.push(verts.len() - 1);

            if pb_dist - b_len > 0.01 {
                verts.push(p1.truncate().extend(0.0));
            }
        }
    }
    let mut j = 0;
    if !verts.is_empty() {
        let s_vert = verts.first().unwrap();
        let e_vert = verts.last().unwrap();
        let l = s_vert.distance(*e_vert);
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
        while j <= v_len {
            let cur_vert = &verts[j % v_len];
            if pre_vert.distance(*cur_vert) > 1.0 {
                if circle_indexs.len() > 0 && j == circle_indexs[0] {
                    let next_vert = &verts[(j + 1) % v_len];
                    // wire.push_back(builder::circle_arc(&pre_vert, next_vert, cur_vert.point()));
                    edges.push(Edge::arc(
                        pre_vert.as_dvec3(),
                        cur_vert.as_dvec3(),
                        next_vert.as_dvec3(),
                    ));
                    pre_vert = next_vert.clone();
                    circle_indexs.remove(0);
                    j += 1;
                } else {
                    // wire.push_back(builder::line(&pre_vert, cur_vert));
                    edges.push(Edge::segment(pre_vert.as_dvec3(), cur_vert.as_dvec3()));
                    pre_vert = cur_vert.clone();
                }
            }
            j += 1;
        }
    }
    Ok(Wire::from_edges(&edges))
}

#[cfg(feature = "opencascade")]
///生成occ的wire
pub fn gen_occ_wire(pts: &Vec<Vec3>, fradius_vec: &Vec<f32>) -> anyhow::Result<Wire> {
    if pts.len() < 3 {
        return Err(anyhow!("Extrusion 的wire 顶点数量不够，小于3。"));
    }
    let ll = pts.len();
    let mut pre_radius = 0.0;
    let mut i = 1;
    let r = fradius_vec[0];
    let mut verts = vec![];
    let mut pre_pt = pts[0];
    let mut circle_indexs = vec![];
    let mut edges = vec![];
    let mut all_on_line = true;
    for i in 0..ll {
        let fradius = fradius_vec[i];
        let pt: Vec3 = pts[i].truncate().extend(0.0);
        //跳过相同的点
        if let Some(&last_pt) = verts.last() {
            if pt.distance(last_pt) < 0.1 {
                continue;
            }
            if i == ll - 1 {
                if pt.distance(verts[0]) < 0.1 {
                    continue;
                }
            }
        }
        if abs_diff_eq!(fradius.abs(), 0.0) {
            let cl = verts.len();
            if cl >= 2 {
                let v1 = (verts[cl - 2] - verts[cl - 1]).normalize();
                let v2 = (pt - verts[cl - 1]).normalize();
                // dbg!(dot);
                let v = v1.cross(v2);
                // dbg!(v);
                // dbg!(fradius);
                //共线的点不要
                if all_on_line && abs_diff_ne!(v.length(), 0.0, epsilon = 0.001) {
                    // dbg!("发现共线的点");
                    all_on_line = false;
                }
            }
            verts.push(pt);
            pre_pt = pts[i];
        } else {
            all_on_line = false;
            let r = fradius;
            let pre_i = (ll + i - 1) % ll;
            let n_i = (i + 1) % ll;
            let pre_pt = pts[pre_i];
            let cur_pt = pts[i % ll];
            let next_pt = pts[n_i];
            let pa_dist = pre_pt.distance(cur_pt);
            let pb_dist = next_pt.distance(cur_pt);
            // dbg!((pa_dist, pb_dist));
            let a_dir = (pre_pt - cur_pt).normalize();
            let b_dir = (next_pt - cur_pt).normalize();
            let angle = a_dir.angle_between(b_dir) / 2.0;
            // dbg!((r, angle));
            let b_len = r / angle.tan();

            let h = r * angle.sin();
            let d = r - h;
            // dbg!(d);
            let p0 = cur_pt + a_dir * b_len;
            let p1 = cur_pt + b_dir * b_len;
            let mid_pt = (p0 + p1) / 2.0;
            let mid_dir = (cur_pt - mid_pt).normalize();
            let transit_pt = mid_pt + mid_dir * d;
            if pa_dist - b_len > 0.01 {
                verts.push(p0.truncate().extend(0.0));
            }

            verts.push(transit_pt.truncate().extend(0.0));
            circle_indexs.push(verts.len() - 1);

            if pb_dist - b_len > 0.01 {
                //let pt: Vec3 = pts[i].truncate().extend(0.0);
                verts.push(p1.truncate().extend(0.0));
            }
        }
    }

    // dbg!(all_on_line);
    // dbg!(&circle_indexs);
    if all_on_line {
        return Err(anyhow!("线圈全部共线"));
    }

    let mut j = 0;
    if verts.len() >= 3 {
        let s_vert = *verts.first().unwrap();
        let e_vert = *verts.last().unwrap();
        let l = s_vert.distance(e_vert);
        if l < 0.1 {
            verts.pop();
        }
        let v_len = verts.len();
        if v_len == 0 {
            return Err(anyhow!("Verts are empty"));
        }
        let mut pre_vert = verts[0];
        j = 1;
        while j <= v_len {
            let cur_vert = verts[j % v_len];
            if pre_vert.distance(cur_vert) > 1.0 {
                if circle_indexs.len() > 0 && j == circle_indexs[0] {
                    let next_vert = verts[(j + 1) % v_len];
                    edges.push(Edge::new_arc(
                        &pre_vert.into(),
                        &cur_vert.into(),
                        &next_vert.into(),
                    )?);
                    pre_vert = next_vert;
                    circle_indexs.remove(0);
                    j += 1;
                } else {
                    edges.push(Edge::new_line(&pre_vert.into(), &cur_vert.into())?);
                    pre_vert = cur_vert.clone();
                }
            }
            j += 1;
        }
    } else {
        return Err(anyhow!("线圈的点数<3"));
    }
    Ok(Wire::from_edges(&edges)?)
}

pub fn gen_wire_test(
    pts: &Vec<Vec3>,
    fradius_vec: &Vec<f32>,
) -> anyhow::Result<truck_modeling::Wire> {
    use cavalier_contours::polyline::*;
    use cgmath::prelude::*;
    use truck_modeling::{builder, Vertex, Wire};
    if pts.len() < 3 || fradius_vec.len() != pts.len() {
        return Err(anyhow!("Extrusion 的wire 顶点数量不够，小于3。"));
    }
    let mut polyline = Polyline::new_closed();
    for i in 0..pts.len() {
        let r = fradius_vec[i] as f64;
        if r.abs() < 0.01 {
            polyline.add(pts[i].x as f64, pts[i].y as f64, 0.0);
            continue;
        }
        dbg!(r);
        let c_pt = pts[i].as_dvec3().truncate();
        let p_pt = pts[(i + pts.len() - 1) % pts.len()].as_dvec3().truncate();
        let n_pt = pts[(i + 1) % pts.len()].as_dvec3().truncate();
        let p_len = p_pt.distance(c_pt);
        let n_len = p_pt.distance(c_pt);
        //以 c_pt 为圆心， r为半径，算出两个端点
        let p_dir = (p_pt - c_pt).normalize();
        let n_dir = (n_pt - c_pt).normalize();
        let r_p_pt = c_pt + p_dir * r;
        let r_n_pt = c_pt + n_dir * r;
        // dbg!((p_dir, n_dir));
        let angle = n_dir.angle_between(p_dir);
        let bulge = angle.signum() * ((PI as f64 - angle.abs()) / 4.0).tan();
        let bulge = f64_ceil_3(bulge);
        dbg!(bulge);

        dbg!(p_len);
        if p_len - r >= 0.01 {
            polyline.add(p_pt.x, p_pt.y, bulge);
            polyline.add(n_pt.x, n_pt.y, 0.0);
        } else {
            dbg!(c_pt);
            dbg!(p_len);
        }

        // polyline.add(pts[i].x as f64, pts[i].y as f64, bulge);
    }

    let polyline = polyline.arcs_to_approx_lines(1e-1).unwrap();
    let new_polyline = if let Some(p) = polyline.remove_redundant(0.01) {
        p
    } else {
        polyline
    };

    let mut wire = Wire::new();
    let first_vert = builder::vertex(Point3::new(new_polyline[0].x, new_polyline[0].y, 0.0));
    let mut prev_vert = first_vert.clone();
    let count = new_polyline.vertex_count();
    for (index, (i, j)) in new_polyline.iter_segments().into_iter().enumerate() {
        let pt = Point3::new(j.x, j.y, 0.0);
        let vert = if index == count - 1 {
            first_vert.clone()
        } else {
            builder::vertex(pt)
        };
        wire.push_back(builder::line(&prev_vert, &vert));
        prev_vert = vert;
    }

    return Ok(wire);
}

fn global_self_intersects<T>(
    polyline: &Polyline<T>,
    aabb_index: &StaticAABB2DIndex<T>,
) -> PlineIntersectsCollection<T>
where
    T: Real,
{
    let mut intrs = Vec::new();
    let mut overlapping_intrs = Vec::new();
    let mut visitor = |intr: PlineIntersect<T>| match intr {
        PlineIntersect::Basic(b) => {
            intrs.push(b);
        }
        PlineIntersect::Overlapping(o) => {
            overlapping_intrs.push(o);
        }
    };

    visit_global_self_intersects(polyline, aabb_index, &mut visitor, T::from(1e-5).unwrap());

    PlineIntersectsCollection::new(intrs, overlapping_intrs)
}

pub fn gen_wire_new(pts: &Vec<Vec3>, fradius_vec: &Vec<f32>) -> anyhow::Result<truck_modeling::Wire> {
    use cavalier_contours::polyline::*;
    use cgmath::prelude::*;
    use truck_modeling::{builder, Vertex, Wire};
    if pts.len() < 3 || fradius_vec.len() != pts.len() {
        return Err(anyhow!("Extrusion 的wire 顶点数量不够，小于3。"));
    }

    //todo 先计算 hull
    // let mut geo_pts = vec![];
    // for pt in pts {
    //     geo_pts.push(coord! { x: pt.x as f64, y: pt.y as f64 });
    // }
    // let res = graham_hull(&mut geo_pts, false);
    // dbg!(&res);
    // assert!(res.is_strictly_ccw_convex());

    let mut pline = Polyline::new_closed();
    // let mut prev_angle = 0.0;

    let mut a1 = vec![];
    let mut a2 = vec![];
    for i in 0..pts.len() {
        let c_pt = pts[i].as_dvec3().truncate();
        let p_pt = pts[(i + pts.len() - 1) % pts.len()].as_dvec3().truncate();
        let n_pt = pts[(i + 1) % pts.len()].as_dvec3().truncate();
        let p_dir = (p_pt - c_pt).normalize();
        let n_dir = (n_pt - c_pt).normalize();
        let angle = p_dir.angle_between(n_dir);
        dbg!(angle.to_degrees());
        if angle.sin().abs() > 0.01 {
            if angle > 0.0 {
                a1.push(i);
            } else {
                a2.push(i);
            }
        }

        pline.add(pts[i].x as f64, pts[i].y as f64, 0.0);
    }
    let has_concave = a1.len() * a2.len() != 0;
    dbg!(has_concave);
    //先只处理只有一种凹的多边形的情况
    let ci = if has_concave {
        if a1.len() < a2.len() {
            a1
        } else {
            a2
        }
    } else {
        vec![]
    };

    // dbg!(&fradius_vec);
    //如果是凹的，需要使用 or 运算
    //如果是凸的，需要使用 not 运算

    dbg!(pline.orientation());
    //这里是有交点，但是凹的边不一定是有交点的
    //有交集的情况需要特殊处理
    let mut concave_pline = Polyline::new_closed();
    // let mut concave_neg_pline = Polyline::new_closed();
    // let pline_as_lines = pline.arcs_to_approx_lines(1e-2).unwrap();
    // let intrs = global_self_intersects(&pline_as_lines, &pline_as_lines.create_approx_aabb_index());
    // // dbg!(&intrs);
    // let mut concave_indexes = intrs
    //     .basic_intersects
    //     .iter()
    //     .map(|x| x.start_index1)
    //     .collect::<BTreeSet<_>>();

    if !ci.is_empty() {
        dbg!(&ci);
        pline.clear();
        for i in 0..pts.len() {
            if !ci.contains(&i) {
                pline.add(pts[i].x as f64, pts[i].y as f64, 0.0);
            }
        }
        let first = (ci[0] - 1 + pts.len()) % pts.len();
        dbg!(first);
        concave_pline.add(pts[first].x as f64, pts[first].y as f64, 0.0);
        for idx in ci.iter() {
            concave_pline.add(pts[*idx].x as f64, pts[*idx].y as f64, 0.0);
        }
        let last = (ci[ci.len() - 1] + 1 + pts.len()) % pts.len();
        dbg!(last);
        concave_pline.add(pts[last].x as f64, pts[last].y as f64, 0.0);
    }
    
    dbg!(&concave_pline);
    // assert_eq!(intrs.basic_intersects.len(), 0);
    // assert_eq!(intrs.overlapping_intersects.len(), 0);

    for (i, &r) in fradius_vec.into_iter().enumerate() {
        let r = r as f64;
        if r.abs() < 0.01 {
            continue;
        }
        let is_concave = ci.contains(&i);
        dbg!(is_concave);
        dbg!(r);
        let c_pt = pts[i].as_dvec3().truncate();
        let p_pt = pts[(i + pts.len() - 1) % pts.len()].as_dvec3().truncate();
        let n_pt = pts[(i + 1) % pts.len()].as_dvec3().truncate();
        //以 c_pt 为圆心， r为半径，算出两个端点
        let p_dir = (p_pt - c_pt).normalize();
        let n_dir = (n_pt - c_pt).normalize();

        let angle = n_dir.angle_between(p_dir);
        dbg!(angle.to_degrees());
        let b_len = r / (angle.abs() / 2.0).tan();
        let p_pt = c_pt + p_dir * b_len;
        let n_pt = c_pt + n_dir * b_len;
        // dbg!((p_dir, n_dir));

        // dbg!((angle / 4.0).tan());
        let bulge = angle.signum() * ((PI as f64 - angle.abs()) / 4.0).tan();
        // let bulge = f64_ceil_2(bulge);
        dbg!(bulge);

        let flag = if n_dir.extend(0.0).cross(p_dir.extend(0.0)).z > 0.0 {
            1.0
        } else {
            -1.0
        };
        //如果是凹的，这里应该会发生变化
        dbg!(flag);

        let mut cut_pline = Polyline::new_closed();
        cut_pline.add(p_pt.x, p_pt.y, 0.0);
        cut_pline.add(c_pt.x, c_pt.y, 0.0);
        cut_pline.add(n_pt.x, n_pt.y, -bulge);
        dbg!(cut_pline.orientation());
        // dbg!(cut_pline.area());
        // dbg!(&cut_pline);
        // cut_pline.arcs_to_approx_lines(error_distance = 0.01);
        let pline_as_lines = cut_pline.arcs_to_approx_lines(1e-1).unwrap();
        dbg!(pline_as_lines.vertex_count());
        // dbg!(pline_as_lines.area());
        // cut_pline.add(n_pt.x, n_pt.y, 0.0);
        // 如果判断目前这里是凹还是凸

        if is_concave {
            dbg!(i);
            let mut result = concave_pline.boolean(&pline_as_lines, BooleanOp::Not);
            if result.pos_plines.len() != 0 {
                dbg!("concave cut  success");
                concave_pline = result.pos_plines.remove(0).pline;
            } else {
                dbg!("concave cut failed");
            }
        } else {
            let mut result = pline.boolean(&pline_as_lines, BooleanOp::Not);
            if result.pos_plines.len() != 0 {
                dbg!("boolean success");
                pline = result.pos_plines.remove(0).pline;
                // break;
            }
        }
    }
    let mut result = pline.boolean(&concave_pline, BooleanOp::Not);
            if result.pos_plines.len() != 0 {
                dbg!("final boolean success");
                pline = result.pos_plines.remove(0).pline;
                // break;
            }
    let mut pline = if let Some(p) = pline.remove_redundant(0.01) {
        p
    } else {
        pline
    };
    // let pline = concave_pline;
    dbg!(pline.orientation());
    let new_polyline = pline.arcs_to_approx_lines(1e-1).unwrap();
    dbg!(new_polyline.vertex_count());
    // dbg!(&pts);
    // dbg!(&fradius_vec);
    let mut wire = Wire::new();

    //将polyline 转换成wire
    let first_vert = builder::vertex(Point3::new(new_polyline[0].x, new_polyline[0].y, 0.0));
    let mut prev_vert = first_vert.clone();
    let count = new_polyline.vertex_count();
    for (index, (i, j)) in new_polyline.iter_segments().into_iter().enumerate() {
        let pt = Point3::new(j.x, j.y, 0.0);
        let vert = if index == count - 1 {
            first_vert.clone()
        } else {
            builder::vertex(pt)
        };
        wire.push_back(builder::line(&prev_vert, &vert));
        prev_vert = vert;
    }
    Ok(wire)
}

///可以使用 cut 的办法
/// 根据顶点信息和fradius半径，生成wire
pub fn gen_wire(
    input_pts: &Vec<Vec3>,
    input_fradius_vec: &Vec<f32>,
) -> anyhow::Result<truck_modeling::Wire> {
    use truck_modeling::{builder, Vertex, Wire};
    if input_pts.len() < 3 || input_fradius_vec.len() != input_pts.len() {
        return Err(anyhow!("Extrusion 的wire 顶点数量不够，小于3。"));
    }
    let t_pts = input_pts
        .into_iter()
        .map(|x| vec3_round_2(*x))
        .collect::<Vec<_>>();
    let mut prev_pt = t_pts[0].truncate();
    let mut deleted = vec![];
    let mut pts = vec![t_pts[0]];
    for i in 1..t_pts.len() {
        if t_pts[i].truncate().distance(prev_pt) < LEN_TOL {
            deleted.push(i);
            continue;
        }
        pts.push(t_pts[i]);
        prev_pt = t_pts[i].truncate();
    }
    let fradius_vec = input_fradius_vec
        .into_iter()
        .enumerate()
        .filter(|(i, _)| !deleted.contains(i))
        .map(|(_, x)| f32_round_2(*x))
        .collect::<Vec<_>>();
    // dbg!(&pts);
    // dbg!(&fradius_vec);
    let mut wire = Wire::new();

    //使用boolean 运算来切割原来的线圈

    let ll = pts.len();
    let mut verts = vec![];
    let mut circle_indexs = vec![];
    for i in 0..ll {
        let fradius = fradius_vec[i];
        let pt = pts[i].point3_without_z();
        //跳过相同的点
        if let Some(last_pt) = verts.last().map(|x: &Vertex| x.point()) {
            if pt.distance(last_pt) < 0.1 {
                continue;
            }
            if i == ll - 1 {
                if pt.distance(verts[0].point()) < 0.1 {
                    continue;
                }
            }
        }
        if abs_diff_eq!(fradius.abs(), 0.0) {
            verts.push(builder::vertex(pt));
        } else {
            let r = fradius;
            let pre_i = (ll + i - 1) % ll;
            let n_i = (i + 1) % ll;
            let pre_pt = pts[pre_i];
            let cur_pt = pts[i % ll];
            let next_pt = pts[n_i];
            let pa_dist = pre_pt.distance(cur_pt);
            let pb_dist = next_pt.distance(cur_pt);
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
            // dbg!(pa_dist);

            // dbg!((pa_dist - b_len));
            // dbg!((pb_dist - b_len));

            // if (pa_dist - b_len < -0.1)  || (pb_dist - b_len) < -0.1{
            //     verts.push(builder::vertex(pt));
            //     continue;
            // }

            if pa_dist - b_len > 0.01 {
                verts.push(builder::vertex(vec3_round_2(p0).point3_without_z()));
            }

            verts.push(builder::vertex(vec3_round_2(transit_pt).point3_without_z()));
            circle_indexs.push(verts.len() - 1);

            if pb_dist - b_len > 0.01 {
                verts.push(builder::vertex(vec3_round_2(p1).point3_without_z()));
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
                }
            }
            j += 1;
        }
    }
    // dbg!(&wire);
    Ok(wire)
}

// if !i.bulge_is_zero() {
// dbg!(i.bulge);
// let h_angle = 2.0 * i.bulge;
// let vec = pt - prev_vert.point();
// // dbg!(&vec);
// let v = vec.normalize();
// let new_v = Basis2::from_angle(Rad(-h_angle)).rotate_vector(v.truncate());
// let s = i.bulge.abs() * vec.magnitude() / 2.0;
// dbg!(s);
// let transit = prev_vert.point() + vec / 2.0 + new_v.extend(0.0) * s;

// // dbg!(prev_vert.point());
// // dbg!(vert.point());

// wire.push_back(builder::circle_arc(
//     &prev_vert,
//     &vert,
//     Point3::new(transit.x, transit.y, 0.0),
// ));
// wire.push_back(builder::line(&prev_vert, &vert));
// }
