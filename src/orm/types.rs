use bevy_reflect::Reflect;
use sea_orm::entity::prelude::*;
use serde::{Serialize, Deserialize};
use sea_orm::FromJsonQueryResult;
use bevy_reflect::prelude::ReflectDefault;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default, FromJsonQueryResult, Reflect)]
#[reflect(Default)]
pub struct StringVec(pub Vec<String>);

#[derive(Serialize, Deserialize,Clone, Debug, PartialEq, Default,  FromJsonQueryResult, Reflect)]
#[reflect(Default)]
pub struct F32Vec(pub Vec<f32>);

#[derive(Serialize, Deserialize,Clone, Debug, PartialEq, Default,  FromJsonQueryResult, Reflect)]
#[reflect(Default)]
pub struct I32Vec(pub Vec<i32>);

#[derive(Serialize, Deserialize,Clone, Debug, PartialEq, Default,  FromJsonQueryResult, Reflect)]
#[reflect(Default)]
pub struct BoolVec(pub Vec<bool>);
