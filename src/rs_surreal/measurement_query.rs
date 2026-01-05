//! Measurement query functions
//!
//! This module provides CRUD operations for the measurements table

use crate::rs_surreal::inst_structs::Measurement;
use crate::rs_surreal::query_structs::MeasurementQueryResult;
use anyhow::Result;
use surrealdb::Surreal;
use surrealdb::engine::any::Any;

/// 创建新的测量记录
///
/// # Arguments
/// * `conn` - 数据库连接
/// * `measurement` - 测量数据结构
///
/// # Returns
/// 返回创建的测量 ID
pub async fn create_measurement(conn: &Surreal<Any>, measurement: &Measurement) -> Result<String> {
    let sql = measurement.to_surql();

    conn.query(&sql).await?;

    Ok(measurement.id.clone())
}

/// 批量创建测量记录
///
/// # Arguments
/// * `conn` - 数据库连接
/// * `measurements` - 测量数据数组
///
/// # Returns
/// 返回创建的所有测量 ID
pub async fn create_measurements_batch(
    conn: &Surreal<Any>,
    measurements: &[Measurement],
) -> Result<Vec<String>> {
    let mut ids = Vec::new();

    for measurement in measurements {
        let id = create_measurement(conn, measurement).await?;
        ids.push(id);
    }

    Ok(ids)
}

/// 查询所有测量记录（支持分页）
///
/// # Arguments
/// * `conn` - 数据库连接
/// * `project_id` - 可选的项目 ID 过滤
/// * `scene_id` - 可选的场景 ID 过滤
/// * `limit` - 分页大小
/// * `offset` - 分页偏移
///
/// # Returns
/// 返回查询结果列表
pub async fn list_measurements(
    conn: &Surreal<Any>,
    project_id: Option<String>,
    scene_id: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<Vec<MeasurementQueryResult>> {
    let mut conditions = Vec::new();

    if let Some(pid) = project_id {
        conditions.push(format!("project_id = '{}'", pid));
    }
    if let Some(sid) = scene_id {
        conditions.push(format!("scene_id = '{}'", sid));
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", conditions.join(" AND "))
    };

    let limit_clause = limit.map_or(String::new(), |l| format!(" LIMIT {}", l));
    let offset_clause = offset.map_or(String::new(), |o| format!(" START {}", o));

    let sql = format!(
        "SELECT * FROM measurement{}{}{}",
        where_clause, limit_clause, offset_clause
    );

    let mut result = conn.query(&sql).await?;
    let measurements: Vec<MeasurementQueryResult> = result.take(0)?;

    Ok(measurements)
}

/// 根据 ID 获取单个测量记录
///
/// # Arguments
/// * `conn` - 数据库连接
/// * `id` - 测量 ID
///
/// # Returns
/// 返回测量数据，如果不存在返回 None
pub async fn get_measurement_by_id(
    conn: &Surreal<Any>,
    id: &str,
) -> Result<Option<MeasurementQueryResult>> {
    let sql = format!("SELECT * FROM {} LIMIT 1", id);

    let mut result = conn.query(&sql).await?;
    let measurement: Option<MeasurementQueryResult> = result.take(0)?;

    Ok(measurement)
}

/// 更新测量记录
///
/// # Arguments
/// * `conn` - 数据库连接
/// * `id` - 测量 ID
/// * `updates` - 要更新的字段值（JSON 格式）
///
/// # Returns
/// 操作结果
pub async fn update_measurement(
    conn: &Surreal<Any>,
    id: &str,
    updates: serde_json::Value,
) -> Result<()> {
    let sql = format!(
        "UPDATE {} MERGE {} SET updated_at = time::now()",
        id,
        serde_json::to_string(&updates)?
    );

    conn.query(&sql).await?;
    Ok(())
}

/// 删除测量记录
///
/// # Arguments
/// * `conn` - 数据库连接
/// * `id` - 测量 ID
///
/// # Returns
/// 操作结果
pub async fn delete_measurement(conn: &Surreal<Any>, id: &str) -> Result<()> {
    let sql = format!("DELETE {}", id);

    conn.query(&sql).await?;
    Ok(())
}

/// 按类型查询测量记录
///
/// # Arguments
/// * `conn` - 数据库连接
/// * `measurement_type` - 测量类型
/// * `limit` - 可选的结果数量限制
///
/// # Returns
/// 返回查询结果列表
pub async fn list_measurements_by_type(
    conn: &Surreal<Any>,
    measurement_type: &str,
    limit: Option<usize>,
) -> Result<Vec<MeasurementQueryResult>> {
    let limit_clause = limit.map_or(String::new(), |l| format!(" LIMIT {}", l));
    let sql = format!(
        "SELECT * FROM measurement WHERE measurement_type = '{}{}",
        measurement_type, limit_clause
    );

    let mut result = conn.query(&sql).await?;
    let measurements: Vec<MeasurementQueryResult> = result.take(0)?;

    Ok(measurements)
}

/// 统计测量记录数量
///
/// # Arguments
/// * `conn` - 数据库连接
/// * `project_id` - 可选的项目 ID 过滤
/// * `scene_id` - 可选的场景 ID 过滤
///
/// # Returns
/// 返回测量记录总数
pub async fn count_measurements(
    conn: &Surreal<Any>,
    project_id: Option<String>,
    scene_id: Option<String>,
) -> Result<usize> {
    let mut conditions = Vec::new();

    if let Some(pid) = project_id {
        conditions.push(format!("project_id = '{}'", pid));
    }
    if let Some(sid) = scene_id {
        conditions.push(format!("scene_id = '{}'", sid));
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", conditions.join(" AND "))
    };

    let sql = format!("SELECT count() FROM measurement{} GROUP ALL", where_clause);

    let mut result = conn.query(&sql).await?;
    let count: Option<usize> = result.take(0)?;

    Ok(count.unwrap_or(0))
}

/// 按创建者查询测量记录
///
/// # Arguments
/// * `conn` - 数据库连接
/// * `created_by` - 创建者 ID
/// * `limit` - 可选的结果数量限制
///
/// # Returns
/// 返回查询结果列表
pub async fn list_measurements_by_creator(
    conn: &Surreal<Any>,
    created_by: &str,
    limit: Option<usize>,
) -> Result<Vec<MeasurementQueryResult>> {
    let limit_clause = limit.map_or(String::new(), |l| format!(" LIMIT {}", l));
    let sql = format!(
        "SELECT * FROM measurement WHERE created_by = '{}'{}'",
        created_by, limit_clause
    );

    let mut result = conn.query(&sql).await?;
    let measurements: Vec<MeasurementQueryResult> = result.take(0)?;

    Ok(measurements)
}

/// 按状态查询测量记录
///
/// # Arguments
/// * `conn` - 数据库连接
/// * `status` - 状态值 (Draft|Pending|Approved|Rejected)
/// * `limit` - 可选的结果数量限制
///
/// # Returns
/// 返回查询结果列表
pub async fn list_measurements_by_status(
    conn: &Surreal<Any>,
    status: &str,
    limit: Option<usize>,
) -> Result<Vec<MeasurementQueryResult>> {
    let limit_clause = limit.map_or(String::new(), |l| format!(" LIMIT {}", l));
    let sql = format!(
        "SELECT * FROM measurement WHERE status = '{}'{}'",
        status, limit_clause
    );

    let mut result = conn.query(&sql).await?;
    let measurements: Vec<MeasurementQueryResult> = result.take(0)?;

    Ok(measurements)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shape::pdms_shape::RsVec3;
    use glam::Vec3;

    #[test]
    fn test_measurement_structure() {
        let points = vec![
            RsVec3(Vec3::new(0.0, 0.0, 0.0)),
            RsVec3(Vec3::new(1.0, 1.0, 1.0)),
        ];

        let measurement = Measurement::new(
            "Test Measurement".to_string(),
            "Distance".to_string(),
            points,
        )
        .with_value(1.732)
        .with_unit("mm".to_string())
        .with_priority("High".to_string());

        assert_eq!(measurement.name, "Test Measurement");
        assert_eq!(measurement.measurement_type, "Distance");
        assert_eq!(measurement.value, Some(1.732));
        assert_eq!(measurement.unit, Some("mm".to_string()));
        assert_eq!(measurement.priority, Some("High".to_string()));
    }

    #[test]
    fn test_measurement_to_surql() {
        let points = vec![
            RsVec3(Vec3::new(0.0, 0.0, 0.0)),
            RsVec3(Vec3::new(1.0, 1.0, 1.0)),
        ];

        let measurement =
            Measurement::new("Test Distance".to_string(), "Distance".to_string(), points)
                .with_value(1.414)
                .with_unit("mm".to_string())
                .with_project("proj_001".to_string());

        let sql = measurement.to_surql();

        assert!(sql.contains("CREATE measurement:"));
        assert!(sql.contains("name = 'Test Distance'"));
        assert!(sql.contains("measurement_type = 'Distance'"));
        assert!(sql.contains("value = 1.414"));
        assert!(sql.contains("unit = 'mm'"));
        assert!(sql.contains("project_id = 'proj_001'"));
    }

    #[test]
    fn test_measurement_gen_sur_json() {
        let points = vec![RsVec3(Vec3::new(0.0, 0.0, 0.0))];

        let measurement = Measurement::new("Angle Test".to_string(), "Angle".to_string(), points)
            .with_status("Approved".to_string());

        let json_str = measurement.gen_sur_json();
        let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(json["name"], "Angle Test");
        assert_eq!(json["measurement_type"], "Angle");
        assert_eq!(json["status"], "Approved");
    }
}
