//! 标注朝向解算 — 移植自 PML `isoOri`。
//!
//! `IsoOri` 从管段方向（`pipedir`）、优选标注方向（`dimdirs`）、优选字符方向（`chardirs`）
//! 和已用方向列表（`used_dirs`）解算出完整的标注坐标系：
//! - `pipedir`（x 轴）：管段方向（可能翻转以确保字符可读性）
//! - `dimdir`（y 轴）：标注偏移方向
//! - `chardir`（z 轴）：字符阅读方向
//! - `ori`（orientation 矩阵）：`y = dimdir, z = chardir`
//!
//! 参考文件：
//! - `rs-core/MBD/markpipe/object/isoori.pmlobj`
//! - `rs-core/MBD/markpipe/function/isoGetHandleDimDir.pmlfnc`

use super::dim_direction::{iso_get_best_dir, iso_get_dim_dir};
use super::primitive::Vec3V2;
use super::used_dir::UsedDirRegistry;

/// 标注朝向解算结果。
#[derive(Debug, Clone, Copy)]
pub struct IsoOri {
    /// 管段方向（可能被翻转以确保字符可读性）。
    pub pipedir: Vec3V2,
    /// 标注偏移方向（标注线从管段向此方向偏移）。
    pub dimdir: Vec3V2,
    /// 字符阅读方向。
    pub chardir: Vec3V2,
}

fn v3(a: Vec3V2) -> glam::Vec3 {
    glam::Vec3::new(a[0], a[1], a[2])
}

fn to_arr(v: glam::Vec3) -> Vec3V2 {
    [v.x, v.y, v.z]
}

fn angle_deg(a: glam::Vec3, b: glam::Vec3) -> f32 {
    a.angle_between(b).to_degrees()
}

/// 计算标注朝向。
///
/// 移植自 `isoOri.getori()`：
/// 1. 如果有已用方向，调用 `get_handle_dim_dir` 找间隙方向
/// 2. 否则用 `iso_get_dim_dir` 从优选方向推导
/// 3. chardir = pipedir × dimdir
/// 4. 如果 chardir 与最优字符方向夹角 > 90°，翻转 pipedir
/// 5. 重新计算 chardir
pub fn compute_iso_ori(
    pipedir: Vec3V2,
    dim_dirs: &[Vec3V2; 3],
    char_dirs: &[Vec3V2; 3],
    used_dir_registry: &UsedDirRegistry,
    min_gap_angle: f32,
) -> IsoOri {
    let pipe = v3(pipedir).normalize();

    let dimdir = if !used_dir_registry.is_empty() {
        get_handle_dim_dir(pipedir, used_dir_registry, min_gap_angle, dim_dirs)
    } else {
        iso_get_dim_dir(pipedir, dim_dirs)
    };

    let dim = v3(dimdir).normalize();

    let chardir_initial = pipe.cross(dim);
    let chardir_initial = if chardir_initial.length_squared() > 1e-6 {
        chardir_initial.normalize()
    } else {
        v3(char_dirs[0])
    };

    let best_chardir = iso_get_best_dir(to_arr(chardir_initial), char_dirs);

    let mut final_pipe = pipe;
    if angle_deg(chardir_initial, v3(best_chardir)) > 90.0 {
        final_pipe = -pipe;
    }

    let chardir_final = final_pipe.cross(dim);
    let chardir_final = if chardir_final.length_squared() > 1e-6 {
        chardir_final.normalize()
    } else {
        v3(char_dirs[0])
    };

    IsoOri {
        pipedir: to_arr(final_pipe),
        dimdir: dimdir,
        chardir: to_arr(chardir_final),
    }
}

/// 在已用方向约束下，从角度间隙中选择最佳标注方向。
///
/// 简化版 `isoGetHandleDimDir`：
/// 1. 过滤掉与管段平行的已用方向
/// 2. 计算每个已用方向在管段垂直平面上的角度
/// 3. 在优选方向中找未被占用的方向
/// 4. 如果所有优选方向都被占用，选最大间隙的中间方向
fn get_handle_dim_dir(
    pipedir: Vec3V2,
    registry: &UsedDirRegistry,
    min_gap_angle: f32,
    good_dim_dirs: &[Vec3V2; 3],
) -> Vec3V2 {
    let pipe = v3(pipedir).normalize();

    let mut used_angles: Vec<f32> = Vec::new();

    let up = find_orthogonal(pipe);
    let right = pipe.cross(up).normalize();

    for used in registry.dirs.iter() {
        let dir = v3(used.direction);
        let angle_to_pipe = angle_deg(dir, pipe);
        if (angle_to_pipe - 90.0).abs() < 1.0 {
            continue;
        }

        let proj = dir - pipe * dir.dot(pipe);
        if proj.length_squared() < 1e-6 {
            continue;
        }
        let proj = proj.normalize();

        let angle = proj.dot(up).acos().to_degrees();
        let sign = if proj.dot(right) >= 0.0 { 1.0 } else { -1.0 };
        let full_angle = if sign >= 0.0 { angle } else { 360.0 - angle };
        used_angles.push(full_angle);
    }

    if used_angles.is_empty() {
        return iso_get_dim_dir(pipedir, good_dim_dirs);
    }

    for try_dir in good_dim_dirs {
        let td = v3(*try_dir);
        if (angle_deg(td, pipe) - 90.0).abs() > 10.0 {
            continue;
        }

        let proj = td - pipe * td.dot(pipe);
        if proj.length_squared() < 1e-6 {
            continue;
        }
        let proj = proj.normalize();
        let angle = proj.dot(up).acos().to_degrees();
        let sign = if proj.dot(right) >= 0.0 { 1.0 } else { -1.0 };
        let try_angle = if sign >= 0.0 { angle } else { 360.0 - angle };

        let mut good = true;
        for &ua in &used_angles {
            let diff = (try_angle - ua).abs();
            let diff = diff.min(360.0 - diff);
            if diff < min_gap_angle {
                good = false;
                break;
            }
        }
        if good {
            return *try_dir;
        }
    }

    used_angles.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let mut best_gap = 0.0f32;
    let mut best_mid_angle = 0.0f32;
    let n = used_angles.len();
    for i in 0..n {
        let next = if i + 1 < n {
            used_angles[i + 1]
        } else {
            used_angles[0] + 360.0
        };
        let gap = next - used_angles[i];
        if gap > best_gap {
            best_gap = gap;
            best_mid_angle = used_angles[i] + gap / 2.0;
        }
    }

    let mid_rad = best_mid_angle.to_radians();
    let mid_dir = up * mid_rad.cos() + right * mid_rad.sin();
    let mid_dir = mid_dir.normalize();

    to_arr(mid_dir)
}

fn find_orthogonal(v: glam::Vec3) -> glam::Vec3 {
    let candidates = [
        glam::Vec3::Y,
        glam::Vec3::Z,
        glam::Vec3::X,
    ];
    for c in &candidates {
        let cross = v.cross(*c);
        if cross.length_squared() > 1e-6 {
            return cross.normalize();
        }
    }
    glam::Vec3::Y
}


#[cfg(test)]
mod tests {
    use super::*;

    const E: Vec3V2 = [1.0, 0.0, 0.0];
    const N: Vec3V2 = [0.0, 1.0, 0.0];
    const U: Vec3V2 = [0.0, 0.0, 1.0];

    fn approx_perp(a: Vec3V2, b: Vec3V2) -> bool {
        let dot = a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        dot.abs() < 0.1
    }

    #[test]
    fn basic_ori_horizontal_pipe() {
        let ori = compute_iso_ori(
            E,
            &[N, [-1.0, 0.0, 0.0], U],
            &[U, E, N],
            &UsedDirRegistry::new(),
            60.0,
        );
        assert!(
            approx_perp(ori.pipedir, ori.dimdir),
            "pipedir should be perpendicular to dimdir"
        );
        assert!(
            approx_perp(ori.pipedir, ori.chardir),
            "pipedir should be perpendicular to chardir"
        );
        assert!(
            approx_perp(ori.dimdir, ori.chardir),
            "dimdir should be perpendicular to chardir"
        );
    }

    #[test]
    fn ori_vertical_pipe() {
        let ori = compute_iso_ori(
            U,
            &[E, N, U],
            &[U, E, N],
            &UsedDirRegistry::new(),
            60.0,
        );
        assert!(
            approx_perp(ori.pipedir, ori.dimdir),
            "vertical pipe: perpendicular"
        );
    }

    #[test]
    fn ori_with_used_dirs_avoids_conflict() {
        let mut registry = UsedDirRegistry::new();
        registry.register(super::super::used_dir::IsoUsedDir::new(
            "ISODIM",
            N,
            0.0,
            100.0,
            "MainDim",
        ));

        let ori = compute_iso_ori(
            E,
            &[N, [-1.0, 0.0, 0.0], U],
            &[U, E, N],
            &registry,
            60.0,
        );
        assert!(
            approx_perp(ori.pipedir, ori.dimdir),
            "with used dirs: still perpendicular"
        );
    }
}
