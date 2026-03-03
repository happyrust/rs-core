#![feature(let_chains)]
#![feature(trivial_bounds)]
#![feature(result_flattening)]
#![feature(async_fn_track_caller)]
#![allow(warnings)]

use crate::error::HandleError;
use config::{Config, File};
use dashmap::DashMap;
#[allow(unused_mut)]
use std::collections::BTreeMap;
use std::io::Read;

pub use types::db_info::PdmsDatabaseInfo;

extern crate bitflags;
extern crate core;
extern crate phf;

// Re-export debug macros from debug_macros module
pub use debug_macros::*;

pub mod accel_tree;
pub mod aios_db_mgr;
pub mod attlib_parser;
pub mod axis_param;

pub mod basic;

pub mod parse; // 兼容下游依赖：保留 aios_core::parse 路径

pub mod plant_transform;
// pub mod cache; // 模块已删除
pub mod consts;
pub mod csg;
pub mod error;
pub mod geom_types;
pub mod table_const;

pub mod geometry;

pub mod helper;
#[cfg(feature = "live")]
pub mod live;
pub mod parsed_data;
pub mod pdms_data;
pub mod pdms_types;
pub mod plot_struct;
pub mod prim_geo;
pub mod shape;
pub mod tiny_expr;
pub mod tool;
pub mod vec3_pool;
// 全自动出图所需的结构体
pub mod bin_data;
pub mod create_attas_structs;
pub mod data_center;
pub mod datacenter_options;
pub mod dblist_parser;
pub mod metadata;
pub mod metadata_manager;
pub mod negative_mesh_type;
pub mod options;
pub mod pdms_pluggin;
pub mod pdms_user;
pub mod penetration;
pub mod plat_user;
pub mod rvm_types;
pub mod ssc_setting;
pub mod three_dimensional_review;
pub mod vague_search;
pub mod version_control;
pub mod virtual_hole;

pub mod achiver;
pub mod plugging_material;
pub mod room_setting;
pub mod water_calculation;

pub mod noun_graph;

pub mod data_state;
pub mod threed_review;

pub mod runtime;
pub mod transform;

#[cfg(feature = "test")]
pub mod test;

pub mod db_adapter;
pub mod db_pool;
#[cfg(feature = "sea-orm")]
pub mod orm;
pub mod query_provider;
pub mod rs_surreal;
pub mod schema;
pub mod sync;
pub mod tree_query;
pub mod types;

pub mod material;
pub mod math;
pub mod mesh_precision;
pub mod room;

pub mod file_helper;

pub mod petgraph;

pub mod db;

#[cfg(not(target_arch = "wasm32"))]
pub mod spatial;

pub mod dblist;

pub mod expression;

pub mod fast_model;
pub mod utils;

pub mod debug_macros;

pub mod color_scheme;

#[cfg(feature = "web_server")]
pub mod web_server;
pub use crate::plant_transform::Transform;
pub use crate::types::*;
pub use rs_surreal::*;
pub use runtime::{
    DbOptionSurrealExt, connect_local_rocksdb,
    init_surreal_with_retry, initialize_databases, is_surreal_server_running,
    start_surreal_server, stop_surreal_server, try_connect_database,
};
pub use tree_query::{
    DbMetaInfo, TreeIndex, TreeQuery, TreeQueryFilter, TreeQueryOptions, get_cached_tree_index,
    get_dbnum_by_ref0, get_dbnum_by_refno, get_tree_index_by_refno, load_db_meta_info,
    load_tree_index_from_dir, load_tree_index_from_path,
};

#[cfg(feature = "web_server")]
pub use web_server::{
    ConnectionConfig, ConnectionHandle, DeploymentConnectionPool, connect_with_config,
    create_required_tables, test_database_connection, verify_connection,
};

pub type BHashMap<K, V> = BTreeMap<K, V>;

use crate::function::define_common_functions;
use crate::options::{DbOption, SecondUnitDbOption};
use once_cell_serde::sync::OnceCell;
use surrealdb::opt::auth::Root;

/// 获取配置文件名，支持环境变量
fn get_config_file_name() -> String {
    std::env::var("DB_OPTION_FILE").unwrap_or_else(|_| "db_options/DbOption".to_string())
}

///获得db option
#[inline]
pub fn get_db_option() -> &'static DbOption {
    static INSTANCE: OnceCell<DbOption> = OnceCell::new();
    INSTANCE.get_or_init(|| {
        use config::{Config, ConfigError, Environment, File};

        let config_file_name = get_config_file_name();

        let s = Config::builder()
            .add_source(File::with_name(&config_file_name))
            .build()
            .unwrap();
        let mut option = s.try_deserialize::<DbOption>().unwrap();
        // 环境变量覆盖 surrealdb 连接模式（web_server auto_start 场景）
        if let Ok(mode) = std::env::var("SURREAL_CONN_MODE") {
            match mode.as_str() {
                "ws" => option.surrealdb.mode = options::DbConnMode::Ws,
                "file" => option.surrealdb.mode = options::DbConnMode::File,
                _ => {}
            }
        }
        if let Ok(ip) = std::env::var("SURREAL_CONN_IP") {
            option.surrealdb.ip = ip;
        }
        if let Ok(port) = std::env::var("SURREAL_CONN_PORT") {
            if let Ok(p) = port.parse::<u16>() {
                option.surrealdb.port = p;
            }
        }
        crate::mesh_precision::set_active_precision(option.mesh_precision.clone());
        option
    })
}

///获取默认的数据库属性元数据信息
pub fn get_default_pdms_db_info() -> &'static PdmsDatabaseInfo {
    static INSTANCE: OnceCell<PdmsDatabaseInfo> = OnceCell::new();
    INSTANCE.get_or_init(|| {
        //会动态维护这个json，所以需要通过文件来加载
        //使用feature，来选择是否加载文件，还是使用include_str
        let mut string = String::new();
        #[cfg(feature = "load_file")]
        {
            let mut file = File::open("all_attr_info.json").unwrap();
            file.read_to_string(&mut string);
        }

        #[cfg(not(feature = "load_file"))]
        {
            string = include_str!("../all_attr_info.json").to_string();
        }

        let mut db_info = serde_json::from_str::<PdmsDatabaseInfo>(&string).unwrap();
        db_info.fill_named_map();
        // dbinfo.fix();
        // dbinfo.save(None);
        db_info
    })
}

pub fn get_uda_info() -> &'static (DashMap<u32, String>, DashMap<String, u32>) {
    static INSTANCE: OnceCell<(DashMap<u32, String>, DashMap<String, u32>)> = OnceCell::new();
    INSTANCE.get_or_init(|| {
        let mut ukey_udna_map = DashMap::new();
        let mut udna_ukey_map = DashMap::new();
        use config::{Config, ConfigError, Environment, File};
        let config_file_name = get_config_file_name();
        let Ok(s) = Config::builder()
            .add_source(File::with_name(&config_file_name))
            .build()
        else {
            return (DashMap::new(), DashMap::new());
        };
        let db_option: DbOption = s.try_deserialize().unwrap();
        for project in db_option.included_projects {
            let path = format!("{}_uda.bin", project);
            if let Ok(mut file) = std::fs::File::open(path) {
                let mut data = Vec::new();
                let _ = file.read_to_end(&mut data);
                let map = serde_json::from_slice::<std::collections::HashMap<u32, String>>(&data)
                    .unwrap_or_default();
                for (k, v) in map {
                    ukey_udna_map.entry(k).or_insert(v.to_string());
                    udna_ukey_map.entry(v).or_insert(k);
                }
            }
        }
        (ukey_udna_map, udna_ukey_map)
    })
}

pub async fn init_test_surreal() -> Result<DbOption, HandleError> {
    let config_file_name = get_config_file_name();
    let s = Config::builder()
        .add_source(File::with_name(&config_file_name))
        .build()
        .map_err(|e| HandleError::SurrealError {
            msg: format!("Failed to load DbOption config: {}", e),
        })?;
    let db_option: DbOption = s.try_deserialize().map_err(|e| HandleError::SurrealError {
        msg: format!("Failed to deserialize DbOption: {}", e),
    })?;

    // 创建配置
    let config = surrealdb::opt::Config::default().ast_payload(); // 启用AST格式

    // Connect to database
    SUL_DB
        .connect((db_option.get_version_db_conn_str(), config))
        .with_capacity(1000)
        .await
        .map_err(|e| HandleError::SurrealError {
            msg: format!("Failed to connect to database: {}", e),
        })?;

    // Sign in first (before setting namespace/database)
    SUL_DB
        .signin(Root {
            username: db_option.v_user.clone(),
            password: db_option.v_password.clone(),
        })
        .await
        .map_err(|e| HandleError::SurrealError {
            msg: format!("Failed to sign in: {}", e),
        })?;

    // Set namespace and database (兼容 SurrealDB 3.x)
    let _ = crate::use_ns_db_compat(&SUL_DB, &db_option.surreal_ns, &db_option.project_name).await;

    // Define common functions (使用 None 从配置文件自动读取路径)
    define_common_functions(None)
        .await
        .map_err(|e| HandleError::SurrealError {
            msg: format!("Failed to define common functions: {}", e),
        })?;

    // 加载属性中文名缓存
    rs_surreal::load_attr_cn_names()
        .await
        .map_err(|e| HandleError::SurrealError {
            msg: format!("Failed to load attribute Chinese names: {}", e),
        })?;

    Ok(db_option)
}

pub async fn init_surreal() -> anyhow::Result<()> {
    let config_file_name = get_config_file_name();
    println!("🔧 正在初始化数据库连接...");
    println!("📄 使用配置文件: {}.toml", config_file_name);

    // 使用 get_db_option() 以复用 OnceCell 缓存并尊重环境变量覆盖
    let db_option = get_db_option();

    let sdb_cfg = db_option.effective_surrealdb();
    println!("🏷️  命名空间: {}", db_option.surreal_ns);
    println!("💾 数据库名: {}", db_option.project_name);

    let config = surrealdb::opt::Config::default().ast_payload();

    match sdb_cfg.mode {
        options::DbConnMode::File => {
            let path = db_option.surrealdb_data_path();
            let conn_str = db_option.surrealdb_conn_str();
            println!("🗄️  后端: 嵌入式 ({})", conn_str);
            println!("📂 数据目录: {}", path);
            match SUL_DB.connect((&conn_str, config)).with_capacity(1000).await {
                Ok(_) => {}
                Err(e) => {
                    if e.to_string().contains("Already connected") {
                    } else {
                        return Err(e.into());
                    }
                }
            }
            // 嵌入式模式无需 signin
        }
        options::DbConnMode::Ws => {
            // WS 模式
            let connection_str = sdb_cfg.conn_str();
            println!("🌐 后端: WebSocket 远程");
            println!("🌐 连接服务器: {}", connection_str);
            println!("👤 用户名: {}", db_option.v_user);
            match SUL_DB
                .connect((connection_str, config))
                .with_capacity(1000)
                .await
            {
                Ok(_) => {}
                Err(e) => {
                    if e.to_string().contains("Already connected") {
                    } else {
                        return Err(e.into());
                    }
                }
            }
            SUL_DB
                .signin(Root {
                    username: db_option.v_user.clone(),
                    password: db_option.v_password.clone(),
                })
                .await?;
        }
    }

    crate::use_ns_db_compat(&SUL_DB, &db_option.surreal_ns, &db_option.project_name).await?;

    println!("✅ 数据库连接成功！");

    // 初始化 KV_DB（模型数据写入目标）
    let kv_cfg = db_option.effective_surrealkv();

    if !kv_cfg.enabled {
        // KV 未启用：模型数据写回主 SurrealDB（SUL_DB）
        println!("🗄️  SurrealKV 已禁用 (surrealkv.enabled=false)，模型数据写回主 SurrealDB");
    } else {
        let kv_conn_str = db_option.surrealkv_conn_str();
        println!("🔧 正在初始化 SurrealKV (模型数据库)...");
        let kv_config = surrealdb::opt::Config::default().ast_payload();
        match kv_cfg.mode {
            options::DbConnMode::File => {
                let kv_path = db_option.surrealkv_data_path();
                println!("📂 KV 数据目录: {}", kv_path);
                match rs_surreal::KV_DB.connect((&kv_conn_str, kv_config)).with_capacity(1000).await {
                    Ok(_) => {}
                    Err(e) => {
                        if !e.to_string().contains("Already connected") {
                            return Err(e.into());
                        }
                    }
                }
            }
            options::DbConnMode::Ws => {
                println!("🌐 KV 连接: {}", kv_conn_str);
                match rs_surreal::KV_DB.connect((&kv_conn_str, kv_config)).with_capacity(1000).await {
                    Ok(_) => {}
                    Err(e) => {
                        if !e.to_string().contains("Already connected") {
                            return Err(e.into());
                        }
                    }
                }
                rs_surreal::KV_DB
                    .signin(Root {
                        username: kv_cfg.user.clone(),
                        password: kv_cfg.password.clone(),
                    })
                    .await?;
            }
        }
        crate::use_ns_db_compat(&rs_surreal::KV_DB, &db_option.surreal_ns, &db_option.project_name).await?;
        rs_surreal::mark_model_kv_enabled();
        println!("✅ SurrealKV 连接成功！");
    }

    // Define common functions (使用 None 从配置文件自动读取路径)
    define_common_functions(None)
        .await
        .map_err(|e| HandleError::SurrealError {
            msg: format!("Failed to define common functions: {}", e),
        })?;

    // 在 KV_DB 上也定义通用函数（仅当 KV 启用时）
    if rs_surreal::is_model_kv_enabled() {
        if let Err(e) = crate::function::define_common_functions_on_db(&rs_surreal::KV_DB, None).await {
            eprintln!("⚠️  KV_DB 通用函数定义失败: {}（写入可能受影响）", e);
        }
    }

    // 加载属性中文名缓存
    rs_surreal::load_attr_cn_names().await?;

    Ok(())
}

/// 连接二号机组
pub async fn init_second_unit_surreal() -> anyhow::Result<()> {
    let s = Config::builder()
        .add_source(File::with_name("SecondUnitDbOption"))
        .build()?;
    let db_option: SecondUnitDbOption = s.try_deserialize()?;
    let config = surrealdb::opt::Config::default().ast_payload(); // 启用AST格式
    SECOND_SUL_DB
        .connect((db_option.get_version_db_conn_str(), config))
        .with_capacity(1000)
        .await?;
    SECOND_SUL_DB
        .signin(Root {
            username: db_option.v_user.clone(),
            password: db_option.v_password.clone(),
        })
        .await?;
    crate::use_ns_db_compat(
        &SECOND_SUL_DB,
        &db_option.surreal_ns,
        &db_option.project_name,
    )
    .await?;
    Ok(())
}

/// 判断是否连接到二号机组
pub async fn b_connected_second_unit() -> anyhow::Result<()> {
    let s = Config::builder()
        .add_source(File::with_name("SecondUnitDbOption"))
        .build()?;
    let db_option: SecondUnitDbOption = s.try_deserialize()?;
    SECOND_SUL_DB
        .signin(Root {
            username: db_option.v_user.clone(),
            password: db_option.v_password.clone(),
        })
        .await?;
    Ok(())
}

/// 初始化测试数据库
pub async fn init_demo_test_surreal() -> Result<DbOption, HandleError> {
    let s = Config::builder()
        .add_source(File::with_name("db_options/DbOption"))
        .build()
        .map_err(|e| HandleError::SurrealError {
            msg: format!("Failed to load DbOption config: {}", e),
        })?;
    let db_option: DbOption = s.try_deserialize().map_err(|e| HandleError::SurrealError {
        msg: format!("Failed to deserialize DbOption: {}", e),
    })?;

    // 创建配置
    let config = surrealdb::opt::Config::default().ast_payload(); // 启用AST格式

    // Connect to database
    SUL_DB
        .connect((db_option.get_version_db_conn_str(), config))
        .with_capacity(1000)
        .await
        .map_err(|e| HandleError::SurrealError {
            msg: format!("Failed to connect to database: {}", e),
        })?;

    // Sign in
    SUL_DB
        .signin(Root {
            username: db_option.v_user.clone(),
            password: db_option.v_password.clone(),
        })
        .await
        .map_err(|e| HandleError::SurrealError {
            msg: format!("Failed to sign in: {}", e),
        })?;

    // Set namespace and database (兼容 SurrealDB 3.x)
    crate::use_ns_db_compat(&SUL_DB, &db_option.surreal_ns, &db_option.project_name)
        .await
        .map_err(|e| HandleError::SurrealError {
            msg: format!("Failed to set namespace and database: {}", e),
        })?;

    // Define common functions (使用 None 从配置文件自动读取路径)
    define_common_functions(None)
        .await
        .map_err(|e| HandleError::SurrealError {
            msg: format!("Failed to define common functions: {}", e),
        })?;

    Ok(db_option)
}

#[cfg(test)]
pub mod test;
