use crate::shape::pdms_shape::{BrepMathTrait, LEN_TOL};
use crate::tool::float_tool::*;
use crate::tool::float_tool::{cal_vec2_hash_string, cal_xy_hash_string, vec3_round_2};
use anyhow::anyhow;
use approx::abs_diff_eq;
use cavalier_contours::core::traits::Real;
use cavalier_contours::polyline::internal::pline_intersects::visit_global_self_intersects;
use cavalier_contours::polyline::*;
use cavalier_contours::static_aabb2d_index::StaticAABB2DIndex;
use cgmath::Basis2;
use geo::convex_hull::{graham_hull, quick_hull};
use geo::{Line, LinesIter, Orient, Polygon, Winding};
use geo::{coord, Contains, ConvexHull, IsConvex};
use geo::{line_string, point, Intersects, LineString};
use glam::{DVec2, DVec3, Quat, Vec3};
use nalgebra::ComplexField;
use serde_derive::{Deserialize, Serialize};
use truck_base::cgmath64::{InnerSpace, MetricSpace, Point3, Rad, Vector3};

use cavalier_contours::core::math::bulge_from_angle;
use cavalier_contours::pline_closed;
use cavalier_contours::polyline::internal::pline_boolean::polyline_boolean;
use clap::builder::TypedValueParser;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::f32::consts::PI;

#[cfg(feature = "occ")]
use opencascade::primitives::{Edge, Wire};
use parry2d::bounding_volume::Aabb;
use parry2d::math::Point;
use truck_modeling::builder;

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

pub fn to_debug_json_str(pline: &Polyline) -> String {
    format!(
        r#"
{{
    "isClosed": {},
    "vertexes": [
        {}
    ]
}}
"#,
        pline.is_closed(),
        pline
            .iter_vertexes()
            // .map(|v| format!("[{:.3}, {:.3}, {:.3}]", v.x, v.y, v.bulge))
            .map(|v| format!("[{}, {}, {}]", v.x, v.y, v.bulge))
            .collect::<Vec<_>>()
            .join(",\n        ")
    )
}

//todo 是否需要考虑wind方向
#[inline]
fn gen_fillet_spline(pt: DVec3, d1: DVec3, d2: DVec3, r: f64, sig_num: f64) -> Polyline {
    let mut pline = Polyline::new_closed();
    let angle = d1.angle_between(d2);
    if angle.abs() < 0.001 {
        return pline;
    }
    // dbg!(angle.to_degrees());
    let bulge = f64_trunc_3(bulge_from_angle(PI as f64 - angle)) * sig_num;
    dbg!(bulge);

    let l = r / (angle / 2.0).tan();
    let p0 = pt + d1 * l;
    let p2 = pt + d2 * l;
    // let bulge = 0.3998724443344795;
    pline.add(f64_round(p0.x), f64_round(p0.y), bulge);
    pline.add(f64_round(p2.x), f64_round(p2.y), 0.0);
    pline.add(f64_round(pt.x), f64_round(pt.y), 0.0);
    // dbg!(&pline);
    // #[cfg(debug_assertions)]
    // println!("pline: {}", to_debug_json_str(&pline));
    pline
}

#[inline]
fn add_fillet_spline(pline: &mut Polyline, pt: DVec3, d1: DVec3, d2: DVec3, r: f64) {
    let angle = d1.angle_between(d2);
    let l = r / (angle / 2.0).tan();
    dbg!(l);
    let p0 = pt + d1 * l;
    let p2 = pt + d2 * l;
    let bulge = f64_trunc_3(bulge_from_angle(PI as f64 - angle));
    pline.add(p0.x, p0.y, bulge);
    pline.add(p2.x, p2.y, 0.0);
}

#[test]
fn test_gen_occ_circle() {
    let pts = vec![
        Vec3::ZERO,
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    ];
    let fradius = vec![0.5; 4];
    gen_occ_wire(&pts, &fradius);
}

#[test]
fn test_gen_occ_reverse_circle() {
    let mut pts = vec![
        Vec3::ZERO,
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    ];
    pts.reverse();
    let mut fradius = vec![0.5; 4];
    gen_occ_wire(&pts, &fradius);
}

#[test]
fn test_gen_occ_circle_part() {
    let pts = vec![
        Vec3::ZERO,
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    ];
    let fradius = vec![0.2; 4];
    gen_occ_wire(&pts, &fradius);
}

#[test]
fn test_gen_occ_cut_circle_big_corner_1() {
    let pts = vec![
        Vec3::ZERO,
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    ];
    let fradius = vec![1.0f32, 0.0, 0.0, 0.0];
    gen_occ_wire(&pts, &fradius);
}

#[test]
fn test_gen_occ_cut_circle_big_corner_2() {
    let pts = vec![
        Vec3::ZERO,
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    ];
    let fradius = vec![1.5, 0.0, 0.0, 0.0];
    gen_occ_wire(&pts, &fradius);
}

#[test]
fn test_gen_occ_concave() {
    let pts = vec![
        Vec3::ZERO,
        Vec3::new(0.5, 0.5, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    ];
    let fradius = vec![0.0, 0.25, 0.0, 0.0, 0.0];
    gen_occ_wire(&pts, &fradius);
}

#[test]
fn test_gen_occ_concave_big() {
    let pts = vec![
        Vec3::ZERO,
        Vec3::new(0.5, 0.5, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    ];
    let fradius = vec![0.0, 1.5, 0.0, 0.0, 0.0];
    gen_occ_wire(&pts, &fradius);
}

#[test]
fn test_complex_circe() {
    let parts = vec![
        pline_closed![
            (-621.874, -748.901, 0.0),
            (659.25, -2445.38, 0.0),
            (-122.01, 1733.97, 0.0),
            (-539.001, 268.39, 0.0),
            (0.0, 0.0, 0.792)
        ],
        pline_closed![
            (-621.874, -748.901, 0.792),
            (268.621, -355.708, 0.0),
            (659.25, -2445.38, 0.0)
        ],
    ];
    println!("polyline: {}", to_debug_json_str(&parts[0]));
    println!("polyline: {}", to_debug_json_str(&parts[1]));

    let mut result = parts[0].boolean(&parts[1], BooleanOp::Not);
    if !result.pos_plines.is_empty() {
        dbg!(&result.pos_plines);
        let p = result.pos_plines.remove(0).pline;
        println!("final: {}", to_debug_json_str(&p));
    } else {
        dbg!("cut failed");
    }
}

#[test]
fn test_complex_half_circle() {
    let pts = vec![
        Vec3::new(233.5, 0.0, 0.0),
        Vec3::new(222.0, 233.5, 0.0),
        Vec3::new(-233.5, 233.5, 0.0),
        Vec3::new(-233.5, 0.0, 0.0),
    ];
    let fradius = vec![0.0, 233.5, 233.5, 0.0];
    gen_occ_wire(&pts, &fradius);
    // .expect("test_complex_half_circle failed");
}

#[test]
fn test_complex_half_circle_1() {
    let pts = vec![
        Vec3::new(233.5, 0.0, 0.0),
        Vec3::new(222.0, 233.5, 0.0),
        Vec3::new(-233.5, 233.5, 0.0),
        Vec3::new(-233.5, 0.0, 0.0),
    ];
    let fradius = vec![0.0, 150.0, 150.0, 0.0];
    gen_occ_wire(&pts, &fradius);
    // .expect("test_complex_half_circle failed");
}

#[test]
fn test_complex_1() {
    let tmp_pts = vec![
        [0.000, 0.000, 0.000],
        [0.000, 15337.730, 0.000],
        [-30432.971, 19187.180, 0.000],
        [-34251.980, 4332.510, 0.000],
        [-38584.891, 5526.540, 0.000],
        [-36528.699, 13400.760, 0.000],
        [-29829.340, 18021.330, 0.000],
        [-11801.380, 30455.260, 0.000],
        [631.700, 12426.700, 0.000],
        [4267.570, 7155.040, 0.000],
        [4486.860, 758.430, 0.000],
    ];
    let pts = tmp_pts
        .iter()
        .map(|x| Vec3::new(x[0], x[1], x[2]))
        .collect::<Vec<_>>();
    let fradius = vec![
        0.0, 17400.0, 17400.0, 0.0, 0.0, 21900.0, 0.0, 21900.0, 0.0, 21900.0, 0.0,
    ];
    gen_occ_wire(&pts, &fradius);
    // .expect("test_complex_half_circle failed");
}

#[test]
fn test_complex_2() {
    let tmp_pts = vec![
        [0.0, 0.0, 2765.0],
        [-1903.0, 947.5800170898438, 2765.0],
        [659.25, -2445.3798828125, 2765.0],
        [-122.01000213623047, 1733.969970703125, 2765.0],
        [-1285.550048828125, -2355.469970703125, 2765.0],
        [146.63999938964844, -784.4299926757812, 2765.0],
    ];
    let pts = tmp_pts
        .iter()
        .map(|x| Vec3::new(x[0], x[1], x[2]))
        .collect::<Vec<_>>();
    let fradius = vec![0.0, 500.0, 500.0, 500.0, 500.0, 0.0];
    gen_occ_wire(&pts, &fradius);
    // .expect("test_complex_half_circle failed");
}

#[cfg(feature = "occ")]
pub fn gen_occ_wire(pts: &Vec<Vec3>, fradius_vec: &Vec<f32>) -> anyhow::Result<Wire> {
    if pts.len() < 3 {
        return Err(anyhow!("Extrusion 的wire 顶点数量不够，小于3。"));
    }
    let mut edges = vec![];
    let len = pts.len();
    let pts = pts
        .iter()
        .map(|t| DVec3::new(t.x as f64, t.y as f64, 0.0))
        .collect::<Vec<_>>();
    let aabb = Aabb::from_points(
        &pts.iter()
            .map(|pt| Point::new(pt.x as f32, pt.y as f32))
            .collect::<Vec<_>>(),
    );
    // let aabb_center = aabb.center();
    let has_fradius = fradius_vec.iter().any(|x| *x > 0.0);
    let mut polyline = Polyline::new_closed();

    //如果没有fradius，直接简单的返回wire
    if !has_fradius {
        for i in 0..len {
            let pt = pts[i];
            // edges.push(Edge::segment(pt, next));
            polyline.add(pt.x, pt.y, 0.0);
        }
    } else {
        let mut neg_parts = vec![];

        let mut tmp_polyline = Polyline::new_closed();
        for i in 0..len {
            let pt = pts[i];
            // edges.push(Edge::segment(pt, next));
            tmp_polyline.add(pt.x, pt.y, 0.0);
        }
        let intrs = global_self_intersects(&tmp_polyline, &tmp_polyline.create_approx_aabb_index());
        // dbg!(&intrs.basic_intersects);
        let exclude_indx_set: HashSet<usize> = intrs.basic_intersects.iter().map(|x| x.start_index1+1).collect();
        // dbg!(&exclude_indx_set);
        // dbg!(intrs.overlapping_intersects.len());


        //需要将自相交的先排除在外
        let polyon = Polygon::new(
            LineString::from(pts.iter().enumerate()
                .filter(|(i, _)| !exclude_indx_set.contains(i))
                .map(|(_, p)| (p.x, p.y)).collect::<Vec<_>>()),
            vec![],
        );
        // dbg!(&polyon);
        let hull = polyon.convex_hull();
        // dbg!(hull.exterior());
        let mut pos_equal_eps = 0.0001;
        // dbg!(hull.exterior());
        for i in 0..len {
            //如果当前点有fradius，其实是前一个点的bulge
            let fradius = fradius_vec[i] as f64;
            let last = pts[(i + len - 1) % len];
            let pt = pts[i];
            //如果fradius > 0.0，需要检查wind 方向
            let next = pts[(i + 1) % len];
            let v1 = (pt - last).normalize();
            let v2 = (next - pt).normalize();
            let mut prev_last = pts[(i + len - 2) % len];
            let mut v0 = (last - prev_last).normalize();
            if (1.0 - v0.dot(v1).abs()).abs() < 0.01 {
                prev_last = pts[(i + len - 3) % len];
                // dbg!(prev_last);
                v0 = (last - prev_last).normalize();
            }

            let wind_sig = 1.0;
            let mut is_concave = false;
            let cur_pt = point!(x: pt.x, y: pt.y);
            if !hull.exterior().contains(&cur_pt) {
                //(pt - last)
                is_concave = true;
            }
            #[cfg(debug_assertions)]
            dbg!((i, pt, is_concave, fradius));
            if fradius > 0.0 {
                let counter_clock = v1.cross(v2).z > 0.0;
                let mut counter_sig = if counter_clock { 1.0 } else { -1.0 };
                if is_concave {
                    // add_fillet_spline(&mut polyline, pt, -v1, v2, fradius);
                    let angle = (-v1).angle_between(v2);
                    if angle.abs() < 0.001 {
                        continue;
                    }
                    let l = fradius / (angle / 2.0).tan();
                    let d1 = (pt - last).length();
                    let d2 = (next - pt).length();
                    // dbg!((l, d1, d2));
                    let extent = aabb.extents().magnitude() as f64;
                    if l > extent {
                        dbg!((l, extent));
                        continue;
                    }
                    let p0 = pt + (-v1) * l;
                    let p2 = pt + v2 * l;
                    let bulge =
                        counter_sig * wind_sig * f64_trunc_3(bulge_from_angle(PI as f64 - angle));
                    polyline.add(f64_round(p0.x), f64_round(p0.y), bulge);
                    if (l - d1).abs() > 0.01 {
                        polyline.add(f64_round(p2.x), f64_round(p2.y), 0.0);
                    }
                } else {
                    let neg_part = gen_fillet_spline(pt, -v1, v2, fradius, counter_sig * wind_sig);
                    neg_parts.push(neg_part);
                    polyline.add(f64_round(pt.x), f64_round(pt.y), 0.0);
                }
            } else {
                polyline.add(f64_round(pt.x), f64_round(pt.y), 0.0);
            }
        }
        let mut i = 0;
        if let Some(new_poly) = polyline.remove_repeat_pos(0.05) {
            dbg!("Found duplicate points");
            polyline = new_poly;
        }
        #[cfg(debug_assertions)]
        println!("origin polyline: {}", to_debug_json_str(&polyline));
        for neg in neg_parts {
            #[cfg(debug_assertions)]
            println!(
                "neg {i}: {}, {}",
                to_debug_json_str(&polyline),
                to_debug_json_str(&neg)
            );
            dbg!(pos_equal_eps);
            let mut result = polyline_boolean(
                &polyline,
                &neg,
                BooleanOp::Not,
                &PlineBooleanOptions {
                    pline1_aabb_index: None,
                    pos_equal_eps,
                },
            );
            if result.pos_plines.is_empty() {
                dbg!("cut failed: ", pos_equal_eps);
                pos_equal_eps = 0.0001;
                result = polyline_boolean(
                    &polyline,
                    &neg,
                    BooleanOp::Not,
                    &PlineBooleanOptions {
                        pline1_aabb_index: None,
                        pos_equal_eps,
                    },
                );
            }
            // dbg!(result.pos_plines.len());
            // dbg!(&result.result_info);
            if !result.pos_plines.is_empty() {
                polyline = result.pos_plines.remove(0).pline;
                // for p in &mut polyline.vertex_data{
                //     p.x = f64_round(p.x);
                //     p.y = f64_round(p.y);
                //     p.bulge = f64_round(p.bulge);
                // }
                // println!("{i}: {}", to_debug_json_str(&polyline));
            } else {
                dbg!("cut failed");
            }
            i += 1;
        }
    }

    if let Some(new_poly) = polyline.remove_repeat_pos(0.05) {
        polyline = new_poly;
    }
    #[cfg(debug_assertions)]
    println!("boolean: {}", to_debug_json_str(&polyline));

    for (p, q) in polyline.iter_segments() {
        if p.bulge.abs() < 0.0001 {
            edges.push(Edge::segment(
                DVec3::new(p.x, p.y, 0.0),
                DVec3::new(q.x, q.y, 0.0),
            ));
        } else {
            let m = seg_midpoint(p, q);
            // dbg!((p,m,q));
            edges.push(Edge::arc(
                DVec3::new(p.x, p.y, 0.0),
                DVec3::new(m.x, m.y, 0.0),
                DVec3::new(q.x, q.y, 0.0),
            ));
        }
    }

    Ok(Wire::from_edges(&edges))
}

#[test]
fn test_convex_hull() {
    //去除有问题的点，在凸包里面的点需要排除

    let poly = Polygon::new(
        LineString::from(vec![
            (0.0, 0.0),
            (5.05, -174.83),
            (-133.47, 146.39),
            (113.89, -100.97),
            (-207.33, 37.55),
            (-32.5, 32.5),
        ]),
        vec![],
    );

    let hull = poly.convex_hull();

    dbg!(hull.interiors());
    dbg!(hull.exterior());
}

pub fn check_wire_ok(pts: &Vec<Vec3>, fradius_vec: &Vec<f32>) -> bool {
    let mut polyline = Polyline::new_closed();
    for i in 0..pts.len() {
        let c_pt = pts[i].as_dvec3();
        polyline.add(c_pt.x, c_pt.y, 0.0.into());
    }
    let intrs = global_self_intersects(&polyline, &polyline.create_approx_aabb_index());
    // dbg!(intrs.basic_intersects.len());
    // dbg!(intrs.overlapping_intersects.len());

    intrs.basic_intersects.len() == 0 && intrs.overlapping_intersects.len() == 0
}

///生成occ的wire
#[cfg(feature = "occ")]
pub fn gen_occ_wire_old(pts: &Vec<Vec3>, fradius_vec: &Vec<f32>) -> anyhow::Result<Wire> {
    if pts.len() < 3 {
        return Err(anyhow!("wire 顶点数量不够，小于3。"));
    }

    // let new_pts = remove_wrong_vertices(pts, fradius_vec);
    // dbg!(&new_pts);
    if !check_wire_ok(pts, fradius_vec) {
        return Err(anyhow!("wire 有自相交。"));
    }

    let mut edges = vec![];
    let ll = pts.len();
    let mut verts: Vec<DVec3> = vec![];
    let mut pre_pt = pts[0].as_dvec3();
    let mut circle_indexs = vec![];
    for i in 0..ll {
        let fradius = fradius_vec[i];
        let pt = pts[i].truncate().extend(0.0).as_dvec3();
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
            pre_pt = pts[i].as_dvec3();
        } else {
            let r = fradius as f64;
            let pre_i = (ll + i - 1) % ll;
            let n_i = (i + 1) % ll;
            let pre_pt = pts[pre_i].as_dvec3();
            let cur_pt = pts[i % ll].as_dvec3();
            let next_pt = pts[n_i].as_dvec3();
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
            // dbg!((pa_dist, b_len));
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
            #[cfg(debug_assertions)]
            {
                dbg!(pts);
                dbg!(fradius_vec);
            }
            return Err(anyhow!(" verts are empty"));
        }
        let mut pre_vert = verts[0].clone();
        j = 1;
        while j <= v_len {
            let cur_vert = verts[j % v_len];
            if pre_vert.distance(cur_vert) > 1.0 {
                if circle_indexs.len() > 0 && j == circle_indexs[0] {
                    let next_vert = verts[(j + 1) % v_len];
                    edges.push(Edge::arc(pre_vert, cur_vert, next_vert));
                    // dbg!((pre_vert,
                    //     cur_vert,
                    //     next_vert));
                    pre_vert = next_vert.clone();
                    circle_indexs.remove(0);
                    j += 1;
                } else {
                    // dbg!((pre_vert, cur_vert));
                    edges.push(Edge::segment(pre_vert, cur_vert));
                    pre_vert = cur_vert.clone();
                }
            }
            j += 1;
        }
    }
    Ok(Wire::from_edges(&edges))
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

#[test]
fn test_gen_circle() {
    let mut pline = pline_closed!(
        (0.5, 0.0, 0.0),
        (1.0, 0.5, 0.0),
        (0.5, 1.0, 0.0),
        (0.0, 0.5, 0.0)
    );

    let arc_cut1 = pline_closed!((0.0, 0.5, 0.0), (0.25, 0.25, -0.4142135), (0.25, 0.75, 0.0));
    let arc_cut2 = pline_closed!((0.25, 0.25, 0.0), (0.5, 0.0, 0.0), (0.75, 0.25, -0.4142135));
    let arc_cut3 = pline_closed![(0.75, 0.25, 0.0), (1.0, 0.5, 0.0), (0.75, 0.75, -0.4142135)];

    let arc_cut4 = pline_closed![(0.75, 0.75, 0.0), (0.5, 1.0, 0.0), (0.25, 0.75, -0.4142135)];

    let mut cuts = vec![arc_cut1, arc_cut2, arc_cut3, arc_cut4];
    for cut in cuts {
        let mut result = pline.boolean(&cut, BooleanOp::Not);
        if !result.pos_plines.is_empty() {
            dbg!(&result.pos_plines);
            pline = result.pos_plines.remove(0).pline;
        } else {
            dbg!("cut failed");
        }
    }
}

#[test]
fn test_concave_circle() {
    let mut pline = pline_closed!(
        (0.5, 0.0, 0.0),
        (0.5, 0.5, 0.0),
        (1.0, 0.5, 0.0),
        (0.5, 1.0, 0.0),
        (0.0, 0.5, 0.0)
    );

    let arc_cut1 = pline_closed!((0.0, 0.5, 0.0), (0.25, 0.25, -0.4142135), (0.25, 0.75, 0.0));
    let arc_cut2 = pline_closed!((0.25, 0.25, 0.0), (0.5, 0.0, 0.0), (0.75, 0.25, -0.4142135));
    let arc_cut3 = pline_closed![(0.75, 0.25, 0.0), (1.0, 0.5, 0.0), (0.75, 0.75, -0.4142135)];

    let arc_cut4 = pline_closed![(0.75, 0.75, 0.0), (0.5, 1.0, 0.0), (0.25, 0.75, -0.4142135)];

    let mut cuts = vec![arc_cut1, arc_cut2, arc_cut3, arc_cut4];
    for cut in cuts {
        let mut result = pline.boolean(&cut, BooleanOp::Not);
        if !result.pos_plines.is_empty() {
            dbg!(&result.pos_plines);
            pline = result.pos_plines.remove(0).pline;
        } else {
            dbg!("cut failed");
        }
    }
}

///可以使用 cut 的办法
/// 根据顶点信息和fradius半径，生成wire
pub fn gen_wire(
    input_pts: &Vec<Vec3>,
    input_fradius_vec: &Vec<f32>,
) -> anyhow::Result<truck_modeling::Wire> {
    use truck_modeling::{builder, Vertex, Wire};
    if input_pts.len() < 3 || input_fradius_vec.len() != input_pts.len() {
        return Err(anyhow!("wire 顶点数量不够，小于3。"));
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

#[test]
pub fn test_check_wire_25688_45293() {
    let data = vec![
        [0.0, 0.0, 480.0],
        [4.46, -173.52, 480.0],
        [-132.5, 145.48, 480.0],
        [112.98, -100.0, 480.0],
        [-206.02, 36.96, 480.0],
        [-32.5, 32.5, 480.0],
    ];
    let pts: Vec<Vec3> = data
        .iter()
        .map(|x| Vec3::new(x[0], x[1], x[2]))
        .collect::<Vec<_>>();
    let fradius_vec = vec![0.0, 33.37, 33.37, 33.37, 33.37, 0.0];

    assert_eq!(check_wire_ok(&pts, &fradius_vec), false);
}

#[test]
pub fn test_check_wire_25688_45261() {
    let data = vec![
        [-23350, 0, 0],
        [-22200, 23350, 0],
        [23350, 23350, 0],
        [23350, 0, 0],
    ];
    let pts: Vec<Vec3> = data
        .iter()
        .map(|x| Vec3::new(x[0] as f32, x[1] as f32, x[2] as f32))
        .collect::<Vec<_>>();
    let fradius_vec = vec![0.0, 23350.0, 23350.0, 0.0];

    assert_eq!(check_wire_ok(&pts, &fradius_vec), true);
    gen_occ_wire(&pts, &fradius_vec).unwrap();
}

#[test]
pub fn test_check_wire_25688_72092() {
    let data = vec![
        [0.0, 0.0, 0.0],
        [0.0, 8188.92, 0.0],
        [-12620.42, 18627.24, 0.0],
        [-20663.97, 17091.12, 0.0],
        [-22737.08, 22684.93, 0.0],
        [7196.01, 29736.53, 0.0],
        [5884.46, -987.96, 0.0],
    ];
    let pts: Vec<Vec3> = data
        .iter()
        .map(|x| Vec3::new(x[0] as f32, x[1] as f32, x[2] as f32))
        .collect::<Vec<_>>();
    let fradius_vec = vec![0.0, 17400.0, 17400.0, 0.0, 0.0, 23300.0, 0.0];

    assert_eq!(check_wire_ok(&pts, &fradius_vec), true);
    gen_occ_wire(&pts, &fradius_vec).unwrap();
}

#[test]
pub fn test_check_wire_17496_254047() {
    let data = vec![
        [31500.0, 79700.0, 0.0],
        [31500.0, 84300.0, 0.0],
        [62600.0, 84300.0, 0.0],
        [62600.0, 42457.41015625, 0.0],
        [62600.01171875, 42457.3984375, 0.0],
        [42696.78125, 50942.25, 0.0],
        [19471.44921875, 14430.48046875, 0.0],
        [34918.640625, 37374.4296875, 0.0],
        [31500.0, 41040.46875, 0.0],
    ];
    let pts: Vec<Vec3> = data
        .iter()
        .map(|x| Vec3::new(x[0] as f32, x[1] as f32, x[2] as f32))
        .collect::<Vec<_>>();
    let fradius_vec = vec![0.0, 0.0, 0.0, 0.0, 0.0, 25500.0, 25500.0, 0.0, 0.0];

    gen_occ_wire(&pts, &fradius_vec).unwrap();
}
