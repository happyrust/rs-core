use bevy::prelude::Transform;
use glam::{Quat, Vec3};
use parry3d::bounding_volume::Aabb;
use serde::{Serialize, Deserialize};
use crate::parsed_data::geo_params_data::CateGeoParam;
use crate::pdms_types::{EleGeoInstanceJson, RefU64};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct RvmGeoInfo {
    pub _key: String,
    pub aabb: Option<Aabb>,
    pub data: Vec<EleGeoInstanceJson>,
    // 相对世界坐标系下的变换矩阵 rot, translation, scale
    pub world_transform: (Quat, Vec3, Vec3),
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct GeomsInfoAql {
    pub _key: String,
    pub geo_params: Vec<GeoParaInfo>,
}

impl GeomsInfoAql {
    pub fn get_refno(&self) -> anyhow::Result<RefU64> {
        RefU64::from_refno_str(&self._key)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GeoParaInfo {
    pub aabb: Aabb,
    pub geometry: CateGeoParam,
    pub transform: (Quat, Vec3, Vec3),
}