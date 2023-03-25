use serde::{Serialize, Deserialize};
use crate::pdms_types::RefU64;


#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ModelDataCrate {
    #[serde(rename="KeyValue")]
    pub key_value: String,
    #[serde(rename="ProjCode")]
    pub proj_code: String,
    #[serde(rename="UserCode")]
    pub user_code: String,
    #[serde(rename="SiteCode")]
    pub site_code: String,
    #[serde(rename="SiteName")]
    pub site_name: String,
    #[serde(rename="UserRole")]
    pub user_role: String,
    #[serde(rename="ModelData")]
    pub model_data: Vec<Vec<(String, String)>>,
    #[serde(rename="FlowPicData")] 
    pub flow_pic_data: Vec<ReviewModelData>,

}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ReviewModelData {
    pub comment: Vec<String>,
    pub reply: String,
    pub associatedElement: Vec<RefU64>,
    pub cloudLine: String,
    pub viewpoint: String,
    pub image: String,
}
