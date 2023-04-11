use bevy::prelude::Resource;
use serde::{Serialize, Deserialize};
use crate::pdms_types::RefU64;


#[derive(Debug, Default, Clone, Serialize, Deserialize, Resource)]
pub struct ThreeDimensionalModelDataCrate {
    #[serde(rename = "KeyValue")]
    pub key_value: String,
    #[serde(rename = "ProjCode")]
    pub proj_code: String,
    #[serde(rename = "UserCode")]
    pub user_code: String,
    #[serde(rename = "SiteCode")]
    pub site_code: String,
    #[serde(rename = "SiteName")]
    pub site_name: String,
    #[serde(rename = "UserRole")]
    pub user_role: String,
    #[serde(rename = "ModelData")]
    pub model_data: (Vec<(RefU64, String)>, Vec<Vec<(String, String)>>),
    #[serde(rename = "FlowPicData")]
    pub flow_pic_data: ThreeDimensionalReviewComment,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ThreeDimensionalModelDataToArango {
    // #[serde(rename = "KeyValue")]
    pub _key: String,
    // pub key_value: String,
    #[serde(rename = "ProjCode")]
    pub proj_code: String,
    #[serde(rename = "UserCode")]
    pub user_code: String,
    #[serde(rename = "SiteCode")]
    pub site_code: String,
    #[serde(rename = "SiteName")]
    pub site_name: String,
    #[serde(rename = "UserRole")]
    pub user_role: String,
    #[serde(rename = "ModelData")]
    pub model_data:(Vec<(RefU64, String)>, Vec<Vec<(String, String)>>),
    #[serde(rename = "FlowPicData")]
    pub flow_pic_data: ThreeDimensionalReviewComment,
}


#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ThreeDimensionalReviewComment {
    pub validation: Vec<ThreeDimensionalReviewData>,
    pub review: Vec<ThreeDimensionalReviewData>,
    pub approval: Vec<ThreeDimensionalReviewData>,
    pub endorsement: Vec<ThreeDimensionalReviewData>,
}


#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ThreeDimensionalReviewData {
    pub comment: Vec<String>,
    pub reply: String,
    pub associatedElement: Vec<RefU64>,
    pub cloudLine: String,
    pub viewpoint: String,
    pub image: String,
}
