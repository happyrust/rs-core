use bevy_ecs::system::Resource;
use serde::{Deserialize,Serialize};

#[derive(Resource, Debug, Default, Clone,Deserialize,Serialize)]
pub struct SearchConditionSave {
    pub user: String,
    pub name: String,
    pub major: String,
    pub note: String,
    pub condition: Vec<String>,
}