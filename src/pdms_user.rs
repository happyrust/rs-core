use serde_derive::{Deserialize, Serialize};


#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PdmsUser {
    pub UserCode: String,
    pub UserName: String,
    pub UserMajor: String,
}

impl PdmsUser {
    pub fn test_user() -> Self {
        Self {
            UserCode: "1".to_string(),
            UserName: "test".to_string(),
            UserMajor: "test".to_string(),
        }
    }
}

impl PdmsUser {
    pub fn get_name(&self) -> String {
        self.UserName.to_string()
    }

    pub fn get_major(&self) -> String {
        self.UserMajor.to_string()
    }
}