use serde_derive::{Deserialize, Serialize};
use axum_login::AuthUser;
use axum_login::secrecy::SecretVec;
use bevy::prelude::Resource;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Role {
    Designer,
    Proofreader,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize,Resource)]
pub struct PdmsUser {
    pub user_name: String,
    pub user_major: String,
    pub b_designer: bool,
}

impl PdmsUser {
    pub fn test_user() -> Self {
        Self {
            user_name: "test".to_string(),
            user_major: "test".to_string(),
            b_designer: true,
        }
    }
    pub fn test_system() -> Self {
        Self {
            user_name: "system".to_string(),
            user_major: "system".to_string(),
            b_designer: false,
        }
    }
}

impl PdmsUser {
    pub fn get_name(&self) -> String {
        self.user_name.to_string()
    }

    pub fn get_major(&self) -> String {
        self.user_major.to_string()
    }
}

impl AuthUser<Role> for PdmsUser {
    fn get_id(&self) -> String { self.user_name.clone() }

    fn get_password_hash(&self) -> SecretVec<u8> { SecretVec::new(vec![]) }
}