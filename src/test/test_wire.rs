use crate::prim_geo::basic::OccSharedShape;
use crate::prim_geo::wire::{gen_occ_wires, to_debug_json_str};
use crate::shape::pdms_shape::PlantMesh;
use crate::RefU64;
use cavalier_contours::pline_closed;
use cavalier_contours::polyline::{seg_midpoint, BooleanOp, PlineSource, Polyline};
use geo::{ConvexHull, LineString, Polygon};
use glam::{DVec3, Vec3};
use opencascade::primitives::{Edge, Face, IntoShape, Wire};
use std::fmt::format;

fn gen_wire_shape(pts: &Vec<Vec3>, fradius: &Vec<f32>, refno: RefU64) {
    let wires = gen_occ_wires(pts, fradius).unwrap();
    let shape = Face::from_wires(&wires)
        .unwrap()
        .extrude(DVec3::new(0., 0.0, 100.0))
        .into_shape();

    shape.write_step(format!("{refno}.step")).unwrap();

    // match PlantMesh::gen_occ_mesh(&shape, 78.0) {
    //     Ok(mesh) => {
    //         // dbg!((id, m_tol, mesh.vertices.len()));
    //         //保存到文件到dir下
    //         if mesh.ser_to_file(&format!("{}.mesh", "25688_45314_wire")).is_ok() {}
    //     }
    //     _ => {
    //         dbg!(("failed"));
    //     }
    // }
}

#[test]
fn test_gen_wire_25688_45314() {
    let data = vec![
        [23350, 0, 0],
        [22200, 23350, 0],
        [-23350, 23350, 0],
        [-23350, 0, 0],
    ];
    let pts: Vec<Vec3> = data
        .iter()
        .map(|x| Vec3::new(x[0] as f32, x[1] as f32, x[2] as f32))
        .collect::<Vec<_>>();
    let fradius_vec = vec![0.0, 23350.0, 23350.0, 0.0];
    gen_wire_shape(&pts, &fradius_vec, "25688_45314".into());
}

#[test]
fn test_gen_wire_24384_23612() {
    let data = vec![
        [23350, 0, 0],
        [22200, 23350, 0],
        [-23350, 23350, 0],
        [-23350, 0, 0],
    ];
    let pts: Vec<Vec3> = data
        .iter()
        .map(|x| Vec3::new(x[0] as f32, x[1] as f32, x[2] as f32))
        .collect::<Vec<_>>();
    let fradius_vec = vec![0.0, 23350.0, 23350.0, 0.0];
    gen_wire_shape(&pts, &fradius_vec, "25688_45314".into());
}

#[test]
fn test_gen_wire_25688_45331() {
    let data = vec![
        [0.0, 0.0, 480.0],
        [162.76, -3.96, 480.0],
        [-139.56, -124.9, 480.0],
        [90.68, 105.34, 480.0],
        [-30.26, -196.97, 480.0],
        [-34.22, -34.22, 480.0],
    ];
    let pts: Vec<Vec3> = data
        .iter()
        .map(|x| Vec3::new(x[0] as f32, x[1] as f32, x[2] as f32))
        .collect::<Vec<_>>();
    let fradius_vec = vec![0.0, 33.41, 33.41, 33.41, 33.41, 0.0];
    gen_wire_shape(&pts, &fradius_vec, "25688_45331".into());
}
#[test]
fn test_gen_wire_25688_45293() {
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
        .map(|x| Vec3::new(x[0] as f32, x[1] as f32, x[2] as f32))
        .collect::<Vec<_>>();
    let fradius_vec = vec![0.0, 33.37, 33.37, 33.37, 33.37, 0.0];
    gen_wire_shape(&pts, &fradius_vec, "25688_45293".into());
}

#[test]
fn test_gen_wire_25688_45339() {
    let data = vec![
        [0.0, 0.0, 0.0],
        [193.05, 38.05, 0.0],
        [-146.36, -161.11, 0.0],
        [108.39, 138.83, 0.0],
        [-33.2, -228.34, 0.0],
        [-26.9, -31.68, 0.0],
    ];
    let pts: Vec<Vec3> = data
        .iter()
        .map(|x| Vec3::new(x[0] as f32, x[1] as f32, x[2] as f32))
        .collect::<Vec<_>>();
    let fradius_vec = vec![0.0, 33.38, 33.38, 33.38, 33.38, 0.0];
    gen_wire_shape(&pts, &fradius_vec, "25688_45339".into());
}

#[test]
fn test_gen_wire_24381_154722() {
    let data = vec![
        [0.0, 0.0, 0.0],
        [0.0, 138.92, 0.0],
        [318.13, 60.72, 0.0],
        [318.92, 98.94, 0.0],
    ];
    let pts: Vec<Vec3> = data
        .iter()
        .map(|x| Vec3::new(x[0] as f32, x[1] as f32, x[2] as f32))
        .collect::<Vec<_>>();
    let fradius_vec = vec![0.0, 0.0, 0.0, 0.0];
    gen_wire_shape(&pts, &fradius_vec, "24381_154722".into());
}

#[test]
fn test_gen_wire_25688_34950() {
    let data = vec![
        [98.2, -0.01, 0.0],
        [0.0, 13510.17, 0.0],
        [23404.92, 27012.3, 0.0],
        [35100.43, 20249.2, 0.0],
        [32096.15, 15053.87, 0.0],
        [23402.19, 20084.61, 0.0],
        [6001.9, 10044.55, 0.0],
        [6006.46, -0.01, 0.0],
    ];
    let pts: Vec<Vec3> = data
        .iter()
        .map(|x| Vec3::new(x[0] as f32, x[1] as f32, x[2] as f32))
        .collect::<Vec<_>>();
    let fradius_vec = vec![0.0, 23391.0, 23391.0, 0.0, 0.0, 17383.4, 17383.4, 0.0];
    gen_wire_shape(&pts, &fradius_vec, "24381_34950".into());
}

#[test]
fn test_gen_wire_25688_7167() {
    let data: Vec<Vec<f64>> = vec![
        vec![0.0, 0.0, 0.0],
        vec![0.0, 15337.73, 0.0],
        vec![-30432.97, 19187.18, 0.0],
        vec![-34251.98, 4332.51, 0.0],
        vec![-38584.89, 5526.54, 0.0],
        vec![-36528.7, 13400.76, 0.0],
        vec![-29829.34, 18021.33, 0.0],
        vec![-11801.38, 30455.26, 0.0],
        vec![631.7, 12426.7, 0.0],
        vec![4267.57, 7155.04, 0.0],
        vec![4486.86, 758.43, 0.0],
    ];
    let pts: Vec<Vec3> = data
        .iter()
        .map(|x| Vec3::new(x[0] as f32, x[1] as f32, x[2] as f32))
        .collect::<Vec<_>>();
    let fradius_vec = vec![
        0.0, 17400.0, 17400.0, 0.0, 0.0, 21900.0, 0.0, 21900.0, 0.0, 21900.0, 0.0,
    ];
    gen_wire_shape(&pts, &fradius_vec, "25688_71674".into());
}

#[test]
fn test_occ_wire_hole() {
    use opencascade::primitives::IntoShape;
    let mut polyline: Polyline<f64> = pline_closed![
        (17849.504810190134, -19836.731016183498, -0.3831744556027582),
        (2.376275628368603, -1.3621621779275301, 0.0),
        (0.0, 0.0, -0.017550452850019185),
        (
            50.443587499059504,
            1400.2352580942163,
            -0.017542485383628553
        ),
        (196.51919247519072, 2793.1043669787723, 0.0),
        (198.7144908451428, 2791.6789528979243, -0.3832098688760669),
        (20647.786411619036, 19935.04976057892, -0.424),
        (39845.509775227954, -1400.004746326602, -0.424)
    ];
    dbg!(polyline.orientation());

    let poly2: Polyline<f64> = pline_closed![
        (3580.1415123545466, -241.96018189584183, 0.3757241361006367),
        (17520.115150850415, -15811.05946772574, 0.427),
        (35255.890750501705, -1288.4182990987792, 0.4269998124212688),
        (19963.96656441985, 15788.510056089764, 0.3756887541155798),
        (3796.228018683687, 2549.8291223217548, 0.0),
        (3795.1298828125, 2548.929931640625, 0.022123443875106084),
        (3625.7767613550095, 1155.6525446648425, 0.022056489713404726),
        (3579.2199911632974, -242.86182836625085, 0.0)
    ];
    dbg!(poly2.orientation());
    //polyline
    let polys = [polyline, poly2];
    let mut wires = vec![];
    for polyline in polys {
        let mut edges = vec![];
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
        wires.push(Wire::from_edges(&edges));
    }
    let shape = OccSharedShape::new(
        Face::from_wires(&wires)
            .unwrap()
            .extrude(DVec3::new(0., 0.0, 100.0))
            .into_shape(),
    );
    //write step
    shape.write_step("test_occ_wire_hole.step").unwrap();
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

#[test]
fn test_occ_wire_overlap_tolerance() {
    use opencascade::primitives::IntoShape;
    let mut polyline = pline_closed![
        (0.0, 0.0, -0.826),
        (-64.01951136204772, -14.021142337752195, -0.826),
        (-9.760048670681556, 22.740048212916307, 0.0)
    ];
    dbg!(polyline.orientation());

    let poly2 = pline_closed![
        (-9.759954554850879, 22.73995409708857, -0.826),
        (-46.521144028904075, -31.519509475009457, 0.0)
    ];
    dbg!(poly2.orientation());

    let mut result = polyline.boolean(&poly2, BooleanOp::Or);
    if !result.pos_plines.is_empty() {
        dbg!(&result.pos_plines);
        let p = result.pos_plines.remove(0).pline;
        println!("final: {}", to_debug_json_str(&p));
    } else {
        dbg!("cut failed");
    }
}
