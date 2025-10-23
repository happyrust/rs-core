use crate::parsed_data::CateAxisParam;
use crate::shape::pdms_shape::RsVec3;
use glam::Vec3;
use std::ops::Neg;

impl Neg for CateAxisParam {
    type Output = Self;

    fn neg(self) -> Self::Output {
        let mut result = self.clone();
        result.dir = result.dir.map(|x| -x);
        result.dir_flag = -1.0;
        result
    }
}

impl CateAxisParam {
    pub fn zero() -> Self {
        Self {
            pt: RsVec3(Vec3::ZERO),
            dir: None,
            pconnect: "".to_string(),
            ..Default::default()
        }
    }

    pub fn x() -> Self {
        Self {
            pt: RsVec3(Vec3::ZERO),
            dir: Some(RsVec3(Vec3::X)),
            pconnect: "".to_string(),
            ..Default::default()
        }
    }

    pub fn y() -> Self {
        Self {
            pt: RsVec3(Vec3::ZERO),
            dir: Some(RsVec3(Vec3::Y)),
            pconnect: "".to_string(),
            ..Default::default()
        }
    }

    pub fn z() -> Self {
        Self {
            pt: RsVec3(Vec3::ZERO),
            dir: Some(RsVec3(Vec3::Z)),
            pconnect: "".to_string(),
            ..Default::default()
        }
    }
}
