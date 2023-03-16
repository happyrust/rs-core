
use serde::{Serialize, Deserialize};


#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ModelDataCrate {
    pub proj_code: String,
    pub user_code: String,
    pub site_code: String,
    pub site_name: String,
    pub model_data: Vec<Vec<(String, String)>>,
    pub flow_data: (String, String),
    pub view_data: (String, String),
    pub key_value:String,
}
