#![feature(mixed_integer_ops)]
// #![feature(drain_filter)]
#![feature(let_chains)]
#![feature(trivial_bounds)]

#[allow(unused_mut)]
use std::collections::BTreeMap;
use std::fs::File;
use std::io::Read;
use bincode::Config;
use dashmap::DashMap;

use crate::pdms_types::PdmsDatabaseInfo;

extern crate bitflags;
extern crate phf;
extern crate core;

pub mod aql_types;
pub mod pdms_types;
pub mod consts;
pub mod prim_geo;
pub mod csg;
pub mod shape;
pub mod tool;
pub mod parsed_data;
pub mod pdms_data;
pub mod axis_param;
pub mod bevy_types;
pub mod geom_types;
pub mod helper;
pub mod db_number;
pub mod cache;
pub mod tiny_expr;
pub mod accel_tree;
pub mod plot_struct;
// 全自动出图所需的结构体
pub mod pdms_user;
pub mod plat_user;
pub mod metadata_manager;
pub mod create_attas_structs;
pub mod three_dimensional_review;
pub mod data_center;
pub mod negative_mesh_type;
pub mod rvm_types;
pub mod options;
pub mod datacenter_options;
pub mod pdms_pluggin;
pub mod virtual_hole;
pub mod ssc_setting;
pub mod version_control;
pub mod penetration;
pub mod vague_search;
pub mod bin_data;

pub mod achiver;
pub mod plugging_material;
pub mod room_setting;
pub mod water_calculation;
pub mod enso_types;

pub mod test;
pub mod data_state;

pub mod schema;
pub mod orm;
pub mod types;


pub type BHashMap<K, V> = BTreeMap<K, V>;

use once_cell_serde::sync::OnceCell;
use crate::options::DbOption;

///获取默认的数据库属性元数据信息
pub fn get_default_pdms_db_info() -> &'static PdmsDatabaseInfo {
    static INSTANCE: OnceCell<PdmsDatabaseInfo> = OnceCell::new();
    INSTANCE.get_or_init(|| {
        //会动态维护这个json，所以需要通过文件来加载
        let mut file = File::open("all_attr_info.json").unwrap();
        let mut string = String::new();
        file.read_to_string(&mut string);
        let dbinfo = serde_json::from_str::<PdmsDatabaseInfo>(&string).unwrap();
        // dbinfo.fix();
        // dbinfo.save(None);
        dbinfo
    })
}

pub fn get_uda_info() -> &'static (DashMap<u32, String>, DashMap<String, u32>) {
    static INSTANCE: OnceCell<(DashMap<u32, String>, DashMap<String, u32>)> = OnceCell::new();
    INSTANCE.get_or_init(|| {
        let mut ukey_udna_map = DashMap::new();
        let mut udna_ukey_map = DashMap::new();
        use config::{Config, ConfigError, Environment, File};
        let Ok(s) = Config::builder()
            .add_source(File::with_name("DbOption"))
            .build() else { return (DashMap::new(),DashMap::new()) };
        let db_option: DbOption = s.try_deserialize().unwrap();
        for project in db_option.included_projects {
            let path = format!("{}_uda.bin", project);
            if let Ok(mut file) = std::fs::File::open(path) {
                let mut data = Vec::new();
                let _ = file.read_to_end(&mut data);
                let map = bincode::deserialize::<DashMap<u32, String>>(&data).unwrap_or_default();
                for (k, v) in map {
                    ukey_udna_map.entry(k).or_insert(v.to_string());
                    udna_ukey_map.entry(v).or_insert(k);
                }
            }
        }
        (ukey_udna_map, udna_ukey_map)
    })
}