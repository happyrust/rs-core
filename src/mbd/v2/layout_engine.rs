//! MBD V2 直算引擎 — 从 `BranchQueryResult` 直接产出 `Vec<MbdPrimitive>`。
//!
//! 替代 assembler.rs 的 V1→V2 翻译路径。不依赖 V1 `LayoutResult` 或任何
//! `Placed*` 中间类型，直接消费 `data_source` 定义的 V2 数据结构。
//!
//! 内部调用链：
//! ```text
//! BranchQueryResult
//!   ├─ compute_segment_dims   → Vec<LinearDimPrimitive>
//!   ├─ compute_port_dims      → Vec<LinearDimPrimitive>
//!   ├─ compute_weld_marks     → Vec<MbdPrimitive> (WeldMark + Label + LeaderLine)
//!   ├─ compute_slope_marks    → Vec<MbdPrimitive> (SlopeMark + AidLine + AidText)
//!   ├─ compute_bend_marks     → Vec<MbdPrimitive> (AngleDim + AidLine + AidArc)
//!   ├─ compute_tag_labels     → Vec<MbdPrimitive> (Label + LeaderLine)
//!   ├─ avoidance              (复用 avoidance.rs)
//!   └─ MbdV2PipeData
//! ```

use super::avoidance::{
    AvoidanceConfig, detect_leader_line_label_conflicts, reroute_leader_lines_around_labels,
    resolve_label_label_conflicts, resolve_linear_dim_text_conflicts,
};
use super::data_source::{BendData, BranchQueryResult, SlopeData, TagData, WeldData};
use super::dim_direction::resolve_dim_direction;
use super::leader_router::route_leader_line;
use super::primitive::*;
use super::text_measurement::mbd_text_width;
use super::used_dir::UsedDirRegistry;

/// 直算引擎的配置上下文。
#[derive(Debug, Clone)]
pub struct LayoutEngineContext {
    pub cheight: f32,
    pub pipe_od: f32,
    pub bbox_center: Option<Vec3V2>,
    pub default_orientation: Vec3V2,
    pub default_up: Vec3V2,
    pub arrow_len: f32,
    pub lane_step_multiplier: f32,
    pub enable_avoidance: bool,
    pub avoidance_config: AvoidanceConfig,
}

impl Default for LayoutEngineContext {
    fn default() -> Self {
        Self {
            cheight: 2.5,
            pipe_od: 229.0,
            bbox_center: None,
            default_orientation: [1.0, 0.0, 0.0],
            default_up: [0.0, 0.0, 1.0],
            arrow_len: 5.0,
            lane_step_multiplier: 1.2,
            enable_avoidance: false,
            avoidance_config: AvoidanceConfig::default(),
        }
    }
}

impl LayoutEngineContext {
    pub fn production_defaults() -> Self {
        Self {
            cheight: 100.0,
            enable_avoidance: true,
            ..Self::default()
        }
    }

    fn base_offset(&self) -> f32 {
        self.pipe_od * 0.5 + self.cheight
    }
}

/// 直算主入口：从 `BranchQueryResult` 产出完整的 V2 primitive 列表。
pub fn compute_v2_primitives(
    qr: &BranchQueryResult,
    ctx: &LayoutEngineContext,
) -> (Vec<MbdPrimitive>, Vec<MbdV2Issue>) {
    let mut primitives = Vec::new();
    let mut issues = Vec::new();
    let mut id_counter = 0u32;

    let mut next_id = |prefix: &str| -> String {
        id_counter += 1;
        format!("{prefix}-{id_counter}")
    };

    let mut used_dirs = UsedDirRegistry::new();

    compute_segment_dims(
        qr,
        ctx,
        &mut next_id,
        &mut used_dirs,
        &mut primitives,
    );
    compute_port_dims(qr, ctx, &mut next_id, &mut primitives);
    compute_weld_marks(&qr.welds, ctx, &mut next_id, &mut primitives);
    compute_slope_marks(&qr.slopes, ctx, &mut next_id, &mut primitives);
    compute_bend_marks(
        &qr.bends,
        ctx,
        &mut next_id,
        &mut primitives,
        &mut issues,
    );
    compute_tag_labels(&qr.tags, ctx, &mut next_id, &mut primitives);

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

    (primitives, issues)
}

// ── 管段标注 ──────────────────────────────────────────────────────

fn compute_segment_dims(
    qr: &BranchQueryResult,
    ctx: &LayoutEngineContext,
    next_id: &mut dyn FnMut(&str) -> String,
    used_dirs: &mut UsedDirRegistry,
    out: &mut Vec<MbdPrimitive>,
) {
    let base_offset = ctx.base_offset();

    for m in &qr.members {
        let start = m.start;
        let end = m.end;
        let pipe_vec = sub_v3(end, start);
        let seg_len = length(pipe_vec);
        if seg_len < 1e-3 {
            continue;
        }

        let pipe_dir = scale_v3(pipe_vec, 1.0 / seg_len);
        let midpoint = mid_v3(start, end);

        let dim_result = resolve_dim_direction(pipe_dir, midpoint, ctx.bbox_center);
        let offset_dir = dim_result.dim_dir;

        let (text_ori, text_up) =
            compute_text_frame(start, end, offset_dir, ctx);

        let ext1_end = add_scaled_v3(start, offset_dir, base_offset);
        let ext2_end = add_scaled_v3(end, offset_dir, base_offset);
        let dim_line_start = ext1_end;
        let dim_line_end = ext2_end;
        let text_anchor = add_scaled_v3(midpoint, offset_dir, base_offset);

        let prim = MbdPrimitive::LinearDim(LinearDimPrimitive {
            common: CommonFields {
                id: next_id("seg"),
                visible: true,
                source_refno: Some(m.refno.clone()),
                ..CommonFields::default()
            },
            sub_kind: LinearDimSubKind::Segment,
            extension_1: LineSegmentEndpoints {
                start,
                end: ext1_end,
            },
            extension_2: LineSegmentEndpoints {
                start: end,
                end: ext2_end,
            },
            dim_line: LineSegmentEndpoints {
                start: dim_line_start,
                end: dim_line_end,
            },
            arrows: [
                LinearDimArrow {
                    position: dim_line_start,
                    direction: pipe_dir,
                },
                LinearDimArrow {
                    position: dim_line_end,
                    direction: negate(pipe_dir),
                },
            ],
            text: TextBlock {
                anchor: text_anchor,
                content: format!("{}", seg_len.round() as i64),
                height_mm: ctx.cheight,
                orientation: text_ori,
                up: text_up,
            },
            level: 0,
        });

        register_used_dir(&prim, used_dirs);
        out.push(prim);
    }
}

// ── 端口标注 ──────────────────────────────────────────────────────

fn compute_port_dims(
    qr: &BranchQueryResult,
    ctx: &LayoutEngineContext,
    next_id: &mut dyn FnMut(&str) -> String,
    out: &mut Vec<MbdPrimitive>,
) {
    let base_offset = ctx.base_offset();

    for m in &qr.members {
        let (arrive, leave) = match (m.arrive_axis, m.leave_axis) {
            (Some(a), Some(l)) => (a, l),
            _ => continue,
        };

        let dv = sub_v3(leave, arrive);
        let port_len = length(dv);
        if port_len < 1e-3 {
            continue;
        }

        let pipe_dir = scale_v3(dv, 1.0 / port_len);
        let midpoint = mid_v3(arrive, leave);

        let dim_result = resolve_dim_direction(pipe_dir, midpoint, ctx.bbox_center);
        let offset_dir = dim_result.dim_dir;

        let (text_ori, text_up) =
            compute_text_frame(arrive, leave, offset_dir, ctx);

        let ext1_end = add_scaled_v3(arrive, offset_dir, base_offset);
        let ext2_end = add_scaled_v3(leave, offset_dir, base_offset);
        let text_anchor = add_scaled_v3(midpoint, offset_dir, base_offset);

        out.push(MbdPrimitive::LinearDim(LinearDimPrimitive {
            common: CommonFields {
                id: next_id("port"),
                visible: true,
                source_refno: Some(m.refno.clone()),
                ..CommonFields::default()
            },
            sub_kind: LinearDimSubKind::Port,
            extension_1: LineSegmentEndpoints {
                start: arrive,
                end: ext1_end,
            },
            extension_2: LineSegmentEndpoints {
                start: leave,
                end: ext2_end,
            },
            dim_line: LineSegmentEndpoints {
                start: ext1_end,
                end: ext2_end,
            },
            arrows: [
                LinearDimArrow {
                    position: ext1_end,
                    direction: pipe_dir,
                },
                LinearDimArrow {
                    position: ext2_end,
                    direction: negate(pipe_dir),
                },
            ],
            text: TextBlock {
                anchor: text_anchor,
                content: format!("{}", port_len.round() as i64),
                height_mm: ctx.cheight,
                orientation: text_ori,
                up: text_up,
            },
            level: 0,
        }));
    }
}

// ── 焊缝标注 ──────────────────────────────────────────────────────

fn compute_weld_marks(
    welds: &[WeldData],
    ctx: &LayoutEngineContext,
    next_id: &mut dyn FnMut(&str) -> String,
    out: &mut Vec<MbdPrimitive>,
) {
    let orientation = normalize_or(ctx.default_orientation, [1.0, 0.0, 0.0]);
    let up = orthogonal_up(orientation, ctx.default_up);
    let label_offset = ctx.base_offset();

    for w in welds {
        let weld_id = next_id("weld");
        let mut linked_label_id: Option<String> = None;

        if !w.label.is_empty() {
            let label_id = next_id("weld-lbl");
            linked_label_id = Some(label_id.clone());

            let label_pos = add_scaled_v3(w.position, up, label_offset);
            let label = LabelPrimitive {
                common: CommonFields {
                    id: label_id,
                    visible: true,
                    function: Some("焊".to_string()),
                    ..CommonFields::default()
                },
                anchor: w.position,
                text_anchor: label_pos,
                content: w.label.clone(),
                height_mm: ctx.cheight,
                orientation,
                up,
                box_shape: LabelBoxShape::None,
                box_padding_mm: 0.0,
            };

            if let Some(leader) = build_leader_for_label(
                &label,
                w.position,
                "weld-leader",
                "焊",
                next_id,
            ) {
                out.push(leader);
            }
            out.push(MbdPrimitive::Label(label));
        }

        out.push(MbdPrimitive::WeldMark(WeldMarkPrimitive {
            common: CommonFields {
                id: weld_id,
                visible: true,
                function: Some("焊".to_string()),
                ..CommonFields::default()
            },
            position: w.position,
            cross_size_mm: 80.0,
            weld_type: if w.is_shop {
                WeldType::Shop
            } else {
                WeldType::Field
            },
            linked_label_id,
        }));
    }
}

// ── 坡度标注 ──────────────────────────────────────────────────────

fn compute_slope_marks(
    slopes: &[SlopeData],
    ctx: &LayoutEngineContext,
    next_id: &mut dyn FnMut(&str) -> String,
    out: &mut Vec<MbdPrimitive>,
) {
    for s in slopes {
        let mid = mid_v3(s.start, s.end);

        out.push(MbdPrimitive::SlopeMark(SlopeMarkPrimitive {
            common: CommonFields {
                id: next_id("slope"),
                visible: true,
                function: Some("坡度".to_string()),
                ..CommonFields::default()
            },
            start: s.start,
            end: s.end,
            slope: s.slope,
            text: TextBlock {
                anchor: mid,
                content: s.text.clone(),
                height_mm: ctx.cheight,
                orientation: ctx.default_orientation,
                up: ctx.default_up,
            },
        }));

        emit_slope_aid_lines(s, ctx, next_id, out);
    }
}

fn emit_slope_aid_lines(
    slope: &SlopeData,
    ctx: &LayoutEngineContext,
    next_id: &mut dyn FnMut(&str) -> String,
    out: &mut Vec<MbdPrimitive>,
) {
    let height_diff = (slope.start[2] - slope.end[2]).abs();
    if height_diff <= 0.5 {
        return;
    }

    let (high, low) = if slope.start[2] >= slope.end[2] {
        (slope.start, slope.end)
    } else {
        (slope.end, slope.start)
    };
    let projected = [high[0], high[1], low[2]];

    let vert_len = (high[2] - low[2]).abs();
    if vert_len > 0.5 {
        out.push(MbdPrimitive::AidLine(AidLinePrimitive {
            common: CommonFields {
                id: next_id("slope-vert"),
                visible: true,
                function: Some("尺寸".to_string()),
                ..CommonFields::default()
            },
            points: vec![projected, high],
            style: AidLineStyle::Solid,
        }));
        out.push(MbdPrimitive::AidText(AidTextPrimitive {
            common: CommonFields {
                id: next_id("slope-vert-text"),
                visible: true,
                function: Some("尺寸".to_string()),
                ..CommonFields::default()
            },
            position: mid_v3(projected, high),
            content: format!("{}", vert_len.round() as i32),
            height_mm: ctx.cheight,
            orientation: ctx.default_orientation,
            up: ctx.default_up,
        }));
    }

    let horiz_len = distance(projected, low);
    if horiz_len > 0.5 {
        out.push(MbdPrimitive::AidLine(AidLinePrimitive {
            common: CommonFields {
                id: next_id("slope-horiz"),
                visible: true,
                function: Some("尺寸".to_string()),
                ..CommonFields::default()
            },
            points: vec![projected, low],
            style: AidLineStyle::Solid,
        }));
        out.push(MbdPrimitive::AidText(AidTextPrimitive {
            common: CommonFields {
                id: next_id("slope-horiz-text"),
                visible: true,
                function: Some("尺寸".to_string()),
                ..CommonFields::default()
            },
            position: mid_v3(projected, low),
            content: format!("{}", horiz_len.round() as i32),
            height_mm: ctx.cheight,
            orientation: ctx.default_orientation,
            up: ctx.default_up,
        }));
    }

    if vert_len > 0.5 && horiz_len > 0.5 {
        let corner_size = (vert_len.min(horiz_len) / 4.0).min(ctx.cheight);
        let horiz_dir = normalize(sub_v3(low, projected));
        let vert_dir = normalize(sub_v3(high, projected));
        let corner_pt = add_scaled_v3(
            add_scaled_v3(projected, horiz_dir, corner_size),
            vert_dir,
            corner_size,
        );
        out.push(MbdPrimitive::AidLine(AidLinePrimitive {
            common: CommonFields {
                id: next_id("slope-ra-h"),
                visible: true,
                function: Some("尺寸".to_string()),
                ..CommonFields::default()
            },
            points: vec![corner_pt, add_scaled_v3(corner_pt, negate(horiz_dir), corner_size)],
            style: AidLineStyle::Solid,
        }));
        out.push(MbdPrimitive::AidLine(AidLinePrimitive {
            common: CommonFields {
                id: next_id("slope-ra-v"),
                visible: true,
                function: Some("尺寸".to_string()),
                ..CommonFields::default()
            },
            points: vec![corner_pt, add_scaled_v3(corner_pt, negate(vert_dir), corner_size)],
            style: AidLineStyle::Solid,
        }));
    }
}

// ── 弯头标注 ──────────────────────────────────────────────────────

fn compute_bend_marks(
    bends: &[BendData],
    ctx: &LayoutEngineContext,
    next_id: &mut dyn FnMut(&str) -> String,
    out: &mut Vec<MbdPrimitive>,
    _issues: &mut Vec<MbdV2Issue>,
) {
    for b in bends {
        let ray_1 = normalize(b.ray_1);
        let ray_2 = normalize(b.ray_2);

        let normal = {
            let n = cross(ray_1, ray_2);
            if length(n) > 1e-6 { normalize(n) } else { ctx.default_up }
        };

        let start_angle_rad = ray_1[1].atan2(ray_1[0]);
        let sweep_rad = dot(ray_1, ray_2).clamp(-1.0, 1.0).acos();
        let arc_radius = b.outside_diameter.unwrap_or(ctx.pipe_od) * 0.5 + ctx.cheight;

        let arc_mid_angle = start_angle_rad + sweep_rad * 0.5;
        let text_pos = [
            b.vertex[0] + arc_radius * arc_mid_angle.cos(),
            b.vertex[1] + arc_radius * arc_mid_angle.sin(),
            b.vertex[2],
        ];

        let tangent_1 = cross(normal, ray_1);
        let tangent_2 = cross(ray_2, normal);

        let arrow1_pos = add_scaled_v3(b.vertex, ray_1, arc_radius);
        let arrow2_pos = add_scaled_v3(b.vertex, ray_2, arc_radius);

        out.push(MbdPrimitive::AngleDim(AngleDimPrimitive {
            common: CommonFields {
                id: next_id("angle"),
                visible: true,
                function: Some("角度".to_string()),
                ..CommonFields::default()
            },
            vertex: b.vertex,
            ray_1,
            ray_2,
            arc: ArcGeometry {
                center: b.vertex,
                radius_mm: arc_radius,
                start_angle_rad,
                sweep_rad,
                normal,
            },
            arrows: [
                AngleDimArrow { position: arrow1_pos, tangent: tangent_1 },
                AngleDimArrow { position: arrow2_pos, tangent: tangent_2 },
            ],
            text: TextBlock {
                anchor: text_pos,
                content: format!("{}°", b.angle_deg.round() as i32),
                height_mm: ctx.cheight,
                orientation: ctx.default_orientation,
                up: ctx.default_up,
            },
        }));

        out.push(MbdPrimitive::AidLine(AidLinePrimitive {
            common: CommonFields {
                id: next_id("bend-ray1"),
                visible: true,
                function: Some("弯头参考线".to_string()),
                ..CommonFields::default()
            },
            points: vec![b.vertex, arrow1_pos],
            style: AidLineStyle::Dashed,
        }));
        out.push(MbdPrimitive::AidLine(AidLinePrimitive {
            common: CommonFields {
                id: next_id("bend-ray2"),
                visible: true,
                function: Some("弯头参考线".to_string()),
                ..CommonFields::default()
            },
            points: vec![b.vertex, arrow2_pos],
            style: AidLineStyle::Dashed,
        }));

        out.push(MbdPrimitive::AidArc(AidArcPrimitive {
            common: CommonFields {
                id: next_id("bend-arc"),
                visible: true,
                function: Some("弯头弧线".to_string()),
                ..CommonFields::default()
            },
            center: b.vertex,
            radius_mm: arc_radius,
            start_angle_rad,
            sweep_rad,
            normal,
        }));
    }
}

// ── 标签标注 ──────────────────────────────────────────────────────

fn compute_tag_labels(
    tags: &[TagData],
    ctx: &LayoutEngineContext,
    next_id: &mut dyn FnMut(&str) -> String,
    out: &mut Vec<MbdPrimitive>,
) {
    let orientation = normalize_or(ctx.default_orientation, [1.0, 0.0, 0.0]);
    let up = orthogonal_up(orientation, ctx.default_up);
    let label_offset = ctx.base_offset();

    for t in tags {
        let text_pos = add_scaled_v3(t.position, up, label_offset);
        let label = LabelPrimitive {
            common: CommonFields {
                id: next_id("tag"),
                visible: true,
                function: Some("标签".to_string()),
                ..CommonFields::default()
            },
            anchor: t.position,
            text_anchor: text_pos,
            content: t.text.clone(),
            height_mm: ctx.cheight,
            orientation,
            up,
            box_shape: LabelBoxShape::Rect,
            box_padding_mm: 1.0,
        };

        if let Some(leader) = build_leader_for_label(
            &label,
            t.position,
            "tag-leader",
            "标签",
            next_id,
        ) {
            out.push(leader);
        }
        out.push(MbdPrimitive::Label(label));
    }
}

// ── 辅助函数 ──────────────────────────────────────────────────────

fn build_leader_for_label(
    label: &LabelPrimitive,
    anchor: Vec3V2,
    id_prefix: &str,
    function_name: &str,
    next_id: &mut dyn FnMut(&str) -> String,
) -> Option<MbdPrimitive> {
    if distance_sq(anchor, label.text_anchor) <= 1e-6 {
        return None;
    }
    let padding = label.box_padding_mm.max(0.0);
    let width = mbd_text_width(&label.content, label.height_mm) + padding * 2.0;
    let height = label.height_mm + padding * 2.0;
    let text_anchor = [
        label.text_anchor[0] - label.orientation[0] * padding - label.up[0] * padding,
        label.text_anchor[1] - label.orientation[1] * padding - label.up[1] * padding,
        label.text_anchor[2] - label.orientation[2] * padding - label.up[2] * padding,
    ];
    let points = route_leader_line(
        anchor,
        text_anchor,
        width,
        height,
        label.orientation,
        label.up,
    );
    if points.len() < 2 || distance_sq(points[0], points[1]) <= 1e-6 {
        return None;
    }
    Some(MbdPrimitive::LeaderLine(LeaderLinePrimitive {
        common: CommonFields {
            id: next_id(id_prefix),
            visible: true,
            function: Some(function_name.to_string()),
            source_refno: Some(label.common.id.clone()),
            ..CommonFields::default()
        },
        points,
        arrow_at: LeaderArrowAt::None,
    }))
}

fn compute_text_frame(
    start: Vec3V2,
    end: Vec3V2,
    dim_dir: Vec3V2,
    ctx: &LayoutEngineContext,
) -> (Vec3V2, Vec3V2) {
    let pipe_vec = sub_v3(end, start);
    let pipe_len_sq = dot_v3(pipe_vec, pipe_vec);

    if pipe_len_sq > 1e-6 {
        let pipedir = normalize_or(pipe_vec, ctx.default_orientation);
        let segment_mid = mid_v3(start, end);

        if let Some(center) = ctx.bbox_center {
            let result = resolve_dim_direction(pipedir, segment_mid, Some(center));
            return (result.text_orientation, result.text_up);
        }
    }

    let orientation = normalize_or(sub_v3(end, start), ctx.default_orientation);
    let up = orthogonal_up(orientation, normalize_or(dim_dir, ctx.default_up));
    (orientation, up)
}

fn register_used_dir(prim: &MbdPrimitive, registry: &mut UsedDirRegistry) {
    if let MbdPrimitive::LinearDim(dim) = prim {
        let dir = dim.text.orientation;
        let start_proj = dot_v3(dim.extension_1.start, dir);
        let end_proj = dot_v3(dim.extension_2.start, dir);
        let (min_proj, max_proj) = if start_proj <= end_proj {
            (start_proj, end_proj)
        } else {
            (end_proj, start_proj)
        };
        registry.register(super::used_dir::IsoUsedDir::new(
            &dim.common.id,
            dir,
            min_proj,
            max_proj,
            "MainDim",
        ));
    }
}

// ── 向量工具 ──────────────────────────────────────────────────────

fn mid_v3(a: Vec3V2, b: Vec3V2) -> Vec3V2 {
    [(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5, (a[2] + b[2]) * 0.5]
}

fn sub_v3(a: Vec3V2, b: Vec3V2) -> Vec3V2 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot_v3(a: Vec3V2, b: Vec3V2) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn add_scaled_v3(base: Vec3V2, dir: Vec3V2, scale: f32) -> Vec3V2 {
    [
        base[0] + dir[0] * scale,
        base[1] + dir[1] * scale,
        base[2] + dir[2] * scale,
    ]
}

fn scale_v3(v: Vec3V2, s: f32) -> Vec3V2 {
    [v[0] * s, v[1] * s, v[2] * s]
}

fn negate(v: Vec3V2) -> Vec3V2 {
    [-v[0], -v[1], -v[2]]
}

fn length(v: Vec3V2) -> f32 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

fn normalize(v: Vec3V2) -> Vec3V2 {
    let len = length(v);
    if len < 1e-10 { [0.0, 0.0, 0.0] } else { [v[0] / len, v[1] / len, v[2] / len] }
}

fn normalize_or(v: Vec3V2, fallback: Vec3V2) -> Vec3V2 {
    let n = normalize(v);
    if length(n) > 1e-6 {
        n
    } else {
        let fb = normalize(fallback);
        if length(fb) > 1e-6 { fb } else { [1.0, 0.0, 0.0] }
    }
}

fn dot(a: Vec3V2, b: Vec3V2) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: Vec3V2, b: Vec3V2) -> Vec3V2 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn distance(a: Vec3V2, b: Vec3V2) -> f32 {
    length(sub_v3(a, b))
}

fn distance_sq(a: Vec3V2, b: Vec3V2) -> f32 {
    let d = sub_v3(a, b);
    dot_v3(d, d)
}

fn orthogonal_up(orientation: Vec3V2, up: Vec3V2) -> Vec3V2 {
    let orientation = normalize_or(orientation, [1.0, 0.0, 0.0]);
    let up = normalize_or(up, [0.0, 1.0, 0.0]);
    let projected = sub_v3(up, scale_v3(orientation, dot(up, orientation)));
    if length(projected) > 1e-6 {
        return normalize(projected);
    }
    let candidates = [[0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]];
    for candidate in candidates {
        let projected = sub_v3(candidate, scale_v3(orientation, dot(candidate, orientation)));
        if length(projected) > 1e-6 {
            return normalize(projected);
        }
    }
    [0.0, 1.0, 0.0]
}

// ── 测试 ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mbd::v2::data_source::*;

    fn simple_straight_branch() -> BranchQueryResult {
        let mut qr = BranchQueryResult {
            members: vec![
                BranchMember {
                    refno: "seg-1".into(),
                    owner_refno: "=BRAN/FOO".into(),
                    start: [0.0, 0.0, 0.0],
                    end: [1000.0, 0.0, 0.0],
                    order: 0,
                    outside_diameter: Some(168.3),
                    ..Default::default()
                },
                BranchMember {
                    refno: "seg-2".into(),
                    owner_refno: "=BRAN/FOO".into(),
                    start: [1000.0, 0.0, 0.0],
                    end: [2500.0, 0.0, 0.0],
                    order: 1,
                    outside_diameter: Some(168.3),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        qr.compute_bbox_center();
        qr
    }

    #[test]
    fn segment_dims_basic_straight_pipe() {
        let qr = simple_straight_branch();
        let ctx = LayoutEngineContext {
            pipe_od: 168.3,
            ..LayoutEngineContext::default()
        };
        let (prims, issues) = compute_v2_primitives(&qr, &ctx);

        assert!(issues.is_empty());
        let dims: Vec<_> = prims
            .iter()
            .filter(|p| matches!(p, MbdPrimitive::LinearDim(_)))
            .collect();
        assert_eq!(dims.len(), 2);

        if let MbdPrimitive::LinearDim(d) = &dims[0] {
            assert_eq!(d.sub_kind, LinearDimSubKind::Segment);
            assert_eq!(d.text.content, "1000");
            assert!(d.text.height_mm > 0.0);
        }
        if let MbdPrimitive::LinearDim(d) = &dims[1] {
            assert_eq!(d.text.content, "1500");
        }
    }

    #[test]
    fn port_dims_from_axis_points() {
        let qr = BranchQueryResult {
            members: vec![BranchMember {
                refno: "seg-1".into(),
                owner_refno: "=BRAN/FOO".into(),
                start: [0.0, 0.0, 0.0],
                end: [1000.0, 0.0, 0.0],
                arrive_axis: Some([50.0, 0.0, 0.0]),
                leave_axis: Some([950.0, 0.0, 0.0]),
                outside_diameter: Some(168.3),
                ..Default::default()
            }],
            ..Default::default()
        };
        let ctx = LayoutEngineContext::default();
        let (prims, _) = compute_v2_primitives(&qr, &ctx);

        let port_dims: Vec<_> = prims.iter().filter(|p| {
            matches!(p, MbdPrimitive::LinearDim(d) if d.sub_kind == LinearDimSubKind::Port)
        }).collect();
        assert_eq!(port_dims.len(), 1);
        if let MbdPrimitive::LinearDim(d) = &port_dims[0] {
            assert_eq!(d.text.content, "900");
        }
    }

    #[test]
    fn weld_marks_produce_weld_label_leader() {
        let qr = BranchQueryResult {
            welds: vec![WeldData {
                id: "w1".into(),
                position: [500.0, 0.0, 0.0],
                is_shop: true,
                label: "A1".into(),
                left_refno: "seg-1".into(),
                right_refno: "seg-2".into(),
            }],
            ..Default::default()
        };
        let ctx = LayoutEngineContext::default();
        let (prims, _) = compute_v2_primitives(&qr, &ctx);

        assert!(prims.iter().any(|p| matches!(p, MbdPrimitive::WeldMark(_))));
        assert!(prims.iter().any(|p| matches!(p, MbdPrimitive::Label(_))));
        assert!(prims.iter().any(|p| matches!(p, MbdPrimitive::LeaderLine(_))));
    }

    #[test]
    fn slope_marks_with_aid_lines() {
        let qr = BranchQueryResult {
            slopes: vec![SlopeData {
                id: "s1".into(),
                start: [0.0, 0.0, 100.0],
                end: [1000.0, 0.0, 0.0],
                slope: -0.1,
                text: "1:10".into(),
            }],
            ..Default::default()
        };
        let ctx = LayoutEngineContext::default();
        let (prims, _) = compute_v2_primitives(&qr, &ctx);

        assert!(prims.iter().any(|p| matches!(p, MbdPrimitive::SlopeMark(_))));
        let aid_lines: Vec<_> = prims.iter().filter(|p| matches!(p, MbdPrimitive::AidLine(_))).collect();
        assert!(aid_lines.len() >= 2, "expect vert+horiz aid lines");
    }

    #[test]
    fn bend_marks_angle_dim_and_aids() {
        let qr = BranchQueryResult {
            bends: vec![BendData {
                id: "b1".into(),
                vertex: [1000.0, 0.0, 0.0],
                angle_deg: 90.0,
                ray_1: [1.0, 0.0, 0.0],
                ray_2: [0.0, 1.0, 0.0],
                outside_diameter: Some(168.3),
            }],
            ..Default::default()
        };
        let ctx = LayoutEngineContext::default();
        let (prims, _) = compute_v2_primitives(&qr, &ctx);

        assert!(prims.iter().any(|p| matches!(p, MbdPrimitive::AngleDim(_))));
        let aid_lines: Vec<_> = prims.iter().filter(|p| matches!(p, MbdPrimitive::AidLine(_))).collect();
        assert_eq!(aid_lines.len(), 2, "expect 2 ray aid lines");
        assert!(prims.iter().any(|p| matches!(p, MbdPrimitive::AidArc(_))));
    }

    #[test]
    fn tag_labels_with_leader() {
        let qr = BranchQueryResult {
            tags: vec![TagData {
                id: "t1".into(),
                text: "VALVE-001".into(),
                position: [500.0, 0.0, 0.0],
                noun: "VALV".into(),
            }],
            ..Default::default()
        };
        let ctx = LayoutEngineContext::default();
        let (prims, _) = compute_v2_primitives(&qr, &ctx);

        assert!(prims.iter().any(|p| matches!(p, MbdPrimitive::Label(l) if l.content == "VALVE-001")));
        assert!(prims.iter().any(|p| matches!(p, MbdPrimitive::LeaderLine(_))));
    }

    #[test]
    fn empty_query_result_no_panic() {
        let qr = BranchQueryResult::default();
        let ctx = LayoutEngineContext::default();
        let (prims, issues) = compute_v2_primitives(&qr, &ctx);
        assert!(prims.is_empty());
        assert!(issues.is_empty());
    }

    #[test]
    fn mixed_annotations_full_pipeline() {
        let mut qr = BranchQueryResult {
            members: vec![
                BranchMember {
                    refno: "seg-1".into(),
                    start: [0.0, 0.0, 0.0],
                    end: [1000.0, 0.0, 0.0],
                    outside_diameter: Some(114.3),
                    ..Default::default()
                },
            ],
            welds: vec![WeldData {
                id: "w1".into(),
                position: [500.0, 0.0, 0.0],
                is_shop: false,
                label: "M1".into(),
                left_refno: String::new(),
                right_refno: String::new(),
            }],
            slopes: vec![SlopeData {
                id: "s1".into(),
                start: [0.0, 0.0, 10.0],
                end: [1000.0, 0.0, 0.0],
                slope: -0.01,
                text: "1:100".into(),
            }],
            tags: vec![TagData {
                id: "t1".into(),
                text: "TEE-A".into(),
                position: [800.0, 0.0, 0.0],
                noun: "TEE".into(),
            }],
            bends: vec![],
            ..Default::default()
        };
        qr.compute_bbox_center();

        let ctx = LayoutEngineContext {
            pipe_od: 114.3,
            ..LayoutEngineContext::default()
        };
        let (prims, issues) = compute_v2_primitives(&qr, &ctx);

        assert!(prims.len() >= 5, "expect at least seg_dim + weld + label + slope + tag");
        assert!(issues.is_empty());

        assert!(prims.iter().any(|p| matches!(p, MbdPrimitive::LinearDim(_))));
        assert!(prims.iter().any(|p| matches!(p, MbdPrimitive::WeldMark(_))));
        assert!(prims.iter().any(|p| matches!(p, MbdPrimitive::Label(_))));
        assert!(prims.iter().any(|p| matches!(p, MbdPrimitive::SlopeMark(_))));
    }

    #[test]
    fn no_nan_or_infinity_in_output() {
        let mut qr = simple_straight_branch();
        qr.welds.push(WeldData {
            id: "w1".into(),
            position: [1000.0, 0.0, 0.0],
            is_shop: true,
            label: "A1".into(),
            left_refno: String::new(),
            right_refno: String::new(),
        });
        let ctx = LayoutEngineContext::default();
        let (prims, _) = compute_v2_primitives(&qr, &ctx);

        let json = serde_json::to_string(&prims).unwrap();
        assert!(!json.contains("NaN"), "output must not contain NaN");
        assert!(!json.contains("Infinity"), "output must not contain Infinity");
    }

    #[test]
    fn production_defaults_avoidance_runs() {
        let mut qr = simple_straight_branch();
        qr.tags.push(TagData {
            id: "t1".into(),
            text: "TAG-A".into(),
            position: [500.0, 0.0, 0.0],
            noun: "VALV".into(),
        });
        qr.tags.push(TagData {
            id: "t2".into(),
            text: "TAG-B".into(),
            position: [500.0, 0.0, 0.0],
            noun: "VALV".into(),
        });

        let ctx = LayoutEngineContext::production_defaults();
        let (prims, _issues) = compute_v2_primitives(&qr, &ctx);
        assert!(!prims.is_empty());
    }
}
