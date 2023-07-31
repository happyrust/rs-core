use std::collections::HashMap;
use crate::pdms_types::RefU64;
use bevy_ecs::prelude::Resource;
use serde::{Deserialize, Serialize};
use bevy_ecs::prelude::Component;
use bevy_ecs::prelude::Event;


#[derive(Resource, Debug, Default, Clone, Deserialize, Serialize)]
pub struct WaterComputeStp {
    //土建
    pub civil_engineering: Vec<HashMap<RefU64, Vec<CivilEngineeringStp>>>,
    //非土建
    pub non_civil_engineering: Vec<RefU64>,
}


///水淹计算中需要封堵的孔洞或门洞
#[derive(Resource, Debug, Default, Clone, Deserialize, Serialize)]
pub struct CivilEngineeringStp {
    pub wall_refno: RefU64,
    pub hole_refno: Option<RefU64>,
    pub door_refno: Option<RefU64>,
}

#[derive(Resource, Clone, Debug, Default, Deserialize, Serialize)]
pub struct FloodingHole {
    pub owner_refno: RefU64,
    pub refno: RefU64,
    pub name: String,
    pub is_selected: bool,
}


#[derive(Resource, Clone, Debug, Default, Deserialize, Serialize)]
pub struct FloodingHoleVec {
    pub data: Vec<FloodingHole>,
}


#[derive(Component, Clone, Debug, Default, Event, Deserialize, Serialize)]
pub struct ExportFloodingStpEvent {
    pub stp: WaterComputeStp,
    pub refnos: Vec<(RefU64, RefU64)>,
}
