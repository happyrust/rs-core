use crate::pdms_types::RefU64;
use bevy_ecs::system::Resource;
use serde::{Serialize,Deserialize};
#[derive(Resource,Serialize, Deserialize, Debug, Default, Clone)]
pub struct SiteData {
    pub refno: RefU64,
    pub name: String,
    pub is_selected: bool,
}

#[derive(Resource,Serialize, Deserialize, Debug, Default, Clone)]
pub struct SiteVec {
    pub data: Vec<SiteData>,
}

#[derive(Resource,Serialize, Deserialize, Debug, Default, Clone)]
pub struct SelectedSiteVec {
    pub data: Vec<SiteData>,
}
