use serde_with::serde_as;
use crate::pdms_types::RefU64;
use serde::{Serialize, Deserialize};
use sea_orm::entity::prelude::*;
use serde_with::DisplayFromStr;


// #[derive(Copy, Clone, Default, Debug, DeriveEntity)]
// pub struct Entity;

// #[derive(Copy, Clone, Debug, EnumIter)]
// pub enum Relation {
//     Cake,
// }

#[serde_as]
#[derive(Serialize, Deserialize, Clone, Debug, Default, DeriveEntityModel)]
#[sea_orm(table_name = "PdmsElement")]
pub struct Model {
    // #[serde(rename="@id")]
    #[sea_orm(primary_key)]
    pub id: String,
    #[serde_as(as = "DisplayFromStr")]
    pub refno: RefU64,
    // #[serde(serialize_with = "ser_refno_as_ref_type")]
    // #[serde(skip_serializing_if = "is_zero")]
    #[serde_as(as = "DisplayFromStr")]
    pub owner: RefU64,
    pub name: String,
    pub noun: String,
    // #[serde(default)]
    // pub order: u32,
    pub dbnum: i32,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_tag: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cata_hash: Option<String>,
}


#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}