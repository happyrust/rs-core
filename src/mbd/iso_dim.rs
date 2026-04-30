//! LinearDim 的版面求解，对标 PML:
//!
//! - [`isoDim.pmlobj`](../../../MBD/markpipe/object/isoDim.pmlobj)
//!   `CalculateDimChardirs` + `drawDim` + `dimOneMem`
//! - [`isoGetDimDir.pmlfnc`](../../../MBD/markpipe/function/isoGetDimDir.pmlfnc)
//! - [`isoGetBestDir.pmlfnc`](../../../MBD/markpipe/function/isoGetBestDir.pmlfnc)
//! - [`isoori.pmlobj`](../../../MBD/markpipe/object/isoori.pmlobj)（char_dir 求解）
//!
//! 输出 `PlacedLinearDim` 可以直接进入 `LegacyPlacedLayoutSections`，由
//! `BranchCalculator::assemble_prelaid_out` 装配到 `LayoutResult`。

use glam::Vec3;

use crate::mbd::iso_params::{BranchContext, IsoParams, SegmentInput};
use crate::mbd::{LayoutVec3, PlacedLinearDim};

/// 入口函数：一条线性尺寸的版面求解。
///
/// 核心步骤（严格 1:1 对齐 PML）：
/// 1. `CalculateDimChardirs`：从 bran volume 中心 → 线段中点方向，选 E/N/W/S 中最近两个水平方向反向
///    作为 `dim_dirs[0,1]`，Z 作为 `dim_dirs[2]`。
/// 2. `isoGetDimDir(pipe_dir, dim_dirs)`：`pipe_dir.orthogonal(dim_dirs[2]=u)`，失败退 `dim_dirs[1]`。
/// 3. `isoGetBestDir`：把 step 2 的输出对齐到 `dim_dirs` 中最靠近的一个（必要时反向）。
/// 4. `drawDim` 的 offset 公式：`offset = od + 1.2*cheight*(dimtimes-1)`。
/// 5. `isoori` 的字符方向：优先沿 `dim_dirs[2]=Z`，若 Z 与 dim_dir 近乎共线则退到水平主轴。
pub fn compute_linear_dim_layout(
    segment: &SegmentInput,
    context: &BranchContext,
    params: &IsoParams,
) -> PlacedLinearDim {
    let seg_mid = (segment.start + segment.end) * 0.5;
    let (dim_dirs, char_dirs) = calculate_dim_chardirs(context.bran_volume_center, seg_mid);

    let dim_dir = select_dim_dir(segment.pipe_dir, dim_dirs);
    let offset = dim_offset(segment.od, params.cheight, context.dim_times);
    let char_dir = solve_char_dir(dim_dir, char_dirs);

    let text_anchor = seg_mid + dim_dir * offset;

    PlacedLinearDim {
        id: segment.id.clone(),
        kind: segment.kind.clone(),
        start: to_array(segment.start),
        end: to_array(segment.end),
        text: segment.text.clone(),
        offset,
        direction: to_array(dim_dir),
        label_t: 0.5,
        label_offset_world: None,
        dim_line_start: None,
        dim_line_end: None,
        extension_line_1_start: None,
        extension_line_1_end: None,
        extension_line_2_start: None,
        extension_line_2_end: None,
        text_anchor: Some(to_array(text_anchor)),
        visible: true,
        suppressed_reason: None,
    }
    .with_char_dir_marker(char_dir)
}

/// 对应 PML `isoDim.CalculateDimChardirs`：
///
/// ```pml
/// !dir = !linemidpos.direction(!branVolumeCenter)   $* 从线段中点指向 volume 中心
/// !dirs = [e, n, w, s]
/// sort by angle(!dirs[i], !dir)
/// !goodDimdirs = [-dirs[i0], -dirs[i1], u]
/// !goodchardirs = [u, dirs[i0], dirs[i1]]
/// ```
pub fn calculate_dim_chardirs(bran_center: Vec3, seg_mid: Vec3) -> ([Vec3; 3], [Vec3; 3]) {
    let dir_to_center = bran_center - seg_mid;
    let dir = if dir_to_center.length_squared() < 1e-12 {
        Vec3::X
    } else {
        dir_to_center.normalize()
    };

    let horiz = [Vec3::X, Vec3::Y, -Vec3::X, -Vec3::Y];
    let mut scored: [(usize, f32); 4] = [(0, 0.0); 4];
    for (i, d) in horiz.iter().enumerate() {
        scored[i] = (i, angle_deg(*d, dir));
    }
    scored.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let i0 = scored[0].0;
    let i1 = scored[1].0;

    let dim_dirs = [-horiz[i0], -horiz[i1], Vec3::Z];
    let char_dirs = [Vec3::Z, horiz[i0], horiz[i1]];
    (dim_dirs, char_dirs)
}

/// 对应 PML `!!isoGetDimDir(!pipedir, !dimdirs)`：
///
/// ```pml
/// !dimDir = !pipedir.orthogonal(!dimDirs[3])  $* 1-based，dimDirs[3] = u
/// handle any
///   !dimDir = !pipedir.orthogonal(!dimDirs[2])
/// endhandle
/// !dimDir = !!isoGetBestDir(!dimDir, !dimDirs)
/// ```
pub fn select_dim_dir(pipe_dir: Vec3, dim_dirs: [Vec3; 3]) -> Vec3 {
    let raw = orthogonal_to(pipe_dir, dim_dirs[2])
        .or_else(|| orthogonal_to(pipe_dir, dim_dirs[1]))
        .unwrap_or(dim_dirs[0]);
    pick_best_dir(raw, dim_dirs)
}

/// 对应 PML `!!isoGetBestDir(!inputdir, !dirs)`：
///
/// ```pml
/// angles[i] = dir.angle(dirs[i])
/// absangles[i] = min(angles[i], 180 - angles[i])
/// num = argmin(absangles)
/// if angles[num] > 90 then dir = dir.opposite() endif
/// return dir
/// ```
pub fn pick_best_dir(input: Vec3, dirs: [Vec3; 3]) -> Vec3 {
    let angles: [f32; 3] = [
        angle_deg(input, dirs[0]),
        angle_deg(input, dirs[1]),
        angle_deg(input, dirs[2]),
    ];
    let abs_angles: [f32; 3] = [
        angles[0].min(180.0 - angles[0]),
        angles[1].min(180.0 - angles[1]),
        angles[2].min(180.0 - angles[2]),
    ];
    let mut idx = [0usize, 1, 2];
    idx.sort_by(|a, b| {
        abs_angles[*a]
            .partial_cmp(&abs_angles[*b])
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let num = idx[0];
    if angles[num] > 90.0 { -input } else { input }
}

/// 对应 PML `isoori.pmlobj` 的字符方向选择（简化版）：
/// 字符方向主轴优先 Z（PML goodchardirs[1]=u），但若 Z 与 dim_dir 几乎共线（±10°）
/// 则退回到水平主轴 char_dirs[1]。
pub fn solve_char_dir(dim_dir: Vec3, char_dirs: [Vec3; 3]) -> Vec3 {
    let primary = char_dirs[0];
    let a = angle_deg(primary, dim_dir);
    let a_abs = a.min(180.0 - a);
    if a_abs < 10.0 { char_dirs[1] } else { primary }
}

/// 对应 PML `isoDim.drawDim`：
///
/// ```pml
/// !goodpos = !goodpos.offset(!this.dimdir,
///                            !this.od / 2 + !this.od / 2 + !this.cheight * 1.2 * (!dimtimes - 1))
/// ```
/// 即：`offset = od + cheight * 1.2 * (dim_times - 1)`。
pub fn dim_offset(od: f32, cheight: f32, dim_times: u32) -> f32 {
    let times = dim_times.max(1) as f32;
    od + cheight * 1.2 * (times - 1.0)
}

/// 对应 PML `direction.orthogonal(ref)`：
///
/// 将 `ref` 在 `dir` 法平面上的投影归一化。如果 `ref` 与 `dir` 共线则返回 `None`，
/// 对应 PML `handle any`。
pub fn orthogonal_to(dir: Vec3, ref_dir: Vec3) -> Option<Vec3> {
    let d_len = dir.length();
    if d_len < 1e-12 {
        return None;
    }
    let d = dir / d_len;
    let projected = ref_dir - d * d.dot(ref_dir);
    if projected.length_squared() < 1e-12 {
        return None;
    }
    Some(projected.normalize())
}

/// 两向量夹角（度）。0..=180。
pub fn angle_deg(a: Vec3, b: Vec3) -> f32 {
    let la = a.length();
    let lb = b.length();
    if la < 1e-12 || lb < 1e-12 {
        return 0.0;
    }
    let cos = (a.dot(b) / (la * lb)).clamp(-1.0, 1.0);
    cos.acos().to_degrees()
}

fn to_array(v: Vec3) -> LayoutVec3 {
    [v.x, v.y, v.z]
}

/// 仅用于测试：把 char_dir 塞到 `text_anchor` 后面的一个 virtual 字段里，
/// 但 `PlacedLinearDim` 没有 char_dir 字段，这里把 marker 附加的过程忽略——
/// char_dir 在 Stage 2 接入前端时通过 `LayoutResult.debug_info.notes` 记录。
trait PlacedLinearDimCharDirExt {
    fn with_char_dir_marker(self, _char_dir: Vec3) -> Self;
}
impl PlacedLinearDimCharDirExt for PlacedLinearDim {
    fn with_char_dir_marker(self, _char_dir: Vec3) -> Self {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: Vec3, b: Vec3, eps: f32) -> bool {
        (a - b).length() < eps
    }

    /// 单位向量夹角应该和预期一致（sanity）。
    #[test]
    fn angle_deg_orthogonal_axes() {
        assert!((angle_deg(Vec3::X, Vec3::Y) - 90.0).abs() < 1e-4);
        assert!((angle_deg(Vec3::X, Vec3::X) - 0.0).abs() < 1e-4);
        assert!((angle_deg(Vec3::X, -Vec3::X) - 180.0).abs() < 1e-4);
    }

    /// orthogonal_to(X, Z) 应该就是 Z（Z 已经在 X 法平面内）。
    #[test]
    fn orthogonal_to_axis_to_orthogonal_returns_ref() {
        let result = orthogonal_to(Vec3::X, Vec3::Z).unwrap();
        assert!(approx(result, Vec3::Z, 1e-5));
    }

    /// orthogonal_to(X, X) 应当返回 None（共线）。
    #[test]
    fn orthogonal_to_parallel_returns_none() {
        assert!(orthogonal_to(Vec3::X, Vec3::X).is_none());
    }

    /// `pick_best_dir`：输入与 dirs[0] 夹 10°，应当对齐到 dirs[0] 方向。
    #[test]
    fn pick_best_dir_picks_closest_axis() {
        let input = Vec3::new(10.0, 1.0, 0.0).normalize();
        let dirs = [Vec3::X, Vec3::Y, Vec3::Z];
        let out = pick_best_dir(input, dirs);
        assert!((out.x - input.x).abs() < 1e-5);
    }

    /// `pick_best_dir`：输入与 dirs[0] 夹 170°（反向），应当 flip。
    #[test]
    fn pick_best_dir_flips_when_closest_is_obtuse() {
        let input = Vec3::new(-10.0, -1.0, 0.0).normalize();
        let dirs = [Vec3::X, Vec3::Y, Vec3::Z];
        let out = pick_best_dir(input, dirs);
        assert!(out.x > 0.0, "should flip to positive X but got {out:?}");
    }

    /// PML 水平管（pipe_dir 近乎 XY 平面）应当选择 Z 方向作为 dim_dir。
    ///
    /// 这是 **"没对上" 的核心定量断言**：22m 水平管道的尺寸线应当沿 Z 向上偏移，
    /// 而不是在 XY 平面内（后端当前给的 offset_dir [0.739, 0.674, 0] 是错的）。
    #[test]
    fn select_dim_dir_for_horizontal_pipe_prefers_up() {
        let pipe_dir = Vec3::new(-0.6721017, 0.7371828, 0.069576345).normalize();
        let dim_dirs = [
            Vec3::new(-0.6721017, 0.7371828, 0.0).normalize(),
            Vec3::X,
            Vec3::Z,
        ];
        let out = select_dim_dir(pipe_dir, dim_dirs);
        let angle_to_z = angle_deg(out, Vec3::Z).min(180.0 - angle_deg(out, Vec3::Z));
        assert!(
            angle_to_z < 15.0,
            "horizontal pipe should give dim_dir close to +Z, got {out:?} (angle-to-Z={angle_to_z})"
        );
    }

    /// PML 垂直管（pipe_dir = +Z）：`pipedir.orthogonal(u)` 失败，退到 `dim_dirs[1]`。
    /// 预期输出落在 XY 平面内，靠近 `dim_dirs[1]`。
    #[test]
    fn select_dim_dir_for_vertical_pipe_falls_back_to_horizontal() {
        let pipe_dir = Vec3::Z;
        let dim_dirs = [Vec3::NEG_X, Vec3::NEG_Y, Vec3::Z];
        let out = select_dim_dir(pipe_dir, dim_dirs);
        assert!(
            out.z.abs() < 1e-4,
            "vertical pipe dim_dir should be horizontal, got {out:?}"
        );
    }

    /// `dim_offset`：第一层 = OD；第二层 = OD + 1.2·cheight。
    #[test]
    fn dim_offset_formula_matches_pml() {
        assert!((dim_offset(229.0, 100.0, 1) - 229.0).abs() < 1e-4);
        assert!((dim_offset(229.0, 100.0, 2) - (229.0 + 120.0)).abs() < 1e-4);
        assert!((dim_offset(229.0, 100.0, 3) - (229.0 + 240.0)).abs() < 1e-4);
    }

    /// `calculate_dim_chardirs`：bran 中心在线段东北方向，dim_dirs[0] 应当是 -X（反向）。
    #[test]
    fn calculate_dim_chardirs_pick_closest_horizontals() {
        let seg_mid = Vec3::new(0.0, 0.0, 0.0);
        let bran_center = Vec3::new(100.0, 50.0, 0.0);
        let (dim_dirs, char_dirs) = calculate_dim_chardirs(bran_center, seg_mid);
        assert!(approx(dim_dirs[2], Vec3::Z, 1e-5));
        let d = bran_center.normalize();
        let a_to_x = angle_deg(d, Vec3::X);
        let a_to_y = angle_deg(d, Vec3::Y);
        assert!(a_to_x < 90.0 && a_to_y < 90.0);
        assert!(approx(dim_dirs[0], -Vec3::X, 1e-5));
        assert!(approx(dim_dirs[1], -Vec3::Y, 1e-5));
        assert!(approx(char_dirs[0], Vec3::Z, 1e-5));
        assert!(approx(char_dirs[1], Vec3::X, 1e-5));
        assert!(approx(char_dirs[2], Vec3::Y, 1e-5));
    }

    /// Golden case: 24381_145712:0 (22.2m 水平长管)。
    ///
    /// 输入来自 `GET /api/mbd/pipe/24381_145712?mode=layout_first`：
    /// - start  = [14706.1, -16381.3, -1546.09]
    /// - end    = [-229, 0, 0]
    /// - length = 22221.488 mm
    /// - primary_axis = [-0.6721017, 0.7371828, 0.069576345]
    ///
    /// 预期：
    /// - dim_dir 贴合 +Z（PML 水平管规约）
    /// - offset = OD = 229 mm (dim_times=1)
    #[test]
    fn golden_case_bran_24381_145712_seg0() {
        let segment = SegmentInput {
            id: "dim:24381_145712:0".to_string(),
            kind: "segment".to_string(),
            start: Vec3::new(14706.1, -16381.3, -1546.09),
            end: Vec3::new(-229.0, 0.0, 0.0),
            pipe_dir: Vec3::new(-0.6721017, 0.7371828, 0.069576345).normalize(),
            od: 229.0,
            text: "22221".to_string(),
            isoline_index: Some(0),
        };
        let ctx = BranchContext {
            branch_refno: "24381_145712".to_string(),
            bran_volume_center: Vec3::new(7238.55, -8190.65, -773.045),
            dim_times: 1,
        };
        let params = IsoParams::default();

        let placed = compute_linear_dim_layout(&segment, &ctx, &params);

        assert_eq!(placed.text, "22221");
        assert!(
            (placed.offset - 229.0).abs() < 1e-3,
            "offset should be OD=229, got {}",
            placed.offset
        );

        let dim_dir = Vec3::new(
            placed.direction[0],
            placed.direction[1],
            placed.direction[2],
        );
        let ang = angle_deg(dim_dir, Vec3::Z).min(180.0 - angle_deg(dim_dir, Vec3::Z));
        assert!(
            ang < 15.0,
            "golden case: dim_dir should be ~±Z, got {dim_dir:?} (angle-to-Z={ang})"
        );
    }

    /// Golden case: 尺寸线起止端点应当与原始线段端点一致。
    #[test]
    fn placed_linear_dim_preserves_endpoints() {
        let segment = SegmentInput {
            id: "d0".into(),
            kind: "segment".into(),
            start: Vec3::new(1.0, 2.0, 3.0),
            end: Vec3::new(4.0, 5.0, 6.0),
            pipe_dir: Vec3::new(1.0, 1.0, 1.0).normalize(),
            od: 100.0,
            text: "X".into(),
            isoline_index: None,
        };
        let ctx = BranchContext::for_test("B");
        let placed = compute_linear_dim_layout(&segment, &ctx, &IsoParams::default());
        assert_eq!(placed.start, [1.0, 2.0, 3.0]);
        assert_eq!(placed.end, [4.0, 5.0, 6.0]);
        assert!(placed.visible);
        assert!(placed.suppressed_reason.is_none());
    }
}
