use crate::pdms_types::RefU64;
use bevy_ecs::prelude::Resource;
use serde::Serialize;
use serde::Deserialize;

#[derive(Resource, Serialize, Deserialize, Debug, Default, Clone)]
pub struct PluggingData {
    pub refno:RefU64,
    pub name: String,
    pub size: String,
    pub room_1: String,
    pub room_2: String,
    pub cable_area: f32,
    pub plugging_area: f32,
    pub plugging_volume: f32,
    pub materials: String,
}

#[derive(Resource, Serialize, Deserialize, Debug, Default, Clone)]
pub struct PluggingVec {
    pub data: Vec<PluggingData>,
}