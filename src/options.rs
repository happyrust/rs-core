use std::path::{Path, PathBuf};

use crate::mesh_precision::MeshPrecisionSettings;
use crate::tree_query::TreeQuery;
use crate::{RefU64, RefnoEnum};
use clap::Parser;
use serde::{Deserialize, Serialize};

/// 数据库连接模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DbConnMode {
    /// 本地嵌入式文件访问
    File,
    /// WebSocket 远程连接
    Ws,
}

impl Default for DbConnMode {
    fn default() -> Self {
        Self::File
    }
}

impl DbConnMode {
    #[inline]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Ws => "ws",
        }
    }
}

/// SurrealDB 连接配置（PE/属性/输入数据读取）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurrealDbConfig {
    /// 连接模式：file（嵌入式）或 ws（WebSocket）
    #[serde(default)]
    pub mode: DbConnMode,
    /// file 模式：本地数据目录路径
    #[serde(default)]
    pub path: Option<String>,
    /// ws 模式：IP 地址
    #[serde(default = "default_surrealdb_ip")]
    pub ip: String,
    /// ws 模式：端口
    #[serde(default = "default_surrealdb_port")]
    pub port: u16,
    /// ws 模式：用户名
    #[serde(default = "default_surrealdb_user")]
    pub user: String,
    /// ws 模式：密码
    #[serde(default = "default_surrealdb_password")]
    pub password: String,
}

impl Default for SurrealDbConfig {
    fn default() -> Self {
        Self {
            mode: DbConnMode::File,
            path: None,
            ip: "localhost".to_string(),
            port: 8020,
            user: "root".to_string(),
            password: "root".to_string(),
        }
    }
}

impl SurrealDbConfig {
    /// 获取连接字符串
    pub fn conn_str(&self) -> String {
        match self.mode {
            DbConnMode::File => {
                let path = self.path.as_deref().unwrap_or("db-data/default.rdb");
                format!("rocksdb://{}", path)
            }
            DbConnMode::Ws => {
                let ip = if self.ip == "localhost" {
                    "127.0.0.1"
                } else {
                    &self.ip
                };
                format!("ws://{}:{}", ip, self.port)
            }
        }
    }
}

/// Web Server 配置
///
/// 控制 web_server 二进制的启动行为，包括监听端口、SurrealDB 自启动等。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebServerConfig {
    /// Web 服务监听端口
    #[serde(default = "default_web_server_port")]
    pub port: u16,
    /// 是否自动启动 SurrealDB 进程
    #[serde(default = "default_true_auto_start")]
    pub auto_start_surreal: bool,
    /// SurrealDB 可执行文件路径（auto_start 时使用）
    #[serde(default = "default_surreal_bin")]
    pub surreal_bin: String,
    /// SurrealDB 数据目录（auto_start 时使用 rocksdb://path）
    #[serde(default)]
    pub surreal_data_path: Option<String>,
    /// SurrealDB 监听地址（auto_start 时使用）
    #[serde(default = "default_surreal_bind")]
    pub surreal_bind: String,
    /// SurrealDB 用户名
    #[serde(default = "default_surrealdb_user")]
    pub surreal_user: String,
    /// SurrealDB 密码
    #[serde(default = "default_surrealdb_password")]
    pub surreal_password: String,
}

impl Default for WebServerConfig {
    fn default() -> Self {
        Self {
            port: 8080,
            auto_start_surreal: true,
            surreal_bin: "surreal".to_string(),
            surreal_data_path: None,
            surreal_bind: "0.0.0.0:8020".to_string(),
            surreal_user: "root".to_string(),
            surreal_password: "root".to_string(),
        }
    }
}

impl WebServerConfig {
    /// 获取 SurrealDB 数据路径，优先使用 web_server 配置，否则回退到 surrealdb.path
    pub fn effective_data_path<'a>(&'a self, fallback: Option<&'a str>) -> &'a str {
        self.surreal_data_path
            .as_deref()
            .or(fallback)
            .unwrap_or("db-data/default.rdb")
    }

    /// 获取 auto_start 时 SurrealDB 的 WebSocket 连接地址
    pub fn ws_conn_str(&self) -> String {
        format!("ws://{}", self.surreal_bind)
    }
}

fn default_web_server_port() -> u16 {
    8080
}
fn default_true_auto_start() -> bool {
    true
}
fn default_surreal_bin() -> String {
    "surreal".to_string()
}
fn default_surreal_bind() -> String {
    "0.0.0.0:8020".to_string()
}

#[derive(Debug, Default, Clone, Parser, Serialize, Deserialize)]
pub struct DbOption {
    /// 是否启用日志
    #[clap(long, default_value = "false")]
    pub enable_log: bool,
    /// 是否全量同步
    #[clap(long)]
    pub total_sync: bool,
    /// 是否启用索引
    #[clap(long)]
    pub enable_index: Option<bool>,
    /// 是否启用 SQLite RTree 空间索引
    #[clap(long)]
    #[serde(default)]
    pub enable_sqlite_rtree: bool,
    /// SQLite 空间索引文件路径
    #[clap(long)]
    pub sqlite_index_path: Option<String>,
    /// 是否同步图数据库
    #[clap(long)]
    pub sync_graph_db: Option<bool>,
    /// 是否同步TiDB
    #[clap(long)]
    pub sync_tidb: Option<bool>,
    /// 是否同步版本化数据,默认为true
    #[clap(long, default_value = "true")]
    pub sync_versioned: Option<bool>,
    /// 是否同步实时数据
    #[clap(long)]
    pub sync_live: Option<bool>,
    /// 是否同步历史数据
    #[clap(long)]
    pub sync_history: Option<bool>,
    /// 是否增量同步
    #[clap(long)]
    pub incr_sync: bool,
    /// 是否只同步系统数据
    #[clap(long)]
    pub sync_only_sys: Option<bool>,
    // #[clap(long)]
    // pub replace_insert: Option<bool>,
    /// 同步的chunk size
    #[clap(long, default_value = "10_0000")]
    pub sync_chunk_size: Option<u32>,
    /// 解析模式: legacy(串行异步) / parallel(并行同步+兼容包装)
    #[clap(long)]
    #[serde(default = "default_parse_mode")]
    pub parse_mode: Option<String>,
    /// 解析侧到写库侧的通道容量（有界通道）
    #[clap(long)]
    #[serde(default = "default_parse_channel_capacity")]
    pub parse_channel_capacity: Option<usize>,

    /// 是否使用tidb
    #[clap(long)]
    pub use_tidb: Option<bool>,

    /// SurrealDB 连接 IP（ws 模式下使用）
    #[clap(long)]
    #[serde(default, alias = "v_ip")]
    pub surreal_ip: String,
    /// SurrealDB 认证用户名
    #[clap(long)]
    #[serde(default, alias = "v_user")]
    pub surreal_user: String,
    /// SurrealDB 认证密码
    #[clap(long)]
    #[serde(default, alias = "v_password")]
    pub surreal_password: String,
    /// SurrealDB 连接端口（ws 模式下使用）
    #[clap(long)]
    #[serde(default, alias = "v_port")]
    pub surreal_port: u16,
    /// mqtt的host
    #[clap(long)]
    pub mqtt_host: String,
    /// mqtt的端口
    #[clap(long)]
    pub mqtt_port: u16,
    /// 需要同步的location
    #[clap(long)]
    pub location: String,
    /// 需要同步的location的db
    #[clap(long)]
    pub location_dbs: Option<Vec<u32>>,

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
    #[clap(skip)]
    #[serde(default)]
    pub mesh_precision: MeshPrecisionSettings,
    #[clap(long)]
    pub mesh_tol_ratio: Option<f32>,
    #[clap(long)]
    pub apply_boolean_operation: bool,
    #[clap(long)]
    pub gen_spatial_tree: bool,
    #[clap(long)]
    pub load_spatial_tree: bool,
    #[clap(long, default_value = "12.1SP4Projects")]
    pub project_path: String,
    pub included_projects: Vec<String>,
    //覆盖project的目录名
    pub project_dirs: Option<Vec<String>>,
    /// 包含的数据库文件列表
    #[clap(skip)]
    pub included_db_files: Option<Vec<String>>,

    // ========================
    // Meilisearch（PDMS 检索索引）配置
    // ========================
    /// Meilisearch URL，例如 http://127.0.0.1:7700
    #[clap(skip)]
    pub meili_url: Option<String>,
    /// Meilisearch API Key（可选）
    #[clap(skip)]
    pub meili_api_key: Option<String>,
    /// PDMS 节点索引名（默认 pdms_nodes）
    #[clap(skip)]
    pub meili_pdms_index: Option<String>,
    /// 解析期写入的 JSONL spool 目录（默认 output/meili_spool）
    #[clap(skip)]
    pub meili_spool_dir: Option<String>,
    /// MDB数据库名称
    #[clap(long)]
    pub mdb_name: String,
    /// 模块名称
    #[clap(long)]
    pub module: String,
    /// 项目名称
    #[clap(long)]
    pub project_name: String,
    /// 项目代码
    #[clap(long)]
    pub project_code: String,
    /// SurrealDB命名空间
    #[clap(skip)]
    pub surreal_ns: String,
    /// SurrealDB 脚本目录路径，默认为 resource/surreal
    #[clap(long)]
    pub surreal_script_dir: Option<String>,
    /// 手动指定的数据库编号列表
    #[clap(skip)]
    pub manual_db_nums: Option<Vec<u32>>,
    /// 需要排除的数据库编号列表
    #[clap(skip)]
    pub exclude_db_nums: Option<Vec<u32>>,
    /// 是否重置MDB项目
    #[clap(long)]
    pub reset_mdb_project: Option<bool>,

    /// 是否打印世界坐标系变换矩阵
    #[clap(long)]
    pub debug_print_world_transform: bool,
    /// 调试用的模型生成参考号列表（仅在生成模型时有效）
    #[clap(skip)]
    pub debug_model_refnos: Option<Vec<String>>,
    /// 是否生成历史模型
    #[clap(long)]
    pub gen_history_model: Option<bool>,
    /// 测试用的引用号
    pub test_refno: Option<String>,
    /// 使用特定引用号生成模型
    pub gen_using_spref_refnos: Option<Vec<String>>,
    /// 手动同步的引用号列表
    #[clap(skip)]
    pub manual_sync_refnos: Option<Vec<String>>,
    /// 房间根节点引用号列表
    #[clap(skip)]
    pub room_root_refnos: Option<Vec<String>>,
    /// 调试用的引用号类型列表
    #[clap(skip)]
    pub debug_refno_types: Vec<String>,
    /// 是否替换网格（已废弃，固定返回 false；覆盖模式由 pre_cleanup_for_regen 替代）
    #[clap(long)]
    #[serde(default)]
    pub replace_mesh: Option<bool>,
    /// 是否生成网格
    #[clap(long)]
    pub gen_mesh: bool,
    /// 是否生成材质
    #[clap(skip)]
    pub gen_material: Option<bool>,
    /// 是否保存到数据库
    #[clap(long)]
    pub save_db: Option<bool>,
    /// 解析期只生成 scene tree 文件，跳过 PE/属性数据保存
    #[clap(long, default_value = "false")]
    #[serde(default)]
    pub gen_tree_only: bool,
    /// 是否导出 JSON 实例文件
    #[clap(long)]
    #[serde(default)]
    pub export_json: bool,
    /// 是否导出 Parquet 文件
    #[clap(long, default_value = "true")]
    #[serde(default = "default_true")]
    pub export_parquet: bool,
    /// 是否需要同步基础引用号
    #[clap(long)]
    pub need_sync_refno_basic: bool,
    /// 是否仅更新数据库信息
    #[clap(long)]
    pub only_update_dbinfo: bool,
    /// 数据库IP地址
    #[clap(long)]
    pub ip: String,
    /// 数据库用户名
    #[clap(long)]
    pub user: String,
    /// 数据库密码
    #[clap(long)]
    pub password: String,
    /// 数据库端口号
    #[clap(long)]
    pub port: String,
    /// SQL线程数量
    #[clap(short)]
    pub sql_threads_number: u32,
    /// 是否重建SSC树
    #[clap(short)]
    pub rebuild_ssc_tree: bool,
    /// 批量插入SQL语句的数量
    #[clap(short)]
    pub batch_insert_sql_cnt: u32,
    /// PE块大小
    #[clap(short)]
    #[serde(default = "default_pe_chunk")]
    pub pe_chunk: u32,
    /// 属性块大小
    #[clap(short)]
    #[serde(default = "default_att_chunk")]
    pub att_chunk: u32,
    /// 生成模型的批处理大小
    #[clap(short)]
    pub gen_model_batch_size: usize,
    /// ArangoDB数据库URL地址
    #[clap(long)]
    #[serde(default)]
    pub arangodb_url: String,
    /// 服务器发布IP地址
    #[clap(long)]
    #[serde(default)]
    pub server_release_ip: String,
    /// ArangoDB数据库用户名
    #[clap(long)]
    #[serde(default)]
    pub arangodb_user: String,
    /// ArangoDB数据库密码
    #[clap(long)]
    #[serde(default)]
    pub arangodb_password: String,
    /// ArangoDB数据库名称
    #[clap(long)]
    #[serde(default)]
    pub arangodb_database: String,
    /// 房间内的引用号列表
    #[clap(skip)]
    pub withing_room_refnos: Option<String>,
    /// 建筑数据库编号列表
    #[clap(skip)]
    pub arch_db_nums: Option<Vec<i32>>,
    /// 是否将空间树保存到数据库
    #[clap(long)]
    pub save_spatial_tree_to_db: bool,
    /// 是否启用多线程
    #[clap(long)]
    #[serde(default)]
    pub multi_threads: bool,
    /// 是否仅同步系统
    #[clap(short)]
    pub only_sync_sys: bool,
    /// 平台URL地址
    #[clap(long)]
    #[serde(default)]
    pub plat_url: String,
    /// 普华数据库IP地址
    #[clap(long)]
    pub puhua_database_ip: String,
    /// 普华数据库用户名
    #[clap(long)]
    pub puhua_database_user: String,
    /// 普华数据库密码
    #[clap(long)]
    pub puhua_database_password: String,

    pub room_key_word: Option<Vec<String>>,

    pub meshes_path: Option<String>,
    // pub geom_live: Option<bool>,
    /// SurrealDB 连接配置（[surrealdb] 子表）
    #[clap(skip)]
    #[serde(default)]
    pub surrealdb: SurrealDbConfig,

    /// Web Server 配置（[web_server] 子表）
    #[clap(skip)]
    #[serde(default)]
    pub web_server: WebServerConfig,

    /// 内存KV数据库IP地址（用于PE数据额外备份）
    #[clap(long)]
    #[serde(default = "default_mem_kv_ip")]
    pub mem_kv_ip: String,

    /// 内存KV数据库端口
    #[clap(long)]
    #[serde(default = "default_mem_kv_port")]
    pub mem_kv_port: String,

    /// 内存KV数据库用户名
    #[clap(long)]
    #[serde(default = "default_mem_kv_user")]
    pub mem_kv_user: String,

    /// 内存KV数据库密码
    #[clap(long)]
    #[serde(default = "default_mem_kv_password")]
    pub mem_kv_password: String,
}

impl DbOption {
    // #[inline]
    // pub fn is_geom_live(&self) -> bool {
    //     self.geom_live.unwrap_or(false)
    // }

    /// 获取 SurrealDB 脚本目录路径，如果未配置则返回默认值 "resource/surreal"
    #[inline]
    pub fn get_surreal_script_dir(&self) -> &str {
        self.surreal_script_dir
            .as_deref()
            .unwrap_or("resource/surreal")
    }

    pub fn get_test_refno(&self) -> Option<RefnoEnum> {
        self.test_refno.as_ref().map(|x| x.as_str().into())
    }

    pub fn build_cate_relate(&self) -> bool {
        self.build_cate_relate.unwrap_or(false)
    }

    /// 已废弃：覆盖模式由 pre_cleanup_for_regen 替代，始终返回 false。
    #[inline]
    #[deprecated(note = "replace_mesh 已废弃，覆盖模式由 pre_cleanup_for_regen 替代")]
    pub fn is_replace_mesh(&self) -> bool {
        false
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
    pub fn parse_mode_str(&self) -> &str {
        match self.parse_mode.as_deref() {
            Some("legacy") => "legacy",
            _ => "parallel",
        }
    }

    #[inline]
    pub fn is_parse_parallel(&self) -> bool {
        self.parse_mode_str() == "parallel"
    }

    #[inline]
    pub fn get_parse_channel_capacity(&self) -> usize {
        self.parse_channel_capacity.unwrap_or(200).max(1)
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
        self.room_key_word
            .clone()
            .unwrap_or(vec!["-RM".to_string()])
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

        // 溯源到不含 lod_ 的基础目录，确保返回的是纯粹的基础路径
        let mut clean_base = pathbuf;
        while let Some(last_component) = clean_base.file_name().and_then(|n| n.to_str()) {
            if last_component.starts_with("lod_") {
                clean_base.pop();
            } else {
                break;
            }
        }

        if !clean_base.exists() {
            std::fs::create_dir_all(&clean_base).unwrap();
        }
        clean_base
    }

    #[inline]
    pub fn mesh_precision(&self) -> &MeshPrecisionSettings {
        &self.mesh_precision
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
    pub fn sqlite_index_enabled(&self) -> bool {
        self.enable_sqlite_rtree
    }

    #[inline]
    pub fn get_sqlite_index_path(&self) -> PathBuf {
        self.sqlite_index_path
            .as_ref()
            .map(|p| Path::new(p).to_path_buf())
            .unwrap_or_else(|| PathBuf::from("aabb_cache.sqlite"))
    }

    #[inline]
    pub fn is_gen_history_model(&self) -> bool {
        self.gen_history_model.unwrap_or(false)
    }

    #[inline]
    pub async fn get_all_debug_refnos(&self) -> Vec<RefnoEnum> {
        let root_refnos: Vec<RefnoEnum> = self
            .debug_model_refnos
            .as_ref()
            .map(|x| x.iter().map(|x| x.as_str().into()).collect::<Vec<_>>())
            .unwrap_or_default();

        if root_refnos.is_empty() {
            return vec![];
        }

        // 使用 TreeIndex 查询子孙节点（内存 BFS，速度快）
        let mut refnos = root_refnos.clone();
        for refno in &root_refnos {
            if let Some(index) = crate::tree_query::get_tree_index_by_refno(refno.refno()) {
                let options = crate::tree_query::TreeQueryOptions {
                    include_self: false, // root 已在 refnos 中
                    max_depth: None,
                    filter: crate::tree_query::TreeQueryFilter::default(),
                    prune_on_match: false,
                };
                let descendants: Vec<RefU64> =
                    index.collect_descendants_bfs(refno.refno(), &options);
                refnos.extend(descendants.into_iter().map(RefnoEnum::from));
            }
        }

        if self.is_gen_history_model() {
            let mut h_refnos = vec![];
            for r in refnos.clone() {
                h_refnos.extend(crate::query_history_pes(r).await.unwrap_or_default());
            }
            refnos.extend(h_refnos);
        }
        //还要补充使用了gen_using_spref_refnos的模型
        let debug_spref_refnos: Vec<RefU64> = self
            .gen_using_spref_refnos
            .as_ref()
            .map(|x| x.iter().map(|x| x.as_str().into()).collect::<Vec<_>>())
            .unwrap_or_default();
        let using_debug_spref_ele_refnos = if !debug_spref_refnos.is_empty() {
            let refnos = crate::query_ele_refnos_by_spres(&debug_spref_refnos)
                .await
                .unwrap();
            refnos
        } else {
            vec![]
        };
        refnos.extend(
            using_debug_spref_ele_refnos
                .into_iter()
                .map(|x| RefnoEnum::Refno(x)),
        );

        refnos
    }

    #[inline]
    pub fn get_manual_sync_refnos(&self) -> Vec<RefU64> {
        self.manual_sync_refnos
            .as_ref()
            .map(|x| x.iter().map(|x| x.as_str().into()).collect::<Vec<_>>())
            .unwrap_or_default()
    }

    /// 获取 SurrealDB 连接配置
    ///
    /// 已知问题（2026-04-28）：`config` crate 在反序列化 nested table 的 `u16`
    /// 字段时，遇到裸整数（如 `port = 18320`）会静默回落到 `#[serde(default)]`，
    /// 导致 `[surrealdb].port` 读到默认 8020，与服务实际监听端口不一致。
    /// 在 rs-core/config 上游修复之前，这里做一个 fallback：当子表 port 仍是
    /// 默认值，但顶层 `surreal_port` 已经被显式配置且不是默认值时，按顶层覆盖。
    /// 详见 `plant-model-gen/docs/plans/2026-04-28-aveva-plant-sample-deployment-test-plan.md`。
    #[inline]
    pub fn effective_surrealdb(&self) -> SurrealDbConfig {
        let mut cfg = self.surrealdb.clone();
        if cfg.port == default_surrealdb_port()
            && self.surreal_port != 0
            && self.surreal_port != default_surrealdb_port()
        {
            cfg.port = self.surreal_port;
        }
        cfg
    }

    /// 获取 SurrealDB 嵌入式模式的数据目录路径
    ///
    /// 优先使用 `[surrealdb].path`，未配置时默认 `db-data/{project_name}_{surreal_port}.rdb`
    #[inline]
    pub fn surrealdb_data_path(&self) -> String {
        self.surrealdb
            .path
            .clone()
            .unwrap_or_else(|| format!("db-data/{}_{}.rdb", self.project_name, self.surreal_port))
    }

    /// 获取 SurrealDB 嵌入式模式的完整连接字符串
    ///
    /// 当 mode=File 时使用 `surrealdb_data_path()` 生成 `rocksdb://` 连接串
    pub fn surrealdb_conn_str(&self) -> String {
        match self.surrealdb.mode {
            DbConnMode::File => format!("rocksdb://{}", self.surrealdb_data_path()),
            DbConnMode::Ws => self.surrealdb.conn_str(),
        }
    }

    /// 获取主 SurrealDB 连接字符串
    #[inline]
    pub fn get_version_db_conn_str(&self) -> String {
        self.surrealdb_conn_str()
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

    // #[inline]
    // pub fn get_mysql_db_conn_str(&self, db: &str) -> String {
    //     let user = self.user.as_str();
    //     let pwd = urlencoding::encode(self.password.as_str());
    //     let ip = self.ip.as_str();
    //     let port = self.port.as_str();
    //     format!("mysql://{user}:{pwd}@{ip}:{port}/{}", db)
    // }
}

#[derive(Debug, Default, Clone, Parser, Serialize, Deserialize)]
pub struct SecondUnitDbOption {
    // 项目名称
    #[clap(long)]
    pub project_name: String,
    /// 项目代码
    #[clap(long)]
    pub project_code: String,
    /// SurrealDB命名空间
    #[clap(skip)]
    pub surreal_ns: String,
    /// SurrealDB 脚本目录路径，默认为 resource/surreal
    #[clap(long)]
    pub surreal_script_dir: Option<String>,
    /// 二号机组 SurrealDB IP
    #[clap(long)]
    #[serde(alias = "v_ip")]
    pub surreal_ip: String,
    /// 二号机组 SurrealDB 用户
    #[clap(long)]
    #[serde(alias = "v_user")]
    pub surreal_user: String,
    /// 二号机组 SurrealDB 密码
    #[clap(long)]
    #[serde(alias = "v_password")]
    pub surreal_password: String,
    /// 二号机组 SurrealDB 端口
    #[clap(long)]
    #[serde(alias = "v_port")]
    pub surreal_port: u16,
}

impl SecondUnitDbOption {
    /// 获取 SurrealDB 脚本目录路径，如果未配置则返回默认值 "resource/surreal"
    #[inline]
    pub fn get_surreal_script_dir(&self) -> &str {
        self.surreal_script_dir
            .as_deref()
            .unwrap_or("resource/surreal")
    }

    #[inline]
    pub fn get_version_db_conn_str(&self) -> String {
        let ip = self.surreal_ip.as_str();
        let port = self.surreal_port;
        format!("ws://{ip}:{port}")
    }
}

// ============================================================================
// SurrealDB/KV 配置默认值函数
// ============================================================================

fn default_surrealdb_ip() -> String {
    "localhost".to_string()
}

fn default_surrealdb_port() -> u16 {
    8020
}

fn default_surrealdb_user() -> String {
    "root".to_string()
}

fn default_surrealdb_password() -> String {
    "root".to_string()
}

// ============================================================================
// 内存KV数据库配置默认值函数
// ============================================================================

fn default_mem_kv_ip() -> String {
    "localhost".to_string()
}

fn default_mem_kv_port() -> String {
    "8011".to_string()
}

fn default_mem_kv_user() -> String {
    "root".to_string()
}

fn default_mem_kv_password() -> String {
    "root".to_string()
}

fn default_true() -> bool {
    true
}

fn default_pe_chunk() -> u32 {
    300
}

fn default_att_chunk() -> u32 {
    200
}

fn default_parse_mode() -> Option<String> {
    Some("parallel".to_string())
}

fn default_parse_channel_capacity() -> Option<usize> {
    Some(200)
}

#[cfg(test)]
mod tests {
    use super::{DbConnMode, DbOption, SurrealDbConfig};

    #[test]
    fn surrealdb_file_mode_conn_str() {
        let cfg = SurrealDbConfig {
            mode: DbConnMode::File,
            path: Some("D:/data/test.db".to_string()),
            ..Default::default()
        };
        assert_eq!(cfg.conn_str(), "rocksdb://D:/data/test.db");
    }

    #[test]
    fn surrealdb_ws_mode_conn_str() {
        let cfg = SurrealDbConfig {
            mode: DbConnMode::Ws,
            ip: "localhost".to_string(),
            port: 8020,
            ..Default::default()
        };
        assert_eq!(cfg.conn_str(), "ws://127.0.0.1:8020");
    }

    #[test]
    fn effective_surrealdb_uses_new_config() {
        let mut opt = DbOption::default();
        opt.surrealdb = SurrealDbConfig {
            mode: DbConnMode::File,
            path: Some("/data/test.db".to_string()),
            ..Default::default()
        };
        let eff = opt.effective_surrealdb();
        assert_eq!(eff.mode, DbConnMode::File);
        assert_eq!(eff.path.as_deref(), Some("/data/test.db"));
        assert_eq!(eff.conn_str(), "rocksdb:///data/test.db");
    }

    #[test]
    fn effective_surrealdb_falls_back_to_top_surreal_port() {
        let mut opt = DbOption::default();
        opt.surreal_port = 18320;
        assert_eq!(opt.surrealdb.port, super::default_surrealdb_port());
        let eff = opt.effective_surrealdb();
        assert_eq!(
            eff.port, 18320,
            "[surrealdb].port 缺失/默认时应回落到顶层 surreal_port"
        );
    }

    #[test]
    fn effective_surrealdb_keeps_explicit_subtable_port() {
        let mut opt = DbOption::default();
        opt.surreal_port = 18320;
        opt.surrealdb.port = 25000;
        let eff = opt.effective_surrealdb();
        assert_eq!(
            eff.port, 25000,
            "若 [surrealdb].port 已显式覆盖，必须以子表为准而非顶层"
        );
    }
}
