//! V1 → V2 图元组装器。
//!
//! 将 V1 [`LayoutResult`](crate::mbd::LayoutResult) 中的各类 `Placed*` 结构
//! 转换为 V2 [`MbdPrimitive`] 列表。这是 Phase 2 的过渡组装器——
//! 未来 Phase 3–4 的 `BranchCalculator v2` 会直接产出 V2 primitive，
//! 届时本模块退化为辅助或被移除。

use crate::mbd::{
    LayoutResult, LayoutVec3, PlacedBend, PlacedLinearDim, PlacedSlope, PlacedTag, PlacedWeld,
};

use super::primitive::*;
use super::small_dim::{DimRow, SmallDimInput, solve_small_dims};

/// 组装上下文：提供 V1 `PlacedXxx` 中缺失但 V2 需要的默认值。
#[derive(Debug, Clone)]
pub struct AssemblerContext {
    /// 默认字高（mm），当 V1 输出不含字高信息时使用。
    pub default_cheight: f32,
    /// 默认标注文字阅读方向。
    pub default_orientation: Vec3V2,
    /// 默认文字上方向。
    pub default_up: Vec3V2,
    /// 默认箭头长度（用于合成 arrow direction）。
    pub default_arrow_len: f32,
}

impl Default for AssemblerContext {
    fn default() -> Self {
        Self {
            default_cheight: 2.5,
            default_orientation: [1.0, 0.0, 0.0],
            default_up: [0.0, 1.0, 0.0],
            default_arrow_len: 3.0,
        }
    }
}

/// 将 V1 `LayoutResult` 组装成 V2 `MbdV2PipeData`（不含 `input_refno`/`branch_refno`，
/// 由调用者填充）。
///
/// 返回 `(primitives, issues)`。
pub fn assemble_v2_primitives(
    layout: &LayoutResult,
    ctx: &AssemblerContext,
) -> (Vec<MbdPrimitive>, Vec<MbdV2Issue>) {
    let mut primitives = Vec::new();
    let mut issues = Vec::new();
    let mut id_counter = 0u32;

    let mut next_id = |prefix: &str| -> String {
        id_counter += 1;
        format!("{}-{}", prefix, id_counter)
    };

    for dim in &layout.linear_dims {
        match assemble_linear_dim(dim, ctx, &mut next_id) {
            Ok(prim) => primitives.push(prim),
            Err(issue) => issues.push(issue),
        }
    }

    for dim in &layout.cut_tubis {
        match assemble_linear_dim(dim, ctx, &mut next_id) {
            Ok(prim) => primitives.push(prim),
            Err(issue) => issues.push(issue),
        }
    }

    for weld in &layout.welds {
        let (weld_prim, label_prim) = assemble_weld(weld, ctx, &mut next_id);
        primitives.push(weld_prim);
        if let Some(lbl) = label_prim {
            primitives.push(lbl);
        }
    }

    for slope in &layout.slopes {
        primitives.push(assemble_slope(slope, ctx, &mut next_id));
    }

    for tag in &layout.tags {
        primitives.push(assemble_tag(tag, ctx, &mut next_id));
    }

    for bend in &layout.bends {
        assemble_bend(bend, ctx, &mut next_id, &mut primitives, &mut issues);
    }

    (primitives, issues)
}

fn assemble_linear_dim(
    dim: &PlacedLinearDim,
    ctx: &AssemblerContext,
    next_id: &mut dyn FnMut(&str) -> String,
) -> Result<MbdPrimitive, MbdV2Issue> {
    let start: Vec3V2 = dim.start;
    let end: Vec3V2 = dim.end;
    let dir: Vec3V2 = dim.direction;

    let mid = midpoint(start, end);
    let offset = dim.offset;

    let ext1_start = start;
    let ext1_end = dim
        .extension_line_1_end
        .unwrap_or_else(|| add_scaled_v3(start, dir, offset));
    let ext2_start = end;
    let ext2_end = dim
        .extension_line_2_end
        .unwrap_or_else(|| add_scaled_v3(end, dir, offset));

    let dim_line_start = dim
        .dim_line_start
        .unwrap_or_else(|| add_scaled_v3(start, dir, offset));
    let dim_line_end = dim
        .dim_line_end
        .unwrap_or_else(|| add_scaled_v3(end, dir, offset));

    let pipe_dir = normalize(sub_v3(end, start));
    let arrow1_dir = pipe_dir;
    let arrow2_dir = negate(pipe_dir);

    let text_anchor = dim
        .text_anchor
        .unwrap_or_else(|| add_scaled_v3(mid, dir, offset));

    let sub_kind = match dim.kind.as_str() {
        "segment" => LinearDimSubKind::Segment,
        "chain" => LinearDimSubKind::Chain,
        "overall" => LinearDimSubKind::Overall,
        "port" => LinearDimSubKind::Port,
        _ => LinearDimSubKind::Segment,
    };

    Ok(MbdPrimitive::LinearDim(LinearDimPrimitive {
        common: CommonFields {
            id: if dim.id.is_empty() {
                next_id("ld")
            } else {
                dim.id.clone()
            },
            visible: dim.visible,
            suppressed_reason: dim.suppressed_reason.clone(),
            ..CommonFields::default()
        },
        sub_kind,
        extension_1: LineSegmentEndpoints {
            start: ext1_start,
            end: ext1_end,
        },
        extension_2: LineSegmentEndpoints {
            start: ext2_start,
            end: ext2_end,
        },
        dim_line: LineSegmentEndpoints {
            start: dim_line_start,
            end: dim_line_end,
        },
        arrows: [
            LinearDimArrow {
                position: dim_line_start,
                direction: arrow1_dir,
            },
            LinearDimArrow {
                position: dim_line_end,
                direction: arrow2_dir,
            },
        ],
        text: TextBlock {
            anchor: text_anchor,
            content: dim.text.clone(),
            height_mm: ctx.default_cheight,
            orientation: ctx.default_orientation,
            up: ctx.default_up,
        },
        level: 0,
    }))
}

fn assemble_weld(
    weld: &PlacedWeld,
    ctx: &AssemblerContext,
    next_id: &mut dyn FnMut(&str) -> String,
) -> (MbdPrimitive, Option<MbdPrimitive>) {
    let weld_id = if weld.id.is_empty() {
        next_id("weld")
    } else {
        weld.id.clone()
    };

    let label_prim = if !weld.label.is_empty() {
        let label_id = next_id("weld-lbl");
        let label_pos = weld
            .label_offset_world
            .map(|off| add_v3(weld.position, off))
            .unwrap_or(weld.position);
        Some(MbdPrimitive::Label(LabelPrimitive {
            common: CommonFields {
                id: label_id.clone(),
                visible: weld.visible,
                suppressed_reason: weld.suppressed_reason.clone(),
                function: Some("焊".to_string()),
                ..CommonFields::default()
            },
            anchor: weld.position,
            text_anchor: label_pos,
            content: weld.label.clone(),
            height_mm: ctx.default_cheight,
            orientation: ctx.default_orientation,
            up: ctx.default_up,
            box_shape: LabelBoxShape::None,
            box_padding_mm: 0.0,
        }))
    } else {
        None
    };

    let linked_label_id = label_prim.as_ref().map(|p| {
        if let MbdPrimitive::Label(lbl) = p {
            lbl.common.id.clone()
        } else {
            String::new()
        }
    });

    let weld_prim = MbdPrimitive::WeldMark(WeldMarkPrimitive {
        common: CommonFields {
            id: weld_id,
            visible: weld.visible,
            suppressed_reason: weld.suppressed_reason.clone(),
            function: Some("焊".to_string()),
            ..CommonFields::default()
        },
        position: weld.position,
        cross_size_mm: weld.cross_size,
        weld_type: if weld.is_shop {
            WeldType::Shop
        } else {
            WeldType::Field
        },
        linked_label_id,
    });

    (weld_prim, label_prim)
}

fn assemble_slope(
    slope: &PlacedSlope,
    ctx: &AssemblerContext,
    next_id: &mut dyn FnMut(&str) -> String,
) -> MbdPrimitive {
    let mid = midpoint(slope.start, slope.end);
    let text_pos = slope
        .label_offset_world
        .map(|off| add_v3(mid, off))
        .unwrap_or(mid);

    MbdPrimitive::SlopeMark(SlopeMarkPrimitive {
        common: CommonFields {
            id: if slope.id.is_empty() {
                next_id("slope")
            } else {
                slope.id.clone()
            },
            visible: slope.visible,
            suppressed_reason: slope.suppressed_reason.clone(),
            function: Some("坡度".to_string()),
            ..CommonFields::default()
        },
        start: slope.start,
        end: slope.end,
        slope: slope.slope,
        text: TextBlock {
            anchor: text_pos,
            content: slope.text.clone(),
            height_mm: ctx.default_cheight,
            orientation: ctx.default_orientation,
            up: ctx.default_up,
        },
    })
}

fn assemble_tag(
    tag: &PlacedTag,
    ctx: &AssemblerContext,
    next_id: &mut dyn FnMut(&str) -> String,
) -> MbdPrimitive {
    let text_pos = tag
        .label_offset_world
        .map(|off| add_v3(tag.position, off))
        .unwrap_or(tag.position);

    MbdPrimitive::Label(LabelPrimitive {
        common: CommonFields {
            id: if tag.id.is_empty() {
                next_id("tag")
            } else {
                tag.id.clone()
            },
            visible: tag.visible,
            suppressed_reason: tag.suppressed_reason.clone(),
            function: Some("标签".to_string()),
            ..CommonFields::default()
        },
        anchor: tag.position,
        text_anchor: text_pos,
        content: tag.text.clone(),
        height_mm: ctx.default_cheight,
        orientation: ctx.default_orientation,
        up: ctx.default_up,
        box_shape: LabelBoxShape::Rect,
        box_padding_mm: 1.0,
    })
}

fn assemble_bend(
    bend: &PlacedBend,
    ctx: &AssemblerContext,
    next_id: &mut dyn FnMut(&str) -> String,
    primitives: &mut Vec<MbdPrimitive>,
    issues: &mut Vec<MbdV2Issue>,
) {
    for dim in &bend.size_dims {
        match assemble_linear_dim(dim, ctx, next_id) {
            Ok(prim) => primitives.push(prim),
            Err(issue) => issues.push(issue),
        }
    }

    if let Some(angle) = &bend.angle {
        let ray_1 = normalize(sub_v3(angle.point1, angle.vertex));
        let ray_2 = normalize(sub_v3(angle.point2, angle.vertex));

        let normal = cross(ray_1, ray_2);
        let normal = if length(normal) > 1e-6 {
            normalize(normal)
        } else {
            ctx.default_up
        };

        let start_angle_rad = ray_1[1].atan2(ray_1[0]);
        let sweep_rad = {
            let cos_a = dot(ray_1, ray_2).clamp(-1.0, 1.0);
            cos_a.acos()
        };

        let arc_mid_angle = start_angle_rad + sweep_rad * 0.5;
        let text_pos = [
            angle.vertex[0] + angle.arc_radius * arc_mid_angle.cos(),
            angle.vertex[1] + angle.arc_radius * arc_mid_angle.sin(),
            angle.vertex[2],
        ];

        let tangent_1 = cross(normal, ray_1);
        let tangent_2 = cross(ray_2, normal);

        primitives.push(MbdPrimitive::AngleDim(AngleDimPrimitive {
            common: CommonFields {
                id: next_id("angle"),
                visible: bend.visible,
                suppressed_reason: bend.suppressed_reason.clone(),
                function: Some("角度".to_string()),
                ..CommonFields::default()
            },
            vertex: angle.vertex,
            ray_1,
            ray_2,
            arc: ArcGeometry {
                center: angle.vertex,
                radius_mm: angle.arc_radius,
                start_angle_rad,
                sweep_rad,
                normal,
            },
            arrows: [
                AngleDimArrow {
                    position: [
                        angle.vertex[0] + angle.arc_radius * ray_1[0],
                        angle.vertex[1] + angle.arc_radius * ray_1[1],
                        angle.vertex[2] + angle.arc_radius * ray_1[2],
                    ],
                    tangent: tangent_1,
                },
                AngleDimArrow {
                    position: [
                        angle.vertex[0] + angle.arc_radius * ray_2[0],
                        angle.vertex[1] + angle.arc_radius * ray_2[1],
                        angle.vertex[2] + angle.arc_radius * ray_2[2],
                    ],
                    tangent: tangent_2,
                },
            ],
            text: TextBlock {
                anchor: text_pos,
                content: angle.text.clone(),
                height_mm: ctx.default_cheight,
                orientation: ctx.default_orientation,
                up: ctx.default_up,
            },
        }));
    }
}

// ── 向量辅助 ──

fn midpoint(a: LayoutVec3, b: LayoutVec3) -> Vec3V2 {
    [
        (a[0] + b[0]) * 0.5,
        (a[1] + b[1]) * 0.5,
        (a[2] + b[2]) * 0.5,
    ]
}

fn add_v3(a: LayoutVec3, b: LayoutVec3) -> Vec3V2 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub_v3(a: LayoutVec3, b: LayoutVec3) -> Vec3V2 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn add_scaled_v3(base: LayoutVec3, dir: LayoutVec3, scale: f32) -> Vec3V2 {
    [
        base[0] + dir[0] * scale,
        base[1] + dir[1] * scale,
        base[2] + dir[2] * scale,
    ]
}

fn negate(v: Vec3V2) -> Vec3V2 {
    [-v[0], -v[1], -v[2]]
}

fn length(v: Vec3V2) -> f32 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

fn normalize(v: Vec3V2) -> Vec3V2 {
    let len = length(v);
    if len < 1e-10 {
        return [0.0, 0.0, 0.0];
    }
    [v[0] / len, v[1] / len, v[2] / len]
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

// ─────────────────────────────────────────────────────────────────────────
// Chain expansion（Phase 3 Step 2）
// ─────────────────────────────────────────────────────────────────────────

/// SmallDimSolver 的运行参数，与 [`SmallDimInput`] 相关字段一一对齐。
#[derive(Debug, Clone)]
pub struct SmallDimChainParams {
    /// 基准字高（mm）。
    pub cheight: f32,
    /// 是否开启小尺寸错层。
    pub sep_small_dim: bool,
    /// 是否开启字高自适应。
    pub change_cheight_auto: bool,
    /// 字高缩小比例下限（默认 0.5）。
    pub change_cheight_auto_bili: f32,
}

impl Default for SmallDimChainParams {
    fn default() -> Self {
        Self {
            cheight: 2.5,
            sep_small_dim: true,
            change_cheight_auto: true,
            change_cheight_auto_bili: 0.5,
        }
    }
}

/// 预分组好的链式尺寸：一组同基线方向、端点相邻的 [`PlacedLinearDim`]。
///
/// 调用方负责把一批 `PlacedLinearDim` 聚类成 chain（按 direction/offset
/// 近似相等 + 端点连接），本 step 不做自动聚类。
pub struct LinearDimChain<'a> {
    /// 组内 dims 的引用，必须满足：
    /// - `direction` / `offset` 近似相等（同基线）；
    /// - `dims[i].end` 与 `dims[i+1].start` 近似相同（端点连接）。
    pub dims: &'a [PlacedLinearDim],
    /// SmallDimSolver 的运行参数。
    pub small_dim_params: SmallDimChainParams,
}

/// 把一条预分组好的 chain 展开成多个 [`LinearDimPrimitive`]。
///
/// 内部走 [`solve_small_dims`]：每个 [`DimRow`] 产出 `row.points.len() - 1`
/// 个 primitive，共享 `row.level` 与 `row.cheight`。
pub fn expand_linear_dim_chain(
    chain: &LinearDimChain<'_>,
    ctx: &AssemblerContext,
    next_id: &mut dyn FnMut(&str) -> String,
) -> Vec<MbdPrimitive> {
    let n = chain.dims.len();
    if n == 0 {
        return Vec::new();
    }

    let start_pt: Vec3V2 = chain.dims[0].start;
    let end_pt: Vec3V2 = chain.dims[n - 1].end;

    let mut points: Vec<Vec3V2> = Vec::with_capacity(n + 1);
    points.push(start_pt);
    for d in chain.dims {
        points.push(d.end);
    }

    let total = sub_v3(end_pt, start_pt);
    let xdir = normalize(total);
    let ydir = normalize(chain.dims[0].direction);
    let offset = chain.dims[0].offset;
    let mid = midpoint(start_pt, end_pt);
    let pos = add_scaled_v3(mid, ydir, offset);

    let input = SmallDimInput {
        points: points.clone(),
        xdir,
        ydir,
        pos,
        cheight: chain.small_dim_params.cheight,
        sep_small_dim: chain.small_dim_params.sep_small_dim,
        change_cheight_auto: chain.small_dim_params.change_cheight_auto,
        change_cheight_auto_bili: chain.small_dim_params.change_cheight_auto_bili,
    };
    let result = solve_small_dims(&input);

    let mut primitives: Vec<MbdPrimitive> = Vec::with_capacity(n);
    let mut seg_cursor = 0usize;
    for row in &result.rows {
        if row.points.len() < 2 || row.texts.is_empty() {
            continue;
        }
        for i in 0..row.points.len() - 1 {
            let source_idx = seg_cursor.min(n - 1);
            let source = &chain.dims[source_idx];
            let prim = build_primitive_for_row_segment(source, row, i, xdir, ctx, next_id);
            primitives.push(prim);
            seg_cursor += 1;
        }
    }

    primitives
}

/// [`assemble_v2_primitives`] 的增强版：对 `linear_dims` / `cut_tubis` 先按
/// [`group_dims_into_chains`] 聚类，再调 [`expand_linear_dim_chain`] 展开（含 level 回填）。
/// 其余 primitive 装配路径（weld/slope/tag/bend）与基础版一致。
pub fn assemble_v2_primitives_with_chain_stacking(
    layout: &LayoutResult,
    ctx: &AssemblerContext,
    chain_tolerance: &ChainTolerance,
    small_dim_params: &SmallDimChainParams,
) -> (Vec<MbdPrimitive>, Vec<MbdV2Issue>) {
    let mut primitives = Vec::new();
    let mut issues = Vec::new();
    let mut id_counter = 0u32;
    let mut next_id = |prefix: &str| -> String {
        id_counter += 1;
        format!("{}-{}", prefix, id_counter)
    };

    let linear_groups = group_dims_into_chains(&layout.linear_dims, chain_tolerance);
    for group in linear_groups {
        assemble_chain_group(
            &layout.linear_dims,
            &group,
            small_dim_params,
            ctx,
            &mut next_id,
            &mut primitives,
            &mut issues,
        );
    }

    let cut_groups = group_dims_into_chains(&layout.cut_tubis, chain_tolerance);
    for group in cut_groups {
        assemble_chain_group(
            &layout.cut_tubis,
            &group,
            small_dim_params,
            ctx,
            &mut next_id,
            &mut primitives,
            &mut issues,
        );
    }

    for weld in &layout.welds {
        let (weld_prim, label_prim) = assemble_weld(weld, ctx, &mut next_id);
        primitives.push(weld_prim);
        if let Some(lbl) = label_prim {
            primitives.push(lbl);
        }
    }
    for slope in &layout.slopes {
        primitives.push(assemble_slope(slope, ctx, &mut next_id));
    }
    for tag in &layout.tags {
        primitives.push(assemble_tag(tag, ctx, &mut next_id));
    }
    for bend in &layout.bends {
        assemble_bend(bend, ctx, &mut next_id, &mut primitives, &mut issues);
    }

    (primitives, issues)
}

fn assemble_chain_group(
    dims: &[PlacedLinearDim],
    group: &ChainGroup,
    small_dim_params: &SmallDimChainParams,
    ctx: &AssemblerContext,
    next_id: &mut dyn FnMut(&str) -> String,
    primitives: &mut Vec<MbdPrimitive>,
    issues: &mut Vec<MbdV2Issue>,
) {
    match group.indices.len() {
        0 => {}
        1 => match assemble_linear_dim(&dims[group.indices[0]], ctx, next_id) {
            Ok(p) => primitives.push(p),
            Err(i) => issues.push(i),
        },
        _ => {
            let chain_dims: Vec<PlacedLinearDim> =
                group.indices.iter().map(|&i| dims[i].clone()).collect();
            let chain = LinearDimChain {
                dims: &chain_dims,
                small_dim_params: small_dim_params.clone(),
            };
            let expanded = expand_linear_dim_chain(&chain, ctx, next_id);
            primitives.extend(expanded);
        }
    }
}

/// Chain 聚类的容差配置。
#[derive(Debug, Clone)]
pub struct ChainTolerance {
    /// direction 单位向量 per-axis 量化步长（越小越严格）。默认 `1e-3`。
    pub direction_quant: f32,
    /// offset 量化步长（mm）。默认 `0.1`。
    pub offset_quant: f32,
    /// 端点连接距离容差（mm）。默认 `0.5`。
    pub endpoint_tolerance: f32,
}

impl Default for ChainTolerance {
    fn default() -> Self {
        Self {
            direction_quant: 1e-3,
            offset_quant: 0.1,
            endpoint_tolerance: 0.5,
        }
    }
}

/// 一个聚类后的 chain 组；`indices` 指向输入 slice 的下标序列。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainGroup {
    /// 组成该 chain 的 dim 在输入 slice 中的索引；顺序即端点连接顺序。
    pub indices: Vec<usize>,
}

/// 把一批 `PlacedLinearDim` 自动聚类成 chain 组。
///
/// 算法（MVP）：
/// 1. 按 `(quantize(direction), quantize(offset))` 分桶，非法向量单独落桶。
/// 2. 桶内按端点连接：`dim[i].end` 与 `dim[j].start` 距离 ≤ `endpoint_tolerance`
///    即可串成一条；贪心优先连最长的链。
/// 3. 未能连到链里的 dim 各自成组（`indices.len() == 1`）。
///
/// 返回组按 `indices[0]` 升序，保证输出稳定。
pub fn group_dims_into_chains(
    dims: &[PlacedLinearDim],
    tolerance: &ChainTolerance,
) -> Vec<ChainGroup> {
    if dims.is_empty() {
        return Vec::new();
    }

    let mut buckets: std::collections::BTreeMap<BucketKey, Vec<usize>> =
        std::collections::BTreeMap::new();
    for (i, dim) in dims.iter().enumerate() {
        let key = bucket_key(dim, tolerance);
        buckets.entry(key).or_default().push(i);
    }

    let mut groups: Vec<ChainGroup> = Vec::new();
    for (_, mut indices) in buckets {
        // 桶内按 start 点的 "沿 direction 投影" 排序，让端点连接顺序稳定
        let probe_dir = if let Some(&i0) = indices.first() {
            dims[i0].direction
        } else {
            continue;
        };
        indices.sort_by(|&a, &b| {
            project_along(dims[a].start, probe_dir)
                .partial_cmp(&project_along(dims[b].start, probe_dir))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // 贪心串联：扫描每个未用过的 i 作为起点，沿 end→start 连接后续
        let mut used = vec![false; indices.len()];
        for start_pos in 0..indices.len() {
            if used[start_pos] {
                continue;
            }
            used[start_pos] = true;
            let mut chain = vec![indices[start_pos]];
            let mut tail_end = dims[indices[start_pos]].end;

            loop {
                let mut best_next: Option<usize> = None;
                for j in 0..indices.len() {
                    if used[j] {
                        continue;
                    }
                    let cand = indices[j];
                    if distance_sq(tail_end, dims[cand].start)
                        <= tolerance.endpoint_tolerance * tolerance.endpoint_tolerance
                    {
                        best_next = Some(j);
                        break;
                    }
                }
                if let Some(j) = best_next {
                    used[j] = true;
                    let cand = indices[j];
                    chain.push(cand);
                    tail_end = dims[cand].end;
                } else {
                    break;
                }
            }

            groups.push(ChainGroup { indices: chain });
        }
    }

    // 稳定输出：按 indices[0] 升序
    groups.sort_by_key(|g| g.indices.first().copied().unwrap_or(usize::MAX));
    groups
}

type BucketKey = (i32, i32, i32, i32);

fn bucket_key(dim: &PlacedLinearDim, tolerance: &ChainTolerance) -> BucketKey {
    let d = normalize(dim.direction);
    let q = tolerance.direction_quant.max(1e-9);
    let qx = (d[0] / q).round() as i32;
    let qy = (d[1] / q).round() as i32;
    let qz = (d[2] / q).round() as i32;
    let qo = (dim.offset / tolerance.offset_quant.max(1e-9)).round() as i32;
    (qx, qy, qz, qo)
}

fn project_along(p: Vec3V2, dir: Vec3V2) -> f32 {
    // dir 不要求归一化；投影到任意参考方向用于稳定排序即可
    p[0] * dir[0] + p[1] * dir[1] + p[2] * dir[2]
}

fn distance_sq(a: Vec3V2, b: Vec3V2) -> f32 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    dx * dx + dy * dy + dz * dz
}

fn build_primitive_for_row_segment(
    source: &PlacedLinearDim,
    row: &DimRow,
    seg_i: usize,
    xdir: Vec3V2,
    ctx: &AssemblerContext,
    next_id: &mut dyn FnMut(&str) -> String,
) -> MbdPrimitive {
    let p_start = row.points[seg_i];
    let p_end = row.points[seg_i + 1];

    let xshift_start = dot(sub_v3(p_start, row.pos), xdir);
    let xshift_end = dot(sub_v3(p_end, row.pos), xdir);

    let dim_line_start = add_scaled_v3(row.pos, xdir, xshift_start);
    let dim_line_end = add_scaled_v3(row.pos, xdir, xshift_end);

    let text_shift = (xshift_start + xshift_end) * 0.5;
    let text_anchor = add_scaled_v3(row.pos, xdir, text_shift);

    let arrow1_dir = xdir;
    let arrow2_dir = negate(xdir);

    let sub_kind = match source.kind.as_str() {
        "segment" => LinearDimSubKind::Segment,
        "chain" => LinearDimSubKind::Chain,
        "overall" => LinearDimSubKind::Overall,
        "port" => LinearDimSubKind::Port,
        _ => LinearDimSubKind::Segment,
    };

    let id = if source.id.is_empty() {
        next_id("ld")
    } else {
        format!("{}/r{}s{}", source.id, row.level, seg_i)
    };

    MbdPrimitive::LinearDim(LinearDimPrimitive {
        common: CommonFields {
            id,
            visible: source.visible,
            suppressed_reason: source.suppressed_reason.clone(),
            ..CommonFields::default()
        },
        sub_kind,
        extension_1: LineSegmentEndpoints {
            start: p_start,
            end: dim_line_start,
        },
        extension_2: LineSegmentEndpoints {
            start: p_end,
            end: dim_line_end,
        },
        dim_line: LineSegmentEndpoints {
            start: dim_line_start,
            end: dim_line_end,
        },
        arrows: [
            LinearDimArrow {
                position: dim_line_start,
                direction: arrow1_dir,
            },
            LinearDimArrow {
                position: dim_line_end,
                direction: arrow2_dir,
            },
        ],
        text: TextBlock {
            anchor: text_anchor,
            content: row.texts[seg_i].clone(),
            height_mm: row.cheight,
            orientation: ctx.default_orientation,
            up: ctx.default_up,
        },
        level: row.level,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_layout_result() -> LayoutResult {
        LayoutResult {
            version: 1,
            linear_dims: vec![PlacedLinearDim {
                id: "dim-1".to_string(),
                kind: "segment".to_string(),
                start: [0.0, 0.0, 0.0],
                end: [500.0, 0.0, 0.0],
                text: "500".to_string(),
                offset: 80.0,
                direction: [0.0, 1.0, 0.0],
                label_t: 0.5,
                visible: true,
                ..Default::default()
            }],
            welds: vec![PlacedWeld {
                id: "weld-1".to_string(),
                position: [100.0, 0.0, 0.0],
                label: "SW".to_string(),
                is_shop: true,
                cross_size: 50.0,
                visible: true,
                ..Default::default()
            }],
            slopes: vec![PlacedSlope {
                id: "slope-1".to_string(),
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
        }
    }

    #[test]
    fn assembler_produces_correct_primitive_count() {
        let layout = sample_layout_result();
        let ctx = AssemblerContext::default();
        let (primitives, issues) = assemble_v2_primitives(&layout, &ctx);

        assert!(issues.is_empty(), "should have no issues");
        // 1 linear_dim + 1 weld_mark + 1 weld_label + 1 slope + 1 tag = 5
        assert_eq!(primitives.len(), 5);
    }

    #[test]
    fn linear_dim_has_correct_structure() {
        let layout = sample_layout_result();
        let ctx = AssemblerContext::default();
        let (primitives, _) = assemble_v2_primitives(&layout, &ctx);

        let dim = primitives.iter().find(|p| matches!(p, MbdPrimitive::LinearDim(_)));
        assert!(dim.is_some());
        if let Some(MbdPrimitive::LinearDim(d)) = dim {
            assert_eq!(d.common.id, "dim-1");
            assert_eq!(d.sub_kind, LinearDimSubKind::Segment);
            assert_eq!(d.text.content, "500");
            assert!(d.common.visible);
        }
    }

    #[test]
    fn weld_generates_linked_label() {
        let layout = sample_layout_result();
        let ctx = AssemblerContext::default();
        let (primitives, _) = assemble_v2_primitives(&layout, &ctx);

        let weld = primitives
            .iter()
            .find(|p| matches!(p, MbdPrimitive::WeldMark(_)));
        assert!(weld.is_some());
        if let Some(MbdPrimitive::WeldMark(w)) = weld {
            assert!(w.linked_label_id.is_some());
            let label_id = w.linked_label_id.as_ref().unwrap();
            let label = primitives
                .iter()
                .find(|p| p.id() == label_id.as_str());
            assert!(label.is_some(), "linked label should exist");
        }
    }

    #[test]
    fn slope_has_text_block() {
        let layout = sample_layout_result();
        let ctx = AssemblerContext::default();
        let (primitives, _) = assemble_v2_primitives(&layout, &ctx);

        let slope = primitives
            .iter()
            .find(|p| matches!(p, MbdPrimitive::SlopeMark(_)));
        assert!(slope.is_some());
        if let Some(MbdPrimitive::SlopeMark(s)) = slope {
            assert_eq!(s.text.content, "1:100");
            assert!((s.slope - 0.01).abs() < 0.001);
        }
    }

    #[test]
    fn all_primitives_serialize_to_valid_json() {
        let layout = sample_layout_result();
        let ctx = AssemblerContext::default();
        let (primitives, _) = assemble_v2_primitives(&layout, &ctx);

        for prim in &primitives {
            let json = serde_json::to_value(prim).expect("should serialize");
            assert!(json.get("kind").is_some(), "primitive must have 'kind'");
            let back: MbdPrimitive = serde_json::from_value(json).expect("should deserialize");
            assert_eq!(&back, prim);
        }
    }

    #[test]
    fn suppressed_dim_is_preserved() {
        let layout = LayoutResult {
            version: 1,
            linear_dims: vec![PlacedLinearDim {
                id: "dim-s".to_string(),
                kind: "segment".to_string(),
                start: [0.0, 0.0, 0.0],
                end: [100.0, 0.0, 0.0],
                text: "100".to_string(),
                offset: 50.0,
                direction: [0.0, 1.0, 0.0],
                visible: false,
                suppressed_reason: Some("too_dense".to_string()),
                ..Default::default()
            }],
            ..Default::default()
        };
        let (primitives, _) = assemble_v2_primitives(&layout, &AssemblerContext::default());
        assert_eq!(primitives.len(), 1);
        assert!(!primitives[0].visible());
        if let MbdPrimitive::LinearDim(d) = &primitives[0] {
            assert_eq!(
                d.common.suppressed_reason.as_deref(),
                Some("too_dense")
            );
        }
    }

    // ── Phase 3 Step 2: chain expansion tests ──

    fn make_chain_dim(id: &str, kind: &str, x_start: f32, x_end: f32) -> PlacedLinearDim {
        PlacedLinearDim {
            id: id.to_string(),
            kind: kind.to_string(),
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

    fn next_id_factory() -> impl FnMut(&str) -> String {
        let mut n = 0u32;
        move |prefix: &str| {
            n += 1;
            format!("{}-{}", prefix, n)
        }
    }

    #[test]
    fn chain_with_all_fitting_segments_produces_level_zero_primitives() {
        let dims = vec![
            make_chain_dim("d-1", "segment", 0.0, 500.0),
            make_chain_dim("d-2", "segment", 500.0, 800.0),
            make_chain_dim("d-3", "segment", 800.0, 1200.0),
        ];
        let chain = LinearDimChain {
            dims: &dims,
            small_dim_params: SmallDimChainParams::default(),
        };
        let ctx = AssemblerContext::default();
        let mut next_id = next_id_factory();

        let prims = expand_linear_dim_chain(&chain, &ctx, &mut next_id);
        assert_eq!(prims.len(), 3, "3 segments -> 3 primitives");
        for (i, prim) in prims.iter().enumerate() {
            if let MbdPrimitive::LinearDim(d) = prim {
                assert_eq!(d.level, 0, "all segments fit -> level 0 (i={})", i);
                assert_eq!(d.sub_kind, LinearDimSubKind::Segment);
                assert!(d.common.visible);
                // text.content 应当来自 row.texts（format_dim_value 格式）
                assert!(!d.text.content.is_empty());
            } else {
                panic!("expected LinearDim primitive");
            }
        }
    }

    #[test]
    fn chain_with_short_middle_segment_triggers_level_bump() {
        // "1" em ≈ 0.5741, text_width @ cheight=2.5 ≈ 1.435; 1mm 段不够
        let dims = vec![
            make_chain_dim("d-1", "segment", 0.0, 500.0),
            make_chain_dim("d-2", "segment", 500.0, 501.0),
            make_chain_dim("d-3", "segment", 501.0, 1000.0),
        ];
        let params = SmallDimChainParams {
            cheight: 2.5,
            sep_small_dim: true,
            change_cheight_auto: false,
            change_cheight_auto_bili: 0.5,
        };
        let chain = LinearDimChain {
            dims: &dims,
            small_dim_params: params,
        };
        let ctx = AssemblerContext::default();
        let mut next_id = next_id_factory();

        let prims = expand_linear_dim_chain(&chain, &ctx, &mut next_id);
        assert_eq!(prims.len(), 3);
        let levels: Vec<u16> = prims
            .iter()
            .map(|p| match p {
                MbdPrimitive::LinearDim(d) => d.level,
                _ => panic!("expected LinearDim"),
            })
            .collect();
        assert!(
            levels.iter().any(|&l| l > 0),
            "short segment should trigger level > 0, got {:?}",
            levels
        );
    }

    #[test]
    fn chain_respects_change_cheight_auto() {
        let dims = vec![
            make_chain_dim("d-1", "segment", 0.0, 500.0),
            make_chain_dim("d-2", "segment", 500.0, 501.0),
        ];
        let params = SmallDimChainParams {
            cheight: 2.5,
            sep_small_dim: false,
            change_cheight_auto: true,
            change_cheight_auto_bili: 0.5,
        };
        let chain = LinearDimChain {
            dims: &dims,
            small_dim_params: params,
        };
        let ctx = AssemblerContext::default();
        let mut next_id = next_id_factory();

        let prims = expand_linear_dim_chain(&chain, &ctx, &mut next_id);
        let shrunk = prims.iter().any(|p| {
            matches!(p, MbdPrimitive::LinearDim(d) if d.text.height_mm < 2.5 - 1e-6)
        });
        assert!(
            shrunk,
            "change_cheight_auto should shrink at least one primitive text height"
        );
    }

    #[test]
    fn chain_empty_dims_produces_empty_result() {
        let chain = LinearDimChain {
            dims: &[],
            small_dim_params: SmallDimChainParams::default(),
        };
        let ctx = AssemblerContext::default();
        let mut next_id = next_id_factory();
        let prims = expand_linear_dim_chain(&chain, &ctx, &mut next_id);
        assert!(prims.is_empty());
    }

    #[test]
    fn chain_propagates_suppressed_reason() {
        let mut dims = vec![
            make_chain_dim("d-1", "segment", 0.0, 500.0),
            make_chain_dim("d-2", "segment", 500.0, 1000.0),
        ];
        dims[1].visible = false;
        dims[1].suppressed_reason = Some("too_dense".to_string());
        let chain = LinearDimChain {
            dims: &dims,
            small_dim_params: SmallDimChainParams::default(),
        };
        let ctx = AssemblerContext::default();
        let mut next_id = next_id_factory();

        let prims = expand_linear_dim_chain(&chain, &ctx, &mut next_id);
        // 至少有 2 个 primitive，其中对应 d-2 的那条 visible=false
        let suppressed = prims.iter().any(|p| {
            matches!(p, MbdPrimitive::LinearDim(d) if d.common.suppressed_reason.as_deref() == Some("too_dense"))
        });
        assert!(suppressed, "d-2 的 suppressed_reason 应当透传到对应 primitive");
    }

    #[test]
    fn chain_expansion_primitives_roundtrip_through_json() {
        let dims = vec![
            make_chain_dim("d-1", "chain", 0.0, 500.0),
            make_chain_dim("d-2", "chain", 500.0, 800.0),
        ];
        let chain = LinearDimChain {
            dims: &dims,
            small_dim_params: SmallDimChainParams::default(),
        };
        let ctx = AssemblerContext::default();
        let mut next_id = next_id_factory();

        let prims = expand_linear_dim_chain(&chain, &ctx, &mut next_id);
        assert!(!prims.is_empty());
        for prim in &prims {
            let json = serde_json::to_value(prim).expect("serialize");
            assert_eq!(json["kind"], serde_json::json!("linear_dim"));
            let back: MbdPrimitive = serde_json::from_value(json).expect("deserialize");
            assert_eq!(&back, prim);
        }
    }

    #[test]
    fn chain_dim_line_lies_along_pos_plus_offset_in_ydir() {
        // 单段 chain：expand 之后 dim_line 应沿 ydir=+Y 偏移 offset=80
        let dims = vec![make_chain_dim("d-1", "segment", 0.0, 1000.0)];
        let chain = LinearDimChain {
            dims: &dims,
            small_dim_params: SmallDimChainParams::default(),
        };
        let ctx = AssemblerContext::default();
        let mut next_id = next_id_factory();
        let prims = expand_linear_dim_chain(&chain, &ctx, &mut next_id);
        assert_eq!(prims.len(), 1);
        if let MbdPrimitive::LinearDim(d) = &prims[0] {
            // dim_line 的 y 坐标 = offset = 80
            assert!((d.dim_line.start[1] - 80.0).abs() < 0.01);
            assert!((d.dim_line.end[1] - 80.0).abs() < 0.01);
            // dim_line 与 p_start/p_end 在 x 上对齐
            assert!((d.dim_line.start[0] - 0.0).abs() < 0.01);
            assert!((d.dim_line.end[0] - 1000.0).abs() < 0.01);
        } else {
            panic!("expected LinearDim");
        }
    }

    // ── Phase 3 Step 2.5: chain grouping tests ──

    fn diag_dim(id: &str, x_start: f32, x_end: f32, dir: [f32; 3], offset: f32) -> PlacedLinearDim {
        PlacedLinearDim {
            id: id.to_string(),
            kind: "segment".to_string(),
            start: [x_start, 0.0, 0.0],
            end: [x_end, 0.0, 0.0],
            text: format!("{}", (x_end - x_start) as i32),
            offset,
            direction: dir,
            label_t: 0.5,
            visible: true,
            ..Default::default()
        }
    }

    #[test]
    fn group_empty_dims_returns_empty() {
        let groups = group_dims_into_chains(&[], &ChainTolerance::default());
        assert!(groups.is_empty());
    }

    #[test]
    fn group_single_dim_forms_single_group() {
        let dims = vec![diag_dim("d-1", 0.0, 500.0, [0.0, 1.0, 0.0], 80.0)];
        let groups = group_dims_into_chains(&dims, &ChainTolerance::default());
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].indices, vec![0]);
    }

    #[test]
    fn group_three_consecutive_dims_form_one_chain() {
        let dir = [0.0, 1.0, 0.0];
        let dims = vec![
            diag_dim("d-1", 0.0, 500.0, dir, 80.0),
            diag_dim("d-2", 500.0, 800.0, dir, 80.0),
            diag_dim("d-3", 800.0, 1200.0, dir, 80.0),
        ];
        let groups = group_dims_into_chains(&dims, &ChainTolerance::default());
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].indices, vec![0, 1, 2]);
    }

    #[test]
    fn group_two_unconnected_dims_form_two_groups() {
        let dir = [0.0, 1.0, 0.0];
        let dims = vec![
            diag_dim("d-1", 0.0, 500.0, dir, 80.0),
            diag_dim("d-2", 1000.0, 1500.0, dir, 80.0), // gap 500mm
        ];
        let groups = group_dims_into_chains(&dims, &ChainTolerance::default());
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].indices, vec![0]);
        assert_eq!(groups[1].indices, vec![1]);
    }

    #[test]
    fn group_different_direction_goes_to_separate_buckets() {
        let dims = vec![
            diag_dim("d-1", 0.0, 500.0, [0.0, 1.0, 0.0], 80.0),
            diag_dim("d-2", 500.0, 1000.0, [1.0, 0.0, 0.0], 80.0),
        ];
        let groups = group_dims_into_chains(&dims, &ChainTolerance::default());
        assert_eq!(groups.len(), 2);
    }

    #[test]
    fn group_tolerant_endpoint_snap_within_tolerance() {
        let dir = [0.0, 1.0, 0.0];
        let mut d1 = diag_dim("d-1", 0.0, 500.0, dir, 80.0);
        d1.end = [500.0, 0.0, 0.0];
        let mut d2 = diag_dim("d-2", 500.2, 1000.0, dir, 80.0);
        d2.start = [500.2, 0.0, 0.0]; // 偏差 0.2mm，容差 0.5 内
        let dims = vec![d1, d2];

        let groups = group_dims_into_chains(&dims, &ChainTolerance::default());
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].indices, vec![0, 1]);
    }

    #[test]
    fn stacking_enabled_expands_chain_into_level_zero_primitives() {
        let dir = [0.0, 1.0, 0.0];
        let layout = LayoutResult {
            version: 1,
            linear_dims: vec![
                diag_dim("d-1", 0.0, 500.0, dir, 80.0),
                diag_dim("d-2", 500.0, 800.0, dir, 80.0),
                diag_dim("d-3", 800.0, 1200.0, dir, 80.0),
            ],
            ..Default::default()
        };
        let (prims, _issues) = assemble_v2_primitives_with_chain_stacking(
            &layout,
            &AssemblerContext::default(),
            &ChainTolerance::default(),
            &SmallDimChainParams::default(),
        );
        let linear_count = prims
            .iter()
            .filter(|p| matches!(p, MbdPrimitive::LinearDim(_)))
            .count();
        assert_eq!(linear_count, 3);
        for p in &prims {
            if let MbdPrimitive::LinearDim(d) = p {
                assert_eq!(d.level, 0);
            }
        }
    }

    #[test]
    fn stacking_enabled_short_segment_triggers_level_bump() {
        let dir = [0.0, 1.0, 0.0];
        let layout = LayoutResult {
            version: 1,
            linear_dims: vec![
                diag_dim("d-1", 0.0, 500.0, dir, 80.0),
                diag_dim("d-2", 500.0, 501.0, dir, 80.0),
                diag_dim("d-3", 501.0, 1000.0, dir, 80.0),
            ],
            ..Default::default()
        };
        let params = SmallDimChainParams {
            sep_small_dim: true,
            change_cheight_auto: false,
            ..SmallDimChainParams::default()
        };
        let (prims, _issues) = assemble_v2_primitives_with_chain_stacking(
            &layout,
            &AssemblerContext::default(),
            &ChainTolerance::default(),
            &params,
        );
        let has_bump = prims.iter().any(|p| {
            matches!(p, MbdPrimitive::LinearDim(d) if d.level > 0)
        });
        assert!(has_bump, "short 1mm segment should bump to level > 0");
    }

    #[test]
    fn stacking_singleton_group_falls_back_to_single_assemble() {
        let layout = LayoutResult {
            version: 1,
            linear_dims: vec![diag_dim("d-1", 0.0, 500.0, [0.0, 1.0, 0.0], 80.0)],
            ..Default::default()
        };
        let (prims, _) = assemble_v2_primitives_with_chain_stacking(
            &layout,
            &AssemblerContext::default(),
            &ChainTolerance::default(),
            &SmallDimChainParams::default(),
        );
        assert_eq!(prims.len(), 1);
        if let MbdPrimitive::LinearDim(d) = &prims[0] {
            // 单段 group 走 assemble_linear_dim（id 为源 id，不带 /rX/sY 后缀）
            assert_eq!(d.common.id, "d-1");
        } else {
            panic!("expected LinearDim");
        }
    }
}
