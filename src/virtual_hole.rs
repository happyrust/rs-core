use bevy::ecs::system::Resource;
use serde::Deserialize;
use serde::Serialize;
use crate::data_center::DataCenterProject;

#[derive(Resource, PartialEq, Default, Debug, Serialize, Deserialize)]
pub struct PersonnelInfo {
    #[serde(rename = "人员工号")]
    pub job_num: String,
    #[serde(rename = "人员名称")]
    pub name: String,
}


#[derive(PartialEq, Resource, Default, Debug, Serialize, Deserialize)]
pub struct PersonnelInfoVec {
    pub data: Vec<PersonnelInfo>,
}