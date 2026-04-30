//! 标注方向决策算法 — 移植自 PML `isoGetDimDir`、`isoGetBestDir`、`CalculateDimChardirs`。
//!
//! 管道标注需要决定"标注线往管段的哪一侧偏移"以及"文字朝哪个方向阅读"。
//! PML 的做法是：
//! 1. 从管道在工厂中的位置推断 3 个"优选方向"（`dimdirs`）；
//! 2. 用管段方向与优选方向叉积得到标注偏移方向（`dimdir`）；
//! 3. 用"最近优选方向一致性"校正方向，确保同侧（`isoGetBestDir`）。
//!
//! 参考文件：
//! - `rs-core/MBD/markpipe/function/isoGetDimDir.pmlfnc`
//! - `rs-core/MBD/markpipe/function/isoGetBestDir.pmlfnc`
//! - `rs-core/MBD/markpipe/object/isoDim.pmlobj` → `CalculateDimChardirs` 方法

use super::primitive::Vec3V2;

const EAST: [f32; 3] = [1.0, 0.0, 0.0];
const NORTH: [f32; 3] = [0.0, 1.0, 0.0];
const WEST: [f32; 3] = [-1.0, 0.0, 0.0];
const SOUTH: [f32; 3] = [0.0, -1.0, 0.0];
const UP: [f32; 3] = [0.0, 0.0, 1.0];

/// 优选方向组：标注偏移方向 + 字符方向。
#[derive(Debug, Clone)]
pub struct PreferredDirs {
    /// 3 个优选标注偏移方向，按优先级排序。
    pub dim_dirs: [Vec3V2; 3],
    /// 3 个优选字符阅读方向，按优先级排序。
    pub char_dirs: [Vec3V2; 3],
}

impl Default for PreferredDirs {
    fn default() -> Self {
        Self {
            dim_dirs: [EAST, NORTH, UP],
            char_dirs: [UP, EAST, NORTH],
        }
    }
}

/// 标注方向解算结果。
#[derive(Debug, Clone, Copy)]
pub struct DimDirectionResult {
    /// 标注偏移方向（标注线从管段向此方向偏移）。
    pub dim_dir: Vec3V2,
    /// 文字阅读方向（水平向右）。
    pub text_orientation: Vec3V2,
    /// 文字上方向。
    pub text_up: Vec3V2,
}

// ── 向量工具函数 ──

fn v3(a: Vec3V2) -> glam::Vec3 {
    glam::Vec3::new(a[0], a[1], a[2])
}

fn to_arr(v: glam::Vec3) -> Vec3V2 {
    [v.x, v.y, v.z]
}

fn angle_deg(a: glam::Vec3, b: glam::Vec3) -> f32 {
    a.angle_between(b).to_degrees()
}

// ── 核心算法 ──

/// 移植自 `isoGetBestDir`：确保 `input_dir` 与最近的优选方向同侧。
///
/// 算法：
/// 1. 计算 `input_dir` 与 `dirs[0..3]` 各自的夹角
/// 2. 取 min(angle, 180 - angle) 作为绝对偏差
/// 3. 找绝对偏差最小的优选方向
/// 4. 如果 `input_dir` 与该方向原始夹角 > 90°，取反
pub fn iso_get_best_dir(input_dir: Vec3V2, dirs: &[Vec3V2; 3]) -> Vec3V2 {
    let dir = v3(input_dir);

    let angles: Vec<f32> = dirs.iter().map(|d| angle_deg(dir, v3(*d))).collect();

    let abs_angles: Vec<f32> = angles
        .iter()
        .map(|&a| a.min(180.0 - a))
        .collect();

    let best_idx = abs_angles
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i)
        .unwrap_or(0);

    if angles[best_idx] > 90.0 {
        to_arr(-dir)
    } else {
        input_dir
    }
}

/// 移植自 `isoGetDimDir`：从管段方向和优选方向推导标注偏移方向。
///
/// 算法：
/// 1. dimdir = pipedir × dimdirs[2]（叉积，垂直于管段且偏向前两个优选方向）
/// 2. 如果管段方向与 dimdirs[2] 平行（叉积接近零），fallback 用 dimdirs[1]
/// 3. 用 `iso_get_best_dir` 校正方向一致性
pub fn iso_get_dim_dir(pipedir: Vec3V2, dim_dirs: &[Vec3V2; 3]) -> Vec3V2 {
    let pipe = v3(pipedir);

    let cross = pipe.cross(v3(dim_dirs[2]));
    let dim_dir = if cross.length_squared() > 1e-6 {
        cross.normalize()
    } else {
        let cross2 = pipe.cross(v3(dim_dirs[1]));
        if cross2.length_squared() > 1e-6 {
            cross2.normalize()
        } else {
            return dim_dirs[0];
        }
    };

    iso_get_best_dir(to_arr(dim_dir), dim_dirs)
}

/// 移植自 `CalculateDimChardirs`：从管段位置和包围盒推断优选方向。
///
/// `segment_midpoint`：标注管段的中点坐标。
/// `bran_bbox_center`：管道分支包围盒的中心坐标。
///
/// 算法：
/// 1. 计算 segment_midpoint → bran_bbox_center 的方向
/// 2. 在 E/N/W/S 四个水平方向中，按与该方向的夹角排序
/// 3. 标注向"远离包围盒中心"方向偏移（取 opposite）
/// 4. 字符优选方向以 U（上）为首
pub fn calculate_dim_char_dirs(
    segment_midpoint: Vec3V2,
    bran_bbox_center: Vec3V2,
) -> PreferredDirs {
    let mid = v3(segment_midpoint);
    let center = v3(bran_bbox_center);
    let diff = center - mid;

    if diff.length_squared() < 1e-6 {
        return PreferredDirs::default();
    }

    let to_center = diff.normalize();

    let cardinal_dirs = [v3(EAST), v3(NORTH), v3(WEST), v3(SOUTH)];
    let mut angles: Vec<(usize, f32)> = cardinal_dirs
        .iter()
        .enumerate()
        .map(|(i, d)| (i, angle_deg(*d, to_center)))
        .collect();
    angles.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let nearest = cardinal_dirs[angles[0].0];
    let second = cardinal_dirs[angles[1].0];

    PreferredDirs {
        dim_dirs: [to_arr(-nearest), to_arr(-second), UP],
        char_dirs: [UP, to_arr(nearest), to_arr(second)],
    }
}

/// 完整的标注方向解算：管段方向 + 包围盒 → 偏移方向 + 文字朝向。
///
/// 这是 Phase 1 集成到 assembler 的主入口。
pub fn resolve_dim_direction(
    pipedir: Vec3V2,
    segment_midpoint: Vec3V2,
    bran_bbox_center: Option<Vec3V2>,
) -> DimDirectionResult {
    let preferred = match bran_bbox_center {
        Some(center) => calculate_dim_char_dirs(segment_midpoint, center),
        None => PreferredDirs::default(),
    };

    let dim_dir = iso_get_dim_dir(pipedir, &preferred.dim_dirs);

    let pipe = v3(pipedir).normalize();
    let dim = v3(dim_dir).normalize();

    let text_orientation = to_arr(pipe);

    let up_candidate = pipe.cross(dim);
    let text_up = if up_candidate.length_squared() > 1e-6 {
        to_arr(up_candidate.normalize())
    } else {
        preferred.char_dirs[0]
    };

    DimDirectionResult {
        dim_dir,
        text_orientation,
        text_up,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const E: Vec3V2 = EAST;
    const N: Vec3V2 = NORTH;
    const W: Vec3V2 = WEST;
    const S: Vec3V2 = SOUTH;
    const U: Vec3V2 = UP;

    fn approx_eq(a: Vec3V2, b: Vec3V2) -> bool {
        (a[0] - b[0]).abs() < 1e-4 && (a[1] - b[1]).abs() < 1e-4 && (a[2] - b[2]).abs() < 1e-4
    }

    fn assert_approx(label: &str, actual: Vec3V2, expected: Vec3V2) {
        assert!(
            approx_eq(actual, expected),
            "{}: expected {:?}, got {:?}",
            label,
            expected,
            actual
        );
    }

    #[test]
    fn iso_get_best_dir_keeps_same_side() {
        let dirs = [E, N, U];
        let result = iso_get_best_dir(E, &dirs);
        assert_approx("east stays east", result, E);
    }

    #[test]
    fn iso_get_best_dir_flips_opposite() {
        let dirs = [E, N, U];
        let result = iso_get_best_dir(W, &dirs);
        assert_approx("west flips to east", result, E);
    }

    #[test]
    fn iso_get_best_dir_picks_closest() {
        let dirs = [E, N, U];
        let northeast = [0.707, 0.707, 0.0];
        let result = iso_get_best_dir(northeast, &dirs);
        assert!(
            result[0] > 0.0 && result[1] > 0.0,
            "northeast should stay northeast, got {:?}",
            result
        );
    }

    #[test]
    fn iso_get_dim_dir_horizontal_pipe_east() {
        let pipedir = E;
        let dim_dirs = [N, W, U];
        let result = iso_get_dim_dir(pipedir, &dim_dirs);
        let v = v3(result);
        let pipe = v3(pipedir);
        let dot = v.dot(pipe).abs();
        assert!(
            dot < 0.01,
            "dim_dir should be perpendicular to pipe, dot = {}",
            dot
        );
    }

    #[test]
    fn iso_get_dim_dir_vertical_pipe() {
        let pipedir = U;
        let dim_dirs = [E, N, U];
        let result = iso_get_dim_dir(pipedir, &dim_dirs);
        let v = v3(result);
        let pipe = v3(pipedir);
        let dot = v.dot(pipe).abs();
        assert!(
            dot < 0.01,
            "dim_dir should be perpendicular to vertical pipe, dot = {}",
            dot
        );
    }

    #[test]
    fn calculate_dim_char_dirs_pipe_on_east_side() {
        let segment_mid = [100.0, 0.0, 0.0];
        let bbox_center = [50.0, 50.0, 0.0];
        let dirs = calculate_dim_char_dirs(segment_mid, bbox_center);
        let first_dim = v3(dirs.dim_dirs[0]);
        let to_center = v3(bbox_center) - v3(segment_mid);
        let angle = angle_deg(first_dim, to_center.normalize());
        assert!(
            angle > 90.0,
            "dim_dir[0] should point away from bbox center, angle = {}",
            angle
        );
    }

    #[test]
    fn calculate_dim_char_dirs_same_point_returns_default() {
        let p = [0.0, 0.0, 0.0];
        let dirs = calculate_dim_char_dirs(p, p);
        assert_approx("default dim[0]", dirs.dim_dirs[0], E);
    }

    #[test]
    fn resolve_dim_direction_with_bbox() {
        let pipedir = E;
        let mid = [0.0, 0.0, 0.0];
        let center = [0.0, 100.0, 0.0];
        let result = resolve_dim_direction(pipedir, mid, Some(center));

        let pipe = v3(pipedir);
        let dim = v3(result.dim_dir);
        assert!(
            pipe.dot(dim).abs() < 0.01,
            "dim_dir perpendicular to pipe"
        );

        assert!(
            result.dim_dir[1] < -0.5,
            "should point away from north bbox, got {:?}",
            result.dim_dir
        );
    }

    #[test]
    fn resolve_dim_direction_without_bbox() {
        let pipedir = E;
        let mid = [0.0, 0.0, 0.0];
        let result = resolve_dim_direction(pipedir, mid, None);
        let pipe = v3(pipedir);
        let dim = v3(result.dim_dir);
        assert!(
            pipe.dot(dim).abs() < 0.01,
            "dim_dir perpendicular to pipe"
        );
    }

    #[test]
    fn resolve_dim_direction_diagonal_pipe() {
        let pipedir = [0.707, 0.707, 0.0];
        let mid = [50.0, 50.0, 0.0];
        let center = [0.0, 0.0, 0.0];
        let result = resolve_dim_direction(pipedir, mid, Some(center));
        let pipe = v3(pipedir).normalize();
        let dim = v3(result.dim_dir).normalize();
        assert!(
            pipe.dot(dim).abs() < 0.1,
            "dim_dir should be roughly perpendicular to diagonal pipe, dot = {}",
            pipe.dot(dim)
        );
    }
}
