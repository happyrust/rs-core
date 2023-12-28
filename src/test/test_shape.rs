use glam::Vec3;
use truck_modeling::builder;
use truck_stepio::out;

use crate::prim_geo::revolution::Revolution;
use crate::prim_geo::sbox::SBox;
use crate::prim_geo::wire::gen_wire;
use crate::shape::pdms_shape::BrepShapeTrait;

#[test]
fn test_key_pts() {

    let cube = SBox::default();
    let key_pts = cube.key_points();
    dbg!(key_pts);

    let revol = Revolution::default();
    dbg!(revol.key_points());
}



#[test]
fn test_gen_wire_0() {
    let pts = vec![
            Vec3::new(-23350.0, 0.0, 0.0),
            Vec3::new(-2200.0, 23350.0, 0.0),
            Vec3::new(23350.0, 23350.0, 0.0),
            Vec3::new(23350.0, 0.0, 0.0),
        ].into_iter().map(|v| v * 0.01).collect::<Vec<_>>();
    let fradius = vec![0.0f32, 23350.0, 23350.0, 0.0].into_iter().map(|v| v * 0.01).collect::<Vec<_>>();

    let wire = gen_wire(&pts, &fradius).unwrap();
    // dbg!(wire);
    let face = builder::try_attach_plane(&[wire]).unwrap();
    let shape = builder::tsweep(&face, truck_modeling::Vector3::new(0.0, 0.0, 1.0) * 100.0);
    let compressed = shape.compress();

    let step_string = out::CompleteStepDisplay::new(
        out::StepModel::from(&compressed),
        out::StepHeaderDescriptor {
            organization_system: "origin".to_owned(),
            ..Default::default()
        },
    )
    .to_string();
    let mut step_file = std::fs::File::create("test_wire_cut.step").unwrap();
    std::io::Write::write_all(&mut step_file, step_string.as_ref()).unwrap();

}

#[test]
fn test_gen_wire_1() {
    let pts = vec![
            Vec3::new(-23350.0, 0.0, 0.0),
            Vec3::new(-23350.0, 23350.0, 0.0),
            Vec3::new(23350.0, 23350.0, 0.0),
            Vec3::new(23350.0, 0.0, 0.0),
        ].into_iter().map(|v| v * 0.01).collect::<Vec<_>>();
    let fradius = vec![0.0f32, 23350.0, 23350.0, 0.0].into_iter().map(|v| v * 0.01).collect::<Vec<_>>();

    let wire = gen_wire(&pts, &fradius).unwrap();
    // dbg!(wire);
    let face = builder::try_attach_plane(&[wire]).unwrap();
    let shape = builder::tsweep(&face, truck_modeling::Vector3::new(0.0, 0.0, 1.0) * 100.0);
    let compressed = shape.compress();

    let step_string = out::CompleteStepDisplay::new(
        out::StepModel::from(&compressed),
        out::StepHeaderDescriptor {
            organization_system: "test_gen_wire_1".to_owned(),
            ..Default::default()
        },
    )
    .to_string();
    let mut step_file = std::fs::File::create("test_gen_wire_1.step").unwrap();
    std::io::Write::write_all(&mut step_file, step_string.as_ref()).unwrap();

}



#[test]
fn test_gen_wire_2() {
    let pts = vec![
            Vec3::new(-23350.0, 0.0, 0.0),
            Vec3::new(-22220.0, 23350.0, 0.0),
            Vec3::new(23350.0, 23350.0, 0.0),
            Vec3::new(23350.0, 0.0, 0.0),
        ].into_iter().map(|v| v * 0.01).collect::<Vec<_>>();
    let fradius = vec![0.0f32, 50000.0, 0.0, 0.0].into_iter().map(|v| v * 0.01).collect::<Vec<_>>();

    let wire = gen_wire(&pts, &fradius).unwrap();
    // dbg!(wire);
    let face = builder::try_attach_plane(&[wire]).unwrap();
    let shape = builder::tsweep(&face, truck_modeling::Vector3::new(0.0, 0.0, 1.0) * 100.0);
    let compressed = shape.compress();

    let step_string = out::CompleteStepDisplay::new(
        out::StepModel::from(&compressed),
        out::StepHeaderDescriptor {
            organization_system: "test_gen_wire_2".to_owned(),
            ..Default::default()
        },
    )
    .to_string();
    let mut step_file = std::fs::File::create("test_gen_wire_2.step").unwrap();
    std::io::Write::write_all(&mut step_file, step_string.as_ref()).unwrap();

}