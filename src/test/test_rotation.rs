use glam::{Mat3, Quat, Vec3};

#[test]
fn test_rotation() {
    let r = Mat3::from_cols(Vec3::Z, Vec3::Y, Vec3::X);
    let r1 = Mat3::from_cols(Vec3::X, Vec3::Y, Vec3::Z);

    let rotation = Quat::from_rotation_arc(Vec3::Z, Vec3::X);
}