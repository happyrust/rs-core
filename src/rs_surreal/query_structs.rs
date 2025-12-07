//! Database query structures for geometry operations
//!
//! This module contains all the data structures used for database queries
//! in geometry generation and boolean operations.

use crate::parsed_data::geo_params_data::PdmsGeoParam;
use crate::rs_surreal::geometry_query::PlantTransform;
use crate::shape::pdms_shape::RsVec3;
use crate::types::{PlantAabb, RecordId, RefnoEnum};
use chrono::NaiveDateTime;
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
    /// Record ID (geo_relate ID)
    pub id: RecordId,
    /// Geometry type
    pub geo_type: String,
    /// Parameter type
    #[serde(default)]
    pub para_type: String,
    /// 几何体局部变换（相对于载体的变换）
    #[serde(rename = "trans")]
    pub geo_local_trans: PlantTransform,
    /// Optional AABB
    pub aabb: Option<PlantAabb>,
    /// 负载体的世界变换（用于计算绝对位置）
    #[serde(default, rename = "carrier_wt")]
    pub carrier_world_trans: Option<PlantTransform>,
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
    /// 实例的世界变换
    #[serde(rename = "wt")]
    pub inst_world_trans: PlantTransform,
    /// AABB
    pub aabb: PlantAabb,
    /// 正几何列表：(geo_relate ID, 几何体局部变换)
    #[serde(rename = "ts")]
    pub pos_geos: Vec<(RecordId, PlantTransform)>,
    /// 负几何列表：(载体refno, 载体世界变换, 负几何信息列表)
    pub neg_ts: Vec<(RefnoEnum, PlantTransform, Vec<NegInfo>)>,
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
    pub trans: PlantTransform,
    /// Optional AABB
    pub aabb: Option<PlantAabb>,
}

/// OpenCascade geometry transformation query
#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
pub struct OccGeoTransQuery {
    /// Reference number
    pub refno: RefnoEnum,
    /// Noun
    pub noun: String,
    /// World transform
    pub wt: PlantTransform,
    /// AABB
    pub aabb: PlantAabb,
    /// Transform list with parameters
    pub ts: Vec<(PdmsGeoParam, PlantTransform)>,
    /// Negative transform list
    pub neg_ts: Vec<(RefnoEnum, PlantTransform, Vec<ParamNegInfo>)>,
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
    pub trans: PlantTransform,
    /// Parameters
    pub param: PdmsGeoParam,
    /// AABB ID - temporarily unchanged
    pub aabb_id: RecordId,
}

/// Measurement query result structure
/// 用于从数据库查询返回的测量数据结构
#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
pub struct MeasurementQueryResult {
    /// 测量ID
    pub id: String,
    /// 测量名称
    pub name: String,
    /// 测量类型 (Distance|Angle|PointToMesh|Diameter|Radius|Coordinate)
    pub measurement_type: String,
    /// 测量点坐标数组
    pub points: Vec<RsVec3>,
    /// 测量结果值
    pub value: Option<f64>,
    /// 单位 (如 "mm", "度")
    pub unit: Option<String>,
    /// 优先级 (Low|Medium|High|Critical)
    pub priority: Option<String>,
    /// 状态 (Draft|Pending|Approved|Rejected)
    pub status: Option<String>,
    /// 项目ID
    pub project_id: Option<String>,
    /// 场景ID
    pub scene_id: Option<String>,
    /// 创建者ID
    pub created_by: Option<String>,
    /// 创建时间 (RFC3339 格式字符串)
    pub created_at: Option<String>,
    /// 更新时间 (RFC3339 格式字符串)
    pub updated_at: Option<String>,
    /// 备注
    pub notes: Option<String>,
    /// 扩展元数据
    pub metadata: Option<serde_json::Value>,
}

/// Annotation 查询结果 DTO
/// 用于从数据库查询返回的批注数据结构
#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
pub struct AnnotationQueryResult {
    /// 批注ID
    pub id: String,
    /// 批注标题
    pub title: String,
    /// 批注描述
    pub description: String,
    /// 批注类型 (Text, Arrow, Rectangle, Circle, Cloud, Highlight, Selection)
    pub annotation_type: String,
    /// 3D 位置坐标
    pub position: Option<RsVec3>,
    /// 颜色（十六进制）
    pub color: Option<String>,
    /// 优先级 (Low, Medium, High, Critical)
    pub priority: Option<String>,
    /// 状态 (Draft, Pending, Approved, Rejected, Resolved)
    pub status: Option<String>,
    /// 绘制样式（JSON）
    pub style: Option<serde_json::Value>,
    /// 关联的 3D 对象列表
    pub associated_refnos: Option<Vec<u64>>,
    /// 项目ID
    pub project_id: Option<String>,
    /// 场景ID
    pub scene_id: Option<String>,
    /// 创建者
    pub created_by: Option<String>,
    /// 指派给
    pub assigned_to: Option<String>,
    /// 创建时间 (RFC3339 格式字符串)
    pub created_at: Option<String>,
    /// 更新时间 (RFC3339 格式字符串)
    pub updated_at: Option<String>,
    /// 解决时间 (RFC3339 格式字符串)
    pub resolved_at: Option<String>,
    /// 扩展元数据
    pub metadata: Option<serde_json::Value>,
}
