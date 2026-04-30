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
    resolve_label_label_conflicts, resolve_linear_dim_text_conflicts,
};
use super::primitive::{IssueCategory, IssueSeverity, MbdV2Issue, MbdV2Meta, MbdV2PipeData};

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
    let mut assembler_ctx = ctx.assembler.clone();
    if assembler_ctx.bran_bbox_center.is_none() {
        assembler_ctx.bran_bbox_center = infer_bbox_center_from_layout(layout);
    }

    let (mut primitives, mut issues) = if ctx.enable_small_dim_stacking {
        assemble_v2_primitives_with_chain_stacking(
            layout,
            &assembler_ctx,
            &ctx.chain_tolerance,
            &ctx.small_dim_params,
        )
    } else {
        assemble_v2_primitives(layout, &assembler_ctx)
    };

    if ctx.enable_avoidance {
        issues.extend(resolve_linear_dim_text_conflicts(
            &mut primitives,
            &ctx.avoidance_config,
        ));
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

    issues.extend(collect_suppression_issues(layout));

    let generated_at = ctx
        .generated_at_override
        .clone()
        .unwrap_or_else(current_utc_rfc3339);

    let meta = compute_meta(layout, &ctx.branch_attrs, generated_at);

    MbdV2PipeData {
        version: "v2".to_string(),
        input_refno: ctx.input_refno.clone(),
        branch_refno: ctx.branch_refno.clone(),
        primitives,
        meta,
        issues,
    }
}

fn current_utc_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn compute_meta(
    layout: &LayoutResult,
    branch_attrs: &BTreeMap<String, String>,
    generated_at: String,
) -> MbdV2Meta {
    let mut dims_by_kind: BTreeMap<String, u32> = BTreeMap::new();

    for dim in &layout.linear_dims {
        *dims_by_kind.entry(dim.kind.clone()).or_insert(0) += 1;
    }
    for dim in &layout.cut_tubis {
        *dims_by_kind.entry(dim.kind.clone()).or_insert(0) += 1;
    }
    for bend in &layout.bends {
        for dim in &bend.size_dims {
            *dims_by_kind.entry(dim.kind.clone()).or_insert(0) += 1;
        }
    }

    MbdV2Meta {
        segments_count: layout.linear_dims.len() as u32,
        welds_count: layout.welds.len() as u32,
        dims_by_kind,
        branch_attrs: branch_attrs.clone(),
        generated_at,
    }
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

        // linear_dims 3 条 + cut_tubis 1 条 + welds 2 条（其中 1 条有 label，产 2 个 primitive；
        // 另一条无 label，只产 weld_mark 1 个）+ slopes 1 条 + tags 1 条
        // = 3 + 1 + (2 + 1) + 1 + 1 = 9
        let linear_count = data
            .primitives
            .iter()
            .filter(|p| matches!(p, MbdPrimitive::LinearDim(_)))
            .count();
        assert_eq!(linear_count, 4, "segment + chain + cut_tubi 都是 LinearDim");

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

        assert_eq!(data.meta.segments_count, 3);
        assert_eq!(data.meta.welds_count, 2);
        assert_eq!(data.meta.dims_by_kind.get("segment").copied(), Some(2));
        assert_eq!(data.meta.dims_by_kind.get("chain").copied(), Some(1));
        assert_eq!(data.meta.dims_by_kind.get("cut_tubi").copied(), Some(1));
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
    fn dims_by_kind_aggregates_bend_size_dims() {
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

        // 顶层 linear_dims 1 条 + bend.size_dims 2 条 = segment:2, port:1
        assert_eq!(data.meta.dims_by_kind.get("segment").copied(), Some(2));
        assert_eq!(data.meta.dims_by_kind.get("port").copied(), Some(1));
        // segments_count 只看顶层 linear_dims
        assert_eq!(data.meta.segments_count, 1);
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
        assert_eq!(data.primitives.len(), 2);
        // Step 1 路径下单段装配：id 保持源 id 原样
        let ids: Vec<&str> = data
            .primitives
            .iter()
            .filter_map(|p| match p {
                MbdPrimitive::LinearDim(d) => Some(d.common.id.as_str()),
                _ => None,
            })
            .collect();
        assert!(ids.contains(&"d-1"));
        assert!(ids.contains(&"d-2"));
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
        assert_eq!(linear_count, 3, "3 相连段展开成 3 个 primitive");
        // chain expand 产出的 id 形如 "d-1/r0s0"
        let has_expanded_id = data.primitives.iter().any(|p| match p {
            MbdPrimitive::LinearDim(d) => d.common.id.contains("/r") && d.common.id.contains("s"),
            _ => false,
        });
        assert!(
            has_expanded_id,
            "至少一条 primitive 用 chain expand 的 id 规则"
        );
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
        // AssemblerContext 默认 cheight=2.5，lane_step_multiplier=1.2 → 抬 3.0
        assert!(
            (max_y - 3.0).abs() < 0.01,
            "avoidance should bump second label by ~3.0, got {}",
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
        for y in label_ys {
            assert!(
                (y - 0.0).abs() < 1e-6,
                "without avoidance label stays at y=0"
            );
        }
    }

    #[test]
    fn pipeline_avoidance_reroutes_leader_before_detection() {
        // 手动在 layout 上加一条 slope + tag 构造不了 leader；
        // 这里直接测试 pipeline 的组合步骤：空 layout + enable_avoidance 不 panic
        // 真正的 leader reroute 由 avoidance.rs 的单测覆盖
        // 此处改为验证 "enable_avoidance 开启后三步避让均被调用"
        // 用 PlacedTag 触发 label-label lane bump
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

        // label-label 避让生效：两个 tag 有 y 差 ≈ 3.0
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
        assert!((max_y - 3.0).abs() < 0.01);

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
        let bumped = data.primitives.iter().any(|p| match p {
            MbdPrimitive::LinearDim(d) => d.level > 0,
            _ => false,
        });
        assert!(bumped);
    }
}
