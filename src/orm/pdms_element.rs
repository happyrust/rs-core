use crate::impl_db_op_trait;
use crate::orm::traits::DbOpTrait;
use crate::types::*;
use sea_orm::{DatabaseBackend, QueryTrait, Schema, entity::prelude::*};
use serde::{Deserialize, Serialize};
use serde_with::DisplayFromStr;
use serde_with::serde_as;
use surrealdb::types::RecordId;

#[serde_as]
#[derive(Serialize, Deserialize, Clone, Debug, Default, DeriveEntityModel)]
#[sea_orm(table_name = "PdmsElement")]
pub struct Model {
    //todo 用来作为sql的主键
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    #[serde_as(as = "DisplayFromStr")]
    pub refno: RefU64,
    #[serde_as(as = "DisplayFromStr")]
    pub owner: RefU64,
    pub name: String,
    pub noun: String,
    pub dbnum: i32,
    pub sesno: i32,
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
}

impl_db_op_trait!();

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    #[inline]
    pub fn get_type_str(&self) -> &str {
        return self.noun.as_str();
    }
    #[inline]
    pub fn get_owner(&self) -> RefU64 {
        return self.owner;
    }
}
