#![feature(default_free_fn)]
#[allow(unused_mut)]

pub mod pdms_types;
pub mod consts;
pub mod prim_geo;
pub mod shape;
pub mod tool;
pub mod parsed_data;
pub mod pdms_data;
pub mod axis_param;

pub type BHashMap<K, V> = bevy::utils::HashMap<K, V>;