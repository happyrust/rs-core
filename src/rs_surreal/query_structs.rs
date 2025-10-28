//! Database query structures for geometry operations
//!
//! This module contains all the data structures used for database queries
//! in geometry generation and boolean operations.

use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::types::{RecordId, RefnoEnum, RsAabb, RsTransform};
use serde::{Deserialize, Serialize};
use surrealdb::types as surrealdb_types;
use surrealdb::types::SurrealValue;

/// Geometry parameter structure
#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
pub struct GeoParam {
    /// Geometry ID
    pub id: String,
    /// Geometry parameters
    pub param: PdmsGeoParam,
}

/// Negative geometry information
#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
pub struct NegInfo {
    /// Record ID
    pub id: RecordId,
    /// Geometry type
    pub geo_type: String,
    /// Parameter type
    #[serde(default)]
    pub para_type: String,
    /// Transform
    pub trans: RsTransform,
    /// Optional AABB
    pub aabb: Option<RsAabb>,
}

/// Manifold geometry transformation query
#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
pub struct ManiGeoTransQuery {
    /// Reference number
    pub refno: RefnoEnum,
    /// Session number
    pub sesno: u32,
    /// Noun
    pub noun: String,
    /// World transform
    pub wt: RsTransform,
    /// AABB
    pub aabb: RsAabb,
    /// Transform list
    pub ts: Vec<(RecordId, RsTransform)>,
    /// Negative transform list
    pub neg_ts: Vec<(RefnoEnum, RsTransform, Vec<NegInfo>)>,
}

/// Parameter negative information
#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
pub struct ParamNegInfo {
    /// Geometry parameters
    pub param: PdmsGeoParam,
    /// Geometry type
    pub geo_type: String,
    /// Parameter type
    pub para_type: String,
    /// Transform
    pub trans: RsTransform,
    /// Optional AABB
    pub aabb: Option<RsAabb>,
}

/// OpenCascade geometry transformation query
#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
pub struct OccGeoTransQuery {
    /// Reference number
    pub refno: RefnoEnum,
    /// Noun
    pub noun: String,
    /// World transform
    pub wt: RsTransform,
    /// AABB
    pub aabb: RsAabb,
    /// Transform list with parameters
    pub ts: Vec<(PdmsGeoParam, RsTransform)>,
    /// Negative transform list
    pub neg_ts: Vec<(RefnoEnum, RsTransform, Vec<ParamNegInfo>)>,
}

/// Catalog negative group
#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
pub struct CataNegGroup {
    /// Reference number
    pub refno: RefnoEnum,
    /// Instance info record ID
    pub inst_info_id: RecordId,
    /// Boolean group
    pub boolean_group: Vec<Vec<RefnoEnum>>,
}

/// Geometry model data
#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
pub struct GmGeoData {
    /// Record ID
    pub id: RecordId,
    /// Geometry reference number
    pub geom_refno: RefnoEnum,
    /// Transform
    pub trans: RsTransform,
    /// Parameters
    pub param: PdmsGeoParam,
    /// AABB ID - temporarily unchanged
    pub aabb_id: RecordId,
}
