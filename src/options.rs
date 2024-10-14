use std::path::{Path, PathBuf};

use crate::{RefU64, RefnoEnum};
use clap::Parser;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Parser, Serialize, Deserialize)]
pub struct DbOption {
    #[clap(long, default_value = "false")]
    pub enable_log: bool,
    #[clap(long)]
    pub total_sync: bool,
    #[clap(long)]
    pub enable_index: Option<bool>,
    #[clap(long)]
    pub sync_graph_db: Option<bool>,
    #[clap(long)]
    pub sync_tidb: Option<bool>,
    #[clap(long, default_value = "true")]
    pub sync_versioned: Option<bool>,
    #[clap(long)]
    pub sync_live: Option<bool>,
    #[clap(long)]
    pub sync_history: Option<bool>,
    #[clap(long)]
    pub incr_sync: bool,
    #[clap(long)]
    pub sync_only_sys: Option<bool>,
    // #[clap(long)]
    // pub replace_insert: Option<bool>,
    #[clap(long, default_value = "10_0000")]
    pub sync_chunk_size: Option<u32>,

    #[clap(long)]
    pub use_tidb: Option<bool>,

    //添加这4个变量
    #[clap(long)]
    pub v_ip: String,
    #[clap(long)]
    pub v_user: String,
    #[clap(long)]
    pub v_password: String,
    #[clap(long)]
    pub v_port: String,

    // #[clap(long)]
    // pub kv_ip: String,
    // #[clap(long)]
    // pub kv_port: String,

    // mqtt_host
    #[clap(long)]
    pub mqtt_host: String,
    #[clap(long)]
    pub mqtt_port: u16,
    #[clap(long)]
    pub location: String,

    #[clap(long)]
    pub remote_file_server_hosts: Vec<String>,

    #[clap(long)]
    pub file_server_host: String,

    #[clap(long)]
    pub replace_dbs: bool,
    #[clap(skip)]
    pub replace_types: Option<Vec<String>>,
    #[clap(long)]
    pub gen_model: bool,
    pub build_cate_relate: Option<bool>,
    #[clap(long)]
    pub mesh_tol_ratio: Option<f32>,
    #[clap(long)]
    pub apply_boolean_operation: bool,
    #[clap(long)]
    pub save_model_mesh_to_graph_db: bool,
    #[clap(long)]
    pub gen_spatial_tree: bool,
    #[clap(long)]
    pub load_spatial_tree: bool,
    #[clap(long, default_value = "12.1SP4Projects")]
    pub project_path: String,
    pub included_projects: Vec<String>,
    //覆盖project的目录名
    pub project_dirs: Option<Vec<String>>,
    #[clap(skip)]
    pub included_db_files: Option<Vec<String>>,
    #[clap(long)]
    pub mdb_name: String,
    #[clap(long)]
    pub module: String,
    #[clap(long)]
    pub project_name: String,
    #[clap(long)]
    pub project_code: String,
    #[clap(skip)]
    pub surreal_ns: String,
    #[clap(skip)]
    pub manual_db_nums: Option<Vec<u32>>,
    #[clap(long)]
    pub reset_mdb_project: Option<bool>,

    #[clap(long)]
    pub debug_print_world_transform: bool,
    #[clap(skip)]
    pub debug_root_refnos: Option<Vec<String>>,
    #[clap(long)]
    pub gen_history_model: Option<bool>,
    pub test_refno: Option<String>,
    pub gen_using_spref_refnos: Option<Vec<String>>,
    #[clap(skip)]
    pub manual_sync_refnos: Option<Vec<String>>,
    #[clap(skip)]
    pub room_root_refnos: Option<Vec<String>>,
    #[clap(skip)]
    pub debug_refno_types: Vec<String>,
    #[clap(long)]
    pub replace_mesh: Option<bool>,
    #[clap(long)]
    pub gen_mesh: bool,
    #[clap(skip)]
    pub gen_material: Option<bool>,
    #[clap(long)]
    pub save_db: Option<bool>,
    #[clap(long)]
    pub need_sync_refno_basic: bool,
    #[clap(long)]
    pub only_update_dbinfo: bool,
    #[clap(long)]
    pub ip: String,
    #[clap(long)]
    pub user: String,
    #[clap(long)]
    pub password: String,
    #[clap(long)]
    pub port: String,
    #[clap(short)]
    pub sql_threads_number: u32,
    #[clap(short)]
    pub rebuild_ssc_tree: bool,
    #[clap(short)]
    pub batch_insert_sql_cnt: u32,
    #[clap(short)]
    pub pe_chunk: u32,
    #[clap(short)]
    pub att_chunk: u32,
    #[clap(short)]
    pub gen_model_batch_size: usize,
    #[clap(long)]
    pub arangodb_url: String,
    #[clap(long)]
    pub server_release_ip: String,
    #[clap(long)]
    pub arangodb_user: String,
    #[clap(long)]
    pub arangodb_password: String,
    #[clap(long)]
    pub arangodb_database: String,
    #[clap(skip)]
    pub withing_room_refnos: Option<String>,
    #[clap(skip)]
    pub arch_db_nums: Option<Vec<i32>>,
    #[clap(long)]
    pub save_spatial_tree_to_db: bool,
    #[clap(long)]
    pub multi_threads: bool,
    #[clap(short)]
    pub only_sync_sys: bool,
    #[clap(long)]
    pub plat_url: String,
    #[clap(long)]
    pub puhua_database_ip: String,
    #[clap(long)]
    pub puhua_database_user: String,
    #[clap(long)]
    pub puhua_database_password: String,

    pub room_key_word: Option<Vec<String>>,

    pub meshes_path: Option<String>,
    // pub geom_live: Option<bool>,
}

impl DbOption {
    // #[inline]
    // pub fn is_geom_live(&self) -> bool {
    //     self.geom_live.unwrap_or(false)
    // }

    pub fn get_test_refno(&self) -> Option<RefnoEnum>{
        self.test_refno.as_ref().map(|x| x.as_str().into())
    }

    pub fn build_cate_relate(&self) -> bool{
        self.build_cate_relate.unwrap_or(false)
    }

    #[inline]
    pub fn is_replace_mesh(&self) -> bool {
        self.replace_mesh.unwrap_or(false)
    }

    #[inline]
    pub fn is_gen_mesh_or_model(&self) -> bool {
        self.gen_mesh || self.gen_model
    }

    #[inline]
    pub fn is_sync_history(&self) -> bool {
        self.sync_history.unwrap_or(false)
    }

    #[inline]
    pub fn mdb_name(&self) -> String {
        if self.mdb_name.starts_with("/") {
            self.mdb_name.clone()
        } else {
            format!("/{}", self.mdb_name)
        }
    }

    #[inline]
    pub fn get_room_key_word(&self) -> Vec<String> {
        self.room_key_word.clone().unwrap_or(vec!["-RM".to_string()])
    }

    #[inline]
    pub fn get_project_path(&self, project: &str) -> Option<PathBuf> {
        let mut data_dir = Path::new(&self.project_path);
        if self.project_dirs.is_none() {
            Some(data_dir.join(project))
        } else {
            let index = self.included_projects.iter().position(|x| x == project)?;
            Some(data_dir.join(&self.project_dirs.as_ref().unwrap()[index]))
        }
    }

    pub fn get_meshes_path(&self) -> PathBuf {
        let pathbuf = self
            .meshes_path
            .as_ref()
            .map(|x| Path::new(x).to_path_buf())
            .unwrap_or("assets/meshes".into());
        if !pathbuf.exists() {
            std::fs::create_dir_all(&pathbuf).unwrap();
        }
        pathbuf
    }

    #[inline]
    pub fn get_project_dir_names(&self) -> &Vec<String> {
        self.project_dirs
            .as_ref()
            .unwrap_or(&self.included_projects)
    }

    #[inline]
    pub fn is_save_db(&self) -> bool {
        self.save_db.unwrap_or(true)
    }

    #[inline]
    pub fn is_gen_history_model(&self) -> bool {
        self.gen_history_model.unwrap_or(false)
    }

    #[inline]
    pub async fn get_all_debug_refnos(&self) -> Vec<RefnoEnum> {
        let mut refnos = self.debug_root_refnos
            .as_ref()
            .map(|x| x.iter().map(|x| x.as_str().into()).collect::<Vec<_>>())
            .unwrap_or_default();
        if self.is_gen_history_model() {
            let mut h_refnos = vec![];
            for r in refnos.clone() {
                h_refnos.extend(crate::query_history_pes(r).await.unwrap_or_default());
            }
            refnos.extend(h_refnos);
        }
        //还要补充使用了gen_using_spref_refnos的模型
        let mut debug_spref_refnos = self
            .gen_using_spref_refnos
            .as_ref()
            .map(|x| {
                x.iter()
                    .map(|x| x.as_str().into())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let using_debug_spref_ele_refnos = if !debug_spref_refnos.is_empty() {
            let refnos = crate::query_ele_refnos_by_spres(&debug_spref_refnos)
                .await
                .unwrap();
            refnos
        } else {
            vec![]
        };
        refnos.extend(using_debug_spref_ele_refnos.into_iter().map(|x| RefnoEnum::Refno(x)));
        
        refnos
    }

    #[inline]
    pub fn get_manual_sync_refnos(&self) -> Vec<RefU64> {
        self.manual_sync_refnos
            .as_ref()
            .map(|x| x.iter().map(|x| x.as_str().into()).collect::<Vec<_>>())
            .unwrap_or_default()
    }

    #[inline]
    pub fn get_version_db_conn_str(&self) -> String {
        let ip = self.v_ip.as_str();
        let port = self.v_port.as_str();
        format!("ws://{ip}:{port}")
    }

    // #[inline]
    // pub fn get_kv_db_conn_str(&self) -> String {
    //     let ip = self.kv_ip.as_str();
    //     let port = self.kv_port.as_str();
    //     format!("ws://{ip}:{port}")
    // }

    #[inline]
    pub fn get_mysql_conn_str(&self) -> String {
        let user = self.user.as_str();
        let pwd = urlencoding::encode(self.password.as_str());
        let ip = self.ip.as_str();
        let port = self.port.as_str();
        format!("mysql://{user}:{pwd}@{ip}:{port}")
    }

    #[inline]
    pub fn get_mysql_project_db_conn_str(&self) -> String {
        let user = self.user.as_str();
        let pwd = urlencoding::encode(self.password.as_str());
        let ip = self.ip.as_str();
        let port = self.port.as_str();
        format!("mysql://{user}:{pwd}@{ip}:{port}/{}", &self.project_name)
    }

    // #[inline]
    // pub fn get_mysql_db_conn_str(&self, db: &str) -> String {
    //     let user = self.user.as_str();
    //     let pwd = urlencoding::encode(self.password.as_str());
    //     let ip = self.ip.as_str();
    //     let port = self.port.as_str();
    //     format!("mysql://{user}:{pwd}@{ip}:{port}/{}", db)
    // }
}
