use crate::shape::pdms_shape::ANGLE_RAD_TOL;
use crate::tool::float_tool::*;
use approx::{abs_diff_eq, abs_diff_ne};
use glam::{DVec3, Mat3, Quat, Vec3, DMat3, DQuat};
use lazy_static::lazy_static;

lazy_static! {
    pub static ref AXIS_VEC_TUPLES: [(glam::Vec3, &'static str); 6] = {
        [
            (Vec3::X, "E"),
            (-Vec3::X, "W"),
            (Vec3::Y, "N"),
            (-Vec3::Y, "S"),
            (Vec3::Z, "U"),
            (-Vec3::Z, "D"),
        ]
    };
    pub static ref AXIS_DVEC_TUPLES: [(glam::DVec3, &'static str); 6] = {
        [
            (DVec3::X, "E"),
            (-DVec3::X, "W"),
            (DVec3::Y, "N"),
            (-DVec3::Y, "S"),
            (DVec3::Z, "U"),
            (-DVec3::Z, "D"),
        ]
    };
}

pub fn cal_mat3_by_zdir(zdir: DVec3) -> DMat3 {
    let quat = DQuat::from_rotation_arc(DVec3::Z, zdir);
    let mut m = DMat3::from_quat(quat);
    if abs_diff_ne!(m.y_axis.z.abs(), 1.0, epsilon = ANGLE_RAD_TOL as f64) {
        m.y_axis.z = 0.0;
        m.y_axis = m.y_axis.normalize();
        m.x_axis = m.y_axis.cross(m.z_axis).normalize();
    } else if m.y_axis.z < 0.0 {
        m.y_axis.z = 1.0;
        m.y_axis = m.y_axis.normalize();
        m.x_axis = m.y_axis.cross(m.z_axis).normalize();
    }
    m
}

pub fn convert_to_xyz(s: &str) -> String {
    s.replace("E", "X")
        .replace("N", "Y")
        .replace("U", "Z")
        .replace("W", "-X")
        .replace("S", "-Y")
        .replace("D", "-Z")
}

pub fn to_pdms_vec_str(vec: &Vec3) -> String {
    for (v, v_str) in AXIS_VEC_TUPLES.iter() {
        if abs_diff_eq!(vec.dot(*v), 1.0, epsilon = ANGLE_RAD_TOL) {
            return (*v_str).to_string();
        }
    }
    //1个象限的组合
    if abs_diff_eq!(vec.x * vec.y * vec.z, 0.0, epsilon = ANGLE_RAD_TOL) {
        let mut x = 0.0;
        let mut y = 0.0;
        let mut x_str = "";
        let mut y_str = "";
        let mut angle = 0.0f32;
        if abs_diff_eq!(vec.x, 0.0, epsilon = ANGLE_RAD_TOL) {
            x = vec.y;
            y = vec.z;
            angle = (y / x).atan().to_degrees();
            x_str = if x > 0.0 { "N" } else { "S" };
            y_str = if y > 0.0 { "U" } else { "D" };
        } else if abs_diff_eq!(vec.y, 0.0, epsilon = ANGLE_RAD_TOL) {
            x = vec.x;
            y = vec.z;
            angle = (y / x).atan().to_degrees();
            x_str = if x > 0.0 { "E" } else { "W" };
            y_str = if y > 0.0 { "U" } else { "D" };
        } else if abs_diff_eq!(vec.z, 0.0, epsilon = ANGLE_RAD_TOL) {
            x = vec.x;
            y = vec.y;
            angle = (y / x).atan().to_degrees();
            x_str = if x > 0.0 { "E" } else { "W" };
            y_str = if y > 0.0 { "N" } else { "S" };
        }
        angle = f32_round_3(angle);
        if angle.abs() < ANGLE_RAD_TOL {
            return x_str.to_string();
        }

        if angle < 0.0 {
            angle = 90.0 + angle;
            if angle > 45.0 {
                let angle = 90.0 - angle;
                return format!("{x_str} {angle} {y_str}");
            } else {
                return format!("{y_str} {angle} {x_str}");
            }
        }
        if angle > 45.0 {
            let angle = 90.0 - angle;
            return format!("{y_str} {angle} {x_str}");
        }

        if angle.is_nan(){
            return "unset".to_string();
        }

        return format!("{x_str} {angle} {y_str}");
    }

    //2个象限的组合, 最后一个留给Z轴
    let plane_vec = Vec3::new(vec.x, vec.y, 0.0);
    let part_str = to_pdms_vec_str(&plane_vec);
    let l = plane_vec.length();
    let mut theta = (vec.z / l).atan().to_degrees();
    let mut z_str = "U";
    theta = f32_round_3(theta);
    if theta < 0.0 {
        theta = -theta;
        z_str = "D";
    }
    if theta < ANGLE_RAD_TOL {
        return format!("{part_str}");
    }

    format!("{part_str} {theta} {z_str}")
}

#[inline]
pub fn to_pdms_ori_str(rot: &Mat3) -> String {
    let y_axis = &rot.y_axis;
    let z_axis = &rot.z_axis;

    format!(
        "Y is {} and Z is {}",
        to_pdms_vec_str(y_axis),
        to_pdms_vec_str(z_axis)
    )
}

#[inline]
pub fn to_pdms_ori_xyz_str(rot: &Mat3) -> String {
    let y_axis = &rot.y_axis;
    let z_axis = &rot.z_axis;
    let s = to_pdms_vec_str(y_axis);
    //讲s里面的 "E" 替换成 "X", "N" 替换成 "Y", "U" 替换成 "Z", "W" 替换成 "-X", "S" 替换成 "-Y", "D" 替换成 "-Z"
    format!(
        "Y is {} and Z is {}",
        convert_to_xyz(&to_pdms_vec_str(y_axis)),
        convert_to_xyz(&to_pdms_vec_str(z_axis))
    )
}

#[inline]
pub fn quat_to_pdms_ori_str(rot: &Quat) -> String {
    let rot = Mat3::from_quat(*rot);
    let y_axis = &rot.y_axis;
    let z_axis = &rot.z_axis;

    // "E".to_string()
    format!(
        "Y is {} and Z is {}",
        to_pdms_vec_str(y_axis),
        to_pdms_vec_str(z_axis)
    )
}

#[inline]
pub fn quat_to_pdms_ori_xyz_str(rot: &Quat) -> String {
    let rot = Mat3::from_quat(*rot);
    let y_axis = &rot.y_axis;
    let z_axis = &rot.z_axis;

    // "E".to_string()
    format!(
        "Y is {} and Z is {}",
        convert_to_xyz(&to_pdms_vec_str(y_axis)),
        convert_to_xyz(&to_pdms_vec_str(z_axis))
    )
}
