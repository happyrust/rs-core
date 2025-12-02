//! Annotation query functions
//!
//! This module provides CRUD operations for the annotations table

use crate::rs_surreal::inst_structs::Annotation;
use crate::rs_surreal::query_structs::AnnotationQueryResult;
use anyhow::Result;
use surrealdb::Surreal;
use surrealdb::engine::any::Any;

/// 创建新的批注记录
///
/// # Arguments
/// * `conn` - 数据库连接
/// * `annotation` - 批注数据结构
///
/// # Returns
/// 返回创建的批注 ID
pub async fn create_annotation(conn: &Surreal<Any>, annotation: &Annotation) -> Result<String> {
    let sql = annotation.to_surql();

    conn.query(&sql).await?;

    Ok(annotation.id.clone())
}

/// 批量创建批注记录
///
/// # Arguments
/// * `conn` - 数据库连接
/// * `annotations` - 批注数据数组
///
/// # Returns
/// 返回创建的所有批注 ID
pub async fn create_annotations_batch(
    conn: &Surreal<Any>,
    annotations: &[Annotation],
) -> Result<Vec<String>> {
    let mut ids = Vec::new();

    for annotation in annotations {
        let id = create_annotation(conn, annotation).await?;
        ids.push(id);
    }

    Ok(ids)
}

/// 查询所有批注记录（支持分页和过滤）
///
/// # Arguments
/// * `conn` - 数据库连接
/// * `project_id` - 可选的项目 ID 过滤
/// * `scene_id` - 可选的场景 ID 过滤
/// * `status` - 可选的状态过滤
/// * `limit` - 分页大小
/// * `offset` - 分页偏移
///
/// # Returns
/// 返回查询结果列表
pub async fn list_annotations(
    conn: &Surreal<Any>,
    project_id: Option<String>,
    scene_id: Option<String>,
    status: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<Vec<AnnotationQueryResult>> {
    let mut conditions = Vec::new();

    if let Some(pid) = project_id {
        conditions.push(format!("project_id = '{}'", pid));
    }
    if let Some(sid) = scene_id {
        conditions.push(format!("scene_id = '{}'", sid));
    }
    if let Some(s) = status {
        conditions.push(format!("status = '{}'", s));
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", conditions.join(" AND "))
    };

    let limit_clause = limit.map_or(String::new(), |l| format!(" LIMIT {}", l));
    let offset_clause = offset.map_or(String::new(), |o| format!(" START {}", o));

    let sql = format!(
        "SELECT * FROM annotation{}{}{}",
        where_clause, limit_clause, offset_clause
    );

    let mut result = conn.query(&sql).await?;
    let annotations: Vec<AnnotationQueryResult> = result.take(0)?;

    Ok(annotations)
}

/// 根据 ID 获取单个批注记录
///
/// # Arguments
/// * `conn` - 数据库连接
/// * `id` - 批注 ID
///
/// # Returns
/// 返回批注数据，如果不存在返回 None
pub async fn get_annotation_by_id(
    conn: &Surreal<Any>,
    id: &str,
) -> Result<Option<AnnotationQueryResult>> {
    let sql = format!("SELECT * FROM {} LIMIT 1", id);

    let mut result = conn.query(&sql).await?;
    let annotation: Option<AnnotationQueryResult> = result.take(0)?;

    Ok(annotation)
}

/// 更新批注记录
///
/// # Arguments
/// * `conn` - 数据库连接
/// * `id` - 批注 ID
/// * `updates` - 要更新的字段值（JSON 格式）
///
/// # Returns
/// 操作结果
pub async fn update_annotation(
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

/// 删除批注记录
///
/// # Arguments
/// * `conn` - 数据库连接
/// * `id` - 批注 ID
///
/// # Returns
/// 操作结果
pub async fn delete_annotation(conn: &Surreal<Any>, id: &str) -> Result<()> {
    let sql = format!("DELETE {}", id);

    conn.query(&sql).await?;
    Ok(())
}

/// 按类型查询批注记录
///
/// # Arguments
/// * `conn` - 数据库连接
/// * `annotation_type` - 批注类型
/// * `limit` - 查询限制数
///
/// # Returns
/// 返回查询结果列表
pub async fn list_annotations_by_type(
    conn: &Surreal<Any>,
    annotation_type: &str,
    limit: Option<usize>,
) -> Result<Vec<AnnotationQueryResult>> {
    let limit_clause = limit.map_or(String::new(), |l| format!(" LIMIT {}", l));
    let sql = format!(
        "SELECT * FROM annotation WHERE annotation_type = '{}'{}",
        annotation_type, limit_clause
    );

    let mut result = conn.query(&sql).await?;
    let annotations: Vec<AnnotationQueryResult> = result.take(0)?;

    Ok(annotations)
}

/// 按创建者查询批注记录
///
/// # Arguments
/// * `conn` - 数据库连接
/// * `creator` - 创建者ID
/// * `limit` - 查询限制数
///
/// # Returns
/// 返回查询结果列表
pub async fn list_annotations_by_creator(
    conn: &Surreal<Any>,
    creator: &str,
    limit: Option<usize>,
) -> Result<Vec<AnnotationQueryResult>> {
    let limit_clause = limit.map_or(String::new(), |l| format!(" LIMIT {}", l));
    let sql = format!(
        "SELECT * FROM annotation WHERE created_by = '{}'{}",
        creator, limit_clause
    );

    let mut result = conn.query(&sql).await?;
    let annotations: Vec<AnnotationQueryResult> = result.take(0)?;

    Ok(annotations)
}

/// 按指派对象查询批注记录
///
/// # Arguments
/// * `conn` - 数据库连接
/// * `assignee` - 指派对象ID
/// * `limit` - 查询限制数
///
/// # Returns
/// 返回查询结果列表
pub async fn list_annotations_by_assignee(
    conn: &Surreal<Any>,
    assignee: &str,
    limit: Option<usize>,
) -> Result<Vec<AnnotationQueryResult>> {
    let limit_clause = limit.map_or(String::new(), |l| format!(" LIMIT {}", l));
    let sql = format!(
        "SELECT * FROM annotation WHERE assigned_to = '{}'{}",
        assignee, limit_clause
    );

    let mut result = conn.query(&sql).await?;
    let annotations: Vec<AnnotationQueryResult> = result.take(0)?;

    Ok(annotations)
}

/// 按关联对象查询批注记录
///
/// # Arguments
/// * `conn` - 数据库连接
/// * `refno` - 3D 对象的 RefU64 值
/// * `limit` - 查询限制数
///
/// # Returns
/// 返回查询结果列表
pub async fn list_annotations_by_associated_object(
    conn: &Surreal<Any>,
    refno: u64,
    limit: Option<usize>,
) -> Result<Vec<AnnotationQueryResult>> {
    let limit_clause = limit.map_or(String::new(), |l| format!(" LIMIT {}", l));
    let sql = format!(
        "SELECT * FROM annotation WHERE {} IN associated_refnos{}",
        refno, limit_clause
    );

    let mut result = conn.query(&sql).await?;
    let annotations: Vec<AnnotationQueryResult> = result.take(0)?;

    Ok(annotations)
}

/// 统计批注记录数量
///
/// # Arguments
/// * `conn` - 数据库连接
/// * `project_id` - 可选的项目 ID 过滤
/// * `scene_id` - 可选的场景 ID 过滤
/// * `status` - 可选的状态过滤
///
/// # Returns
/// 返回记录数量
pub async fn count_annotations(
    conn: &Surreal<Any>,
    project_id: Option<String>,
    scene_id: Option<String>,
    status: Option<String>,
) -> Result<usize> {
    let mut conditions = Vec::new();
    if let Some(pid) = project_id {
        conditions.push(format!("project_id = '{}'", pid));
    }
    if let Some(sid) = scene_id {
        conditions.push(format!("scene_id = '{}'", sid));
    }
    if let Some(s) = status {
        conditions.push(format!("status = '{}'", s));
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", conditions.join(" AND "))
    };

    let sql = format!("SELECT count() FROM annotation{} GROUP ALL", where_clause);

    let mut result = conn.query(&sql).await?;
    let count: Option<usize> = result.take(0)?;

    Ok(count.unwrap_or(0))
}

/// 更新批注状态为已解决
///
/// # Arguments
/// * `conn` - 数据库连接
/// * `id` - 批注 ID
///
/// # Returns
/// 操作结果
pub async fn resolve_annotation(conn: &Surreal<Any>, id: &str) -> Result<()> {
    let sql = format!(
        "UPDATE {} SET status = 'Resolved', resolved_at = time::now(), updated_at = time::now()",
        id
    );

    conn.query(&sql).await?;
    Ok(())
}

/// 按优先级查询批注记录
///
/// # Arguments
/// * `conn` - 数据库连接
/// * `priority` - 优先级 (Low, Medium, High, Critical)
/// * `limit` - 查询限制数
///
/// # Returns
/// 返回查询结果列表
pub async fn list_annotations_by_priority(
    conn: &Surreal<Any>,
    priority: &str,
    limit: Option<usize>,
) -> Result<Vec<AnnotationQueryResult>> {
    let limit_clause = limit.map_or(String::new(), |l| format!(" LIMIT {}", l));
    let sql = format!(
        "SELECT * FROM annotation WHERE priority = '{}' ORDER BY created_at DESC{}",
        priority, limit_clause
    );

    let mut result = conn.query(&sql).await?;
    let annotations: Vec<AnnotationQueryResult> = result.take(0)?;

    Ok(annotations)
}

/// 按时间范围查询批注记录
///
/// # Arguments
/// * `conn` - 数据库连接
/// * `start_time` - 开始时间（RFC3339 格式）
/// * `end_time` - 结束时间（RFC3339 格式）
/// * `limit` - 查询限制数
///
/// # Returns
/// 返回查询结果列表
pub async fn list_annotations_by_time_range(
    conn: &Surreal<Any>,
    start_time: &str,
    end_time: &str,
    limit: Option<usize>,
) -> Result<Vec<AnnotationQueryResult>> {
    let limit_clause = limit.map_or(String::new(), |l| format!(" LIMIT {}", l));
    let sql = format!(
        "SELECT * FROM annotation WHERE created_at >= '{}' AND created_at <= '{}' ORDER BY created_at DESC{}",
        start_time, end_time, limit_clause
    );

    let mut result = conn.query(&sql).await?;
    let annotations: Vec<AnnotationQueryResult> = result.take(0)?;

    Ok(annotations)
}
