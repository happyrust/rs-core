use glam::{Mat3, Quat, Vec3};
use crate::tool::dir_tool::parse_ori_str_to_quat;
use crate::tool::direction_parse::parse_expr_to_dir;
use crate::tool::math_tool::quat_to_pdms_ori_xyz_str;

#[test]
fn test_rotation() {

    let z_dir = parse_expr_to_dir("N 5 W").unwrap().as_vec3();
    let axis_dir = Vec3::Y;
    let rot1 = Quat::from_rotation_arc(Vec3::Y, Vec3::Z);
    // let rot = rot2 * rot1;
    // dbg!(quat_to_pdms_ori_xyz_str(&rot1));

    let rot2 = Quat::from_rotation_arc(Vec3::Z, z_dir);
    // let rot = rot2 * rot1;
    dbg!(quat_to_pdms_ori_xyz_str(&rot2));

    // let rot3 =  rot1 * rot2;
    // dbg!(quat_to_pdms_ori_xyz_str(&rot3));

    let rotation = parse_ori_str_to_quat("Y is Y 5 -X and Z is -Z").unwrap_or(Quat::IDENTITY);
    dbg!(quat_to_pdms_ori_xyz_str(&rotation));
}

#[test]
fn test_rotation_PZAXI() {

    let z_dir = parse_expr_to_dir("N 5 W").unwrap().as_vec3();
    let axis_dir = Vec3::Y;
    // let rot1 = Quat::from_rotation_arc(Vec3::Y, Vec3::Z);
    // // let rot = rot2 * rot1;
    // dbg!(quat_to_pdms_ori_xyz_str(&rot1));

    let rot2 = Quat::from_rotation_arc(axis_dir, z_dir);
    // let rot = rot2 * rot1;
    dbg!(quat_to_pdms_ori_xyz_str(&rot2));

    // let rot3 =  rot1 * rot2;
    // dbg!(quat_to_pdms_ori_xyz_str(&rot3));

    let rotation = parse_ori_str_to_quat("Y is Y 5 -X and Z is -Z").unwrap_or(Quat::IDENTITY);
    dbg!(quat_to_pdms_ori_xyz_str(&rotation));
}