use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct SearchConditionSave {
    pub user: String,
    pub name: String,
    pub major: String,
    pub note: String,
    pub condition: Vec<String>,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct SearchConditionVec {
    pub data: Vec<SearchConditionSave>,
}
