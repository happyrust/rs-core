use bevy_math::prelude::Vec3;
use crate::pdms_types::RefU64;
use bevy_ecs::system::Resource;
use serde::{Serialize,Deserialize};

#[derive(Resource,Serialize, Deserialize, Debug, Default, Clone)]
pub struct PenetrationData {
    pub refno: RefU64,
    pub name: String,
    pub position: Vec3,
}

#[derive(Resource,Serialize, Deserialize, Debug, Default, Clone)]
pub struct PenetrationVec {
    pub data: Vec<PenetrationData>,
}