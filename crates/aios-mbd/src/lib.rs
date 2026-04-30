use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub mod iso_branch;
pub mod iso_dim;
pub mod iso_extras;
pub mod iso_params;

pub use iso_branch::{UsedDirEntry, UsedDirRegistry, solve_linear_dim_series};
pub use iso_dim::compute_linear_dim_layout;
pub use iso_extras::{
    BendInput, SlopeInput, TagInput, WeldInput, classify_horizontal_axis, solve_bend,
    solve_cut_tubi, solve_slope, solve_tag, solve_weld,
};
pub use iso_params::{BranchContext, IsoParams, SegmentInput};

pub type LayoutVec3 = [f32; 3];

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum BranchLayoutMode {
    #[default]
    LayoutFirst,
    Construction,
    Inspection,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LayoutRequest {
    pub mode: BranchLayoutMode,
    pub include_chain_dims: bool,
    pub include_overall_dim: bool,
    pub include_port_dims: bool,
    pub include_welds: bool,
    pub include_slopes: bool,
    pub include_bends: bool,
    pub include_cut_tubis: bool,
    pub include_tags: bool,
    pub include_fittings: bool,
    pub look_angle: Option<LayoutVec3>,
    pub consider_pre_next_dir: bool,
    pub ignore_line: bool,
    pub auto_text_scale: bool,
    pub min_text_scale: f32,
    pub allow_layer_split: bool,
}

impl Default for LayoutRequest {
    fn default() -> Self {
        Self {
            mode: BranchLayoutMode::LayoutFirst,
            include_chain_dims: true,
            include_overall_dim: true,
            include_port_dims: false,
            include_welds: true,
            include_slopes: true,
            include_bends: true,
            include_cut_tubis: true,
            include_tags: true,
            include_fittings: true,
            look_angle: None,
            consider_pre_next_dir: true,
            ignore_line: false,
            auto_text_scale: true,
            min_text_scale: 0.75,
            allow_layer_split: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct PlacedLinearDim {
    pub id: String,
    pub kind: String,
    pub start: LayoutVec3,
    pub end: LayoutVec3,
    pub text: String,
    pub offset: f32,
    pub direction: LayoutVec3,
    pub label_t: f32,
    pub label_offset_world: Option<LayoutVec3>,
    pub dim_line_start: Option<LayoutVec3>,
    pub dim_line_end: Option<LayoutVec3>,
    pub extension_line_1_start: Option<LayoutVec3>,
    pub extension_line_1_end: Option<LayoutVec3>,
    pub extension_line_2_start: Option<LayoutVec3>,
    pub extension_line_2_end: Option<LayoutVec3>,
    pub text_anchor: Option<LayoutVec3>,
    pub visible: bool,
    pub suppressed_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct PlacedWeld {
    pub id: String,
    pub position: LayoutVec3,
    pub label: String,
    pub subtitle: Option<String>,
    pub is_shop: bool,
    pub cross_size: f32,
    pub label_offset_world: Option<LayoutVec3>,
    pub visible: bool,
    pub suppressed_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct PlacedSlope {
    pub id: String,
    pub start: LayoutVec3,
    pub end: LayoutVec3,
    pub text: String,
    pub slope: f32,
    pub label_offset_world: Option<LayoutVec3>,
    pub visible: bool,
    pub suppressed_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct PlacedTag {
    pub id: String,
    pub text: String,
    pub position: LayoutVec3,
    pub label_offset_world: Option<LayoutVec3>,
    pub visible: bool,
    pub suppressed_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct PlacedFitting {
    pub id: String,
    pub kind: String,
    pub text: String,
    pub position: LayoutVec3,
    pub label_offset_world: Option<LayoutVec3>,
    pub visible: bool,
    pub suppressed_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct PlacedAngle {
    pub vertex: LayoutVec3,
    pub point1: LayoutVec3,
    pub point2: LayoutVec3,
    pub arc_radius: f32,
    pub text: String,
    pub label_t: f32,
    pub label_offset_world: Option<LayoutVec3>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct PlacedBend {
    pub id: String,
    pub visible: bool,
    pub suppressed_reason: Option<String>,
    pub size_dims: Vec<PlacedLinearDim>,
    pub angle: Option<PlacedAngle>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SuppressedItem {
    pub id: String,
    pub kind: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct LayoutStats {
    pub linear_dims_count: usize,
    pub cut_tubis_count: usize,
    pub welds_count: usize,
    pub slopes_count: usize,
    pub bends_count: usize,
    pub tags_count: usize,
    pub fittings_count: usize,
    pub suppressed_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LayoutDebugInfo {
    pub solver_version: String,
    pub notes: Vec<String>,
    pub isoline_count: usize,
    pub inferred_face_center_count: usize,
    pub suppressed_by_reason: BTreeMap<String, usize>,
    pub legacy_diff_summary: Vec<String>,
}

impl Default for LayoutDebugInfo {
    fn default() -> Self {
        Self {
            solver_version: "old_pml_branch_solver_v1".to_string(),
            notes: Vec::new(),
            isoline_count: 0,
            inferred_face_center_count: 0,
            suppressed_by_reason: BTreeMap::new(),
            legacy_diff_summary: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct LayoutResult {
    pub version: u32,
    pub mode: BranchLayoutMode,
    pub stats: LayoutStats,
    pub linear_dims: Vec<PlacedLinearDim>,
    pub cut_tubis: Vec<PlacedLinearDim>,
    pub welds: Vec<PlacedWeld>,
    pub slopes: Vec<PlacedSlope>,
    pub bends: Vec<PlacedBend>,
    pub tags: Vec<PlacedTag>,
    pub fittings: Vec<PlacedFitting>,
    pub suppressed_items: Vec<SuppressedItem>,
    pub debug_info: Option<LayoutDebugInfo>,
}

#[derive(Debug, Clone, Default)]
pub struct LegacyPlacedLayoutSections {
    pub linear_dims: Vec<PlacedLinearDim>,
    pub cut_tubis: Vec<PlacedLinearDim>,
    pub welds: Vec<PlacedWeld>,
    pub slopes: Vec<PlacedSlope>,
    pub bends: Vec<PlacedBend>,
    pub tags: Vec<PlacedTag>,
    pub fittings: Vec<PlacedFitting>,
    pub suppressed_items: Vec<SuppressedItem>,
    pub notes: Vec<String>,
    pub inferred_face_center_count: usize,
    pub isoline_count: usize,
    pub legacy_diff_summary: Vec<String>,
}

/// `solve_branch` 的输入束，把各类 iso* 子模块的输入打包到一起，避免签名爆炸。
pub struct SolveBranchInput<'a> {
    pub context: &'a iso_params::BranchContext,
    pub params: &'a iso_params::IsoParams,
    pub linear_dims: &'a [iso_params::SegmentInput],
    pub cut_tubis: &'a [iso_params::SegmentInput],
    pub slopes: &'a [iso_extras::SlopeInput],
    pub welds: &'a [iso_extras::WeldInput],
    pub tags: &'a [iso_extras::TagInput],
    pub bends: &'a [iso_extras::BendInput],
}

impl<'a> SolveBranchInput<'a> {
    /// 构造只填 linear_dims 的最小输入（其它字段默认空切片）。
    pub fn linear_only(
        context: &'a iso_params::BranchContext,
        params: &'a iso_params::IsoParams,
        linear_dims: &'a [iso_params::SegmentInput],
    ) -> Self {
        Self {
            context,
            params,
            linear_dims,
            cut_tubis: &[],
            slopes: &[],
            welds: &[],
            tags: &[],
            bends: &[],
        }
    }
}

/// 内部辅助：按 registry 逐条求解一批 SegmentInput，返回 PlacedLinearDim 序列。
/// 参数 `registry` 被就地更新，供后续序列复用（linear + cut_tubi 之间互相感知）。
fn solve_series_with_registry(
    inputs: &[iso_params::SegmentInput],
    context: &iso_params::BranchContext,
    params: &iso_params::IsoParams,
    registry: &mut iso_branch::UsedDirRegistry,
) -> Vec<PlacedLinearDim> {
    use glam::Vec3;
    let mut out = Vec::with_capacity(inputs.len());
    for input in inputs {
        let probe = iso_dim::compute_linear_dim_layout(
            input,
            &iso_params::BranchContext {
                dim_times: 1,
                ..context.clone()
            },
            params,
        );
        let dir = Vec3::from_array(probe.direction);
        let mid = (input.start + input.end) * 0.5;
        let length = input.start.distance(input.end);
        let dis_start = input.start.length();
        let dis_end = input.end.length();
        let dim_times = registry.next_dim_times(dir, dis_start, dis_end, mid, length);
        let placed = if dim_times == 1 {
            probe
        } else {
            iso_dim::compute_linear_dim_layout(
                input,
                &iso_params::BranchContext {
                    dim_times,
                    ..context.clone()
                },
                params,
            )
        };
        registry.record(
            dir,
            dis_start,
            dis_end,
            input.kind.clone(),
            dim_times,
            mid,
            length,
        );
        out.push(placed);
    }
    out
}

pub struct BranchCalculator;

impl BranchCalculator {
    /// MVP solver：逐模块对齐 PML isoXxx 语义，产出完整 `LegacyPlacedLayoutSections`。
    ///
    /// 覆盖：
    /// - `linear_dims` ↔ [`isoDim.pmlobj`](../../../MBD/markpipe/object/isoDim.pmlobj)
    /// - `slopes` ↔ [`isoSlope.pmlobj`](../../../MBD/markpipe/object/isoSlope.pmlobj)
    /// - `welds` ↔ [`isoWeldText.pmlobj`](../../../MBD/markpipe/object/isoWeldText.pmlobj)
    /// - `tags` ↔ [`isoTag.pmlobj`](../../../MBD/markpipe/object/isoTag.pmlobj)
    /// - `bends` ↔ [`isoelbopad.pmlobj`](../../../MBD/markpipe/object/isoelbopad.pmlobj) +
    ///   [`isombdangle.pmlobj`](../../../MBD/markpipe/object/isombdangle.pmlobj)
    /// - `cut_tubis` 沿用 `iso_dim`
    ///
    /// iso_branch 在 MVP 里仅做"单 lane"处理；
    /// 多层 lane 分配 + `isoUsedDir` 已用方向惩罚留给后续迭代。
    pub fn solve_branch(input: SolveBranchInput<'_>) -> LegacyPlacedLayoutSections {
        let mut sections = LegacyPlacedLayoutSections::default();

        // linear_dims 和 cut_tubis 共享同一个 UsedDirRegistry，这样两类线性尺寸之间也会
        // 互相 stagger（PML 里它们都属于 isoline 的 mainDim/useddirs 体系）。
        let mut registry = iso_branch::UsedDirRegistry::new();
        let linear_placed = solve_series_with_registry(
            input.linear_dims,
            input.context,
            input.params,
            &mut registry,
        );
        sections.linear_dims.extend(linear_placed);

        let cut_placed = solve_series_with_registry(
            input.cut_tubis,
            input.context,
            input.params,
            &mut registry,
        );
        sections.cut_tubis.extend(cut_placed);

        for slope in input.slopes {
            sections.slopes.push(iso_extras::solve_slope(slope, input.params));
        }
        for weld in input.welds {
            sections.welds.push(iso_extras::solve_weld(weld));
        }
        for tag in input.tags {
            sections.tags.push(iso_extras::solve_tag(tag));
        }
        for bend in input.bends {
            sections
                .bends
                .push(iso_extras::solve_bend(bend, input.context, input.params));
        }

        sections.notes.push(format!(
            "solve_branch: {} linear_dims, {} cut_tubis, {} slopes, {} welds, {} tags, {} bends, used_dir_entries={} (branch {})",
            sections.linear_dims.len(),
            sections.cut_tubis.len(),
            sections.slopes.len(),
            sections.welds.len(),
            sections.tags.len(),
            sections.bends.len(),
            registry.len(),
            input.context.branch_refno,
        ));
        sections.isoline_count = 1;
        sections
    }

    pub fn assemble_prelaid_out(
        request: &LayoutRequest,
        mut sections: LegacyPlacedLayoutSections,
    ) -> LayoutResult {
        let mut suppressed_items = sections.suppressed_items;
        suppressed_items.extend(
            sections
                .linear_dims
                .iter()
                .filter_map(|item| suppressed_item_from_linear(item, "linear_dim")),
        );
        suppressed_items.extend(
            sections
                .cut_tubis
                .iter()
                .filter_map(|item| suppressed_item_from_linear(item, "cut_tubi")),
        );
        suppressed_items.extend(sections.welds.iter().filter_map(|item| {
            suppressed_item_from_aux(
                item.visible,
                item.suppressed_reason.as_deref(),
                &item.id,
                "weld",
            )
        }));
        suppressed_items.extend(sections.slopes.iter().filter_map(|item| {
            suppressed_item_from_aux(
                item.visible,
                item.suppressed_reason.as_deref(),
                &item.id,
                "slope",
            )
        }));
        suppressed_items.extend(sections.bends.iter().filter_map(|item| {
            suppressed_item_from_aux(
                item.visible,
                item.suppressed_reason.as_deref(),
                &item.id,
                "bend",
            )
        }));
        suppressed_items.extend(sections.bends.iter().flat_map(|bend| {
            bend.size_dims
                .iter()
                .filter_map(|item| suppressed_item_from_linear(item, "bend_size_dim"))
        }));
        suppressed_items.extend(sections.tags.iter().filter_map(|item| {
            suppressed_item_from_aux(
                item.visible,
                item.suppressed_reason.as_deref(),
                &item.id,
                "tag",
            )
        }));
        suppressed_items.extend(sections.fittings.iter().filter_map(|item| {
            suppressed_item_from_aux(
                item.visible,
                item.suppressed_reason.as_deref(),
                &item.id,
                "fitting",
            )
        }));

        let mut suppressed_by_reason = BTreeMap::new();
        for item in &suppressed_items {
            *suppressed_by_reason.entry(item.reason.clone()).or_insert(0) += 1;
        }

        if sections.notes.is_empty() {
            sections
                .notes
                .push("branch-calculator assembled from legacy pre-laid sections".to_string());
        }

        LayoutResult {
            version: 1,
            mode: request.mode,
            stats: LayoutStats {
                linear_dims_count: sections.linear_dims.len(),
                cut_tubis_count: sections.cut_tubis.len(),
                welds_count: sections.welds.len(),
                slopes_count: sections.slopes.len(),
                bends_count: sections.bends.len(),
                tags_count: sections.tags.len(),
                fittings_count: sections.fittings.len(),
                suppressed_count: suppressed_items.len(),
            },
            linear_dims: sections.linear_dims,
            cut_tubis: sections.cut_tubis,
            welds: sections.welds,
            slopes: sections.slopes,
            bends: sections.bends,
            tags: sections.tags,
            fittings: sections.fittings,
            suppressed_items,
            debug_info: Some(LayoutDebugInfo {
                solver_version: "old_pml_branch_solver_v1".to_string(),
                notes: sections.notes,
                isoline_count: sections.isoline_count,
                inferred_face_center_count: sections.inferred_face_center_count,
                suppressed_by_reason,
                legacy_diff_summary: sections.legacy_diff_summary,
            }),
        }
    }
}

fn suppressed_item_from_linear(item: &PlacedLinearDim, kind: &str) -> Option<SuppressedItem> {
    item.suppressed_reason
        .as_ref()
        .map(|reason| SuppressedItem {
            id: item.id.clone(),
            kind: kind.to_string(),
            reason: reason.clone(),
        })
}

fn suppressed_item_from_aux(
    visible: bool,
    suppressed_reason: Option<&str>,
    id: &str,
    kind: &str,
) -> Option<SuppressedItem> {
    if visible && suppressed_reason.is_none() {
        return None;
    }
    suppressed_reason.map(|reason| SuppressedItem {
        id: id.to_string(),
        kind: kind.to_string(),
        reason: reason.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assemble_prelaid_out_should_collect_suppressed_reason_stats() {
        let result = BranchCalculator::assemble_prelaid_out(
            &LayoutRequest::default(),
            LegacyPlacedLayoutSections {
                linear_dims: vec![PlacedLinearDim {
                    id: "dim-1".to_string(),
                    kind: "segment".to_string(),
                    visible: false,
                    suppressed_reason: Some("too_dense".to_string()),
                    ..Default::default()
                }],
                bends: vec![PlacedBend {
                    id: "bend-1".to_string(),
                    visible: false,
                    suppressed_reason: Some("invalid_bend_layout_points".to_string()),
                    ..Default::default()
                }],
                inferred_face_center_count: 2,
                isoline_count: 3,
                ..Default::default()
            },
        );

        assert_eq!(result.stats.linear_dims_count, 1);
        assert_eq!(result.stats.bends_count, 1);
        assert_eq!(result.stats.suppressed_count, 2);
        assert_eq!(
            result
                .debug_info
                .as_ref()
                .and_then(|info| info.suppressed_by_reason.get("too_dense"))
                .copied(),
            Some(1)
        );
        assert_eq!(
            result
                .debug_info
                .as_ref()
                .and_then(|info| info.suppressed_by_reason.get("invalid_bend_layout_points"))
                .copied(),
            Some(1)
        );
        assert_eq!(
            result
                .debug_info
                .as_ref()
                .map(|info| info.inferred_face_center_count),
            Some(2)
        );
        assert_eq!(
            result.debug_info.as_ref().map(|info| info.isoline_count),
            Some(3)
        );
    }
}
