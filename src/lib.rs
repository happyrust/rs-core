#![feature(default_free_fn)]
#![feature(mixed_integer_ops)]
#![feature(drain_filter)]
#![feature(let_chains)]
#[allow(unused_mut)]
use std::collections::BTreeMap;

extern crate bitflags;
extern crate phf;

pub mod pdms_types;
pub mod consts;
pub mod prim_geo;
pub mod shape;
pub mod tool;
pub mod parsed_data;
pub mod pdms_data;
pub mod axis_param;
pub mod bevy_types;
pub mod rvm_types;
pub mod helper;
pub mod db_number;
pub mod cache;
pub mod tiny_expr;
pub mod accel_tree;
pub mod plot_struct; // 全自动出图所需的结构体
pub mod pdms_user;
pub mod metadata_manager;
pub mod create_attas_structs;
pub mod three_dimensional_review;
pub mod data_center;


// pub type BHashMap<K, V> = bevy::utils::HashMap<K, V>;
pub type BHashMap<K, V> = BTreeMap<K, V>;
