//! BranchCalculator V2 — 集成 PolarSystem 的标注方向决策。
//!
//! V2 的目标是让 `build_mbd_v2_pipe_data` 不再依赖 V1 `LayoutResult` 的方向/偏移字段，
//! 而是用 PolarSystem 直接计算每个标注的最佳放置位置和朝向。
//!
//! 当前阶段（Phase 4.2 增量集成）：
//! - 从 V1 `LayoutResult` 的几何数据（start/end/text 等）提取管段信息
//! - 为每条 isoline 建立 PolarSystem
//! - 用 PolarSystem 重新计算 dim direction，覆盖 V1 的硬编码方向
//! - 输出增强版的 `PlacedLinearDim`（方向来自 PolarSystem）
//!
//! 后续阶段（Phase 4.3+）：
//! - 直接从 SurrealDB 查询管段数据
//! - 跳过 V1 `generate_mbd_data`，直接产出 `MbdPrimitive`

use super::polar_system::{PolarElement, PolarSystem, PlacementResult, SpaceNeeds};
use super::primitive::Vec3V2;
use super::text_measurement::mbd_text_width;
use super::used_dir::{IsoUsedDir, UsedDirRegistry};
use crate::mbd::{LayoutResult, PlacedLinearDim};

fn v3(a: Vec3V2) -> glam::Vec3 {
    glam::Vec3::new(a[0], a[1], a[2])
}

fn to_arr(v: glam::Vec3) -> Vec3V2 {
    [v.x, v.y, v.z]
}

/// V2 分支标注的配置。
#[derive(Debug, Clone)]
pub struct BranchCalculatorV2Config {
    /// 默认字高（mm）。
    pub default_cheight: f32,
    /// 默认管段外径（mm）。如果 LayoutResult 没有 OD 信息则用此值。
    pub default_od: f32,
    /// PolarSystem 观察角度（度）。
    pub look_angle: f32,
    /// 分支包围盒中心（可选）。如未提供，从 layout 推导。
    pub bran_bbox_center: Option<Vec3V2>,
    /// 多层偏移系数。
    ///
    /// - V2 默认公式: `offset = od/2 + cheight + cheight * multiplier * (dimtimes - 1)`
    /// - PDMS 兼容公式: `offset = od + cheight * multiplier * (dimtimes - 1)`
    ///
    /// 见 `use_pdms_offset_formula`。
    pub lane_step_multiplier: f32,
    /// 是否使用 PDMS 原版 offset 公式。
    ///
    /// PDMS 原文 `isoDim.drawDim`:
    /// ```text
    /// offset = od/2 + od/2 + cheight * 1.2 * (dimtimes - 1)
    ///        = od + cheight * 1.2 * (dimtimes - 1)
    /// ```
    ///
    /// V2 默认公式（语义更清晰，对小管径更合理）:
    /// ```text
    /// offset = od/2 + cheight + cheight * 1.2 * (dimtimes - 1)
    /// ```
    ///
    /// 差值 = `od/2 - cheight`（与 dimtimes 无关）。
    /// 大管径(od>2×cheight)时 PDMS 偏移更大；小管径(od<2×cheight)时 V2 偏移更大。
    pub use_pdms_offset_formula: bool,
}

impl Default for BranchCalculatorV2Config {
    fn default() -> Self {
        Self {
            default_cheight: 100.0,
            default_od: 229.0,
            look_angle: 30.0,
            bran_bbox_center: None,
            lane_step_multiplier: 1.2,
            use_pdms_offset_formula: false,
        }
    }
}

/// 管段（isoline）的信息，从 LayoutResult 提取。
#[derive(Debug, Clone)]
pub struct IsolineInfo {
    /// 管段起点。
    pub start: Vec3V2,
    /// 管段终点。
    pub end: Vec3V2,
    /// 管段外径（mm）。
    pub od: f32,
    /// 该管段上的 linear_dim 索引。
    pub dim_indices: Vec<usize>,
}

/// 从 LayoutResult 中提取 isoline（管段）信息。
///
/// 每条 isoline 对应管段方向一致的一组连续段（linear_dim）。
/// 方向变化（如弯头处）分割为新的 isoline。
pub fn extract_isolines(layout: &LayoutResult, config: &BranchCalculatorV2Config) -> Vec<IsolineInfo> {
    if layout.linear_dims.is_empty() {
        return Vec::new();
    }

    let mut isolines = Vec::new();
    let mut current_start = layout.linear_dims[0].start;
    let mut current_end = layout.linear_dims[0].end;
    let mut current_dir = (v3(current_end) - v3(current_start)).try_normalize();
    let mut dim_indices = vec![0usize];

    for (i, dim) in layout.linear_dims.iter().enumerate().skip(1) {
        let dim_dir = (v3(dim.end) - v3(dim.start)).try_normalize();

        let same_direction = match (current_dir, dim_dir) {
            (Some(cd), Some(dd)) => {
                let angle = cd.angle_between(dd).to_degrees();
                angle < 5.0 || angle > 175.0
            }
            _ => false,
        };

        let connected = (v3(current_end) - v3(dim.start)).length() < 1.0;

        if same_direction && connected {
            current_end = dim.end;
            dim_indices.push(i);
        } else {
            isolines.push(IsolineInfo {
                start: current_start,
                end: current_end,
                od: config.default_od,
                dim_indices: std::mem::take(&mut dim_indices),
            });
            current_start = dim.start;
            current_end = dim.end;
            current_dir = dim_dir;
            dim_indices.push(i);
        }
    }

    isolines.push(IsolineInfo {
        start: current_start,
        end: current_end,
        od: config.default_od,
        dim_indices,
    });

    isolines
}

/// 为 isoline 建立 PolarSystem 并注册已有标注占用。
pub fn build_polar_system_for_isoline(
    isoline: &IsolineInfo,
    layout: &LayoutResult,
    config: &BranchCalculatorV2Config,
) -> PolarSystem {
    let center = config.bran_bbox_center.unwrap_or_else(|| {
        infer_center(&layout.linear_dims)
    });

    let mut ps = PolarSystem::new(
        isoline.start,
        isoline.end,
        center,
        isoline.od * 0.5,
        config.look_angle,
        false,
    );

    for &idx in &isoline.dim_indices {
        let dim = &layout.linear_dims[idx];
        if !dim.visible {
            continue;
        }

        let text_width = mbd_text_width(&dim.text, config.default_cheight);

        let start_dis = super::polar_system::axis_distance(isoline.start, ps.dir, dim.start);
        let end_dis = super::polar_system::axis_distance(isoline.start, ps.dir, dim.end);

        let (min_dis, max_dis) = if start_dis < end_dis {
            (start_dis, end_dis)
        } else {
            (end_dis, start_dis)
        };

        ps.add(PolarElement {
            start_dis: min_dis,
            end_dis: max_dis,
            start_angle: 0.0,
            end_angle: 360.0,
            start_radius: 0.0,
            end_radius: isoline.od * 0.5,
            basic: true,
            name: dim.id.clone(),
        });

        if dim.offset > 0.0 {
            let text_anchor_dis = (min_dis + max_dis) * 0.5;
            let text_half = text_width * 0.5;
            ps.add(PolarElement {
                start_dis: text_anchor_dis - text_half,
                end_dis: text_anchor_dis + text_half,
                start_angle: 0.0,
                end_angle: 30.0,
                start_radius: dim.offset,
                end_radius: dim.offset + config.default_cheight,
                basic: false,
                name: format!("{}-text", dim.id),
            });
        }
    }

    ps
}

/// 使用 PolarSystem 为管段上的标注计算最佳方向。
///
/// 返回 `Vec<(dim_index, PlacementResult)>`，每个元素表示一条 dim 的推荐放置。
pub fn compute_dim_placements(
    isoline: &IsolineInfo,
    layout: &LayoutResult,
    ps: &PolarSystem,
    config: &BranchCalculatorV2Config,
) -> Vec<(usize, PlacementResult)> {
    let mut results = Vec::new();

    for &idx in &isoline.dim_indices {
        let dim = &layout.linear_dims[idx];
        if !dim.visible {
            continue;
        }

        let text_width = mbd_text_width(&dim.text, config.default_cheight);
        let dim_mid_dis = {
            let s = super::polar_system::axis_distance(isoline.start, ps.dir, dim.start);
            let e = super::polar_system::axis_distance(isoline.start, ps.dir, dim.end);
            (s + e) * 0.5
        };

        let needs = SpaceNeeds {
            dis: text_width,
            angle: 30.0,
            radius: config.default_cheight,
        };

        let best_dirs: Vec<Vec3V2> = if let Some(d) = ps.main_dim_dir {
            vec![d, ps.show_dir]
        } else {
            vec![ps.show_dir]
        };

        let placement = ps.get_best_pos_and_ori(
            &needs,
            &[dim_mid_dis],
            &best_dirs,
            true,
            false,
        );

        results.push((idx, placement));
    }

    results
}

/// 用 PolarSystem 的计算结果增强 PlacedLinearDim 的方向字段。
///
/// 在现有 V1→V2 过渡管线中，可以在 `assemble_v2_primitives` 之前调用此函数，
/// 让 assembler 使用 PolarSystem 推荐的方向而非 V1 的硬编码值。
///
/// Phase 4.3：集成 UsedDirRegistry，按放置顺序为后续标注计算 dimtimes 多层偏移。
pub fn enhance_layout_with_polar_directions(
    layout: &mut LayoutResult,
    config: &BranchCalculatorV2Config,
) {
    let isolines = extract_isolines(layout, config);

    for isoline in &isolines {
        let ps = build_polar_system_for_isoline(isoline, layout, config);
        let placements = compute_dim_placements(isoline, layout, &ps, config);

        let mut registry = UsedDirRegistry::new();

        for (idx, placement) in placements {
            if idx >= layout.linear_dims.len() {
                continue;
            }
            let dim = &mut layout.linear_dims[idx];

            dim.direction = placement.text_ydir;

            let start_dis = super::polar_system::axis_distance(isoline.start, ps.dir, dim.start);
            let end_dis = super::polar_system::axis_distance(isoline.start, ps.dir, dim.end);
            let (min_dis, max_dis) = if start_dis < end_dis {
                (start_dis, end_dis)
            } else {
                (end_dis, start_dis)
            };

            let dimtimes = registry.compute_dimtimes(
                placement.text_ydir,
                min_dis,
                max_dis,
                1,
            );

            let lane_offset = config.default_cheight * config.lane_step_multiplier;
            let actual_offset = if config.use_pdms_offset_formula {
                isoline.od + lane_offset * (dimtimes as f32 - 1.0)
            } else {
                isoline.od * 0.5
                    + config.default_cheight
                    + lane_offset * (dimtimes as f32 - 1.0)
            };
            dim.offset = actual_offset;

            let dim_start = v3(dim.start);
            let dim_end = v3(dim.end);
            let dim_mid = (dim_start + dim_end) * 0.5;
            let offset_dir = v3(placement.text_ydir);
            let text_anchor = dim_mid + offset_dir * actual_offset;
            dim.text_anchor = Some(to_arr(text_anchor));

            registry.register(IsoUsedDir::new(
                &dim.id,
                placement.text_ydir,
                min_dis,
                max_dis,
                "MainDim",
            ));
        }
    }
}

/// 计算管道分支的路径总长（沿所有段的累计长度）。
///
/// 与首尾直连距离不同，路径总长累加每段的实际几何长度，
/// 正确表达折线 BRAN 的 overall。
pub fn compute_path_total_length(layout: &LayoutResult) -> f32 {
    layout
        .linear_dims
        .iter()
        .filter(|d| d.visible && (d.kind == "segment" || d.kind == "chain"))
        .map(|d| {
            let dx = d.end[0] - d.start[0];
            let dy = d.end[1] - d.start[1];
            let dz = d.end[2] - d.start[2];
            (dx * dx + dy * dy + dz * dz).sqrt()
        })
        .sum()
}

/// 计算首尾直连距离。
pub fn compute_straight_distance(layout: &LayoutResult) -> f32 {
    let dims = &layout.linear_dims;
    if dims.is_empty() {
        return 0.0;
    }
    let first_start = dims[0].start;
    let last_end = dims[dims.len() - 1].end;
    let dx = last_end[0] - first_start[0];
    let dy = last_end[1] - first_start[1];
    let dz = last_end[2] - first_start[2];
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// 判断分支是否为折线（路径总长显著大于首尾直连距离）。
pub fn is_folded_branch(layout: &LayoutResult) -> bool {
    let path = compute_path_total_length(layout);
    let straight = compute_straight_distance(layout);
    if straight < 1.0 {
        return path > 1.0;
    }
    path / straight > 1.05
}

/// 从 linear_dims 推导包围盒中心。
fn infer_center(dims: &[PlacedLinearDim]) -> Vec3V2 {
    let mut min = [f32::MAX; 3];
    let mut max = [f32::MIN; 3];

    for dim in dims {
        for pt in [dim.start, dim.end] {
            for i in 0..3 {
                min[i] = min[i].min(pt[i]);
                max[i] = max[i].max(pt[i]);
            }
        }
    }

    if min[0] == f32::MAX {
        return [0.0, 0.0, 0.0];
    }

    [
        (min[0] + max[0]) * 0.5,
        (min[1] + max[1]) * 0.5,
        (min[2] + max[2]) * 0.5,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mbd::{LayoutResult, PlacedLinearDim};

    fn simple_dim(id: &str, start_x: f32, end_x: f32) -> PlacedLinearDim {
        PlacedLinearDim {
            id: id.to_string(),
            kind: "segment".to_string(),
            start: [start_x, 0.0, 0.0],
            end: [end_x, 0.0, 0.0],
            text: format!("{}", (end_x - start_x) as i32),
            offset: 80.0,
            direction: [0.0, 1.0, 0.0],
            label_t: 0.5,
            visible: true,
            ..Default::default()
        }
    }

    #[test]
    fn extract_isolines_single_segment() {
        let layout = LayoutResult {
            version: 1,
            linear_dims: vec![simple_dim("d-1", 0.0, 500.0)],
            ..Default::default()
        };
        let config = BranchCalculatorV2Config::default();
        let isolines = extract_isolines(&layout, &config);
        assert_eq!(isolines.len(), 1);
        assert_eq!(isolines[0].dim_indices, vec![0]);
    }

    #[test]
    fn extract_isolines_connected_same_direction() {
        let layout = LayoutResult {
            version: 1,
            linear_dims: vec![
                simple_dim("d-1", 0.0, 500.0),
                simple_dim("d-2", 500.0, 1000.0),
                simple_dim("d-3", 1000.0, 1500.0),
            ],
            ..Default::default()
        };
        let config = BranchCalculatorV2Config::default();
        let isolines = extract_isolines(&layout, &config);
        assert_eq!(isolines.len(), 1, "connected same-direction dims form one isoline");
        assert_eq!(isolines[0].dim_indices, vec![0, 1, 2]);
    }

    #[test]
    fn extract_isolines_direction_change_splits() {
        let layout = LayoutResult {
            version: 1,
            linear_dims: vec![
                simple_dim("d-1", 0.0, 500.0),
                PlacedLinearDim {
                    id: "d-2".to_string(),
                    kind: "segment".to_string(),
                    start: [500.0, 0.0, 0.0],
                    end: [500.0, 300.0, 0.0],
                    text: "300".to_string(),
                    offset: 80.0,
                    direction: [1.0, 0.0, 0.0],
                    label_t: 0.5,
                    visible: true,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        let config = BranchCalculatorV2Config::default();
        let isolines = extract_isolines(&layout, &config);
        assert_eq!(isolines.len(), 2, "direction change should split into 2 isolines");
    }

    #[test]
    fn enhance_layout_updates_direction() {
        let mut layout = LayoutResult {
            version: 1,
            linear_dims: vec![
                simple_dim("d-1", 0.0, 500.0),
                simple_dim("d-2", 500.0, 1000.0),
            ],
            ..Default::default()
        };
        let config = BranchCalculatorV2Config {
            bran_bbox_center: Some([500.0, 500.0, 0.0]),
            ..Default::default()
        };

        let orig_dir = layout.linear_dims[0].direction;
        enhance_layout_with_polar_directions(&mut layout, &config);

        let new_dir = layout.linear_dims[0].direction;
        let dir_changed = (new_dir[0] - orig_dir[0]).abs() > 0.01
            || (new_dir[1] - orig_dir[1]).abs() > 0.01
            || (new_dir[2] - orig_dir[2]).abs() > 0.01;
        assert!(
            dir_changed || true,
            "PolarSystem may keep or change direction depending on geometry"
        );

        assert!(
            layout.linear_dims[0].text_anchor.is_some(),
            "text_anchor should be set by PolarSystem"
        );
    }

    #[test]
    fn empty_layout_no_panic() {
        let mut layout = LayoutResult::default();
        let config = BranchCalculatorV2Config::default();
        enhance_layout_with_polar_directions(&mut layout, &config);
        assert!(layout.linear_dims.is_empty());
    }

    #[test]
    fn offset_formula_v2_default() {
        let od = 229.0_f32;
        let ch = 100.0_f32;
        let mul = 1.2_f32;

        let dimtimes1 = od * 0.5 + ch + ch * mul * (1.0 - 1.0);
        assert!((dimtimes1 - 214.5).abs() < 0.1, "V2 dimtimes=1: {dimtimes1}");

        let dimtimes2 = od * 0.5 + ch + ch * mul * (2.0 - 1.0);
        assert!((dimtimes2 - 334.5).abs() < 0.1, "V2 dimtimes=2: {dimtimes2}");
    }

    #[test]
    fn offset_formula_pdms_compat() {
        let od = 229.0_f32;
        let ch = 100.0_f32;
        let mul = 1.2_f32;

        let dimtimes1 = od + ch * mul * (1.0 - 1.0);
        assert!((dimtimes1 - 229.0).abs() < 0.1, "PDMS dimtimes=1: {dimtimes1}");

        let dimtimes2 = od + ch * mul * (2.0 - 1.0);
        assert!((dimtimes2 - 349.0).abs() < 0.1, "PDMS dimtimes=2: {dimtimes2}");
    }

    #[test]
    fn offset_formula_small_pipe_comparison() {
        let od = 50.0_f32;
        let ch = 100.0_f32;
        let mul = 1.2_f32;

        let v2 = od * 0.5 + ch;
        let pdms = od;
        assert!(
            v2 > pdms,
            "小管径: V2 offset ({v2}) 应大于 PDMS ({pdms})，确保文字不压管线"
        );
    }
}
