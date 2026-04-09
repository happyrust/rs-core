use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

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

pub struct BranchCalculator;

impl BranchCalculator {
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
        suppressed_items.extend(
            sections
                .welds
                .iter()
                .filter_map(|item| suppressed_item_from_aux(
                    item.visible,
                    item.suppressed_reason.as_deref(),
                    &item.id,
                    "weld",
                )),
        );
        suppressed_items.extend(
            sections
                .slopes
                .iter()
                .filter_map(|item| suppressed_item_from_aux(
                    item.visible,
                    item.suppressed_reason.as_deref(),
                    &item.id,
                    "slope",
                )),
        );
        suppressed_items.extend(
            sections
                .bends
                .iter()
                .filter_map(|item| suppressed_item_from_aux(
                    item.visible,
                    item.suppressed_reason.as_deref(),
                    &item.id,
                    "bend",
                )),
        );
        suppressed_items.extend(sections.bends.iter().flat_map(|bend| {
            bend.size_dims
                .iter()
                .filter_map(|item| suppressed_item_from_linear(item, "bend_size_dim"))
        }));
        suppressed_items.extend(
            sections
                .tags
                .iter()
                .filter_map(|item| suppressed_item_from_aux(
                    item.visible,
                    item.suppressed_reason.as_deref(),
                    &item.id,
                    "tag",
                )),
        );
        suppressed_items.extend(
            sections
                .fittings
                .iter()
                .filter_map(|item| suppressed_item_from_aux(
                    item.visible,
                    item.suppressed_reason.as_deref(),
                    &item.id,
                    "fitting",
                )),
        );

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
    item.suppressed_reason.as_ref().map(|reason| SuppressedItem {
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
            result.debug_info.as_ref().map(|info| info.inferred_face_center_count),
            Some(2)
        );
        assert_eq!(
            result.debug_info.as_ref().map(|info| info.isoline_count),
            Some(3)
        );
    }
}
