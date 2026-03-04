use anyhow::{Result, anyhow};
use glam::Vec2;
use spade::{ConstrainedDelaunayTriangulation, Point2, Triangulation, mitigate_underflow};
use std::collections::HashMap;

const SEGMENT_EPS: f32 = 1e-4;
const TRIANGLE_AREA2_EPS: f32 = 1e-8;

/// 使用 spade 对简单闭合多边形进行三角化。
///
/// 返回的索引直接指向输入 `points`（保持原轮廓点序），便于与侧面/端面共享顶点体系。
pub fn triangulate_polygon_indices_spade(points: &[Vec2]) -> Result<Vec<u32>> {
    if points.len() < 3 {
        return Err(anyhow!("三角化失败：点数不足（< 3）"));
    }

    let mut cdt = ConstrainedDelaunayTriangulation::<Point2<f64>>::new();
    let mut handles = Vec::with_capacity(points.len());
    let mut handle_to_input_index: HashMap<usize, u32> = HashMap::with_capacity(points.len());

    for (i, p) in points.iter().enumerate() {
        let v = mitigate_underflow(Point2::new(p.x as f64, p.y as f64));
        let handle = cdt
            .insert(v)
            .map_err(|e| anyhow!("spade 插入顶点失败: idx={}, err={}", i, e))?;
        handles.push(handle);
        handle_to_input_index
            .entry(handle.index())
            .or_insert(i as u32);
    }

    for i in 0..points.len() {
        let from = handles[i];
        let to = handles[(i + 1) % points.len()];
        if from == to || cdt.exists_constraint(from, to) {
            continue;
        }
        if !cdt.can_add_constraint(from, to) {
            return Err(anyhow!(
                "spade 约束边添加失败（可能自交）: edge=({}->{})",
                i,
                (i + 1) % points.len()
            ));
        }
        cdt.add_constraint(from, to);
    }

    let mut indices = Vec::new();

    for face in cdt.inner_faces() {
        let [v0, v1, v2] = face.vertices();
        let tri = [
            *handle_to_input_index
                .get(&v0.fix().index())
                .ok_or_else(|| anyhow!("spade 顶点映射失败: {}", v0.fix().index()))?,
            *handle_to_input_index
                .get(&v1.fix().index())
                .ok_or_else(|| anyhow!("spade 顶点映射失败: {}", v1.fix().index()))?,
            *handle_to_input_index
                .get(&v2.fix().index())
                .ok_or_else(|| anyhow!("spade 顶点映射失败: {}", v2.fix().index()))?,
        ];

        if tri[0] == tri[1] || tri[1] == tri[2] || tri[0] == tri[2] {
            continue;
        }

        let p0 = points[tri[0] as usize];
        let p1 = points[tri[1] as usize];
        let p2 = points[tri[2] as usize];
        let area2 = (p1 - p0).perp_dot(p2 - p0).abs();
        if area2 <= TRIANGLE_AREA2_EPS {
            continue;
        }

        let centroid = (p0 + p1 + p2) / 3.0;
        if !point_in_polygon_inclusive(centroid, points) {
            continue;
        }

        indices.extend_from_slice(&tri);
    }

    if indices.is_empty() {
        return Err(anyhow!("spade 三角化后无有效三角形"));
    }

    Ok(indices)
}

fn point_in_polygon_inclusive(p: Vec2, polygon: &[Vec2]) -> bool {
    if polygon.len() < 3 {
        return false;
    }

    for i in 0..polygon.len() {
        let a = polygon[i];
        let b = polygon[(i + 1) % polygon.len()];
        if point_on_segment_eps(p, a, b, SEGMENT_EPS) {
            return true;
        }
    }

    let mut inside = false;
    for i in 0..polygon.len() {
        let a = polygon[i];
        let b = polygon[(i + 1) % polygon.len()];
        let intersects = ((a.y > p.y) != (b.y > p.y))
            && (p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x);
        if intersects {
            inside = !inside;
        }
    }
    inside
}

fn point_on_segment_eps(p: Vec2, a: Vec2, b: Vec2, eps: f32) -> bool {
    let ab = b - a;
    let ap = p - a;
    let ab_len = ab.length();
    if ab_len <= f32::EPSILON {
        return ap.length() <= eps;
    }

    let cross = ab.perp_dot(ap).abs();
    if cross > eps * ab_len {
        return false;
    }

    let dot = ap.dot(ab);
    dot >= -eps && dot <= ab.length_squared() + eps
}
