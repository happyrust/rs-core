use crate::ssc_setting::SiteData;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct RoomSiteVec {
    pub data: Vec<SiteData>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct RoomSelectedSiteVec {
    pub data: Vec<SiteData>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct FlagRoomSiteVec {
    pub data: Vec<SiteData>,
}
