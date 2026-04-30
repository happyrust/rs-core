//! iso_slope / iso_tag / iso_weld / iso_bend / iso_cut_tubi 的最小对齐实现。
//!
//! MVP 策略（与 PML 语义对齐的要点，不是逐行翻译）：
//!
//! - [`iso_slope`](./iso_extras.rs#solve_slope) ↔
//!   [`isoSlope.pmlobj`](../../../MBD/markpipe/object/isoSlope.pmlobj)：
//!   根据 `(gradient, od)` 判断是否在 `[min_slope, max_slope]` 内，若在就生成 `PlacedSlope`，
//!   `label_offset_world` 按 PML `!p4 = !p1.offset(!ydir, !bore/4 + 10)` 的思路抬升
//!   `od/2 + 20` 毫米。超出阈值或 tubi 长度 < od 的直接 suppress。
//! - [`iso_weld`](./iso_extras.rs#solve_weld) ↔
//!   [`isoWeldText.pmlobj`](../../../MBD/markpipe/object/isoWeldText.pmlobj)：
//!   PML 侧本质是 POD；后端 dto 已经给了 `position/label/is_shop`，这里原样透传到 `PlacedWeld`。
//! - [`iso_tag`](./iso_extras.rs#solve_tag) ↔
//!   [`isoTag.pmlobj`](../../../MBD/markpipe/object/isoTag.pmlobj)：
//!   tag 的 text 生成逻辑在 PDMS 侧已经完成（infoarr），后端 `MbdTagDto.text` 已经是拼接结果，
//!   这里落地为 `PlacedTag` 的装配层。
//! - [`iso_bend`](./iso_extras.rs#solve_bend) ↔
//!   [`isoelbopad.pmlobj`](../../../MBD/markpipe/object/isoelbopad.pmlobj) +
//!   [`isombdangle.pmlobj`](../../../MBD/markpipe/object/isombdangle.pmlobj)：
//!   MVP 只装配 `size_dims`（每个 face_center 作为一条从 vertex 到 face 的线性尺寸），
//!   角度 arc 留给前端 AngleDimension3D。
//! - [`iso_cut_tubi`](./iso_extras.rs#solve_cut_tubi)：沿用 `iso_dim::compute_linear_dim_layout`，
//!   因为"切管段"本质就是 LinearDim。

use glam::Vec3;

use crate::mbd::iso_dim::{angle_deg, compute_linear_dim_layout};
use crate::mbd::iso_params::{BranchContext, IsoParams, SegmentInput};
use crate::mbd::{PlacedAngle, PlacedBend, PlacedLinearDim, PlacedSlope, PlacedTag, PlacedWeld};

/// iso_slope 输入。
#[derive(Debug, Clone)]
pub struct SlopeInput {
    pub id: String,
    pub tubi_start: Vec3,
    pub tubi_end: Vec3,
    /// 后端已经算好的坡度（dz/水平距离，带符号）。对应 PML `Gradient of $!tubi`。
    pub slope: f32,
    /// 外径（mm）。对应 PML `aod of $!tubi`。
    pub od: f32,
    /// 展示文本（已经由后端格式化好，形如 "slope 2.5%"）。
    pub text: String,
}

/// iso_tag 输入。
#[derive(Debug, Clone)]
pub struct TagInput {
    pub id: String,
    pub position: Vec3,
    pub text: String,
}

/// iso_weld 输入。
#[derive(Debug, Clone)]
pub struct WeldInput {
    pub id: String,
    pub position: Vec3,
    pub label: String,
    pub is_shop: bool,
    pub subtitle: Option<String>,
}

/// iso_bend 输入。
#[derive(Debug, Clone)]
pub struct BendInput {
    pub id: String,
    pub vertex: Vec3,
    pub face_center_1: Option<Vec3>,
    pub face_center_2: Option<Vec3>,
    /// 弯角度数。对应 PML `angle of $!elbo`。
    pub angle_deg: Option<f32>,
    /// 外径 OD（mm），用于派生 size_dims 的 offset。
    pub od: f32,
    /// size_dim 的显示文本：`[face1_text, face2_text]`
    pub face_texts: [Option<String>; 2],
    /// 角度文本，形如 "90°"
    pub angle_text: String,
}

/// 根据 PML `isoSlope` 决定是否生成坡度箭头，并产出 `PlacedSlope`。
///
/// - 若 `|slope| > max_slope` 或 `|slope| < min_slope`：返回 `suppressed_reason`。
/// - 若 tubi 长度小于 OD：返回 `suppressed_reason = "slope_tubi_too_short"`。
/// - 否则产出 `PlacedSlope`，`label_offset_world` 按 PML 公式抬升 `od/2 + 20`（偏 +Y）。
pub fn solve_slope(input: &SlopeInput, params: &IsoParams) -> PlacedSlope {
    let abs_slope = input.slope.abs();
    let tubi_len = input.tubi_start.distance(input.tubi_end);
    if abs_slope > params.max_slope {
        return PlacedSlope {
            id: input.id.clone(),
            start: to_array(input.tubi_start),
            end: to_array(input.tubi_end),
            text: input.text.clone(),
            slope: input.slope,
            label_offset_world: None,
            visible: false,
            suppressed_reason: Some("slope_exceeds_max".to_string()),
        };
    }
    if abs_slope < params.min_slope {
        return PlacedSlope {
            id: input.id.clone(),
            start: to_array(input.tubi_start),
            end: to_array(input.tubi_end),
            text: input.text.clone(),
            slope: input.slope,
            label_offset_world: None,
            visible: false,
            suppressed_reason: Some("slope_below_min".to_string()),
        };
    }
    if tubi_len < input.od {
        return PlacedSlope {
            id: input.id.clone(),
            start: to_array(input.tubi_start),
            end: to_array(input.tubi_end),
            text: input.text.clone(),
            slope: input.slope,
            label_offset_world: None,
            visible: false,
            suppressed_reason: Some("slope_tubi_too_short".to_string()),
        };
    }

    let label_offset_len = input.od * 0.5 + 20.0;
    let label_offset = Vec3::new(0.0, 0.0, label_offset_len);
    PlacedSlope {
        id: input.id.clone(),
        start: to_array(input.tubi_start),
        end: to_array(input.tubi_end),
        text: input.text.clone(),
        slope: input.slope,
        label_offset_world: Some(to_array(label_offset)),
        visible: true,
        suppressed_reason: None,
    }
}

/// iso_weld → `PlacedWeld`：PML 侧本质是 POD，这里等价透传。
pub fn solve_weld(input: &WeldInput) -> PlacedWeld {
    PlacedWeld {
        id: input.id.clone(),
        position: to_array(input.position),
        label: input.label.clone(),
        subtitle: input.subtitle.clone(),
        is_shop: input.is_shop,
        cross_size: 0.0,
        label_offset_world: None,
        visible: true,
        suppressed_reason: None,
    }
}

/// iso_tag → `PlacedTag`：text 已由后端拼接好，直接装配。
pub fn solve_tag(input: &TagInput) -> PlacedTag {
    PlacedTag {
        id: input.id.clone(),
        text: input.text.clone(),
        position: to_array(input.position),
        label_offset_world: None,
        visible: true,
        suppressed_reason: None,
    }
}

/// iso_bend → `PlacedBend`：装配 `size_dims` + `angle`。
///
/// `size_dims[i]` 用 `compute_linear_dim_layout` 处理从 `vertex → face_center_i` 的线性尺寸，
/// OD/cheight 从 [`IsoParams`] 读取。
pub fn solve_bend(input: &BendInput, context: &BranchContext, params: &IsoParams) -> PlacedBend {
    let mut size_dims = Vec::new();
    for (i, face_center) in [input.face_center_1, input.face_center_2]
        .iter()
        .enumerate()
    {
        if let Some(face) = face_center {
            let pipe_dir = {
                let d = (*face - input.vertex).normalize_or_zero();
                if d.length_squared() < 1e-6 {
                    Vec3::X
                } else {
                    d
                }
            };
            let seg = SegmentInput {
                id: format!("{}::size_dim_{}", input.id, i + 1),
                kind: "bend_size".to_string(),
                start: input.vertex,
                end: *face,
                pipe_dir,
                od: input.od,
                text: input.face_texts[i].clone().unwrap_or_default(),
                isoline_index: None,
            };
            size_dims.push(compute_linear_dim_layout(&seg, context, params));
        }
    }

    let angle = match (input.angle_deg, input.face_center_1, input.face_center_2) {
        (Some(deg), Some(p1), Some(p2)) => {
            let arc_radius = input.od.max(50.0);
            Some(PlacedAngle {
                vertex: to_array(input.vertex),
                point1: to_array(p1),
                point2: to_array(p2),
                arc_radius,
                text: if input.angle_text.is_empty() {
                    format!("{deg:.1}°")
                } else {
                    input.angle_text.clone()
                },
                label_t: 0.5,
                label_offset_world: None,
            })
        }
        _ => None,
    };

    PlacedBend {
        id: input.id.clone(),
        visible: true,
        suppressed_reason: None,
        size_dims,
        angle,
    }
}

/// iso_cut_tubi → 沿用 iso_dim；cut tubi 语义就是 LinearDim 的一种。
pub fn solve_cut_tubi(
    input: &SegmentInput,
    context: &BranchContext,
    params: &IsoParams,
) -> PlacedLinearDim {
    let mut placed = compute_linear_dim_layout(input, context, params);
    // cut tubi 的 kind 保持与输入一致（通常 "cut_tubi"）。
    if placed.kind.is_empty() {
        placed.kind = "cut_tubi".to_string();
    }
    placed
}

/// 统计方向辅助：返回 pipe_dir 对应的"主轴水平方向"（E/N/W/S 中最接近投影方向的那个），
/// 对应 PML `isoori` 的辅助判定。主要给 Stage 3.6 iso_branch 用于 lane 分配时区分方向簇。
pub fn classify_horizontal_axis(pipe_dir: Vec3) -> [f32; 3] {
    let horiz = [Vec3::X, Vec3::Y, -Vec3::X, -Vec3::Y];
    let mut best = horiz[0];
    let mut best_angle = f32::INFINITY;
    let projected = {
        let h = Vec3::new(pipe_dir.x, pipe_dir.y, 0.0);
        if h.length_squared() < 1e-9 {
            return [0.0, 0.0, 1.0];
        }
        h.normalize()
    };
    for axis in horiz.iter() {
        let a = angle_deg(*axis, projected);
        if a < best_angle {
            best_angle = a;
            best = *axis;
        }
    }
    [best.x, best.y, best.z]
}

fn to_array(v: Vec3) -> [f32; 3] {
    [v.x, v.y, v.z]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slope_within_range_emits_visible() {
        let slope = solve_slope(
            &SlopeInput {
                id: "slope:0".into(),
                tubi_start: Vec3::new(0.0, 0.0, 0.0),
                tubi_end: Vec3::new(10000.0, 0.0, 0.0),
                slope: 0.02,
                od: 229.0,
                text: "slope 2.0%".into(),
            },
            &IsoParams::default(),
        );
        assert!(slope.visible);
        assert!(slope.suppressed_reason.is_none());
        assert!(slope.label_offset_world.is_some());
    }

    #[test]
    fn slope_above_max_is_suppressed() {
        let slope = solve_slope(
            &SlopeInput {
                id: "slope:1".into(),
                tubi_start: Vec3::new(0.0, 0.0, 0.0),
                tubi_end: Vec3::new(10000.0, 0.0, 0.0),
                slope: 0.5, // > max_slope=0.1
                od: 229.0,
                text: "slope 50%".into(),
            },
            &IsoParams::default(),
        );
        assert!(!slope.visible);
        assert_eq!(
            slope.suppressed_reason.as_deref(),
            Some("slope_exceeds_max")
        );
    }

    #[test]
    fn bend_emits_two_size_dims_when_both_faces_present() {
        let placed = solve_bend(
            &BendInput {
                id: "bend:0".into(),
                vertex: Vec3::ZERO,
                face_center_1: Some(Vec3::new(200.0, 0.0, 0.0)),
                face_center_2: Some(Vec3::new(0.0, 200.0, 0.0)),
                angle_deg: Some(90.0),
                od: 100.0,
                face_texts: [Some("200".into()), Some("200".into())],
                angle_text: "90°".into(),
            },
            &BranchContext::for_test("B"),
            &IsoParams::default(),
        );
        assert_eq!(placed.size_dims.len(), 2);
        assert!(placed.angle.is_some());
        let angle = placed.angle.unwrap();
        assert_eq!(angle.text, "90°");
    }

    #[test]
    fn classify_horizontal_axis_maps_to_nearest() {
        let pipe = Vec3::new(-0.6721017, 0.7371828, 0.069576345).normalize();
        let axis = classify_horizontal_axis(pipe);
        // (-0.67, 0.74) 在水平面上最接近 +Y 轴 (0,1,0)
        assert!((axis[1] - 1.0).abs() < 1e-4, "expected +Y, got {axis:?}");
    }
}
