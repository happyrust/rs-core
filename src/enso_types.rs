use crate::pdms_types::NamedAttrValue;
use serde::{Serialize,Deserialize};
use bevy_ecs::prelude::Component;

#[derive(Serialize, Deserialize, Clone, Debug, Component)]
pub struct EnsoJson {
    pub header: Vec<String>,
    pub value: Vec<Vec<NamedAttrValue>>
}