#![feature(default_free_fn)]
#![feature(mixed_integer_ops)]

use std::collections::BTreeMap;

#[allow(unused_mut)]

pub mod pdms_types;
pub mod consts;
pub mod prim_geo;
pub mod shape;
pub mod tool;
pub mod parsed_data;
pub mod pdms_data;
pub mod axis_param;
pub mod bevy_types;
pub mod helper;
pub mod db_number;

// pub type BHashMap<K, V> = bevy::utils::HashMap<K, V>;
pub type BHashMap<K, V> = BTreeMap<K, V>;
