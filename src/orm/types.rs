use sea_orm::entity::prelude::*;
use serde::{Serialize, Deserialize};
use sea_orm::FromJsonQueryResult;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default, FromJsonQueryResult)]
pub struct StringVec(pub Vec<String>);

#[derive(Serialize, Deserialize,Clone, Debug, PartialEq, Default,  FromJsonQueryResult)]
pub struct F32Vec(pub Vec<f32>);

#[derive(Serialize, Deserialize,Clone, Debug, PartialEq, Default,  FromJsonQueryResult)]
pub struct I32Vec(pub Vec<i32>);

#[derive(Serialize, Deserialize,Clone, Debug, PartialEq, Default,  FromJsonQueryResult)]
pub struct BoolVec(pub Vec<bool>);
