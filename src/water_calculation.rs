use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
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
    pub file_name: String,
    pub save_time: String,
    pub stp: WaterComputeStp,
    pub refnos: Vec<(RefU64, RefU64)>,
    pub model_list: Vec<(RefU64, String)>,
    pub all_hole_list: Vec<FloodingHole>,
    pub selected_hole_list: Vec<FloodingHole>,
}

impl ExportFloodingStpEvent {
    pub fn to_arango_struct(self) -> FloodingStpToArangodb {
        let mut hasher = DefaultHasher::new();
        self.file_name.hash(&mut hasher);
        let hash_name = hasher.finish();
        FloodingStpToArangodb {
            _key: hash_name.to_string(),
            save_time: self.save_time,
            file_name: self.file_name,
            model_list: self.model_list,
            all_hole_list: self.all_hole_list,
            selected_hole_list: self.selected_hole_list,
        }
    }
}


#[derive(Component, Clone, Debug, Default, Event, Deserialize, Serialize)]
pub struct FloodingStpToArangodb {
    pub _key: String,
    pub save_time: String,
    pub file_name: String,
    pub model_list: Vec<(RefU64, String)>,
    pub all_hole_list: Vec<FloodingHole>,
    pub selected_hole_list: Vec<FloodingHole>,
}

#[derive(Component, Resource, Clone, Debug, Default, Event, Deserialize, Serialize)]
pub struct FloodingStpToArangodbVec {
    pub data: Vec<FloodingStpToArangodb>,
}

#[derive(Component, Clone, Debug, Default, Event, Deserialize, Serialize)]
pub struct DownloadStpEvent {
    pub file_name: String,
    pub contents: Vec<u8>,
}
