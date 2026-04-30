use thiserror::Error;

#[cfg(feature = "sqlite")]
use crate::RefU64;
#[cfg(feature = "sqlite")]
use crate::spatial::sqlite::{self, ItemColumnSupport, detect_item_columns, query_item_metadata};
use crate::spatial::types::{
    AabbDto, Point3Dto, QueryShape, SpatialQueryFilter, SpatialQueryItem, SpatialQueryOptions,
    SpatialQueryRequest, SpatialQueryResponse, SpatialQueryTarget, SpatialStatsResponse,
};
#[cfg(feature = "sqlite")]
use parry3d::bounding_volume::Aabb;
#[cfg(feature = "sqlite")]
use rusqlite::Connection;
#[cfg(feature = "sqlite")]
use std::str::FromStr;

const BACKEND_NAME: &str = "sqlite-index";
const DEFAULT_LIMIT: usize = 5_000;
const HARD_LIMIT: usize = 10_000;
const MAX_RADIUS_MM: f32 = 100_000.0;
const POST_FILTER_SCAN_FACTOR: usize = 8;

#[derive(Debug, Error)]
pub enum SpatialQueryError {
    #[error("{0}")]
    InvalidRequest(String),
    #[error("{0}")]
    QueryFailed(String),
}

#[cfg(feature = "sqlite")]
#[derive(Debug, Clone)]
struct QueryRecord {
    refno: RefU64,
    noun: String,
    spec_value: Option<i64>,
    aabb: Aabb,
    distance: Option<f32>,
}

#[cfg(feature = "sqlite")]
#[derive(Debug, Clone)]
struct QueryPlan {
    query_aabb: Aabb,
    ids: Vec<(RefU64, Aabb, Option<String>)>,
}

#[derive(Debug, Clone, Default)]
struct NormalizedFilter {
    nouns: Option<Vec<String>>,
    spec_values: Option<Vec<i64>>,
}

#[derive(Debug, Clone)]
struct NormalizedOptions {
    limit: usize,
    scan_limit: usize,
    include_aabb: bool,
    include_distance: bool,
}

pub struct SpatialQueryService;

impl Default for SpatialQueryService {
    fn default() -> Self {
        Self
    }
}

impl SpatialQueryService {
    pub fn new() -> Self {
        Self
    }

    pub async fn query(
        &self,
        request: SpatialQueryRequest,
    ) -> Result<SpatialQueryResponse, SpatialQueryError> {
        tokio::task::spawn_blocking(move || Self::query_blocking(request))
            .await
            .map_err(|err| SpatialQueryError::QueryFailed(format!("空间查询线程异常: {err}")))?
    }

    pub async fn stats(&self) -> Result<SpatialStatsResponse, SpatialQueryError> {
        tokio::task::spawn_blocking(Self::stats_blocking)
            .await
            .map_err(|err| SpatialQueryError::QueryFailed(format!("空间统计线程异常: {err}")))?
    }

    fn stats_blocking() -> Result<SpatialStatsResponse, SpatialQueryError> {
        #[cfg(not(feature = "sqlite"))]
        {
            Err(SpatialQueryError::QueryFailed(
                "当前未启用 SQLite 空间索引功能".to_string(),
            ))
        }

        #[cfg(feature = "sqlite")]
        {
            let conn = sqlite::open_connection()
                .map_err(|err| SpatialQueryError::QueryFailed(err.to_string()))?;
            let total: i64 = conn
                .query_row("SELECT COUNT(1) FROM aabb_index", [], |row| row.get(0))
                .map_err(|err| SpatialQueryError::QueryFailed(err.to_string()))?;

            Ok(SpatialStatsResponse {
                success: true,
                backend: BACKEND_NAME.to_string(),
                total_elements: usize::try_from(total.max(0)).unwrap_or(0),
                index_type: "sqlite-rtree".to_string(),
                error: None,
            })
        }
    }

    fn query_blocking(
        request: SpatialQueryRequest,
    ) -> Result<SpatialQueryResponse, SpatialQueryError> {
        #[cfg(not(feature = "sqlite"))]
        {
            let _ = request;
            Err(SpatialQueryError::QueryFailed(
                "当前未启用 SQLite 空间索引功能".to_string(),
            ))
        }

        #[cfg(feature = "sqlite")]
        {
            let conn = sqlite::open_connection()
                .map_err(|err| SpatialQueryError::QueryFailed(err.to_string()))?;
            let item_columns = detect_item_columns(&conn)
                .map_err(|err| SpatialQueryError::QueryFailed(err.to_string()))?;
            let filter = normalize_filter(request.filter);
            let options = normalize_options(request.options, filter.spec_values.is_some());
            let plan = Self::build_query_plan(&conn, &request.target, &filter, &options)?;

            let mut records = Self::load_records(
                &conn,
                &plan.ids,
                item_columns,
                &request.target,
                &filter,
                &options,
            )?;
            records.sort_by(|lhs, rhs| match (lhs.distance, rhs.distance) {
                (Some(a), Some(b)) => a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => lhs.refno.0.cmp(&rhs.refno.0),
            });

            let total = records.len();
            let truncated = total > options.limit;
            if truncated {
                records.truncate(options.limit);
            }

            Ok(SpatialQueryResponse {
                success: true,
                items: records
                    .into_iter()
                    .map(|record| SpatialQueryItem {
                        refno: record.refno.to_string(),
                        noun: record.noun,
                        spec_value: record.spec_value,
                        aabb: options.include_aabb.then(|| AabbDto::from(&record.aabb)),
                        distance: if options.include_distance {
                            record.distance
                        } else {
                            None
                        },
                    })
                    .collect(),
                total,
                truncated,
                query_aabb: Some(AabbDto::from(&plan.query_aabb)),
                backend: BACKEND_NAME.to_string(),
                error: None,
            })
        }
    }

    #[cfg(feature = "sqlite")]
    fn build_query_plan(
        conn: &Connection,
        target: &SpatialQueryTarget,
        filter: &NormalizedFilter,
        options: &NormalizedOptions,
    ) -> Result<QueryPlan, SpatialQueryError> {
        let type_filter = filter.nouns.as_deref();
        match target {
            SpatialQueryTarget::PointRadius {
                center,
                radius,
                shape: _,
            } => {
                let center = validate_center(*center)?;
                let radius = validate_radius(*radius)?;
                let query_aabb = Aabb::new(
                    parry3d::math::Point::new(
                        center.x - radius,
                        center.y - radius,
                        center.z - radius,
                    ),
                    parry3d::math::Point::new(
                        center.x + radius,
                        center.y + radius,
                        center.z + radius,
                    ),
                );
                let ids = sqlite::query_overlap_with_conn(
                    conn,
                    &query_aabb,
                    type_filter,
                    Some(options.scan_limit),
                    &[],
                )
                .map_err(|err| SpatialQueryError::QueryFailed(err.to_string()))?;
                Ok(QueryPlan { query_aabb, ids })
            }
            SpatialQueryTarget::Bbox { bbox } => {
                let query_aabb = bbox
                    .try_to_aabb()
                    .map_err(SpatialQueryError::InvalidRequest)?;
                let ids = sqlite::query_overlap_with_conn(
                    conn,
                    &query_aabb,
                    type_filter,
                    Some(options.scan_limit),
                    &[],
                )
                .map_err(|err| SpatialQueryError::QueryFailed(err.to_string()))?;
                Ok(QueryPlan { query_aabb, ids })
            }
            SpatialQueryTarget::RefnoNeighborhood {
                refno,
                expand_distance,
                include_self,
            } => {
                let refno = parse_refno(refno)?;
                let mut query_aabb = sqlite::query_aabb_with_conn(conn, refno)
                    .map_err(|err| SpatialQueryError::QueryFailed(err.to_string()))?
                    .ok_or_else(|| {
                        SpatialQueryError::QueryFailed(format!("空间索引中不存在 refno: {}", refno))
                    })?;
                if *expand_distance > 0.0 {
                    query_aabb.mins.x -= *expand_distance;
                    query_aabb.mins.y -= *expand_distance;
                    query_aabb.mins.z -= *expand_distance;
                    query_aabb.maxs.x += *expand_distance;
                    query_aabb.maxs.y += *expand_distance;
                    query_aabb.maxs.z += *expand_distance;
                }
                let excludes = if *include_self { vec![] } else { vec![refno] };
                let ids = sqlite::query_overlap_with_conn(
                    conn,
                    &query_aabb,
                    type_filter,
                    Some(options.scan_limit),
                    &excludes,
                )
                .map_err(|err| SpatialQueryError::QueryFailed(err.to_string()))?;
                Ok(QueryPlan { query_aabb, ids })
            }
        }
    }

    #[cfg(feature = "sqlite")]
    fn load_records(
        conn: &Connection,
        ids: &[(RefU64, Aabb, Option<String>)],
        item_columns: ItemColumnSupport,
        target: &SpatialQueryTarget,
        filter: &NormalizedFilter,
        options: &NormalizedOptions,
    ) -> Result<Vec<QueryRecord>, SpatialQueryError> {
        let mut records = Vec::with_capacity(ids.len().min(options.scan_limit));
        for (refno, aabb, fallback_noun) in ids {
            let meta = query_item_metadata(conn, *refno, item_columns)
                .map_err(|err| SpatialQueryError::QueryFailed(err.to_string()))?;
            let noun = meta
                .noun
                .clone()
                .or_else(|| fallback_noun.clone())
                .unwrap_or_default();

            if let Some(spec_values) = &filter.spec_values {
                let Some(spec_value) = meta.spec_value else {
                    continue;
                };
                if !spec_values.contains(&spec_value) {
                    continue;
                }
            }

            let distance = match target {
                SpatialQueryTarget::PointRadius {
                    center,
                    radius,
                    shape,
                } => {
                    let center_point = center.to_point();
                    let dist = sqlite::distance_point_aabb(
                        glam::Vec3::new(center_point.x, center_point.y, center_point.z),
                        aabb,
                    );
                    if matches!(shape, QueryShape::Sphere) && dist > *radius {
                        continue;
                    }
                    Some(dist)
                }
                SpatialQueryTarget::Bbox { .. } => None,
                SpatialQueryTarget::RefnoNeighborhood { .. } => None,
            };

            records.push(QueryRecord {
                refno: *refno,
                noun,
                spec_value: meta.spec_value,
                aabb: *aabb,
                distance,
            });
        }
        Ok(records)
    }
}

fn normalize_filter(filter: Option<SpatialQueryFilter>) -> NormalizedFilter {
    let Some(filter) = filter else {
        return NormalizedFilter::default();
    };
    let nouns = filter.nouns.and_then(|values| {
        let items = values
            .into_iter()
            .map(|value| value.trim().to_uppercase())
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>();
        if items.is_empty() { None } else { Some(items) }
    });
    let spec_values = filter.spec_values.and_then(|values| {
        if values.is_empty() {
            None
        } else {
            Some(values)
        }
    });
    NormalizedFilter { nouns, spec_values }
}

fn normalize_options(
    options: Option<SpatialQueryOptions>,
    has_post_filter: bool,
) -> NormalizedOptions {
    let options = options.unwrap_or_default();
    let limit = options.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, HARD_LIMIT);
    let scan_limit = if has_post_filter {
        limit
            .saturating_mul(POST_FILTER_SCAN_FACTOR)
            .clamp(limit, HARD_LIMIT)
    } else {
        limit.saturating_add(1).clamp(limit, HARD_LIMIT)
    };
    NormalizedOptions {
        limit,
        scan_limit,
        include_aabb: options.include_aabb.unwrap_or(true),
        include_distance: options.include_distance.unwrap_or(true),
    }
}

fn validate_center(center: Point3Dto) -> Result<Point3Dto, SpatialQueryError> {
    if !center.is_finite() {
        return Err(SpatialQueryError::InvalidRequest(
            "center 非法：坐标必须为有限数值".to_string(),
        ));
    }
    Ok(center)
}

fn validate_radius(radius: f32) -> Result<f32, SpatialQueryError> {
    if !radius.is_finite() || radius <= 0.0 || radius > MAX_RADIUS_MM {
        return Err(SpatialQueryError::InvalidRequest(format!(
            "radius 非法：必须满足 0 < radius <= {MAX_RADIUS_MM}"
        )));
    }
    Ok(radius)
}

#[cfg(feature = "sqlite")]
fn parse_refno(input: &str) -> Result<RefU64, SpatialQueryError> {
    let refno = RefU64::from_str(input.trim()).map_err(|_| {
        SpatialQueryError::InvalidRequest(format!("refno 格式非法: {}", input.trim()))
    })?;
    if refno.is_unset() {
        return Err(SpatialQueryError::InvalidRequest(format!(
            "refno 格式非法: {}",
            input.trim()
        )));
    }
    Ok(refno)
}
