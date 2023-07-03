use bevy_math::prelude::Vec3;
use crate::pdms_types::RefU64;
use bevy_ecs::system::Resource;
use serde::{Serialize,Deserialize};

#[derive(Resource,Serialize, Deserialize, Debug, Default, Clone)]
pub struct PenetrationData {
    pub owner_refno: RefU64,
    pub refno:RefU64,
    pub name: String,
    pub position: Vec3,
    pub x_deviation_angle:String,
    pub inner_room_num:String,
    pub outer_room_num:String,
}

#[derive(Resource,Serialize, Deserialize, Debug, Default, Clone)]
pub struct PenetrationVec {
    pub data: Vec<PenetrationData>,
}