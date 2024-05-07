use crate::RefU64;
use bevy_ecs::system::Resource;
use serde::{Deserialize, Serialize};
use serde_json::{json, to_string_pretty};
use serde_with::DisplayFromStr;
use std::fmt::format;
use surrealdb::sql::Thing;
use crate::pdms_types::PdmsElement;

#[derive(Serialize, Deserialize, Clone, Debug, Resource, Default)]
pub struct SPdmsElement {
    //指向具体的类型
    pub refno: RefU64,
    pub owner: RefU64,
    pub name: String,
    pub noun: String,
    pub dbnum: i32,
    pub e3d_version: i32,
    ///大版本号
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_tag: Option<String>,
    ///小版本号
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_tag: Option<String>,
    #[serde(skip)]
    pub cata_hash: String,
    ///锁定模型
    pub lock: bool,
    pub deleted: bool,
}

impl SPdmsElement {
    pub fn gen_sur_json(&self) -> String {
        let mut json_string = to_string_pretty(&json!({
            "name": self.name,
            "noun": self.noun,
            "dbnum": self.dbnum,
            "e3d_version": self.e3d_version,
            "version_tag": self.version_tag,
            "status_tag": self.status_tag,
            "cata_hash": self.cata_hash,
            "lock": self.lock,
            "deleted": self.deleted,
        }))
        .unwrap();
        json_string.remove(json_string.len() - 1);
        json_string.push_str(",");
        json_string.push_str(&format!(
            r#""refno": {},"#,
            self.refno.to_table_key(&self.noun)
        ));
        json_string.push_str(&format!(r#""id": {},"#, self.refno.to_pe_key()));
        json_string.push_str(&format!(r#""owner": {}"#, self.owner.to_pe_key()));
        json_string.push_str("}");
        json_string
    }

    #[inline]
    pub fn get_type_str(&self) -> &str {
        return self.noun.as_str();
    }

    #[inline]
    pub fn get_owner(&self) -> RefU64 {
        return self.owner;
    }
}
