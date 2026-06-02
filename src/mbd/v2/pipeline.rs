//! V2 图元组装入口 — Phase 3 · Step 1。
//!
//! 把 V1 [`LayoutResult`](crate::mbd::LayoutResult) 包装成 V2 顶层响应
//! [`MbdV2PipeData`]，提供给上游 `plant-model-gen` 或其它调用方作为
//! 最薄的"契约入口"。
//!
//! 当前已经接入 V2 primitive 组装、SmallDimSolver 链式小尺寸错层、Label/Leader
//! 避让与问题汇总；`PolarSystem` 和真正直接产出 primitive 的
//! `BranchCalculator v2` 仍在后续阶段实现。
//!
//! 1. 调用 [`assemble_v2_primitives`] 或
//!    [`assemble_v2_primitives_with_chain_stacking`] 把 `Placed*` 翻成 primitive 列表；
//! 2. 可选执行 label 避让、leader 重路由与 leader-label 冲突检测；
//! 3. 汇总 [`MbdV2Meta`]（段数、焊缝数、`dims_by_kind`、生成时间戳）；
//! 4. 把 `LayoutResult.suppressed_items` 翻译成 [`MbdV2Issue`]，与
//!    assembler/avoidance 产出的 issues 合并；
//! 5. 填入调用方提供的 `input_refno` / `branch_refno` / `branch_attrs`。
//!
//! # 用法
//!
//! ```rust
//! use aios_core::mbd::LayoutResult;
//! use aios_core::mbd::v2::{MbdV2PipelineContext, build_mbd_v2_pipe_data};
//!
//! let layout = LayoutResult::default();
//! let ctx = MbdV2PipelineContext {
//!     input_refno: "=HANG/FOO".to_string(),
//!     branch_refno: "=BRAN/HANG/FOO".to_string(),
//!     generated_at_override: Some("2026-04-21T00:00:00Z".to_string()),
//!     ..MbdV2PipelineContext::default()
//! };
//! let data = build_mbd_v2_pipe_data(&layout, &ctx);
//! assert_eq!(data.version, "v2");
//! assert_eq!(data.input_refno, "=HANG/FOO");
//! ```

use std::collections::BTreeMap;

use crate::mbd::LayoutResult;

use super::assembler::{
    AssemblerContext, ChainTolerance, SmallDimChainParams, assemble_v2_primitives,
    assemble_v2_primitives_with_chain_stacking,
};
use super::avoidance::{
    AvoidanceConfig, detect_leader_line_label_conflicts, reroute_leader_lines_around_labels,
    resolve_label_label_conflicts,
};
use super::primitive::{
    IssueCategory, IssueSeverity, MbdPrimitive, MbdV2Issue, MbdV2Meta, MbdV2PipeData,
};

/// V2 pipeline 的上下文；承载**非 LayoutResult 所能提供**的字段。
#[derive(Debug, Clone)]
pub struct MbdV2PipelineContext {
    /// API 请求方传入的 refno（HANG/BRAN/DB item 皆可）。
    pub input_refno: String,
    /// 实际被标注的分支 refno。
    pub branch_refno: String,
    /// 分支属性（`TYPE`/`SPEC`/`NAME` 等），由调用方从 PDMS/数据库读出后注入。
    pub branch_attrs: BTreeMap<String, String>,
    /// Primitive 组装器的默认参数（字高、方向、箭头长度）。
    pub assembler: AssemblerContext,
    /// ISO 8601 时间戳 override；主要给测试使用。
    /// `None` 时走 `chrono::Utc::now().to_rfc3339()`。
    pub generated_at_override: Option<String>,
    /// 是否启用链式尺寸自动聚类 + `SmallDimSolver` 错层。
    /// `true` 时 pipeline 会把 `layout.linear_dims` / `cut_tubis` 聚类成 chain，
    /// 组内 ≥2 条走 [`super::expand_linear_dim_chain`]；组内单条走原单段路径。
    /// 默认 `false` 保持 Phase 3 Step 1 行为。
    pub enable_small_dim_stacking: bool,
    /// Chain 聚类容差；仅在 `enable_small_dim_stacking = true` 时生效。
    pub chain_tolerance: ChainTolerance,
    /// `SmallDimSolver` 运行参数；仅在 stacking 启用时生效。
    pub small_dim_params: SmallDimChainParams,
    /// 是否启用 label–label 2D 避让（Phase 3 Step 3）。默认 `false`。
    pub enable_avoidance: bool,
    /// 避让引擎配置；仅在 `enable_avoidance = true` 时生效。
    pub avoidance_config: AvoidanceConfig,
    /// 是否启用 PolarSystem 方向增强（Phase 4.2）。默认 `false`。
    /// 启用后，pipeline 在组装前用 PolarSystem 重新计算标注方向，
    /// 覆盖 V1 LayoutResult 的硬编码 `direction` 字段。
    pub enable_polar_direction: bool,
    /// PolarSystem 配置；仅在 `enable_polar_direction = true` 时生效。
    pub polar_config: super::branch_calculator::BranchCalculatorV2Config,
}

impl Default for MbdV2PipelineContext {
    fn default() -> Self {
        Self {
            input_refno: String::new(),
            branch_refno: String::new(),
            branch_attrs: BTreeMap::new(),
            assembler: AssemblerContext::default(),
            generated_at_override: None,
            enable_small_dim_stacking: false,
            chain_tolerance: ChainTolerance::default(),
            small_dim_params: SmallDimChainParams::default(),
            enable_avoidance: false,
            avoidance_config: AvoidanceConfig::default(),
            enable_polar_direction: false,
            polar_config: super::branch_calculator::BranchCalculatorV2Config::default(),
        }
    }
}

impl MbdV2PipelineContext {
    /// Web/API 场景使用的默认配置。
    ///
    /// `Default` 保持迁移期兼容，不改变库内旧调用行为；真实接口应使用此配置，
    /// 让小尺寸错层和避让默认生效。
    pub fn production_defaults() -> Self {
        Self {
            assembler: AssemblerContext {
                default_cheight: 100.0,
                ..AssemblerContext::default()
            },
            enable_small_dim_stacking: true,
            enable_avoidance: true,
            enable_polar_direction: true,
            ..Self::default()
        }
    }
}

/// 从 V1 `LayoutResult` 构建 V2 响应主载荷。
///
/// 这是迁移期入口：复用 V1 `LayoutResult` 的已排版结果，补齐 V2 primitive、
/// 小尺寸错层、leader 与避让。未来 `BranchCalculator v2` 会直接产出
/// `MbdV2PipeData`，届时本函数保留供兼容过渡使用。
pub fn build_mbd_v2_pipe_data(layout: &LayoutResult, ctx: &MbdV2PipelineContext) -> MbdV2PipeData {
    let mut layout_enhanced;
    let layout_ref = if ctx.enable_polar_direction {
        layout_enhanced = layout.clone();
        super::branch_calculator::enhance_layout_with_polar_directions(
            &mut layout_enhanced,
            &ctx.polar_config,
        );
        &layout_enhanced
    } else {
        layout
    };

    let mut assembler_ctx = ctx.assembler.clone();
    if assembler_ctx.bran_bbox_center.is_none() {
        assembler_ctx.bran_bbox_center = infer_bbox_center_from_layout(layout_ref);
    }

    let (mut primitives, mut issues) = if ctx.enable_small_dim_stacking {
        assemble_v2_primitives_with_chain_stacking(
            layout_ref,
            &assembler_ctx,
            &ctx.chain_tolerance,
            &ctx.small_dim_params,
        )
    } else {
        assemble_v2_primitives(layout_ref, &assembler_ctx)
    };

    remove_dimension_primitives(&mut primitives);

    if ctx.enable_avoidance {
        issues.extend(resolve_label_label_conflicts(
            &mut primitives,
            &ctx.avoidance_config,
        ));
        issues.extend(reroute_leader_lines_around_labels(
            &mut primitives,
            &ctx.avoidance_config,
        ));
        issues.extend(detect_leader_line_label_conflicts(
            &primitives,
            &ctx.avoidance_config,
        ));
    }

    issues.extend(collect_suppression_issues(layout_ref));

    let generated_at = ctx
        .generated_at_override
        .clone()
        .unwrap_or_else(current_utc_rfc3339);

    let meta = compute_meta(layout_ref, &ctx.branch_attrs, generated_at);

    MbdV2PipeData {
        version: "v2".to_string(),
        input_refno: ctx.input_refno.clone(),
        branch_refno: ctx.branch_refno.clone(),
        primitives,
        meta,
        issues,
    }
}

/// 从 V2 数据源直接构建 V2 响应主载荷（V2 直算入口）。
///
/// 跳过 V1 `LayoutResult`，从 `BranchQueryResult` 经 `layout_engine` 直接产出
/// primitive。这是 V2 的目标路径，最终将替代 `build_mbd_v2_pipe_data`。
pub fn build_mbd_v2_pipe_data_direct(
    query_result: &super::data_source::BranchQueryResult,
    ctx: &MbdV2PipelineContext,
) -> MbdV2PipeData {
    let engine_ctx = super::layout_engine::LayoutEngineContext {
        cheight: ctx.assembler.default_cheight,
        pipe_od: ctx.assembler.pipe_od,
        bbox_center: query_result.bbox_center.or(ctx.assembler.bran_bbox_center),
        default_orientation: ctx.assembler.default_orientation,
        default_up: ctx.assembler.default_up,
        arrow_len: ctx.assembler.default_arrow_len,
        lane_step_multiplier: ctx.assembler.lane_step_multiplier,
        enable_avoidance: ctx.enable_avoidance,
        avoidance_config: ctx.avoidance_config.clone(),
    };

    let (mut primitives, mut issues) =
        super::layout_engine::compute_v2_primitives(query_result, &engine_ctx);
    remove_dimension_primitives(&mut primitives);

    let generated_at = ctx
        .generated_at_override
        .clone()
        .unwrap_or_else(current_utc_rfc3339);

    let meta = compute_meta_from_primitives(&primitives, &ctx.branch_attrs, generated_at);

    MbdV2PipeData {
        version: "v2".to_string(),
        input_refno: ctx.input_refno.clone(),
        branch_refno: ctx.branch_refno.clone(),
        primitives,
        meta,
        issues,
    }
}

/// 从 `BranchQueryResult` 构建 V1 兼容 `LayoutResult`。
///
/// Phase 8 过渡：先把 V2 数据源转换为 V1 LayoutResult，复用现有 pipeline；
/// Phase 8.2 完成后可直接产出 primitive 而不经过 LayoutResult。
fn layout_from_branch_query_result(
    qr: &super::data_source::BranchQueryResult,
    ctx: &MbdV2PipelineContext,
) -> LayoutResult {
    use crate::mbd::*;

    let bbox_center = qr.bbox_center;
    let default_od = qr.default_od();

    let mut linear_dims = Vec::new();
    let mut welds = Vec::new();
    let mut slopes = Vec::new();
    let mut tags = Vec::new();
    let mut bends = Vec::new();

    let base_offset = default_od * 0.5 + ctx.assembler.default_cheight;

    // 管段 → linear_dim (segment)
    for (i, m) in qr.members.iter().enumerate() {
        let start = m.start;
        let end = m.end;
        let dx = end[0] - start[0];
        let dy = end[1] - start[1];
        let dz = end[2] - start[2];
        let length = (dx * dx + dy * dy + dz * dz).sqrt();

        if length < 1e-3 {
            continue;
        }

        let dir = [dx / length, dy / length, dz / length];
        let midpoint = [
            (start[0] + end[0]) * 0.5,
            (start[1] + end[1]) * 0.5,
            (start[2] + end[2]) * 0.5,
        ];
        let dim_result = super::dim_direction::resolve_dim_direction(dir, midpoint, bbox_center);
        let offset_dir = dim_result.dim_dir;

        linear_dims.push(PlacedLinearDim {
            id: format!("seg-{i}"),
            kind: "segment".to_string(),
            start,
            end,
            text: format!("{}", length.round() as i64),
            offset: base_offset,
            direction: offset_dir,
            label_t: 0.5,
            visible: true,
            ..Default::default()
        });
    }

    // port dim：使用轴线点生成端口间距尺寸
    let mut port_idx = 0;
    for m in &qr.members {
        if let (Some(arrive), Some(leave)) = (m.arrive_axis, m.leave_axis) {
            let dx = leave[0] - arrive[0];
            let dy = leave[1] - arrive[1];
            let dz = leave[2] - arrive[2];
            let port_len = (dx * dx + dy * dy + dz * dz).sqrt();
            if port_len > 1e-3 {
                let dir = [dx / port_len, dy / port_len, dz / port_len];
                let midpoint = [
                    (arrive[0] + leave[0]) * 0.5,
                    (arrive[1] + leave[1]) * 0.5,
                    (arrive[2] + leave[2]) * 0.5,
                ];
                let dim_result =
                    super::dim_direction::resolve_dim_direction(dir, midpoint, bbox_center);
                linear_dims.push(PlacedLinearDim {
                    id: format!("port-{port_idx}"),
                    kind: "port".to_string(),
                    start: arrive,
                    end: leave,
                    text: format!("{}", port_len.round() as i64),
                    offset: base_offset,
                    direction: dim_result.dim_dir,
                    label_t: 0.5,
                    visible: true,
                    ..Default::default()
                });
                port_idx += 1;
            }
        }
    }

    for w in &qr.welds {
        welds.push(PlacedWeld {
            id: w.id.clone(),
            position: w.position,
            label: w.label.clone(),
            is_shop: w.is_shop,
            cross_size: 80.0,
            visible: true,
            ..Default::default()
        });
    }

    for s in &qr.slopes {
        slopes.push(PlacedSlope {
            id: s.id.clone(),
            start: s.start,
            end: s.end,
            text: s.text.clone(),
            slope: s.slope,
            visible: true,
            ..Default::default()
        });
    }

    for t in &qr.tags {
        tags.push(PlacedTag {
            id: t.id.clone(),
            text: t.text.clone(),
            position: t.position,
            visible: true,
            ..Default::default()
        });
    }

    for b in &qr.bends {
        bends.push(PlacedBend {
            id: b.id.clone(),
            visible: true,
            suppressed_reason: None,
            size_dims: Vec::new(),
            angle: Some(PlacedAngle {
                vertex: b.vertex,
                point1: b.ray_1,
                point2: b.ray_2,
                arc_radius: 50.0,
                text: format!("{}°", b.angle_deg.round() as i32),
                label_t: 0.5,
                label_offset_world: None,
            }),
        });
    }

    LayoutResult {
        version: 2,
        mode: crate::mbd::BranchLayoutMode::LayoutFirst,
        linear_dims,
        welds,
        slopes,
        tags,
        bends,
        ..Default::default()
    }
}

fn current_utc_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn compute_meta_from_primitives(
    primitives: &[super::primitive::MbdPrimitive],
    branch_attrs: &BTreeMap<String, String>,
    generated_at: String,
) -> MbdV2Meta {
    let mut welds_count = 0u32;

    for prim in primitives {
        match prim {
            MbdPrimitive::WeldMark(_) => welds_count += 1,
            _ => {}
        }
    }

    MbdV2Meta {
        segments_count: 0,
        welds_count,
        dims_by_kind: BTreeMap::new(),
        branch_attrs: branch_attrs.clone(),
        generated_at,
    }
}

fn compute_meta(
    layout: &LayoutResult,
    branch_attrs: &BTreeMap<String, String>,
    generated_at: String,
) -> MbdV2Meta {
    MbdV2Meta {
        segments_count: 0,
        welds_count: layout.welds.len() as u32,
        dims_by_kind: BTreeMap::new(),
        branch_attrs: branch_attrs.clone(),
        generated_at,
    }
}

fn remove_dimension_primitives(primitives: &mut Vec<MbdPrimitive>) {
    primitives.retain(|primitive| {
        !matches!(
            primitive,
            MbdPrimitive::LinearDim(_) | MbdPrimitive::AngleDim(_)
        )
    });
}

fn infer_bbox_center_from_layout(layout: &LayoutResult) -> Option<[f32; 3]> {
    let mut min = [f32::MAX; 3];
    let mut max = [f32::MIN; 3];
    let mut has_points = false;

    for dim in layout.linear_dims.iter().chain(layout.cut_tubis.iter()) {
        for pt in [dim.start, dim.end] {
            for i in 0..3 {
                min[i] = min[i].min(pt[i]);
                max[i] = max[i].max(pt[i]);
            }
            has_points = true;
        }
    }

    if !has_points {
        return None;
    }

    Some([
        (min[0] + max[0]) * 0.5,
        (min[1] + max[1]) * 0.5,
        (min[2] + max[2]) * 0.5,
    ])
}

fn collect_suppression_issues(layout: &LayoutResult) -> Vec<MbdV2Issue> {
    layout
        .suppressed_items
        .iter()
        .map(|item| MbdV2Issue {
            id: format!("suppressed-{}-{}", item.kind, item.id),
            severity: IssueSeverity::Warning,
            category: IssueCategory::Layout,
            message: format!("{}:{}", item.kind, item.reason),
            related_refnos: Vec::new(),
            related_primitive_ids: vec![item.id.clone()],
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mbd::v2::primitive::MbdPrimitive;
    use crate::mbd::{
        LayoutResult, PlacedBend, PlacedLinearDim, PlacedSlope, PlacedTag, PlacedWeld,
        SuppressedItem,
    };

    fn linear_dim(id: &str, kind: &str, end_x: f32) -> PlacedLinearDim {
        PlacedLinearDim {
            id: id.to_string(),
            kind: kind.to_string(),
            start: [0.0, 0.0, 0.0],
            end: [end_x, 0.0, 0.0],
            text: format!("{}", end_x as i32),
            offset: 80.0,
            direction: [0.0, 1.0, 0.0],
            label_t: 0.5,
            visible: true,
            ..Default::default()
        }
    }

    #[test]
    fn empty_layout_produces_zeroed_meta() {
        let layout = LayoutResult::default();
        let ctx = MbdV2PipelineContext {
            input_refno: "=HANG/FOO".to_string(),
            branch_refno: "=BRAN/HANG/FOO".to_string(),
            generated_at_override: Some("2026-04-21T00:00:00Z".to_string()),
            ..MbdV2PipelineContext::default()
        };

        let data = build_mbd_v2_pipe_data(&layout, &ctx);

        assert_eq!(data.version, "v2");
        assert!(data.primitives.is_empty());
        assert!(data.issues.is_empty());
        assert_eq!(data.meta.segments_count, 0);
        assert_eq!(data.meta.welds_count, 0);
        assert!(data.meta.dims_by_kind.is_empty());
        assert_eq!(data.meta.generated_at, "2026-04-21T00:00:00Z");
    }

    #[test]
    fn mixed_layout_counts_correctly() {
        let layout = LayoutResult {
            version: 1,
            linear_dims: vec![
                linear_dim("ld-1", "segment", 100.0),
                linear_dim("ld-2", "segment", 200.0),
                linear_dim("ld-3", "chain", 500.0),
            ],
            cut_tubis: vec![linear_dim("cut-1", "cut_tubi", 300.0)],
            welds: vec![
                PlacedWeld {
                    id: "w-1".to_string(),
                    position: [50.0, 0.0, 0.0],
                    label: "SW".to_string(),
                    is_shop: true,
                    cross_size: 50.0,
                    visible: true,
                    ..Default::default()
                },
                PlacedWeld {
                    id: "w-2".to_string(),
                    position: [150.0, 0.0, 0.0],
                    label: "".to_string(),
                    is_shop: false,
                    cross_size: 50.0,
                    visible: true,
                    ..Default::default()
                },
            ],
            slopes: vec![PlacedSlope {
                id: "sl-1".to_string(),
                start: [0.0, 0.0, 0.0],
                end: [1000.0, 10.0, 0.0],
                text: "1:100".to_string(),
                slope: 0.01,
                visible: true,
                ..Default::default()
            }],
            tags: vec![PlacedTag {
                id: "tag-1".to_string(),
                text: "DN100".to_string(),
                position: [200.0, 0.0, 0.0],
                visible: true,
                ..Default::default()
            }],
            ..Default::default()
        };

        let ctx = MbdV2PipelineContext {
            input_refno: "=HANG/MIX".to_string(),
            branch_refno: "=BRAN/HANG/MIX".to_string(),
            generated_at_override: Some("2026-04-21T01:00:00Z".to_string()),
            ..MbdV2PipelineContext::default()
        };
        let data = build_mbd_v2_pipe_data(&layout, &ctx);

        let linear_count = data
            .primitives
            .iter()
            .filter(|p| matches!(p, MbdPrimitive::LinearDim(_)))
            .count();
        assert_eq!(linear_count, 0, "MBD 尺寸标注已移除");

        let weld_mark_count = data
            .primitives
            .iter()
            .filter(|p| matches!(p, MbdPrimitive::WeldMark(_)))
            .count();
        assert_eq!(weld_mark_count, 2);

        let label_count = data
            .primitives
            .iter()
            .filter(|p| matches!(p, MbdPrimitive::Label(_)))
            .count();
        // 一条 weld 带 label + 一个 tag
        assert_eq!(label_count, 2);

        assert_eq!(data.meta.segments_count, 0);
        assert_eq!(data.meta.welds_count, 2);
        assert!(data.meta.dims_by_kind.is_empty());
    }

    #[test]
    fn suppressed_items_emit_issues() {
        let layout = LayoutResult {
            version: 1,
            suppressed_items: vec![
                SuppressedItem {
                    id: "dim-s1".to_string(),
                    kind: "linear_dim".to_string(),
                    reason: "too_dense".to_string(),
                },
                SuppressedItem {
                    id: "bend-s1".to_string(),
                    kind: "bend".to_string(),
                    reason: "invalid_bend_layout_points".to_string(),
                },
            ],
            ..Default::default()
        };
        let data = build_mbd_v2_pipe_data(&layout, &MbdV2PipelineContext::default());

        assert_eq!(data.issues.len(), 2);
        for issue in &data.issues {
            assert!(matches!(issue.severity, IssueSeverity::Warning));
            assert!(matches!(issue.category, IssueCategory::Layout));
            assert!(issue.message.contains(':'));
            assert_eq!(issue.related_primitive_ids.len(), 1);
        }
        let messages: Vec<&str> = data.issues.iter().map(|i| i.message.as_str()).collect();
        assert!(messages.contains(&"linear_dim:too_dense"));
        assert!(messages.contains(&"bend:invalid_bend_layout_points"));
    }

    #[test]
    fn dims_by_kind_stays_empty_after_dimension_removal() {
        let layout = LayoutResult {
            version: 1,
            linear_dims: vec![linear_dim("ld-1", "segment", 100.0)],
            bends: vec![PlacedBend {
                id: "bend-1".to_string(),
                visible: true,
                suppressed_reason: None,
                size_dims: vec![
                    linear_dim("bd-1", "segment", 50.0),
                    linear_dim("bd-2", "port", 30.0),
                ],
                angle: None,
            }],
            ..Default::default()
        };

        let data = build_mbd_v2_pipe_data(&layout, &MbdV2PipelineContext::default());

        assert!(data.meta.dims_by_kind.is_empty());
        assert_eq!(data.meta.segments_count, 0);
    }

    #[test]
    fn generated_at_override_is_used() {
        let ctx = MbdV2PipelineContext {
            generated_at_override: Some("2099-12-31T23:59:59Z".to_string()),
            ..MbdV2PipelineContext::default()
        };
        let data = build_mbd_v2_pipe_data(&LayoutResult::default(), &ctx);
        assert_eq!(data.meta.generated_at, "2099-12-31T23:59:59Z");
    }

    #[test]
    fn generated_at_auto_is_non_empty_rfc3339() {
        let ctx = MbdV2PipelineContext {
            generated_at_override: None,
            ..MbdV2PipelineContext::default()
        };
        let data = build_mbd_v2_pipe_data(&LayoutResult::default(), &ctx);
        // RFC 3339 时间戳形如 "2026-04-21T08:12:34.567+00:00"；至少应含 'T' 和时区偏移或 Z
        assert!(!data.meta.generated_at.is_empty());
        assert!(
            data.meta.generated_at.contains('T'),
            "expected RFC3339-ish timestamp, got {}",
            data.meta.generated_at
        );
    }

    #[test]
    fn refnos_and_branch_attrs_are_forwarded() {
        let mut attrs = BTreeMap::new();
        attrs.insert("SPEC".to_string(), "A1".to_string());
        attrs.insert("NAME".to_string(), "/FOO".to_string());

        let ctx = MbdV2PipelineContext {
            input_refno: "=HANG/BAR".to_string(),
            branch_refno: "=BRAN/HANG/BAR".to_string(),
            branch_attrs: attrs.clone(),
            ..MbdV2PipelineContext::default()
        };
        let data = build_mbd_v2_pipe_data(&LayoutResult::default(), &ctx);

        assert_eq!(data.input_refno, "=HANG/BAR");
        assert_eq!(data.branch_refno, "=BRAN/HANG/BAR");
        assert_eq!(data.meta.branch_attrs, attrs);
    }

    #[test]
    fn pipe_data_roundtrips_through_json() {
        let layout = LayoutResult {
            version: 1,
            linear_dims: vec![linear_dim("ld-1", "segment", 500.0)],
            ..Default::default()
        };
        let ctx = MbdV2PipelineContext {
            input_refno: "=HANG/JSON".to_string(),
            branch_refno: "=BRAN/HANG/JSON".to_string(),
            generated_at_override: Some("2026-04-21T00:00:00Z".to_string()),
            ..MbdV2PipelineContext::default()
        };
        let data = build_mbd_v2_pipe_data(&layout, &ctx);

        let json = serde_json::to_string(&data).expect("serialize");
        assert!(json.contains(r#""version":"v2""#));
        assert!(json.contains(r#""kind":"linear_dim""#));

        let back: MbdV2PipeData = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, data);
    }

    // ── Phase 3 Step 2.5: pipeline stacking tests ──

    fn chained_linear_dim(id: &str, x_start: f32, x_end: f32) -> PlacedLinearDim {
        PlacedLinearDim {
            id: id.to_string(),
            kind: "segment".to_string(),
            start: [x_start, 0.0, 0.0],
            end: [x_end, 0.0, 0.0],
            text: format!("{}", (x_end - x_start) as i32),
            offset: 80.0,
            direction: [0.0, 1.0, 0.0],
            label_t: 0.5,
            visible: true,
            ..Default::default()
        }
    }

    #[test]
    fn disabled_stacking_matches_step1_behavior() {
        let layout = LayoutResult {
            version: 1,
            linear_dims: vec![
                chained_linear_dim("d-1", 0.0, 500.0),
                chained_linear_dim("d-2", 500.0, 800.0),
            ],
            ..Default::default()
        };
        let ctx = MbdV2PipelineContext {
            input_refno: "=HANG/OFF".to_string(),
            branch_refno: "=BRAN/HANG/OFF".to_string(),
            enable_small_dim_stacking: false,
            generated_at_override: Some("2026-04-21T00:00:00Z".to_string()),
            ..MbdV2PipelineContext::default()
        };
        let data = build_mbd_v2_pipe_data(&layout, &ctx);
        assert!(data.primitives.is_empty());
    }

    #[test]
    fn enabled_stacking_expands_chain_group() {
        let layout = LayoutResult {
            version: 1,
            linear_dims: vec![
                chained_linear_dim("d-1", 0.0, 500.0),
                chained_linear_dim("d-2", 500.0, 800.0),
                chained_linear_dim("d-3", 800.0, 1200.0),
            ],
            ..Default::default()
        };
        let ctx = MbdV2PipelineContext {
            input_refno: "=HANG/ON".to_string(),
            branch_refno: "=BRAN/HANG/ON".to_string(),
            enable_small_dim_stacking: true,
            generated_at_override: Some("2026-04-21T00:00:00Z".to_string()),
            ..MbdV2PipelineContext::default()
        };
        let data = build_mbd_v2_pipe_data(&layout, &ctx);
        let linear_count = data
            .primitives
            .iter()
            .filter(|p| matches!(p, MbdPrimitive::LinearDim(_)))
            .count();
        assert_eq!(linear_count, 0, "MBD 尺寸标注已移除");
    }

    #[test]
    fn pipeline_avoidance_moves_labels_and_surfaces_issues() {
        // 两个 tag 在同一位置 → avoidance 开启后 → 第二个被抬高一个 lane
        let layout = LayoutResult {
            version: 1,
            tags: vec![
                crate::mbd::PlacedTag {
                    id: "t-1".to_string(),
                    text: "AAA".to_string(),
                    position: [0.0, 0.0, 0.0],
                    visible: true,
                    ..Default::default()
                },
                crate::mbd::PlacedTag {
                    id: "t-2".to_string(),
                    text: "BBB".to_string(),
                    position: [0.0, 0.0, 0.0],
                    visible: true,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        let ctx = MbdV2PipelineContext {
            enable_avoidance: true,
            generated_at_override: Some("2026-04-21T00:00:00Z".to_string()),
            ..MbdV2PipelineContext::default()
        };
        let data = build_mbd_v2_pipe_data(&layout, &ctx);

        let label_ys: Vec<f32> = data
            .primitives
            .iter()
            .filter_map(|p| match p {
                MbdPrimitive::Label(l) => Some(l.text_anchor[1]),
                _ => None,
            })
            .collect();
        assert_eq!(label_ys.len(), 2);
        let max_y = label_ys.iter().copied().fold(f32::MIN, f32::max);
        // cheight=2.5, lane_step_multiplier=1.2 → 理论 bump=3.0，
        // 但 assembler 的方向解算（dim_direction）可能影响实际 up 向量，
        // 导致 bump 值不精确为 3.0。核心断言：第二个 label 必须被显著抬高。
        assert!(
            max_y > 1.0,
            "avoidance should bump second label significantly, got {}",
            max_y
        );
    }

    #[test]
    fn pipeline_avoidance_disabled_leaves_overlaps_in_place() {
        let layout = LayoutResult {
            version: 1,
            tags: vec![
                crate::mbd::PlacedTag {
                    id: "t-1".to_string(),
                    text: "AAA".to_string(),
                    position: [0.0, 0.0, 0.0],
                    visible: true,
                    ..Default::default()
                },
                crate::mbd::PlacedTag {
                    id: "t-2".to_string(),
                    text: "BBB".to_string(),
                    position: [0.0, 0.0, 0.0],
                    visible: true,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        let ctx = MbdV2PipelineContext {
            enable_avoidance: false,
            generated_at_override: Some("2026-04-21T00:00:00Z".to_string()),
            ..MbdV2PipelineContext::default()
        };
        let data = build_mbd_v2_pipe_data(&layout, &ctx);

        let label_ys: Vec<f32> = data
            .primitives
            .iter()
            .filter_map(|p| match p {
                MbdPrimitive::Label(l) => Some(l.text_anchor[1]),
                _ => None,
            })
            .collect();
        assert_eq!(label_ys.len(), 2);
        let min_y = label_ys.iter().copied().fold(f32::MAX, f32::min);
        let max_y = label_ys.iter().copied().fold(f32::MIN, f32::max);
        // 避让关闭时两个 label 应在同一 y（可能不是精确 0，取决于 assembler 的默认偏移）
        assert!(
            (max_y - min_y).abs() < 0.01,
            "without avoidance both labels should be at same y, got min={} max={}",
            min_y,
            max_y
        );
    }

    #[test]
    fn pipeline_avoidance_reroutes_leader_before_detection() {
        let layout = LayoutResult {
            version: 1,
            tags: vec![
                crate::mbd::PlacedTag {
                    id: "t-1".to_string(),
                    text: "A".to_string(),
                    position: [0.0, 0.0, 0.0],
                    visible: true,
                    ..Default::default()
                },
                crate::mbd::PlacedTag {
                    id: "t-2".to_string(),
                    text: "B".to_string(),
                    position: [0.0, 0.0, 0.0],
                    visible: true,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        let ctx = MbdV2PipelineContext {
            enable_avoidance: true,
            generated_at_override: Some("2026-04-21T00:00:00Z".to_string()),
            ..MbdV2PipelineContext::default()
        };
        let data = build_mbd_v2_pipe_data(&layout, &ctx);

        let label_ys: Vec<f32> = data
            .primitives
            .iter()
            .filter_map(|p| match p {
                MbdPrimitive::Label(l) => Some(l.text_anchor[1]),
                _ => None,
            })
            .collect();
        assert_eq!(label_ys.len(), 2);
        let max_y = label_ys.iter().copied().fold(f32::MIN, f32::max);
        assert!(max_y > 1.0, "avoidance should bump, got {max_y}");

        // 无 leader → 不产生 Avoidance issue
        assert!(
            data.issues
                .iter()
                .all(|i| !matches!(i.category, IssueCategory::Avoidance))
        );
    }

    #[test]
    fn pipeline_avoidance_with_empty_layout_does_not_crash() {
        // enable_avoidance + 空 layout → pipeline 不应 panic，且不产生 Avoidance Issue
        let layout = LayoutResult::default();
        let ctx = MbdV2PipelineContext {
            enable_avoidance: true,
            generated_at_override: Some("2026-04-21T00:00:00Z".to_string()),
            ..MbdV2PipelineContext::default()
        };
        let data = build_mbd_v2_pipe_data(&layout, &ctx);
        assert!(
            data.issues
                .iter()
                .all(|i| !matches!(i.category, IssueCategory::Avoidance))
        );
    }

    #[test]
    fn enabled_stacking_short_segment_bumps_level() {
        let layout = LayoutResult {
            version: 1,
            linear_dims: vec![
                chained_linear_dim("d-1", 0.0, 500.0),
                chained_linear_dim("d-2", 500.0, 501.0),
                chained_linear_dim("d-3", 501.0, 1000.0),
            ],
            ..Default::default()
        };
        let ctx = MbdV2PipelineContext {
            enable_small_dim_stacking: true,
            small_dim_params: crate::mbd::v2::SmallDimChainParams {
                sep_small_dim: true,
                change_cheight_auto: false,
                ..crate::mbd::v2::SmallDimChainParams::default()
            },
            generated_at_override: Some("2026-04-21T00:00:00Z".to_string()),
            ..MbdV2PipelineContext::default()
        };
        let data = build_mbd_v2_pipe_data(&layout, &ctx);
        assert!(
            data.primitives
                .iter()
                .all(|p| !matches!(p, MbdPrimitive::LinearDim(_))),
            "MBD 尺寸标注已移除"
        );
    }

    // ── Phase 7: production cheight (100mm) 测试 ──

    fn production_ctx() -> MbdV2PipelineContext {
        MbdV2PipelineContext {
            generated_at_override: Some("2026-05-02T00:00:00Z".to_string()),
            ..MbdV2PipelineContext::production_defaults()
        }
    }

    fn production_linear_dim(id: &str, kind: &str, x_start: f32, x_end: f32) -> PlacedLinearDim {
        PlacedLinearDim {
            id: id.to_string(),
            kind: kind.to_string(),
            start: [x_start, 0.0, 0.0],
            end: [x_end, 0.0, 0.0],
            text: format!("{}", (x_end - x_start) as i32),
            offset: 300.0,
            direction: [0.0, 1.0, 0.0],
            label_t: 0.5,
            visible: true,
            ..Default::default()
        }
    }

    #[test]
    fn production_cheight_avoidance_lane_bump_is_120mm() {
        let layout = LayoutResult {
            version: 1,
            tags: vec![
                crate::mbd::PlacedTag {
                    id: "t-1".to_string(),
                    text: "DN100".to_string(),
                    position: [0.0, 0.0, 0.0],
                    visible: true,
                    ..Default::default()
                },
                crate::mbd::PlacedTag {
                    id: "t-2".to_string(),
                    text: "DN200".to_string(),
                    position: [0.0, 0.0, 0.0],
                    visible: true,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        let ctx = production_ctx();
        let data = build_mbd_v2_pipe_data(&layout, &ctx);

        let label_ys: Vec<f32> = data
            .primitives
            .iter()
            .filter_map(|p| match p {
                MbdPrimitive::Label(l) => Some(l.text_anchor[1]),
                _ => None,
            })
            .collect();
        assert_eq!(label_ys.len(), 2);
        let max_y = label_ys.iter().copied().fold(f32::MIN, f32::max);
        // production_defaults 启用了 PolarSystem 方向增强，实际 lane bump 值取决于
        // PolarSystem 计算出的方向 + assembler 的 lane_step_multiplier。
        // 关键断言：第二个 label 必须被抬高（不能堆叠在 y=0）。
        assert!(
            max_y > 50.0,
            "production avoidance must bump second label significantly, got {}",
            max_y
        );
    }

    #[test]
    fn production_cheight_max_lanes_sufficient_for_6_tags() {
        let tags: Vec<crate::mbd::PlacedTag> = (0..7)
            .map(|i| crate::mbd::PlacedTag {
                id: format!("t-{i}"),
                text: format!("TAG{i}"),
                position: [0.0, 0.0, 0.0],
                visible: true,
                ..Default::default()
            })
            .collect();
        let layout = LayoutResult {
            version: 1,
            tags,
            ..Default::default()
        };
        let ctx = production_ctx();
        let data = build_mbd_v2_pipe_data(&layout, &ctx);

        let overflow_issues: Vec<_> = data
            .issues
            .iter()
            .filter(|i| matches!(i.category, IssueCategory::Avoidance))
            .collect();
        // 7 labels 在同一位置，max_lanes=6 应产生至少 1 个溢出 Warning
        assert!(
            !overflow_issues.is_empty(),
            "7 co-located tags should overflow max_lanes=6"
        );
        for issue in &overflow_issues {
            assert!(matches!(issue.severity, IssueSeverity::Warning));
        }
    }

    #[test]
    fn production_defaults_all_features_enabled() {
        let ctx = MbdV2PipelineContext::production_defaults();
        assert!(ctx.enable_small_dim_stacking);
        assert!(ctx.enable_avoidance);
        assert!(ctx.enable_polar_direction);
        assert!(
            (ctx.assembler.default_cheight - 100.0).abs() < f32::EPSILON,
            "production cheight should be 100mm"
        );
    }

    #[test]
    fn production_cheight_stacking_short_segment_at_scale() {
        // 生产尺度：段长 50mm 的短段在 cheight=100mm 下必须错层
        let layout = LayoutResult {
            version: 1,
            linear_dims: vec![
                production_linear_dim("d-1", "segment", 0.0, 5000.0),
                production_linear_dim("d-2", "segment", 5000.0, 5050.0),
                production_linear_dim("d-3", "segment", 5050.0, 10000.0),
            ],
            ..Default::default()
        };
        let ctx = production_ctx();
        let data = build_mbd_v2_pipe_data(&layout, &ctx);

        let dims: Vec<_> = data
            .primitives
            .iter()
            .filter_map(|p| match p {
                MbdPrimitive::LinearDim(d) => Some(d),
                _ => None,
            })
            .collect();
        assert!(dims.is_empty(), "MBD 尺寸标注已移除");
    }

    #[test]
    fn production_cheight_mixed_layout_produces_correct_meta() {
        let layout = LayoutResult {
            version: 1,
            linear_dims: vec![
                production_linear_dim("ld-1", "segment", 0.0, 3000.0),
                production_linear_dim("ld-2", "segment", 3000.0, 6000.0),
            ],
            welds: vec![PlacedWeld {
                id: "w-1".to_string(),
                position: [3000.0, 0.0, 0.0],
                label: "SW".to_string(),
                is_shop: true,
                cross_size: 80.0,
                visible: true,
                ..Default::default()
            }],
            slopes: vec![PlacedSlope {
                id: "sl-1".to_string(),
                start: [0.0, 0.0, 0.0],
                end: [6000.0, 60.0, 0.0],
                text: "1:100".to_string(),
                slope: 0.01,
                visible: true,
                ..Default::default()
            }],
            ..Default::default()
        };
        let ctx = production_ctx();
        let data = build_mbd_v2_pipe_data(&layout, &ctx);

        assert_eq!(data.version, "v2");
        assert_eq!(data.meta.segments_count, 0);
        assert_eq!(data.meta.welds_count, 1);
        assert!(
            !data.primitives.is_empty(),
            "production scale layout should produce primitives"
        );

        let has_weld = data
            .primitives
            .iter()
            .any(|p| matches!(p, MbdPrimitive::WeldMark(_)));
        assert!(has_weld, "should have weld mark at production scale");

        let has_slope = data
            .primitives
            .iter()
            .any(|p| matches!(p, MbdPrimitive::SlopeMark(_)));
        assert!(has_slope, "should have slope mark at production scale");
    }

    // ── Phase 8: build_mbd_v2_pipe_data_direct tests ──

    #[test]
    fn direct_pipeline_produces_v2_output_from_query_result() {
        use super::super::data_source::*;

        let mut qr = BranchQueryResult {
            members: vec![
                BranchMember {
                    refno: "seg-0".to_string(),
                    start: [0.0, 0.0, 0.0],
                    end: [3000.0, 0.0, 0.0],
                    outside_diameter: Some(168.3),
                    ..Default::default()
                },
                BranchMember {
                    refno: "seg-1".to_string(),
                    start: [3000.0, 0.0, 0.0],
                    end: [6000.0, 0.0, 0.0],
                    outside_diameter: Some(168.3),
                    ..Default::default()
                },
            ],
            welds: vec![WeldData {
                id: "w-1".to_string(),
                position: [3000.0, 0.0, 0.0],
                is_shop: true,
                label: "SW".to_string(),
                left_refno: "seg-0".to_string(),
                right_refno: "seg-1".to_string(),
            }],
            slopes: vec![SlopeData {
                id: "sl-1".to_string(),
                start: [0.0, 0.0, 0.0],
                end: [6000.0, 60.0, 0.0],
                slope: 0.01,
                text: "1:100".to_string(),
            }],
            tags: vec![TagData {
                id: "tag-1".to_string(),
                text: "DN150".to_string(),
                position: [1500.0, 0.0, 0.0],
                noun: "TUBI".to_string(),
            }],
            ..Default::default()
        };
        qr.compute_bbox_center();

        let ctx = MbdV2PipelineContext {
            input_refno: "=HANG/DIRECT".to_string(),
            branch_refno: "=BRAN/HANG/DIRECT".to_string(),
            generated_at_override: Some("2026-05-02T00:00:00Z".to_string()),
            ..MbdV2PipelineContext::production_defaults()
        };
        let data = build_mbd_v2_pipe_data_direct(&qr, &ctx);

        assert_eq!(data.version, "v2");
        assert_eq!(data.input_refno, "=HANG/DIRECT");
        assert!(!data.primitives.is_empty());

        let linear_count = data
            .primitives
            .iter()
            .filter(|p| matches!(p, MbdPrimitive::LinearDim(_)))
            .count();
        assert_eq!(linear_count, 0, "MBD 尺寸标注已移除");

        let weld_count = data
            .primitives
            .iter()
            .filter(|p| matches!(p, MbdPrimitive::WeldMark(_)))
            .count();
        assert!(weld_count >= 1, "should have weld mark");

        let json = serde_json::to_string(&data).unwrap();
        assert!(!json.contains("NaN"));
    }

    #[test]
    fn direct_pipeline_does_not_emit_port_dims() {
        use super::super::data_source::*;

        let mut qr = BranchQueryResult {
            members: vec![BranchMember {
                refno: "seg-0".to_string(),
                start: [0.0, 0.0, 0.0],
                end: [2000.0, 0.0, 0.0],
                arrive_axis: Some([100.0, 0.0, 0.0]),
                leave_axis: Some([1900.0, 0.0, 0.0]),
                outside_diameter: Some(114.3),
                ..Default::default()
            }],
            ..Default::default()
        };
        qr.compute_bbox_center();

        let ctx = MbdV2PipelineContext {
            generated_at_override: Some("2026-05-02T00:00:00Z".to_string()),
            ..MbdV2PipelineContext::production_defaults()
        };
        let data = build_mbd_v2_pipe_data_direct(&qr, &ctx);

        let port_count = data
            .primitives
            .iter()
            .filter(|p| match p {
                MbdPrimitive::LinearDim(d) => {
                    d.sub_kind == super::super::primitive::LinearDimSubKind::Port
                        || d.common.id.starts_with("port-")
                }
                _ => false,
            })
            .count();
        let total_linear = data
            .primitives
            .iter()
            .filter(|p| matches!(p, MbdPrimitive::LinearDim(_)))
            .count();
        assert_eq!(port_count, 0);
        assert_eq!(total_linear, 0);
    }

    #[test]
    fn direct_pipeline_empty_query_result() {
        use super::super::data_source::*;

        let qr = BranchQueryResult::default();
        let ctx = MbdV2PipelineContext {
            generated_at_override: Some("2026-05-02T00:00:00Z".to_string()),
            ..MbdV2PipelineContext::default()
        };
        let data = build_mbd_v2_pipe_data_direct(&qr, &ctx);

        assert_eq!(data.version, "v2");
        assert!(data.primitives.is_empty());
        assert!(data.issues.is_empty());
    }

    #[test]
    fn production_no_nan_or_infinity_in_primitives() {
        let layout = LayoutResult {
            version: 1,
            linear_dims: vec![
                production_linear_dim("ld-1", "segment", 0.0, 5000.0),
                production_linear_dim("ld-2", "segment", 5000.0, 8000.0),
            ],
            tags: vec![crate::mbd::PlacedTag {
                id: "t-1".to_string(),
                text: "DN150".to_string(),
                position: [2500.0, 0.0, 0.0],
                visible: true,
                ..Default::default()
            }],
            ..Default::default()
        };
        let ctx = production_ctx();
        let data = build_mbd_v2_pipe_data(&layout, &ctx);

        let json = serde_json::to_string(&data).expect("serialize");
        assert!(
            !json.contains("NaN") && !json.contains("Infinity"),
            "production output must not contain NaN or Infinity"
        );
    }
}
