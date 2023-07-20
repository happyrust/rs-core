use std::default;
use std::ops::Neg;
use glam::{Vec3, Vec3A};
use crate::parsed_data::CateAxisParam;

impl Neg for CateAxisParam {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self {
            dir: Vec3::new(-self.dir[0].clone(), -self.dir[1].clone(), -self.dir[2].clone()),
            ..self.clone()
        }
    }
}

impl CateAxisParam {
    pub fn zero() -> Self {
        Self {
            pt: Vec3::ZERO,
            dir: Vec3::ZERO,
            pconnect: "".to_string(),
            ..Default::default()
        }
    }

    pub fn x() -> Self {
        Self {
            pt: Vec3::ZERO,
            dir: Vec3::X,
            pconnect: "".to_string(),
            ..Default::default()
        }
    }

    pub fn y() -> Self {
        Self {
            pt: Vec3::ZERO,
            dir: Vec3::Y,
            pconnect: "".to_string(),
            ..Default::default()
        }
    }

    pub fn z() -> Self {
        Self {
            pt: Vec3::ZERO,
            dir: Vec3::Z,
            pconnect: "".to_string(),
            ..Default::default()
        }
    }


}
