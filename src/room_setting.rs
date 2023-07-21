use crate::ssc_setting::SiteData;
use bevy_ecs::prelude::Resource;
use serde::{Serialize, Deserialize};

#[derive(Resource, Serialize, Deserialize, Debug, Default, Clone)]
pub struct RoomSiteVec {
    pub data: Vec<SiteData>,
}

#[derive(Resource, Serialize, Deserialize, Debug, Default, Clone)]
pub struct SelectedRoomSiteVec {
    pub data: Vec<SiteData>,
}
