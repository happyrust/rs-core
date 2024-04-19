use serde::{Serialize, Deserialize};
use clap::Parser;


#[derive(Debug, Default, Clone, Parser, Serialize, Deserialize)]
pub struct DbOption {
    #[clap(long, default_value="false")]
    pub enable_log: bool,
    #[clap(long)]
    pub total_sync: bool,
    #[clap(long)]
    pub sync_graph_db: Option<bool>,
    #[clap(long)]
    pub sync_tidb: Option<bool>,
    #[clap(long, default_value="true")]
    pub sync_versioned: Option<bool>,
    #[clap(long)]
    pub sync_localdb: Option<bool>,
    #[clap(long)]
    pub incr_sync: bool,
    // #[clap(long)]
    // pub replace_insert: Option<bool>,
    #[clap(long, default_value="10_0000")]
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
    pub manual_db_nums: Option<Vec<i32>>,
    #[clap(long)]
    pub reset_mdb_project: Option<bool>,

    #[clap(long)]
    pub debug_print_world_transform: bool,
    #[clap(skip)]
    pub debug_root_refnos: Option<Vec<String>>,
    #[clap(skip)]
    pub room_root_refnos: Option<Vec<String>>,
    #[clap(skip)]
    pub debug_branch_refno: Option<String>,
    #[clap(skip)]
    pub debug_refno_types: Vec<String>,
    #[clap(long)]
    pub replace_mesh: bool,
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
    pub plat_url:String,
    #[clap(long)]
    pub puhua_database_ip: String,
    #[clap(long)]
    pub puhua_database_user: String,
    #[clap(long)]
    pub puhua_database_password: String,
}


impl DbOption{

    #[inline]
    pub fn get_version_db_conn_str(&self) -> String {
        let ip = self.v_ip.as_str();
        let port = self.v_port.as_str();
        format!("ws://{ip}:{port}")
    }

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

    #[inline]
    pub fn get_mysql_db_conn_str(&self, db: &str) -> String {
        let user = self.user.as_str();
        let pwd = urlencoding::encode(self.password.as_str());
        let ip = self.ip.as_str();
        let port = self.port.as_str();
        format!("mysql://{user}:{pwd}@{ip}:{port}/{}", db)
    }
}