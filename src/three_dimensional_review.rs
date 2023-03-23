
use serde::{Serialize, Deserialize};


#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ModelDataCrate {
    pub ProjCode: String,
    pub UserCode: String,
    pub SiteCode: String,
    pub SiteName: String,
    pub ModelData: Vec<Vec<(String, String)>>,
    pub FlowData: (String, String),
    pub ViewData: (String, String),
    pub KeyValue:String,
}
