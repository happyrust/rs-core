//! 引线路由 — 复刻 PDMS `mlabel.addleadline`。
//!
//! 标签文字放置完成后，需要从标注关联点（`dim_pos`）画一条引线到文字框。
//! 引线的终点选择文字方框的**最近角点**，与 PDMS `mlabel.addleadline` 一致。

use super::primitive::Vec3V2;

/// 计算引线路由：从 `dim_pos`（标注关联点）到文字框最近角的折线。
///
/// 文字框由 `text_anchor`（左下角基线起点）、`text_width_mm`、`text_height_mm`、
/// `orientation`（文字阅读方向/右向量）和 `up`（文字上方向）确定四个角点。
///
/// 返回引线折线点序列（至少 2 个点：`[dim_pos, nearest_corner]`）。
pub fn route_leader_line(
    dim_pos: Vec3V2,
    text_anchor: Vec3V2,
    text_width_mm: f32,
    text_height_mm: f32,
    orientation: Vec3V2,
    up: Vec3V2,
) -> Vec<Vec3V2> {
    let corners = text_box_corners(text_anchor, text_width_mm, text_height_mm, orientation, up);
    let nearest = nearest_corner(dim_pos, &corners);
    vec![dim_pos, nearest]
}

/// 计算文字方框四个角点。
///
/// 角点顺序：左下(anchor) → 右下 → 右上 → 左上。
fn text_box_corners(
    anchor: Vec3V2,
    width: f32,
    height: f32,
    orientation: Vec3V2,
    up: Vec3V2,
) -> [Vec3V2; 4] {
    let p1 = anchor; // 左下
    let p2 = add_scaled(anchor, orientation, width); // 右下
    let p3 = add_scaled(p2, up, height); // 右上
    let p4 = add_scaled(anchor, up, height); // 左上
    [p1, p2, p3, p4]
}

/// 从四角中选距 `target` 最近的角。
fn nearest_corner(target: Vec3V2, corners: &[Vec3V2; 4]) -> Vec3V2 {
    let mut best = corners[0];
    let mut best_dist_sq = dist_sq(target, best);
    for &c in &corners[1..] {
        let d = dist_sq(target, c);
        if d < best_dist_sq {
            best_dist_sq = d;
            best = c;
        }
    }
    best
}

fn add_scaled(base: Vec3V2, dir: Vec3V2, scale: f32) -> Vec3V2 {
    [
        base[0] + dir[0] * scale,
        base[1] + dir[1] * scale,
        base[2] + dir[2] * scale,
    ]
}

fn dist_sq(a: Vec3V2, b: Vec3V2) -> f32 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    dx * dx + dy * dy + dz * dz
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leader_to_nearest_corner_bottom_left() {
        // text at origin, orientation +X, up +Y, dim_pos below-left
        let result = route_leader_line(
            [-10.0, -10.0, 0.0], // dim_pos
            [0.0, 0.0, 0.0],     // text_anchor
            20.0,                // width
            5.0,                 // height
            [1.0, 0.0, 0.0],     // orientation
            [0.0, 1.0, 0.0],     // up
        );
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], [-10.0, -10.0, 0.0]); // dim_pos
        assert_eq!(result[1], [0.0, 0.0, 0.0]); // nearest corner = anchor
    }

    #[test]
    fn leader_to_nearest_corner_top_right() {
        let result = route_leader_line(
            [30.0, 10.0, 0.0], // dim_pos (beyond top-right)
            [0.0, 0.0, 0.0],
            20.0,
            5.0,
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
        );
        // corners: [0,0,0], [20,0,0], [20,5,0], [0,5,0]
        // nearest to [30,10,0] is [20,5,0]
        assert_eq!(result[1], [20.0, 5.0, 0.0]);
    }

    #[test]
    fn leader_3d_case() {
        let result = route_leader_line(
            [0.0, 0.0, -10.0],
            [0.0, 0.0, 0.0],
            10.0,
            3.0,
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
        );
        // closest corner to [0,0,-10] is [0,0,0] (p1)
        assert_eq!(result[1], [0.0, 0.0, 0.0]);
    }

    #[test]
    fn text_box_corners_basic() {
        let corners =
            text_box_corners([0.0, 0.0, 0.0], 10.0, 5.0, [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
        assert_eq!(corners[0], [0.0, 0.0, 0.0]); // bottom-left
        assert_eq!(corners[1], [10.0, 0.0, 0.0]); // bottom-right
        assert_eq!(corners[2], [10.0, 5.0, 0.0]); // top-right
        assert_eq!(corners[3], [0.0, 5.0, 0.0]); // top-left
    }
}
