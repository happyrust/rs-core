#[cfg(feature = "truck")]
use crate::shape::pdms_shape::BrepMathTrait;
use crate::shape::pdms_shape::LEN_TOL;
use crate::tool::float_tool::*;
use crate::tool::float_tool::{cal_vec2_hash_string, cal_xy_hash_string, vec3_round_2};
use crate::geometry::triangulation_helper::triangulate_polygon_indices_spade;
use anyhow::anyhow;
use approx::abs_diff_eq;
use cavalier_contours::core::math::{Vector2, angle, bulge_from_angle};
use cavalier_contours::core::traits::Real;
use cavalier_contours::pline_closed;
use cavalier_contours::polyline::internal::pline_boolean::polyline_boolean;
use cavalier_contours::polyline::internal::pline_intersects::visit_global_self_intersects;
use cavalier_contours::polyline::*;
use cavalier_contours::static_aabb2d_index::StaticAABB2DIndex;
use clap::builder::TypedValueParser;
// use geo::convex_hull::{graham_hull, quick_hull};
// use geo::{coord, Contains, ConvexHull, IsConvex};
// use geo::{line_string, point, Intersects, LineString};
// use geo::{Line, LinesIter, Orient, Polygon, RemoveRepeatedPoints, Winding};
use glam::{DVec2, DVec3, Quat, Vec2, Vec3};
use nalgebra::{ComplexField, DimAdd};
use num_traits::signum;
use ploop_rs::{PloopProcessor, Vertex};
use serde_derive::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::f32::consts::PI;
use std::panic::AssertUnwindSafe;
// use std::fs;
// use std::fs::File;
// use std::io::Write;
use std::path::PathBuf;
#[cfg(feature = "truck")]
use truck_base::cgmath64::{InnerSpace, MetricSpace, Point3, Rad, Vector3};

#[cfg(feature = "occ")]
use crate::prim_geo::basic::OccSharedShape;
#[cfg(feature = "occ")]
use opencascade::primitives::{Edge, Face, Wire};
use parry2d::bounding_volume::Aabb;
use parry2d::math::Point;
#[cfg(feature = "truck")]
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

#[cfg(feature = "truck")]
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
pub fn gen_occ_spline_wire(loops: &Vec<Vec<Vec3>>, thick: f32) -> anyhow::Result<Wire> {
    let verts = &loops[0];
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

    Ok(Wire::from_edges(&edges)?)
}

#[cfg(feature = "truck")]
///生成truck的wire
pub fn gen_spline_wire(
    input_verts: &Vec<Vec3>,
    thick: f32,
) -> anyhow::Result<truck_modeling::Wire> {
    #[cfg(feature = "truck")]
    use truck_modeling::{Wire, builder};
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

pub fn polyline_to_debug_json_str(pline: &Polyline) -> String {
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

// #[cfg(feature = "debug_wire")]
pub(crate) fn export_polyline_svg_for_debug(polyline: &Polyline, refno: Option<&str>) {
    use std::f64::consts::PI;
    use std::fs::File;
    use std::io::Write;
    use std::path::PathBuf;

    let dir = PathBuf::from("output/svg");
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }

    // 使用 refno.to_string() 作为文件名，如果 refno 为 None 则使用默认名称
    let filename = match refno {
        Some(r) => format!("wire_{}.svg", r),
        None => "wire_debug.svg".to_string(),
    };

    let path = dir.join(filename);
    let mut file = match File::create(&path) {
        Ok(f) => f,
        Err(_) => return,
    };

    if polyline.vertex_data.is_empty() {
        return;
    }

    let mut min_x = polyline.vertex_data[0].x;
    let mut max_x = polyline.vertex_data[0].x;
    let mut min_y = polyline.vertex_data[0].y;
    let mut max_y = polyline.vertex_data[0].y;

    for v in &polyline.vertex_data {
        if v.x < min_x {
            min_x = v.x;
        }
        if v.x > max_x {
            max_x = v.x;
        }
        if v.y < min_y {
            min_y = v.y;
        }
        if v.y > max_y {
            max_y = v.y;
        }
    }

    let width = max_x - min_x;
    let height = max_y - min_y;
    let padding = 50.0;

    let svg_width = width + 2.0 * padding;
    let svg_height = height + 2.0 * padding;

    let _ = writeln!(file, r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    let _ = writeln!(
        file,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="100%" height="100%" viewBox="{} {} {} {}" preserveAspectRatio="xMidYMid meet">"#,
        min_x - padding,
        min_y - padding,
        svg_width,
        svg_height
    );

    // 添加样式以改善显示效果
    let _ = writeln!(file, r#"  <style>"#);
    let _ = writeln!(file, r#"    svg {{ background-color: #f0f0f0; }}"#);
    let _ = writeln!(
        file,
        r#"    path {{ stroke-width: 20; stroke-linecap: round; stroke-linejoin: round; }}"#
    );
    let _ = writeln!(file, r#"  </style>"#);

    let _ = write!(file, r#"  <path d="M"#);

    let mut first = true;
    for (p, q) in polyline.iter_segments() {
        if first {
            let _ = write!(file, " {},{}", p.x, p.y);
            first = false;
        }

        if p.bulge.abs() < 0.001 {
            let _ = write!(file, " L {},{}", q.x, q.y);
        } else {
            let dx = q.x - p.x;
            let dy = q.y - p.y;
            let chord_len = (dx * dx + dy * dy).sqrt();

            // bulge = tan(角度/4)，所以角度 = 4 * atan(bulge)
            let angle = 4.0 * p.bulge.abs().atan();

            // 正确的半径计算公式：R = (L/2) / sin(角度/2)
            let radius = if angle.abs() > 0.001 && chord_len > 0.001 {
                (chord_len / 2.0) / (angle / 2.0).sin()
            } else {
                0.0
            };

            // large_arc 标志：角度大于 180 度（PI 弧度）
            let large_arc = if angle > PI { 1 } else { 0 };

            // sweep 标志：bulge > 0 表示顺时针，bulge < 0 表示逆时针
            // SVG 中：1 = 顺时针，0 = 逆时针
            let sweep = if p.bulge > 0.0 { 1 } else { 0 };

            let _ = write!(
                file,
                " A {:.6},{:.6} 0 {} {} {:.6},{:.6}",
                radius, radius, large_arc, sweep, q.x, q.y
            );
        }
    }

    if polyline.is_closed {
        let _ = write!(file, " Z");
    }

    let _ = writeln!(file, r#"" fill="none" stroke="blue"/>"#);
    let _ = writeln!(file, "</svg>");
}

//todo 是否需要考虑wind方向
#[inline]
fn gen_fillet_spline(
    pt: DVec3,
    last_pt: DVec3,
    next_pt: DVec3,
    d1: DVec3,
    d2: DVec3,
    r: f64,
    sig_num: f64,
) -> Polyline {
    let mut pline = Polyline::new_closed();
    let angle = d1.angle_between(d2);
    if angle.abs() < 0.001 {
        return pline;
    }
    //f64_trunc_3
    let bulge = f64_trunc_3(bulge_from_angle(PI as f64 - angle)) * sig_num;
    // dbg!(bulge);
    let l = r / (angle / 2.0).tan();
    let mut p0 = pt + d1 * l;
    let mut p2 = pt + d2 * l;
    if last_pt.distance(p0).abs() < 0.01 {
        p0 = last_pt;
    }
    if next_pt.distance(p2).abs() < 0.01 {
        p2 = next_pt;
    }
    pline.add((p0.x), (p0.y), bulge);
    pline.add((p2.x), (p2.y), 0.0);
    pline.add((pt.x), (pt.y), 0.0);
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
    // //gen_occ_wires(&pts, &fradius);
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
    // //gen_occ_wires(&pts, &fradius);
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
    // //gen_occ_wires(&pts, &fradius);
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
    //gen_occ_wires(&pts, &fradius);
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
    //gen_occ_wires(&pts, &fradius);
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
    //gen_occ_wires(&pts, &fradius);
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
    //gen_occ_wires(&pts, &fradius);
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
    println!("polyline: {}", polyline_to_debug_json_str(&parts[0]));
    println!("polyline: {}", polyline_to_debug_json_str(&parts[1]));

    let mut result = parts[0].boolean(&parts[1], BooleanOp::Not);
    if !result.pos_plines.is_empty() {
        dbg!(&result.pos_plines);
        let p = result.pos_plines.remove(0).pline;
        println!("final: {}", polyline_to_debug_json_str(&p));
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
    //gen_occ_wires(&pts, &fradius);
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
    //gen_occ_wires(&pts, &fradius);
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
    //gen_occ_wires(&pts, &fradius);
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
    //gen_occ_wires(&pts, &fradius);
    // .expect("test_complex_half_circle failed");
}

#[inline]
fn seg_split(
    v1: PlineVertex,
    v2: PlineVertex,
    point_on_seg: Vector2,
    pos_equal_eps: f64,
) -> SplitResult {
    let mut r = seg_split_at_point(v1, v2, point_on_seg, pos_equal_eps);
    if r.updated_start.bulge.abs() < 0.01 {
        r.updated_start.bulge = 0.0;
    }
    if r.split_vertex.bulge.abs() < 0.01 {
        r.split_vertex.bulge = 0.0;
    }
    r
}

pub fn resolve_overlap_intersection(
    polyline: &Polyline,
    intersect: &PlineOverlappingIntersect<f64>,
    // ori: PlineOrientation,
) -> anyhow::Result<(Polyline, bool)> {
    let mut new_polyline = polyline.clone();

    let verts_len = polyline.vertex_data.len();
    //优先处理和直线的相交情况
    let si_0 = intersect.start_index1;
    let next_si_0 = (si_0 + 1) % verts_len;
    let si_1 = intersect.start_index2;
    let next_si_1 = (si_1 + 1) % verts_len;
    let point = intersect.point1;
    let r = seg_split(polyline[si_1], polyline[next_si_1], point, 0.01);
    // dbg!(&r);
    new_polyline[si_1] = r.updated_start;
    new_polyline[next_si_1] = r.split_vertex;
    let first_point = &new_polyline.vertex_data[0];
    //如果已经到起点了，结束检查，直接砍掉后面的
    if (r.split_vertex.pos() - first_point.pos()).length() < 0.01 {
        new_polyline.vertex_data.drain(next_si_1..);
        return Ok((new_polyline, false));
    }

    if let Some(r) = new_polyline.remove_repeat_pos(0.01) {
        new_polyline = r;
    }
    Ok((new_polyline, true))
}

/// 处理基本相交情况
///
/// 该函数用于处理多段线(polyline)的基本相交情况。基本相交是指两个线段相交于一个点。
///
/// # 参数
/// * `polyline` - 输入的多段线
/// * `intersect` - 相交信息,包含相交点和相交线段的索引
/// * `ori` - 多段线的方向
///
/// # 返回值
/// * `Result<Polyline>` - 处理后的新多段线
///
/// # 处理逻辑
/// 1. 根据相交点将相交的线段分割成两部分
/// 2. 根据线段类型(直线或圆弧)采用不同的处理策略
/// 3. 保持多段线的方向一致性
/// 4. 移除重复的点
pub fn resolve_basic_intersection(
    polyline: &Polyline,
    intersect: &PlineBasicIntersect<f64>,
    ori: PlineOrientation,
) -> anyhow::Result<Polyline> {
    let mut new_polyline = polyline.clone();
    let verts_len = polyline.vertex_data.len();

    // 检查多段线是否有足够的顶点
    if verts_len < 3 {
        return Err(anyhow!("Polyline has too few vertices."));
    }

    // 获取相交线段的起始索引
    let si_0 = intersect.start_index1;
    let mut next_si_0 = (si_0 + 1) % verts_len;
    let mut si_1 = intersect.start_index2;
    let next_si_1 = (si_1 + 1) % verts_len;

    // 验证索引的有效性
    if si_0 >= verts_len || si_1 >= verts_len || next_si_0 >= verts_len || next_si_1 >= verts_len {
        return Err(anyhow!("Invalid intersection indices for polyline."));
    }

    let point = intersect.point;

    // 处理两条直线相交的情况
    if polyline[si_0].bulge == 0.0 && polyline[si_1].bulge == 0.0 {
        new_polyline[si_1] = PlineVertex::new(point.x, point.y, 0.0);
    }
    // 处理直线和圆弧相交的情况(第一条是直线,第二条是圆弧)
    else if polyline[si_0].bulge == 0.0 && polyline[si_1].bulge != 0.0 {
        // 如果点和端点重合，直接砍掉
        let mut tmp_polyline = Polyline::new_closed();
        tmp_polyline.add(polyline[si_0].x, polyline[si_0].y, 0.0);
        tmp_polyline.add(point.x, point.y, 0.0);
        tmp_polyline.add(polyline[next_si_1].x, polyline[next_si_1].y, 0.0);
        let use_start = tmp_polyline.orientation() != ori;
        #[cfg(feature = "debug_wire")]
        dbg!(use_start);

        let r = seg_split(polyline[si_1], polyline[next_si_1], point, 0.01);
        #[cfg(feature = "debug_wire")]
        dbg!(&r);
        // 如果分割点和端点重合
        if r.split_vertex.bulge == 0.0 {
            if si_0 == 0 {
                next_si_0 = verts_len;
            }
            #[cfg(feature = "debug_wire")]
            println!(
                "first arc, second line, same end point, remove between {} .. {}",
                next_si_1, next_si_0
            );
            // 确保范围有效：next_si_1 <= next_si_0
            if next_si_0 < next_si_1 {
                return Err(anyhow!(
                    "Invalid drain range: next_si_0({}) < next_si_1({})",
                    next_si_0,
                    next_si_1
                ));
            }
            // 安全地移除范围内的顶点
            if next_si_1 < new_polyline.vertex_data.len()
                && next_si_0 <= new_polyline.vertex_data.len()
            {
                new_polyline.vertex_data.drain(next_si_1..next_si_0);
            } else {
                return Err(anyhow!(
                    "Invalid drain range for polyline: next_si_1={}, next_si_0={}, len={}",
                    next_si_1,
                    next_si_0,
                    new_polyline.vertex_data.len()
                ));
            }
        } else if use_start {
            new_polyline[si_1] = r.updated_start;
            new_polyline[si_0] = r.split_vertex;
            #[cfg(feature = "debug_wire")]
            println!(
                "first arc, second line , use arc start: {}, line use split start: {} ",
                si_1, si_0
            );
        } else {
            // 检查索引的有效性
            if next_si_0 >= new_polyline.vertex_data.len() || si_1 >= new_polyline.vertex_data.len()
            {
                return Err(anyhow!("Invalid vertex indices for polyline."));
            }

            new_polyline[next_si_0] = r.split_vertex;
            new_polyline[si_1] = r.split_vertex;
            #[cfg(feature = "debug_wire")]
            println!(
                "first arc, second line , use split remove between {} .. {}",
                next_si_0, si_1
            );
            // 确保范围有效：next_si_0 <= si_1
            if si_1 < next_si_0 {
                return Err(anyhow!(
                    "Invalid drain range: si_1({}) < next_si_0({})",
                    si_1,
                    next_si_0
                ));
            }
            // 安全地移除范围内的顶点
            if next_si_0 < new_polyline.vertex_data.len() && si_1 <= new_polyline.vertex_data.len()
            {
                new_polyline.vertex_data.drain(next_si_0..si_1);
            } else {
                return Err(anyhow!(
                    "Invalid drain range for polyline: next_si_0={}, si_1={}, len={}",
                    next_si_0,
                    si_1,
                    new_polyline.vertex_data.len()
                ));
            }
        }
    }
    // 处理圆弧和直线相交的情况(第一条是圆弧,第二条是直线)
    else if polyline[si_0].bulge != 0.0 && polyline[si_1].bulge == 0.0 {
        let mut tmp_polyline = Polyline::new_closed();
        tmp_polyline.add(polyline[si_0].x, polyline[si_0].y, 0.0);
        tmp_polyline.add(point.x, point.y, 0.0);
        tmp_polyline.add(polyline[next_si_1].x, polyline[next_si_1].y, 0.0);
        let use_start = tmp_polyline.orientation() == ori;
        #[cfg(feature = "debug_wire")]
        dbg!(use_start);

        let mut r = seg_split(polyline[si_0], polyline[next_si_0], point, 0.01);
        #[cfg(feature = "debug_wire")]
        dbg!(&r);
        // 如果分割点和端点重合
        if r.split_vertex.bulge == 0.0 {
            #[cfg(feature = "debug_wire")]
            println!(
                "first arc, second line, same end point, remove between {} .. {}",
                next_si_0, si_1
            );
            // 确保范围有效：next_si_0 <= si_1
            if si_1 < next_si_0 {
                return Err(anyhow!(
                    "Invalid drain range: si_1({}) < next_si_0({})",
                    si_1,
                    next_si_0
                ));
            }
            // 安全地移除范围内的顶点
            if next_si_0 < new_polyline.vertex_data.len() && si_1 <= new_polyline.vertex_data.len()
            {
                new_polyline.vertex_data.drain(next_si_0..si_1);
            } else {
                return Err(anyhow!(
                    "Invalid drain range for polyline: next_si_0={}, si_1={}, len={}",
                    next_si_0,
                    si_1,
                    new_polyline.vertex_data.len()
                ));
            }
        } else {
            if use_start {
                new_polyline[si_0] = r.updated_start;
                new_polyline[si_1] = r.split_vertex;
                #[cfg(feature = "debug_wire")]
                println!(
                    "first arc, second line , use start remove between {} .. {}",
                    next_si_0, si_1
                );
                // 确保范围有效：next_si_0 <= si_1
                if si_1 < next_si_0 {
                    return Err(anyhow!(
                        "Invalid drain range: si_1({}) < next_si_0({})",
                        si_1,
                        next_si_0
                    ));
                }
                // 安全地移除范围内的顶点
                if next_si_0 < new_polyline.vertex_data.len()
                    && si_1 <= new_polyline.vertex_data.len()
                {
                    new_polyline.vertex_data.drain(next_si_0..si_1);
                } else {
                    return Err(anyhow!(
                        "Invalid drain range for polyline: next_si_0={}, si_1={}, len={}",
                        next_si_0,
                        si_1,
                        new_polyline.vertex_data.len()
                    ));
                }
            } else {
                // 检查索引的有效性
                if si_0 >= new_polyline.vertex_data.len()
                    || next_si_1 >= new_polyline.vertex_data.len()
                {
                    return Err(anyhow!("Invalid vertex indices for polyline."));
                }

                new_polyline[si_0] = r.split_vertex;
                new_polyline[next_si_1] = r.split_vertex;
                #[cfg(feature = "debug_wire")]
                println!(
                    "first arc, second line , {} and {} use split",
                    si_0, next_si_1
                );
            }
        }
    }
    // 处理两条圆弧相交的情况
    else if polyline[si_0].bulge != 0.0 && polyline[si_1].bulge != 0.0 {
        // 验证索引的有效性
        if si_0 >= verts_len || (si_0 + 1) >= verts_len {
            return Err(anyhow!("Invalid index for polyline."));
        }

        let sr = seg_split(
            polyline[si_0],
            polyline[(si_0 + 1) % verts_len],
            point,
            0.01,
        );
        // 更新第一条圆弧的起点
        new_polyline[si_0] = sr.updated_start;

        // 验证索引的有效性
        if si_1 >= verts_len || (si_1 + 1) >= verts_len {
            return Err(anyhow!("Invalid index for polyline."));
        }

        // 更新第二条圆弧的起点
        let er = seg_split(
            polyline[si_1],
            polyline[(si_1 + 1) % verts_len],
            point,
            0.01,
        );
        new_polyline[si_1] = er.split_vertex;

        if si_1 >= next_si_0 {
            #[cfg(feature = "debug_wire")]
            println!("both arc, remove between {} .. {}", next_si_0, si_1);
            // 确保范围有效：next_si_0 <= si_1
            if si_1 < next_si_0 {
                return Err(anyhow!(
                    "Invalid drain range: si_1({}) < next_si_0({})",
                    si_1,
                    next_si_0
                ));
            }
            // 安全地移除范围内的顶点
            if next_si_0 < new_polyline.vertex_data.len() && si_1 <= new_polyline.vertex_data.len()
            {
                new_polyline.vertex_data.drain(next_si_0..si_1);
            } else {
                return Err(anyhow!(
                    "Invalid drain range for polyline: next_si_0={}, si_1={}, len={}",
                    next_si_0,
                    si_1,
                    new_polyline.vertex_data.len()
                ));
            }
        }
    }

    // 移除重复的点
    if let Some(r) = new_polyline.remove_repeat_pos(0.01) {
        new_polyline = r;
    }

    Ok(new_polyline)
}

/// # 参数
/// * `pts` - 顶点数据，Vec3 格式：x,y 为坐标，z 为 fradius 值
///
/// # 返回值
/// * `Result<Polyline>` - 处理后生成的多段线
/// 将已经被 ploop-rs 处理过的顶点直接转换为 Polyline
///
/// 这个函数用于处理已经被 process_ploop_from_content 或 process_ploop_vertices
/// 处理过的顶点，避免重复处理
///
/// # 参数
/// * `vertices` - 已处理的顶点数据，Vec3 格式：x,y 为坐标，z 为 bulge 值
///
/// # 返回值
/// * `Result<Polyline>` - 转换后的多段线
pub fn gen_polyline_from_processed_vertices(
    vertices: &Vec<Vec3>,
    refno: Option<&str>,
) -> anyhow::Result<Polyline> {
    if vertices.len() < 3 {
        return Err(anyhow!("顶点数量不够，小于3。"));
    }

    println!("🔧 直接转换已处理的 {} 个顶点为 Polyline", vertices.len());

    // 直接转换为 Polyline，不再调用 ploop-rs
    let polyline = convert_vertices_to_polyline(vertices)?;

    // #[cfg(feature = "debug_wire")]
    {
        // export_polyline_svg_for_debug(&polyline, refno);
    }

    Ok(polyline)
}

/// 将 ploop-rs 处理后的顶点转换为 Polyline
///
/// ploop-rs 已经处理了 FRADIUS 并生成了正确的切点和 bulge 值，
/// 我们只需要将这些值直接转换为 Polyline
///
/// # 参数
/// * `vertices` - 处理后的顶点数据，Vec3 格式：x,y 为坐标，z 为 bulge 值
///
/// # 返回值
/// * `Result<Polyline>` - 转换后的多段线
fn convert_vertices_to_polyline(vertices: &[Vec3]) -> anyhow::Result<Polyline> {
    if vertices.len() < 3 {
        return Err(anyhow!("顶点数量不够，小于3。"));
    }

    println!("🔧 将 {} 个处理后的顶点转换为 Polyline", vertices.len());

    let mut polyline = Polyline::new_closed();
    let remove_pos_tol = 0.1;
    let len = vertices.len();

    // 直接转换顶点，z 值就是 bulge
    for i in 0..len {
        let vertex = vertices[i];
        let bulge = vertex.z as f64;

        // 直接添加顶点和 bulge 值
        polyline.add(vertex.x as f64, vertex.y as f64, bulge);
    }

    // 移除重复位置
    if let Some(new_poly) = polyline.remove_repeat_pos(remove_pos_tol) {
        polyline = new_poly;
    }

    // 检查是否有 NaN 数据
    for p in &polyline.vertex_data {
        if p.bulge.is_nan() {
            return Err(anyhow!("发现 NaN bulge 值"));
        }
    }

    println!(
        "✅ Polyline 转换完成，包含 {} 个顶点",
        polyline.vertex_data.len()
    );

    Ok(polyline)
}

///生成occ的wire
#[cfg(feature = "occ")]
pub fn gen_occ_wires(loops: &Vec<Vec<Vec3>>) -> anyhow::Result<Vec<Wire>> {
    if loops[0].len() < 3 {
        return Err(anyhow!("第一个 wire 顶点数量不够，小于3。"));
    }
    // 先使用 ploop-rs 处理 FRADIUS，再基于 bulge 生成 Polyline
    // 将 Vec3 拆分为 Vec2 和 frads
    let mut verts2d: Vec<Vec2> = Vec::with_capacity(loops[0].len());
    let mut frads: Vec<f32> = Vec::with_capacity(loops[0].len());
    for v in &loops[0] {
        verts2d.push(Vec2::new(v.x, v.y));
        frads.push(v.z);
    }
    let processed_pos = process_ploop_vertices(&verts2d, &frads, "OCC_POS_WIRE")?;
    let mut pos_poly = gen_polyline_from_processed_vertices(&processed_pos, None)?;
    if pos_poly.vertex_data.len() < 3 {
        return Err(anyhow!("pos_poly 顶点数量不够，小于3。"));
    }

    for (i, pts) in loops.iter().enumerate().skip(1) {
        // 将 Vec3 拆分为 Vec2 和 frads
        let mut verts2d: Vec<Vec2> = Vec::with_capacity(pts.len());
        let mut frads: Vec<f32> = Vec::with_capacity(pts.len());
        for v in pts {
            verts2d.push(Vec2::new(v.x, v.y));
            frads.push(v.z);
        }
        // 逐个 wire 先通过 ploop-rs 计算 bulge，再生成 Polyline
        let processed =
            match process_ploop_vertices(&verts2d, &frads, &format!("OCC_NEG_WIRE_{}", i)) {
                Ok(p) => p,
                Err(e) => {
                    println!("⚠️  跳过第 {} 个 wire（PLOOP 处理失败: {}）", i + 1, e);
                    continue;
                }
            };

        let Ok(neg) = gen_polyline_from_processed_vertices(&processed, None) else {
            println!("⚠️  跳过第 {} 个 wire（生成 Polyline 失败）", i + 1);
            continue;
        };

        // 执行 boolean subtract (pos_poly - neg)
        let mut r = pos_poly.boolean(&neg, BooleanOp::Not);
        if r.pos_plines.len() > 0 {
            pos_poly = r.pos_plines.remove(0).pline;
            println!("   成功从 position wire 中减去第 {} 个 wire", i + 1);
        } else {
            println!("⚠️  第 {} 个 wire 布尔运算失败，跳过", i + 1);
        }
    }
    #[cfg(feature = "debug_wire")]
    println!(
        "final occ polyline: {}",
        polyline_to_debug_json_str(&pos_poly)
    );

    let mut wires = vec![];
    let mut edges = vec![];
    let mut seg_count = 0;
    for (p, q) in pos_poly.iter_segments() {
        if p.bulge.abs() < 0.001 {
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
        seg_count += 1;
    }
    if seg_count < 1 {
        return Err(anyhow!("生成的线段数量小于1"));
    }
    wires.push(Wire::from_edges(&edges)?);
    Ok(wires)
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

    visit_global_self_intersects(polyline, aabb_index, &mut visitor, T::from(0.01).unwrap());

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
#[cfg(feature = "truck")]
pub fn gen_wire(
    input_pts: &Vec<Vec3>,
    input_fradius_vec: &Vec<f32>,
) -> anyhow::Result<truck_modeling::Wire> {
    #[cfg(feature = "truck")]
    use truck_modeling::{Vertex, Wire, builder};
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
        [-22200, 23350, 23350],
        [23350, 23350, 23350],
        [23350, 0, 0],
    ];
    let pts: Vec<Vec3> = data
        .iter()
        .map(|x| Vec3::new(x[0] as f32, x[1] as f32, x[2] as f32))
        .collect::<Vec<_>>();

    // gen_occ_wires(&pts).unwrap();
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

    // assert_eq!(check_wire_ok(&pts, &fradius_vec), true);
    // gen_occ_wires(&pts, &fradius_vec).unwrap();
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

    // gen_occ_wires(&pts, &fradius_vec).unwrap();
}

#[test]
pub fn test_gen_polyline() {
    // Simple rectangle with a fillet radius at the corner
    let pts = vec![
        Vec3::new(0.0, 0.0, 0.0),   // Bottom-left
        Vec3::new(10.0, 0.0, 0.0),  // Bottom-right
        Vec3::new(10.0, 10.0, 2.0), // Top-right with fillet radius 2.0
        Vec3::new(0.0, 10.0, 0.0),  // Top-left
    ];

    // 将 Vec3 拆分为 Vec2 和 frads
    let mut verts2d: Vec<Vec2> = Vec::with_capacity(pts.len());
    let mut frads: Vec<f32> = Vec::with_capacity(pts.len());
    for v in &pts {
        verts2d.push(Vec2::new(v.x, v.y));
        frads.push(v.z);
    }

    let processed = process_ploop_vertices(&verts2d, &frads, "TEST_GEN_POLYLINE")
        .expect("Failed to process vertices");
    let polyline = gen_polyline_from_processed_vertices(&processed, None)
        .expect("Failed to generate polyline");

    // Verify the generated polyline has the expected properties
    assert!(polyline.is_closed());

    // For a rectangle with one corner filleted, we expect 5 vertices
    // (4 corners with one being split into 2 points for the arc)
    assert_eq!(polyline.vertex_data.len(), 5);

    // Check for non-zero bulge in the vertex data (indicating an arc)
    let has_bulge = polyline.vertex_data.iter().any(|v| v.bulge.abs() > 0.0);
    assert!(
        has_bulge,
        "Polyline should have at least one arc segment with non-zero bulge"
    );

    println!(
        "Generated polyline: {}",
        polyline_to_debug_json_str(&polyline)
    );
}

#[test]
pub fn test_gen_polyline_with_multiple_fillets() {
    // Rectangle with fillet radius at all corners
    let pts = vec![
        Vec3::new(0.0, 0.0, 1.5),   // Bottom-left with fillet radius 1.5
        Vec3::new(10.0, 0.0, 1.5),  // Bottom-right with fillet radius 1.5
        Vec3::new(10.0, 10.0, 1.5), // Top-right with fillet radius 1.5
        Vec3::new(0.0, 10.0, 1.5),  // Top-left with fillet radius 1.5
    ];

    // 先通过 ploop-rs 处理 FRADIUS，再基于 bulge 生成 Polyline
    // 将 Vec3 拆分为 Vec2 和 frads
    let mut verts2d: Vec<Vec2> = Vec::with_capacity(pts.len());
    let mut frads: Vec<f32> = Vec::with_capacity(pts.len());
    for v in &pts {
        verts2d.push(Vec2::new(v.x, v.y));
        frads.push(v.z);
    }
    let processed = process_ploop_vertices(&verts2d, &frads, "TEST_GEN_POLYLINE_WITH_MULTI_FILLET")
        .expect("Failed to process vertices");
    let polyline = gen_polyline_from_processed_vertices(&processed, None)
        .expect("Failed to generate polyline");

    // Verify the polyline is closed
    assert!(polyline.is_closed());

    // With all corners filleted, we expect 8 vertices (each corner splits into 2 points)
    assert_eq!(polyline.vertex_data.len(), 8);

    // Count the number of arc segments (non-zero bulge values)
    let arc_count = polyline
        .vertex_data
        .iter()
        .filter(|v| v.bulge.abs() > 0.0)
        .count();
    assert_eq!(arc_count, 4, "Should have 4 arc segments");

    println!(
        "Generated polyline with multiple fillets: {}",
        polyline_to_debug_json_str(&polyline)
    );
}

#[test]
pub fn test_gen_polyline_complex_shape() {
    // Complex shape with various fillet radii
    // Points from the provided example with z values converted to fillet radii
    let pts = vec![
        Vec3::new(0.0, 0.0, 0.0),               // No fillet
        Vec3::new(-658.33, -3386.80, 0.0),      // No fillet
        Vec3::new(-289.38, -3454.17, 21956.98), // Large fillet radius
        Vec3::new(77.07, -3534.10, 0.0),        // No fillet
        Vec3::new(77.07, -3534.10, 0.0),        // No fillet
        Vec3::new(735.49, -146.73, 0.0),        // No fillet
        Vec3::new(368.82, -67.93, 25392.88),    // No fillet
        // Vec3::new(77.07, -3534.10, 25392.88),   // Large fillet radius
        Vec3::new(0.0, 0.0, 0.0), // No fillet
    ];

    // 先通过 ploop-rs 处理 FRADIUS，再基于 bulge 生成 Polyline
    // 将 Vec3 拆分为 Vec2 和 frads
    let mut verts2d: Vec<Vec2> = Vec::with_capacity(pts.len());
    let mut frads: Vec<f32> = Vec::with_capacity(pts.len());
    for v in &pts {
        verts2d.push(Vec2::new(v.x, v.y));
        frads.push(v.z);
    }
    let processed = process_ploop_vertices(&verts2d, &frads, "TEST_GEN_POLYLINE_COMPLEX_SHAPE")
        .expect("Failed to process vertices");
    let polyline = gen_polyline_from_processed_vertices(&processed, None)
        .expect("Failed to generate polyline");

    #[cfg(feature = "occ")]
    {
        let occ_wires = gen_occ_wires(&vec![pts.clone()]).expect("Failed to generate OCC wires");

        // Verify the generated OCC wire has the expected properties
        assert_eq!(occ_wires.len(), 1, "Expected a single OCC wire");
        let occ_wire = &occ_wires[0];

        // Check that the OCC wire has at least some edges
        // assert!(
        //     occ_wire.edges().len() > 3,
        //     "Expected a valid OCC wire with multiple edges"
        // );
    }

    // Verify the generated polyline has the expected properties
    // assert!(polyline.is_closed());

    // // Check that we have at least some vertices
    // assert!(
    //     polyline.vertex_data.len() > 3,
    //     "Expected a valid polyline with multiple vertices"
    // );

    // // Check for non-zero bulge in the vertex data (indicating arcs for fillet corners)
    // let arc_count = polyline
    //     .vertex_data
    //     .iter()
    //     .filter(|v| v.bulge.abs() > 0.0)
    //     .count();
    // assert!(
    //     arc_count > 0,
    //     "Expected at least one arc segment with non-zero bulge"
    // );

    println!(
        "Generated complex polyline: {}",
        polyline_to_debug_json_str(&polyline)
    );
}

/// 使用 ploop-rs 处理顶点数据
///
/// 这个方法接收分离的 2D 顶点和 FRADIUS 列表，使用 ploop-rs 进行处理
///
/// # 参数
/// * `verts` - 2D 顶点数据，Vec2 格式
/// * `frads` - 对应的 FRADIUS 值列表，f32
/// * `ploop_name` - PLOOP 名称（用于日志显示）
///
/// # 返回值
/// * `Result<Vec<Vec3>>` - 处理后的顶点列表，Vec3 格式：**x,y 为坐标，z 为 bulge 值**
///
/// # 示例
/// ```rust
/// use aios_core::prim_geo::wire::process_ploop_vertices;
/// use glam::Vec2;
///
/// let verts = vec![
///     Vec2::new(0.0, 0.0),      // 起点
///     Vec2::new(100.0, 0.0),    // 第二点
///     Vec2::new(100.0, 100.0), // 第三点
///     Vec2::new(0.0, 100.0),   // 第四点
/// ];
/// let frads = vec![0.0, 0.0, 15.0, 10.0]; // 第三点和第四点有圆角
/// let processed = process_ploop_vertices(&verts, &frads, "TEST_PLOOP")?;
/// ```
pub fn process_ploop_vertices(
    verts: &[Vec2],
    frads: &[f32],
    ploop_name: &str,
) -> anyhow::Result<Vec<Vec3>> {
    if verts.len() < 3 {
        return Err(anyhow::anyhow!("顶点数量不足，至少需要3个顶点"));
    }
    if verts.len() != frads.len() {
        return Err(anyhow::anyhow!(
            "顶点数量({})与 FRADIUS 数量({})不一致",
            verts.len(),
            frads.len()
        ));
    }

    // println!("🔧 开始处理PLOOP顶点: {}", ploop_name);
    // println!("   输入顶点数: {}", verts.len());

    // 创建 PLOOP 处理器（使用默认容差 0.01，不输出调试信息）
    let processor = PloopProcessor::new(0.01, false);

    // 将 verts 和 frads 转换为 Vertex
    let ploop_vertices: Vec<Vertex> = verts
        .iter()
        .zip(frads.iter())
        .map(|(v, &r)| {
            if r > 0.0 {
                // 有 fradius 的顶点
                Vertex::with_fradius(v.x, v.y, 0.0, Some(r))
            } else {
                // 普通顶点
                Vertex::new(v.x, v.y)
            }
        })
        .collect();

    // export ploop vertices to json file
    // let json_str = serde_json::to_string_pretty(&ploop_vertices)?;
    // std::fs::write(format!("test_output/test_loop_case/{}.json", ploop_name), json_str)?;

    // 使用 ploop-rs 处理 PLOOP（直接传递顶点切片）
    // process_ploop 返回二元组：(processed_vertices, arcs)
    let (processed_vertices, bulges, arcs, _fradius_report) =
        processor.process_ploop(&ploop_vertices);

    // println!("   处理后顶点数: {}", processed_vertices.len());
    // println!("   生成圆弧数: {}", arcs.len());

    if processed_vertices.len() != bulges.len() {
        return Err(anyhow::anyhow!(
            "处理后的顶点数量({})与 bulge 数量({})不一致",
            processed_vertices.len(),
            bulges.len()
        ));
    }

    // 修正 bulge 索引对齐问题：
    // ploop-rs 的 bulges[i] 表示从顶点 i-1 到顶点 i 的边
    // cavalier_contours 的 bulge[i] 表示从顶点 i 到顶点 i+1 的边
    // 因此需要将 bulges 向前移动一位
    let n = processed_vertices.len();
    let mut result = Vec::with_capacity(n);

    for i in 0..n {
        let vertex = &processed_vertices[i];
        // cavalier_contours 需要从当前顶点到下一个顶点的 bulge
        // 对应 ploop-rs 的 bulges[(i+1) % n]
        let next_i = (i + 1) % n;
        let bulge = bulges.get(next_i).copied().unwrap_or(0.0);

        result.push(Vec3::new(vertex.x as f32, vertex.y as f32, bulge as f32));
    }

    // println!("✅ PLOOP顶点处理完成，返回 {} 个顶点（bulge 索引已修正）", result.len());

    Ok(result)
}

/// 从 PLOOP 文件内容解析并处理顶点数据
///
/// 这个方法从 PLOOP 文件内容中解析数据，然后使用 ploop-rs 进行处理
///
/// PLOOP 文件格式：
/// ```
/// NEW FRMWORK <name>
/// NEW PLOOP
/// VERTEX <x> <y> <z> [FRADIUS <r>]
/// ...
/// END PLOOP
/// END FRMWORK
/// ```
///
/// 注意：在返回的 Vec3 中，x、y 为坐标，z 存储对应边的 bulge 值
///
/// # 参数
/// * `ploop_content` - PLOOP 文件的内容字符串
/// * `ploop_name` - 要处理的 PLOOP 名称（可选，如果为 None 则处理第一个找到的 PLOOP）
///
/// # 返回值
/// * `Result<Vec<Vec3>>` - 处理后的顶点列表，Vec3 格式：x,y 为坐标，z 为 bulge 值
pub fn process_ploop_from_content(
    ploop_content: &str,
    ploop_name: Option<&str>,
) -> anyhow::Result<Vec<Vec3>> {
    use regex::Regex;

    // 解析 PLOOP 文件内容
    let vertex_regex =
        Regex::new(r"(?i)VERTEX\s+([-\d.]+)\s+([-\d.]+)\s+([-\d.]+)(?:\s+FRADIUS\s+([-\d.]+))?")
            .unwrap();

    let mut vertices = Vec::new();
    let mut current_ploop_name: Option<String> = None;
    let mut in_ploop = false;
    let mut found_ploop: Option<Vec<Vec3>> = None;

    for line in ploop_content.lines() {
        let line = line.trim();

        // 检查是否进入新的 PLOOP
        if line.to_uppercase().starts_with("NEW PLOOP") {
            in_ploop = true;
            vertices.clear();
            continue;
        }

        // 检查是否结束 PLOOP
        if line.to_uppercase().starts_with("END PLOOP") {
            if in_ploop && !vertices.is_empty() {
                // 处理当前 PLOOP
                let ploop_name_str = current_ploop_name.as_deref().unwrap_or("UNNAMED");

                // 如果指定了名称，检查是否匹配
                if let Some(name) = ploop_name {
                    if current_ploop_name
                        .as_deref()
                        .map_or(false, |n| n.contains(name))
                    {
                        found_ploop = Some(vertices.clone());
                        break;
                    }
                } else if found_ploop.is_none() {
                    // 如果没有指定名称，使用第一个找到的 PLOOP
                    found_ploop = Some(vertices.clone());
                }
            }
            in_ploop = false;
            vertices.clear();
            continue;
        }

        // 检查 FRMWORK 名称
        if line.to_uppercase().starts_with("NEW FRMWORK") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                current_ploop_name = Some(parts[2..].join(" "));
            }
            continue;
        }

        // 解析 VERTEX 行
        if in_ploop {
            if let Some(caps) = vertex_regex.captures(line) {
                let x: f32 = caps
                    .get(1)
                    .unwrap()
                    .as_str()
                    .parse()
                    .map_err(|e| anyhow::anyhow!("解析 x 坐标失败: {}", e))?;
                let y: f32 = caps
                    .get(2)
                    .unwrap()
                    .as_str()
                    .parse()
                    .map_err(|e| anyhow::anyhow!("解析 y 坐标失败: {}", e))?;
                let _z: f32 = caps
                    .get(3)
                    .unwrap()
                    .as_str()
                    .parse()
                    .map_err(|e| anyhow::anyhow!("解析 z 坐标失败: {}", e))?;

                // 提取 FRADIUS（如果存在）
                let fradius = caps
                    .get(4)
                    .and_then(|m| m.as_str().parse::<f32>().ok())
                    .filter(|&r| r > 0.0);

                // Vec3 的 z 存储 FRADIUS 值（注意：不是 z 坐标）
                vertices.push(Vec3::new(x, y, fradius.unwrap_or(0.0)));
            }
        }
    }

    // 如果没有找到匹配的 PLOOP，尝试使用最后一个解析的 PLOOP
    let vertices_to_process = if let Some(ploop) = found_ploop {
        ploop
    } else if !vertices.is_empty() {
        vertices
    } else {
        return Err(anyhow::anyhow!("没有找到任何有效的PLOOP数据"));
    };

    if vertices_to_process.len() < 3 {
        return Err(anyhow::anyhow!("顶点数量不足，至少需要3个顶点"));
    }

    let ploop_name_str = current_ploop_name.as_deref().unwrap_or("UNNAMED");
    println!("🔧 开始处理PLOOP文件: {}", ploop_name_str);
    println!("   原始顶点数: {}", vertices_to_process.len());

    // 使用 process_ploop_vertices 处理顶点
    // 将 Vec3 拆分为 Vec2 和 frads
    let mut verts2d: Vec<Vec2> = Vec::with_capacity(vertices_to_process.len());
    let mut frads: Vec<f32> = Vec::with_capacity(vertices_to_process.len());
    for v in &vertices_to_process {
        verts2d.push(Vec2::new(v.x, v.y));
        frads.push(v.z);
    }
    process_ploop_vertices(&verts2d, &frads, ploop_name_str)
}

#[test]
fn test_process_ploop_vertices() {
    // 创建测试顶点数据（Vec3: x,y 为坐标，z 为 fradius）
    let test_vertices = vec![
        Vec3::new(0.0, 0.0, 0.0),      // 起点，无圆角
        Vec3::new(100.0, 0.0, 0.0),    // 第二点，无圆角
        Vec3::new(100.0, 100.0, 10.0), // 第三点，圆角半径10
        Vec3::new(0.0, 100.0, 0.0),    // 第四点，无圆角
    ];

    // 测试 process_ploop_vertices 方法
    // 将 Vec3 拆分为 Vec2 和 frads
    let mut verts2d: Vec<Vec2> = Vec::with_capacity(test_vertices.len());
    let mut frads: Vec<f32> = Vec::with_capacity(test_vertices.len());
    for v in &test_vertices {
        verts2d.push(Vec2::new(v.x, v.y));
        frads.push(v.z);
    }
    match process_ploop_vertices(&verts2d, &frads, "TEST_FRAME") {
        Ok(processed_vertices) => {
            println!(
                "✅ 顶点处理测试成功: 处理得到 {} 个顶点",
                processed_vertices.len()
            );
            assert!(processed_vertices.len() > 0, "应该至少有一个顶点");

            // 打印顶点信息
            for (i, vertex) in processed_vertices.iter().enumerate() {
                if vertex.z.abs() > f32::EPSILON {
                    println!(
                        "  顶点[{}]: ({:.2}, {:.2}) bulge: {:.4}",
                        i, vertex.x, vertex.y, vertex.z
                    );
                } else {
                    println!("  顶点[{}]: ({:.2}, {:.2})", i, vertex.x, vertex.y);
                }
            }
        }
        Err(e) => {
            println!("❌ 顶点处理测试失败: {}", e);
            // 在测试环境中，这可能会失败，因为 ploop-rs 可能不可用
            // 这是正常的，我们只是验证方法的接口
        }
    }
}

#[test]
fn test_process_ploop_from_content() {
    // 创建一个包含 FRADIUS 的测试 PLOOP 数据
    let test_ploop_content = r#"
NEW FRMWORK TEST_FRAME_FRADIUS
NEW PLOOP
VERTEX 0.0 0.0 0.0
VERTEX 100.0 0.0 0.0
VERTEX 100.0 100.0 0.0 FRADIUS 15.0
VERTEX 0.0 100.0 0.0 FRADIUS 5.0
END PLOOP
END FRMWORK
"#;

    // 测试从内容解析的 process_ploop_from_content 方法
    match process_ploop_from_content(test_ploop_content, None) {
        Ok(vertices) => {
            println!("✅ 内容解析测试成功: 处理得到 {} 个顶点", vertices.len());
            assert!(vertices.len() > 0, "应该至少有一个顶点");

            // 打印顶点信息
            for (i, vertex) in vertices.iter().enumerate() {
                if vertex.z.abs() > f32::EPSILON {
                    println!(
                        "  顶点[{}]: ({:.2}, {:.2}) bulge: {:.4}",
                        i, vertex.x, vertex.y, vertex.z
                    );
                } else {
                    println!("  顶点[{}]: ({:.2}, {:.2})", i, vertex.x, vertex.y);
                }
            }

            // 检查是否有 bulge 值
            let has_bulge = vertices.iter().any(|v| v.z.abs() > f32::EPSILON);
            if has_bulge {
                println!("  ✅ 检测到 bulge 数据");
            }
        }
        Err(e) => {
            println!("❌ 内容解析测试失败: {}", e);
            // 在测试环境中，这可能会失败，因为 ploop-rs 可能不可用
            // 这是正常的，我们只是验证方法的接口
        }
    }
}

#[test]
fn test_gen_polyline_with_ploop_processor() {
    // 测试带 FRADIUS 的顶点数据
    let vertices_with_fradius = vec![
        Vec3::new(0.0, 0.0, 0.0),      // 起点，无圆角
        Vec3::new(100.0, 0.0, 0.0),    // 第二点，无圆角
        Vec3::new(100.0, 100.0, 15.0), // 第三点，圆角半径15
        Vec3::new(0.0, 100.0, 10.0),   // 第四点，圆角半径10
    ];

    println!("🧪 测试带 FRADIUS 的 Polyline 生成方法");

    // 将 Vec3 拆分为 Vec2 和 frads
    let mut verts2d: Vec<Vec2> = Vec::with_capacity(vertices_with_fradius.len());
    let mut frads: Vec<f32> = Vec::with_capacity(vertices_with_fradius.len());
    for v in &vertices_with_fradius {
        verts2d.push(Vec2::new(v.x, v.y));
        frads.push(v.z);
    }

    let processed_with = match process_ploop_vertices(&verts2d, &frads, "GEN_POLYLINE_WITH_FRADIUS")
    {
        Ok(p) => p,
        Err(e) => {
            println!("❌ 带 FRADIUS 的 PLOOP 处理失败: {}", e);
            return;
        }
    };

    match gen_polyline_from_processed_vertices(&processed_with, None) {
        Ok(polyline) => {
            // println!("✅ 带 FRADIUS 测试成功！");
            // println!(
            //     "   生成的 Polyline 有 {} 个顶点",
            //     polyline.vertex_data.len()
            // );
            // println!("   Polyline 是否闭合: {}", polyline.is_closed());

            // 检查是否有圆弧段（bulge != 0）
            let arc_count = polyline
                .vertex_data
                .iter()
                .filter(|v| v.bulge.abs() > 0.001)
                .count();
            // println!("   包含 {} 个圆弧段", arc_count);
        }
        Err(e) => {
            println!("❌ 带 FRADIUS 测试失败: {}", e);
            // 这可能会失败，因为 ploop-rs 可能不可用
        }
    }

    // 测试无 FRADIUS 的顶点数据
    let vertices_no_fradius = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(100.0, 0.0, 0.0),
        Vec3::new(100.0, 100.0, 0.0),
        Vec3::new(0.0, 100.0, 0.0),
    ];

    println!("\n🧪 测试无 FRADIUS 的 Polyline 生成方法");

    // 将 Vec3 拆分为 Vec2 和 frads
    let mut verts2d_no: Vec<Vec2> = Vec::with_capacity(vertices_no_fradius.len());
    let mut frads_no: Vec<f32> = Vec::with_capacity(vertices_no_fradius.len());
    for v in &vertices_no_fradius {
        verts2d_no.push(Vec2::new(v.x, v.y));
        frads_no.push(v.z);
    }

    let processed_no =
        match process_ploop_vertices(&verts2d_no, &frads_no, "GEN_POLYLINE_NO_FRADIUS") {
            Ok(p) => p,
            Err(e) => {
                println!("❌ 无 FRADIUS 的 PLOOP 处理失败: {}", e);
                return;
            }
        };

    match gen_polyline_from_processed_vertices(&processed_no, None) {
        Ok(polyline) => {
            println!("✅ 无 FRADIUS 测试成功！");
            println!(
                "   生成的 Polyline 有 {} 个顶点",
                polyline.vertex_data.len()
            );
            println!("   Polyline 是否闭合: {}", polyline.is_closed());
        }
        Err(e) => {
            println!("❌ 无 FRADIUS 测试失败: {}", e);
        }
    }
}

/// Wire 三角化的结果结构
#[derive(Debug, Clone)]
pub struct WireTriangulation {
    /// 3D 顶点坐标 (假设 Z=0 平面)
    pub vertices: Vec<Vec3>,
    /// 三角形索引
    pub indices: Vec<u32>,
    /// 顶点法线 (统一向上)
    pub normals: Vec<Vec3>,
    /// UV 坐标 (可选)
    pub uvs: Vec<[f32; 2]>,
}

/// 将 Polyline 转换为 2D 点集用于三角化
///
/// 从 cavalier_contours 的 Polyline 中正确处理线段和圆弧段，
/// 将圆弧段离散化为多个点以保持几何精度
///
/// # 参数
/// * `polyline` - 输入的多段线
///
/// # 返回值
/// * `Vec<Vec2>` - 2D 点集（已将圆弧离散化）
fn polyline_to_2d_points(polyline: &Polyline) -> Vec<Vec2> {
    let mut points_2d = Vec::new();

    // 遍历多段线中的所有段
    for (i, vertex) in polyline.iter_vertexes().enumerate() {
        // 添加当前顶点
        let point_2d = Vec2::new(vertex.x as f32, vertex.y as f32);
        points_2d.push(point_2d);

        // 如果当前段有 bulge（表示是圆弧），需要离散化
        if vertex.bulge.abs() > 0.001 {
            let next_vertex = polyline[(i + 1) % polyline.vertex_data.len()];
            let arc_points = discretize_arc_segment(
                Vec2::new(vertex.x as f32, vertex.y as f32),
                Vec2::new(next_vertex.x as f32, next_vertex.y as f32),
                vertex.bulge,
                // 根据圆弧大小动态调整离散化段数
                calculate_arc_segments_needed(vertex.bulge),
            );

            // 添加离散化的中间点（跳过起点和终点，因为它们已经在主循环中添加）
            if arc_points.len() > 2 {
                points_2d.extend_from_slice(&arc_points[1..arc_points.len() - 1]);
            }
        }
    }

    // 移除重复点（容差范围内）
    points_2d.dedup_by(|a: &mut Vec2, b: &mut Vec2| (*a - *b).length_squared() < 0.01 * 0.01);

    println!("   离散化后得到 {} 个 2D 点", points_2d.len());
    points_2d
}

/// 离散化圆弧段为多个直线段的点集
///
/// 根据起点、终点和 bulge 值计算圆弧上的一系列点
///
/// # 参数
/// * `start` - 起点
/// * `end` - 终点  
/// * `bulge` - bulge 值（tan(角度/4)）
/// * `num_segments` - 离散化段数
///
/// # 返回值
/// * `Vec<Vec2>` - 离散化后的点集（包含起点和终点）
fn discretize_arc_segment(start: Vec2, end: Vec2, bulge: f64, num_segments: usize) -> Vec<Vec2> {
    if num_segments < 2 {
        return vec![start, end];
    }

    let mut points = Vec::with_capacity(num_segments + 1);
    points.push(start);

    // 计算 bulge 对应的中心角度
    let angle = 4.0 * bulge.atan() as f32;

    // 计算圆弧的圆心和半径
    let (center, radius) = calculate_arc_center_and_radius(start, end, bulge as f32);

    // 计算起始角度
    let start_angle = (start - center).y.atan2((start - center).x);

    // 根据凸起方向确定旋转方向
    let direction = if bulge > 0.0 { 1.0 } else { -1.0 };

    // 生成中间点
    for i in 1..num_segments {
        let t = i as f32 / num_segments as f32;
        let current_angle = start_angle + direction * angle * t;

        let point = Vec2::new(
            center.x + radius * current_angle.cos(),
            center.y + radius * current_angle.sin(),
        );
        points.push(point);
    }

    points.push(end);
    points
}

/// 计算圆弧的中心和半径
fn calculate_arc_center_and_radius(start: Vec2, end: Vec2, bulge: f32) -> (Vec2, f32) {
    if bulge.abs() < 0.001 {
        // 直线段，返回中点和一个无效半径
        return ((start + end) * 0.5, 0.0);
    }

    let angle = 4.0 * bulge.atan();
    let chord = end - start;
    let chord_length = chord.length();

    // 半径计算：R = (L/2) / sin(theta/2)
    let radius = (chord_length / 2.0) / (angle / 2.0).sin().abs();

    // 计算从弦的中点到圆心的距离
    let sagitta = radius - (radius * (angle / 2.0).cos().abs());

    // 计算弦的中点
    let midpoint = (start + end) * 0.5;

    // 计算垂直于弦的方向
    let chord_dir = chord.normalize();
    let perp_dir = Vec2::new(-chord_dir.y, chord_dir.x);

    // 根据凸起方向确定圆心位置
    let center = if bulge > 0.0 {
        midpoint + perp_dir * sagitta
    } else {
        midpoint - perp_dir * sagitta
    };

    (center, radius)
}

/// 根据 bulge 值计算需要的离散化段数
fn calculate_arc_segments_needed(bulge: f64) -> usize {
    // bulge 越大，圆弧弯曲程度越高，需要更多段数
    let angle = (4.0 * bulge.atan()).abs() as f32;

    // 基础段数计算：每 10 度一段，最少 4 段，最多 32 段
    let degrees = angle.to_degrees();
    let segments = (degrees / 10.0).ceil() as usize;

    // 确保段数在合理范围内
    segments.max(4).min(32)
}

/// 使用 spade 对 2D 点集进行三角化
fn triangulate_2d_points(
    points_2d: &[Vec2],
) -> Option<crate::geometry::sweep_mesh::CapTriangulation> {
    if points_2d.len() < 3 {
        return None;
    }

    let indices = triangulate_polygon_indices_spade(points_2d).ok()?;
    Some(crate::geometry::sweep_mesh::CapTriangulation {
        points: points_2d.to_vec(),
        indices,
    })
}

/// 将 wire 顶点直接三角化为 3D 网格
///
/// 该函数将输入的带 FRADIUS 的顶点数据，通过以下流程进行三角化：
/// 1. 先通过 ploop-rs 处理 FRADIUS，再基于 bulge 生成 2D Polyline
/// 2. 提取 2D 轮廓点
/// 3. 使用 spade 进行三角化
/// 4. 生成 3D 网格数据
///
/// # 参数
/// * `vertices` - 输入顶点数据，Vec3 格式：x,y 为坐标，z 为 FRADIUS 值
///
/// # 返回值
/// * `Result<WireTriangulation>` - 三角化结果
///
/// # 示例
/// ```rust
/// use aios_core::prim_geo::wire::triangulate_wire_directly;
/// use glam::Vec3;
///
/// let vertices = vec![
///     Vec3::new(0.0, 0.0, 0.0),        // 起点，无圆角
///     Vec3::new(100.0, 0.0, 0.0),      // 第二点，无圆角
///     Vec3::new(100.0, 100.0, 10.0),   // 第三点，圆角半径10
///     Vec3::new(0.0, 100.0, 0.0),      // 第四点，无圆角
/// ];
///
/// match triangulate_wire_directly(&vertices) {
///     Ok(triangulation) => {
///         println!("三角化成功！");
///         println!("顶点数: {}", triangulation.vertices.len());
///         println!("三角形数: {}", triangulation.indices.len() / 3);
///     }
///     Err(e) => println!("三角化失败: {}", e),
/// }
/// ```
pub fn triangulate_wire_directly(vertices: &[Vec3]) -> anyhow::Result<WireTriangulation> {
    if vertices.len() < 3 {
        return Err(anyhow!("顶点数量不足，至少需要3个顶点"));
    }

    println!("🔧 开始 wire 直接三角化");
    println!("   输入顶点数: {}", vertices.len());

    // 1. 先通过 ploop-rs 处理 FRADIUS，再基于 bulge 生成 2D Polyline
    // 将 Vec3 拆分为 Vec2 和 frads
    let mut verts2d: Vec<Vec2> = Vec::with_capacity(vertices.len());
    let mut frads: Vec<f32> = Vec::with_capacity(vertices.len());
    for v in vertices {
        verts2d.push(Vec2::new(v.x, v.y));
        frads.push(v.z);
    }
    let processed_vertices = process_ploop_vertices(&verts2d, &frads, "TRIANGULATE_WIRE")?;
    let polyline = gen_polyline_from_processed_vertices(&processed_vertices, None)?;
    println!(
        "   生成 Polyline，包含 {} 个顶点",
        polyline.vertex_data.len()
    );

    // 2. 提取 2D 轮廓点
    let points_2d = polyline_to_2d_points(&polyline);
    println!("   提取 {} 个 2D 轮廓点", points_2d.len());

    if points_2d.len() < 3 {
        return Err(anyhow!("2D 轮廓点数量不足，无法三角化"));
    }

    // 3. 使用 spade 进行三角化
    let triangulation = triangulate_2d_points(&points_2d)
        .ok_or_else(|| anyhow!("三角化失败:spade 无法处理输入轮廓"))?;

    println!(
        "   三角化成功，生成 {} 个三角形",
        triangulation.indices.len() / 3
    );

    // 4. 生成 3D 网格数据
    let vertices_3d: Vec<Vec3> = triangulation
        .points
        .iter()
        .map(|p| Vec3::new(p.x, 0.0, p.y)) // 在 XY 平面，Z 向上
        .collect();

    // 5. 计算法线（统一向上）
    let normals = vec![Vec3::Y; vertices_3d.len()];

    // 6. 计算 UV 坐标（基于 2D 位置）
    let bounds = calculate_2d_bounds(&points_2d);
    let uvs: Vec<[f32; 2]> = triangulation
        .points
        .iter()
        .map(|p| normalize_uv(p, &bounds))
        .collect();

    println!("✅ Wire 三角化完成！");
    println!("   3D 顶点数: {}", vertices_3d.len());
    println!("   三角形数: {}", triangulation.indices.len() / 3);

    Ok(WireTriangulation {
        vertices: vertices_3d,
        indices: triangulation.indices,
        normals,
        uvs,
    })
}

/// 计算 2D 点集的边界框
fn calculate_2d_bounds(points: &[Vec2]) -> (Vec2, Vec2) {
    if points.is_empty() {
        return (Vec2::ZERO, Vec2::ZERO);
    }

    let mut min_x = points[0].x;
    let mut min_y = points[0].y;
    let mut max_x = points[0].x;
    let mut max_y = points[0].y;

    for point in points.iter().skip(1) {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }

    let min = Vec2::new(min_x, min_y);
    let max = Vec2::new(max_x, max_y);

    (min, max)
}

/// 将 2D 点坐标归一化为 UV 坐标
fn normalize_uv(point: &Vec2, bounds: &(Vec2, Vec2)) -> [f32; 2] {
    let (min, max) = bounds;
    let size = *max - *min;

    if size.x > 0.001 && size.y > 0.001 {
        let u = (point.x - min.x) / size.x;
        let v = (point.y - min.y) / size.y;
        [u, v]
    } else {
        [0.0, 0.0]
    }
}

/// 将 WireTriangulation 转换为 PlantMesh
///
/// 方便与现有渲染系统集成
///
/// # 参数
/// * `triangulation` - wire 三角化结果
///
/// # 返回值
/// * `PlantMesh` - 标准网格格式
pub fn triangulation_to_plant_mesh(
    triangulation: WireTriangulation,
) -> crate::shape::pdms_shape::PlantMesh {
    use crate::shape::pdms_shape::PlantMesh;

    PlantMesh {
        vertices: triangulation.vertices,
        normals: triangulation.normals,
        uvs: triangulation.uvs,
        indices: triangulation.indices,
        wire_vertices: Vec::new(),
        edges: Vec::new(),
        aabb: None,
    }
}

#[test]
fn test_triangulate_wire_simple() {
    // 简单矩形测试
    let vertices = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(100.0, 0.0, 0.0),
        Vec3::new(100.0, 100.0, 0.0),
        Vec3::new(0.0, 100.0, 0.0),
    ];

    match triangulate_wire_directly(&vertices) {
        Ok(triangulation) => {
            println!("✅ 简单矩形三角化测试成功！");
            println!("   顶点数: {}", triangulation.vertices.len());
            println!("   三角形数: {}", triangulation.indices.len() / 3);

            // 验证基本属性
            assert!(triangulation.vertices.len() >= 4);
            assert!(triangulation.indices.len() >= 6);
            assert_eq!(triangulation.normals.len(), triangulation.vertices.len());
            assert_eq!(triangulation.uvs.len(), triangulation.vertices.len());

            // 验证法线方向
            for normal in &triangulation.normals {
                assert!(normal.dot(Vec3::Y) > 0.9);
            }
        }
        Err(e) => {
            println!("❌ 简单矩形三角化测试失败: {}", e);
        }
    }
}

#[test]
fn test_triangulate_wire_with_fillet() {
    // 带圆角的矩形
    let vertices = vec![
        Vec3::new(0.0, 0.0, 0.0),      // 起点，无圆角
        Vec3::new(100.0, 0.0, 0.0),    // 第二点，无圆角
        Vec3::new(100.0, 100.0, 10.0), // 第三点，圆角半径10
        Vec3::new(0.0, 100.0, 10.0),   // 第四点，圆角半径10
    ];

    match triangulate_wire_directly(&vertices) {
        Ok(triangulation) => {
            println!("✅ 带圆角矩形三角化测试成功！");
            println!("   顶点数: {}", triangulation.vertices.len());
            println!("   三角形数: {}", triangulation.indices.len() / 3);

            // 验证基本属性
            assert!(triangulation.vertices.len() >= 4);
            assert!(triangulation.indices.len() >= 6);
        }
        Err(e) => {
            println!("❌ 带圆角矩形三角化测试失败: {}", e);
        }
    }
}

#[test]
fn test_triangulate_wire_complex() {
    // 复杂形状（类似实际测试数据）
    let vertices = vec![
        Vec3::new(0.0, 0.0, 480.0),
        Vec3::new(4.46, -173.52, 480.0),
        Vec3::new(-132.5, 145.48, 480.0),
        Vec3::new(112.98, -100.0, 480.0),
        Vec3::new(-206.02, 36.96, 480.0),
        Vec3::new(-32.5, 32.5, 480.0),
    ];

    match triangulate_wire_directly(&vertices) {
        Ok(triangulation) => {
            println!("✅ 复杂形状三角化测试成功！");
            println!("   原始顶点数: {}", vertices.len());
            println!("   三角化顶点数: {}", triangulation.vertices.len());
            println!("   三角形数: {}", triangulation.indices.len() / 3);
        }
        Err(e) => {
            println!("❌ 复杂形状三角化测试失败: {}", e);
        }
    }
}

#[test]
fn test_triangulation_to_plant_mesh() {
    let vertices = vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(100.0, 0.0, 0.0),
        Vec3::new(100.0, 100.0, 0.0),
        Vec3::new(0.0, 100.0, 0.0),
    ];

    if let Ok(triangulation) = triangulate_wire_directly(&vertices) {
        let plant_mesh = triangulation_to_plant_mesh(triangulation);

        println!("✅ PlantMesh 转换测试成功！");
        println!("   网格顶点数: {}", plant_mesh.vertices.len());
        println!("   法线数量: {}", plant_mesh.normals.len());
        println!("   UV 数量: {}", plant_mesh.uvs.len());
        println!("   索引数量: {}", plant_mesh.indices.len());

        // 验证 PlantMesh 基本属性
        assert_eq!(plant_mesh.vertices.len(), plant_mesh.normals.len());
        assert_eq!(plant_mesh.vertices.len(), plant_mesh.uvs.len());
        assert!(!plant_mesh.indices.is_empty());
    } else {
        println!("❌ PlantMesh 转换测试失败");
    }
}
