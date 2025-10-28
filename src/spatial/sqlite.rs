#![cfg(not(target_arch = "wasm32"))]

use anyhow::{Context, Result, anyhow};
use glam::Vec3;
use nalgebra::{Point3, Vector3};
use parry3d::bounding_volume::Aabb;
use rusqlite::{Connection, OpenFlags, OptionalExtension, Row, params};

use crate::{RefU64, get_db_option};

fn ensure_sqlite_enabled() -> Result<()> {
    let db_option = get_db_option();
    if !db_option.sqlite_index_enabled() {
        return Err(anyhow!("未启用 SQLite 空间索引，请检查 DbOption 配置"));
    }
    Ok(())
}

pub fn open_connection() -> Result<Connection> {
    ensure_sqlite_enabled()?;
    let db_option = get_db_option();
    let path = db_option.get_sqlite_index_path();
    if !path.exists() {
        return Err(anyhow!("SQLite 空间索引文件不存在: {}", path.display()));
    }
    Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("无法打开 SQLite 空间索引文件 {}", path.display()))
}

fn map_row_to_aabb(row: &Row<'_>) -> rusqlite::Result<Aabb> {
    let min_x: f64 = row.get(1)?;
    let max_x: f64 = row.get(2)?;
    let min_y: f64 = row.get(3)?;
    let max_y: f64 = row.get(4)?;
    let min_z: f64 = row.get(5)?;
    let max_z: f64 = row.get(6)?;
    Ok(Aabb::new(
        parry3d::math::Point::new(min_x as f32, min_y as f32, min_z as f32),
        parry3d::math::Point::new(max_x as f32, max_y as f32, max_z as f32),
    ))
}

pub fn query_containing_point(point: Vec3, limit: usize) -> Result<Vec<(RefU64, Aabb)>> {
    let conn = open_connection()?;
    query_containing_point_with_conn(&conn, point, limit)
}

pub fn query_containing_point_with_conn(
    conn: &Connection,
    point: Vec3,
    limit: usize,
) -> Result<Vec<(RefU64, Aabb)>> {
    let mut stmt = conn.prepare(
        "SELECT id, min_x, max_x, min_y, max_y, min_z, max_z
         FROM aabb_index
         WHERE min_x <= ?1 AND max_x >= ?1
           AND min_y <= ?2 AND max_y >= ?2
           AND min_z <= ?3 AND max_z >= ?3
         LIMIT ?4",
    )?;
    let rows = stmt.query_map(
        params![point.x as f64, point.y as f64, point.z as f64, limit as i64],
        |row| {
            let refno = RefU64(row.get::<_, i64>(0)? as u64);
            let aabb = map_row_to_aabb(row)?;
            Ok((refno, aabb))
        },
    )?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

pub fn query_aabb(refno: RefU64) -> Result<Option<Aabb>> {
    let conn = open_connection()?;
    query_aabb_with_conn(&conn, refno)
}

pub fn query_aabb_with_conn(conn: &Connection, refno: RefU64) -> Result<Option<Aabb>> {
    Ok(conn
        .query_row(
            "SELECT id, min_x, max_x, min_y, max_y, min_z, max_z
         FROM aabb_index
         WHERE id = ?1
         LIMIT 1",
            params![refno.0 as i64],
            |row| map_row_to_aabb(row),
        )
        .optional()?)
}

pub fn query_overlap(
    expanded: &Aabb,
    types: Option<&[String]>,
    limit: Option<usize>,
    exclude: &[RefU64],
) -> Result<Vec<(RefU64, Aabb, Option<String>)>> {
    let conn = open_connection()?;
    query_overlap_with_conn(&conn, expanded, types, limit, exclude)
}

pub fn query_overlap_with_conn(
    conn: &Connection,
    expanded: &Aabb,
    types: Option<&[String]>,
    limit: Option<usize>,
    exclude: &[RefU64],
) -> Result<Vec<(RefU64, Aabb, Option<String>)>> {
    use rusqlite::{ToSql, params_from_iter};

    let mut sql = String::from(
        "SELECT aabb_index.id, min_x, max_x, min_y, max_y, min_z, max_z, items.noun
         FROM aabb_index
         LEFT JOIN items ON items.id = aabb_index.id
         WHERE max_x >= ?1 AND min_x <= ?2
           AND max_y >= ?3 AND min_y <= ?4
           AND max_z >= ?5 AND min_z <= ?6",
    );
    let mut params: Vec<Box<dyn ToSql>> = vec![
        Box::new(expanded.mins.x as f64),
        Box::new(expanded.maxs.x as f64),
        Box::new(expanded.mins.y as f64),
        Box::new(expanded.maxs.y as f64),
        Box::new(expanded.mins.z as f64),
        Box::new(expanded.maxs.z as f64),
    ];

    if let Some(t) = types {
        if !t.is_empty() {
            sql.push_str(" AND items.noun IN (");
            for (idx, _) in t.iter().enumerate() {
                if idx > 0 {
                    sql.push(',');
                }
                sql.push('?');
            }
            sql.push(')');
            for ty in t {
                params.push(Box::new(ty.clone()));
            }
        }
    }

    if !exclude.is_empty() {
        sql.push_str(" AND aabb_index.id NOT IN (");
        for (idx, _) in exclude.iter().enumerate() {
            if idx > 0 {
                sql.push(',');
            }
            sql.push('?');
        }
        sql.push(')');
        for refno in exclude {
            params.push(Box::new(refno.0 as i64));
        }
    }

    if let Some(limit) = limit {
        sql.push_str(" LIMIT ?");
        params.push(Box::new(limit as i64));
    }

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(params.iter().map(|p| &**p)), |row| {
        let refno = RefU64(row.get::<_, i64>(0)? as u64);
        let aabb = map_row_to_aabb(row)?;
        let noun: Option<String> = row.get(7)?;
        Ok((refno, aabb, noun))
    })?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

pub fn query_knn(
    point: Vec3,
    k: usize,
    search_radius: Option<f32>,
    types: Option<&[String]>,
) -> Result<Vec<(RefU64, Aabb, f32, Option<String>)>> {
    let conn = open_connection()?;
    query_knn_with_conn(&conn, point, k, search_radius, types)
}

pub fn query_knn_with_conn(
    conn: &Connection,
    point: Vec3,
    k: usize,
    search_radius: Option<f32>,
    types: Option<&[String]>,
) -> Result<Vec<(RefU64, Aabb, f32, Option<String>)>> {
    let mut radius = search_radius.unwrap_or(1.0);
    let mut best: Vec<(RefU64, Aabb, f32, Option<String>)> = Vec::new();

    for _ in 0..10 {
        let expanded = Aabb::new(
            parry3d::math::Point::new(point.x - radius, point.y - radius, point.z - radius),
            parry3d::math::Point::new(point.x + radius, point.y + radius, point.z + radius),
        );
        let mut hits = query_overlap_with_conn(conn, &expanded, types, Some(k * 8), &[])?;
        hits.sort_by_key(|(refno, _, _)| refno.0);
        hits.dedup_by_key(|(refno, _, _)| refno.0);

        let mut enriched = Vec::with_capacity(hits.len());
        for (refno, aabb, noun) in hits {
            let dist = distance_point_aabb(point, &aabb);
            enriched.push((refno, aabb, dist, noun));
        }
        enriched.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));

        if enriched.len() >= k {
            enriched.truncate(k);
            return Ok(enriched);
        }

        best = enriched;
        radius *= 2.0;
    }

    if best.len() > k {
        best.truncate(k);
    }
    Ok(best)
}

pub fn distance_point_aabb(point: Vec3, aabb: &Aabb) -> f32 {
    let dx = if point.x < aabb.mins.x {
        aabb.mins.x - point.x
    } else if point.x > aabb.maxs.x {
        point.x - aabb.maxs.x
    } else {
        0.0
    };
    let dy = if point.y < aabb.mins.y {
        aabb.mins.y - point.y
    } else if point.y > aabb.maxs.y {
        point.y - aabb.maxs.y
    } else {
        0.0
    };
    let dz = if point.z < aabb.mins.z {
        aabb.mins.z - point.z
    } else if point.z > aabb.maxs.z {
        point.z - aabb.maxs.z
    } else {
        0.0
    };

    (dx * dx + dy * dy + dz * dz).sqrt()
}

pub fn ray_aabb_toi(
    origin: Point3<f32>,
    dir: Vector3<f32>,
    bb: &Aabb,
    max_distance: f32,
) -> Option<f32> {
    let mut tmin = f32::NEG_INFINITY;
    let mut tmax = f32::INFINITY;

    // X axis
    if dir.x != 0.0 {
        let inv = 1.0 / dir.x;
        let t1 = (bb.mins.x - origin.x) * inv;
        let t2 = (bb.maxs.x - origin.x) * inv;
        tmin = tmin.max(t1.min(t2));
        tmax = tmax.min(t1.max(t2));
    } else if origin.x < bb.mins.x || origin.x > bb.maxs.x {
        return None;
    }

    // Y axis
    if dir.y != 0.0 {
        let inv = 1.0 / dir.y;
        let t1 = (bb.mins.y - origin.y) * inv;
        let t2 = (bb.maxs.y - origin.y) * inv;
        tmin = tmin.max(t1.min(t2));
        tmax = tmax.min(t1.max(t2));
    } else if origin.y < bb.mins.y || origin.y > bb.maxs.y {
        return None;
    }

    // Z axis
    if dir.z != 0.0 {
        let inv = 1.0 / dir.z;
        let t1 = (bb.mins.z - origin.z) * inv;
        let t2 = (bb.maxs.z - origin.z) * inv;
        tmin = tmin.max(t1.min(t2));
        tmax = tmax.min(t1.max(t2));
    } else if origin.z < bb.mins.z || origin.z > bb.maxs.z {
        return None;
    }

    if tmax < 0.0 {
        return None;
    }
    let t = if tmin >= 0.0 { tmin } else { tmax };
    if t >= 0.0 && t <= max_distance {
        Some(t)
    } else {
        None
    }
}
