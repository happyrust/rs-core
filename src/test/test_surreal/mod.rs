use crate::SUL_DB;
use crate::function::define_common_functions;
use crate::options::DbOption;
use config::{Config, File};
use surrealdb::opt::auth::Root;
pub mod test_mdb;
// pub mod test_query_fuzzy;

// pub mod test_query_regex;

pub mod test_basic_query;

// pub mod test_refno_enum;
pub mod test_helpers;
// pub mod test_memory_db;
pub mod test_simple_refno;

pub mod test_query_group;

pub mod test_graph;

pub mod test_collect_children;

pub mod test_query_insts;

pub mod test_serde;

pub mod test_spatial;

// pub mod test_room;

pub mod test_geom;

// pub mod test_uda;

// pub mod test_pbs;

// pub mod test_scom_query;
pub mod test_type_hierarchy;

pub mod test_collect_inst;
