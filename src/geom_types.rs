use bevy::prelude::Transform;
use glam::{Quat, Vec3};
use parry3d::bounding_volume::Aabb;
use serde::{Serialize, Deserialize};
use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::pdms_types::{EleGeoInstance, RefU64};

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct RvmGeoInfo {
    pub _key: String,
    pub aabb: Option<Aabb>,
    pub data: Vec<EleGeoInstance>,
    // 相对世界坐标系下的变换矩阵 rot, translation, scale
    pub world_transform: Transform,
}