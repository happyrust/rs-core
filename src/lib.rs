#![feature(default_free_fn)]
#![feature(mixed_integer_ops)]
#[allow(unused_mut)]
use std::collections::BTreeMap;

extern crate bitflags;

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
pub mod cache;
pub mod tiny_expr;

// pub type BHashMap<K, V> = bevy::utils::HashMap<K, V>;
pub type BHashMap<K, V> = BTreeMap<K, V>;
