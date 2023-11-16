use std::fmt::format;
use serde_with::serde_as;
use serde::{Serialize, Deserialize};
use surrealdb::sql::Thing;
use crate::RefU64;
use serde_with::DisplayFromStr;
use regex::Regex;
use serde_json::{json, to_string_pretty};

#[serde_as]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SPdmsElement {
    //todo 用来作为sql的主键
    pub id: String,
    #[serde_as(as = "DisplayFromStr")]
    pub refno: RefU64,
    #[serde_as(as = "DisplayFromStr")]
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
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cata_hash: Option<String>,
    ///锁定模型
    pub lock: bool,
    pub deleted: bool,
}

impl SPdmsElement {
    pub fn gen_sur_json(&self) -> String {
        let mut json_string = to_string_pretty(&json!({
            "id": self.id,
            // "refno": self.refno.to_string(),
            // "owner": self.owner.to_string(),
            "name": self.name,
            "noun": self.noun,
            "dbnum": self.dbnum,
            "e3d_version": self.e3d_version,
            "version_tag": self.version_tag,
            "status_tag": self.status_tag,
            "cata_hash": self.cata_hash,
            "lock": self.lock,
            "deleted": self.deleted,
        })).unwrap();
        json_string.remove(json_string.len() - 1);
        json_string.push_str(",");
        json_string.push_str(&format!(r#""refno": {}:{},"#, &self.noun, self.refno.to_string()));
        json_string.push_str(&format!(r#""owner": pe:{}"#, self.owner.to_string()));
        json_string.push_str("}");
        // println!("json string: {}", &json_string);
        json_string
    }

    #[inline]
    pub fn get_type_str(&self) -> &str{
        return self.noun.as_str()
    }

    #[inline]
    pub fn get_owner(&self) -> RefU64{
        return self.owner
    }
}