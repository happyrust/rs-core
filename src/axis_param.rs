use std::ops::Neg;
use glam::{Vec3, Vec3A};
use crate::parsed_data::CateAxisParam;

impl Neg for CateAxisParam {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self {
            pt: self.pt.clone(),
            dir: vec![-self.dir[0].clone(), -self.dir[1].clone(), -self.dir[2].clone()],
            pconnect: self.pconnect.clone(),
            pbore: self.pbore.clone()
        }
    }
}

impl CateAxisParam {
    pub fn zero() -> Self {
        Self {
            pt: vec![0.0; 3],
            dir: vec![0.0; 3],
            pconnect: "".to_string(),
            pbore: 0.0
        }
    }

    pub fn x() -> Self {
        Self {
            pt: vec![0.0; 3],
            dir: vec![1.0, 0.0, 0.0],
            pconnect: "".to_string(),
            pbore: 0.0
        }
    }

    pub fn y() -> Self {
        Self {
            pt: vec![0.0; 3],
            dir: vec![0.0, 1.0, 0.0],
            pconnect: "".to_string(),
            pbore: 0.0
        }
    }

    pub fn z() -> Self {
        Self {
            pt: vec![0.0; 3],
            dir: vec![0.0, 0.0, 1.0],
            pconnect: "".to_string(),
            pbore: 0.0
        }
    }

    pub fn get_dir_as_vec3(&self) -> Option<Vec3> {
        if self.pt.len() == 3 && self.dir.len() == 3 {
            Some(Vec3::new(self.dir[0] as f32, self.dir[1] as f32, self.dir[2] as f32))
        } else {
            None
        }
    }

    pub fn get_pt_as_vec3(&self) -> Option<Vec3> {
        if self.pt.len() == 3 && self.dir.len() == 3 {
            Some(Vec3::new(self.pt[0] as f32, self.pt[1] as f32, self.pt[2] as f32))
        } else {
            None
        }
    }

    pub fn get_dir_as_vec3a(&self) -> Option<Vec3A> {
        if self.pt.len() == 3 && self.dir.len() == 3 {
            Some(Vec3A::new(self.dir[0] as f32, self.dir[1] as f32, self.dir[2] as f32))
        } else {
            None
        }
    }

    pub fn get_pt_as_vec3a(&self) -> Option<Vec3A> {
        if self.pt.len() == 3 && self.dir.len() == 3 {
            Some(Vec3A::new(self.pt[0] as f32, self.pt[1] as f32, self.pt[2] as f32))
        } else {
            None
        }
    }

}
