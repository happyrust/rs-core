use bevy::prelude::{Quat, Vec3};

#[inline]
pub fn cal_ref_axis(v: &Vec3) -> Vec3 {
    let v = v.normalize();
    let a = v.x.abs();
    let b = v.y.abs();
    let c = v.z.abs();
    let mut dx = Vec3::new(1.0f32, 0.0, 0.0);
    if b <= a && b <= c {
        dx = Vec3::new(-v.z as f32, 0.0, v.x);
    } else if a <= b && a <= c {
        dx = Vec3::new(0.0, -v.z, v.y);
    } else {
        dx = Vec3::new(-v.y, v.x, 0.0);
    }
    dx
}

///针对torus的角度求解
pub fn rotate_from_vec3_to_vec3(dir: Vec3, from: Vec3, to: Vec3) -> Quat {
    let mut angle = from.angle_between(to);
    if angle.abs() < 1.0e-3 || (angle.abs() - std::f32::consts::PI).abs() < 1.0e-3 {
        let mut rotation_angle = angle;
        let mut ref_axis = dir;
        Quat::from_axis_angle(
            from.cross(ref_axis).normalize(),
            rotation_angle,
        )
    } else {
        // 不平行
        let a1 = dir.angle_between(from);
        let a2 = dir.angle_between(to);

        let mut z_dir = from.cross(to).normalize();
        if ((a1 + a2) - angle).abs() > 1.0e-3 {
            angle =  std::f32::consts::TAU - angle;
            z_dir = -z_dir;
        }
        Quat::from_axis_angle(z_dir, angle)
    }
}

pub fn quad_indices(indices: &mut Vec<usize>, l: &mut usize, o: usize, v0: usize, v1: usize, v2: usize, v3: usize){
    indices.push(o + v0);
    indices.push(o + v1);
    indices.push(o + v2);
    indices.push(o + v2);
    indices.push(o + v3);
    indices.push(o + v0);
    *l += 6;
}


#[derive(Default, Debug)]
pub struct RotateInfo {
    pub center: Vec3,
    pub angle: f32,
    pub rot_axis: Vec3,
    pub radius: f32,
}

impl RotateInfo {
    pub fn cal_rotate_info(paax_dir: Vec3, paax_pt: Vec3, pbax_dir: Vec3, pbax_pt: Vec3) -> Option<RotateInfo> {
        let mut rotate_info = RotateInfo::default();
        let pa_dir = Vec3::new(paax_dir.x, paax_dir.y, paax_dir.z).normalize();
        let pb_dir = Vec3::new(pbax_dir.x, pbax_dir.y, pbax_dir.z).normalize();
        let x_dir = (pbax_pt - paax_pt).normalize();
        let quat = rotate_from_vec3_to_vec3(x_dir, -pa_dir, pb_dir);
        let (mut axis_z, angle) = quat.to_axis_angle();
        rotate_info.rot_axis = axis_z;
        rotate_info.angle = angle.to_degrees();
        let mid_pt = (paax_pt + pbax_pt) / 2.0;
        let x_len = pbax_pt.distance(paax_pt);
        if x_len < 1.0e-3 {
            return None;
        }
        if (angle - std::f32::consts::PI).abs() < 1.0e-3 || angle.abs() < 1.0e-3 {
            rotate_info.center = mid_pt;
            rotate_info.radius = x_len / 2.0;
        } else {
            let mut y_dir = rotate_info.rot_axis.cross(x_dir);
            let ref_dir = rotate_info.rot_axis.cross(pbax_dir.normalize()).normalize();
            let p = pbax_pt - mid_pt;
            let px = p.dot(x_dir);
            let _py = p.dot(y_dir);
            if px < 1.0e-3 {
                return None;
            }
            let beta = angle / 2.0;
            rotate_info.radius = px / beta.sin().abs();
            rotate_info.center = pbax_pt + ref_dir * rotate_info.radius;
        }
        return Some(rotate_info);
    }
}