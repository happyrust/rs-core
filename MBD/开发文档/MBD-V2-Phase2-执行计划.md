# MBD V2 Phase 2 执行计划

> **状态**：底层模块已完成；验收口径已按真实页面最终验收更新  
> **开始日期**：2026-04-21  
> **前置**：Phase 1（类型定义）已完成 ✅

---

## 目标

实现 MBD V2 后端底层模块，为 Phase 3（AvoidanceEngine + V2 API）打基础。

Phase 2 聚焦四个模块 + 一个组装器：

| 模块 | 文件 | PDMS 参考 | 复杂度 |
|------|------|----------|--------|
| TextMeasurement | `src/mbd/v2/text_measurement.rs` | `mbdtextlen.pmlfnc` | 低 |
| SmallDimSolver | `src/mbd/v2/small_dim.rs` | `lindim.sepSmallDim` + `changeCheightAuto` + `getgoodcheight` | 中 |
| LeaderLineRouter | `src/mbd/v2/leader_router.rs` | `mlabel.addleadline` | 低 |
| PrimitiveAssembler | `src/mbd/v2/assembler.rs` | 新建 | 中 |

PolarSystem（polarsystem.pmlobj, 3177 行, 极高复杂度）推迟到 Phase 2.5 或 Phase 3。

---

## 执行步骤

### Step 1: TextMeasurement ✅

**目标**：精确复制 PDMS `mbdtextlen.pmlfnc` 的字符宽度查表逻辑。

- 静态 `HashMap<char, f32>` 或数组 + index 查找
- 未知字符默认宽度 `1.02`
- `mbd_text_len(text: &str) -> f32` — 返回无量纲 em 总和
- `mbd_text_width(text: &str, cheight: f32) -> f32` — 返回 mm 宽度
- Golden test：与 PDMS 逐字符对齐

### Step 2: SmallDimSolver

**目标**：复制 `lindim.sepSmallDim` + `changeCheightAuto` + `getgoodcheight`。

核心算法：
1. 对每段尺寸，计算文字宽度 vs 段长度
2. 文字宽 ≤ 段长：累积到当前 span
3. 文字宽 > 段长：
   a. 尝试缩小字高（`changeCheightAuto`）
   b. 若仍不够且 `sepSmallDim=true`：错层（`currentLevel++`）
4. 输出：`Vec<DimRow>` 每行包含 points、pos、cheight、level

接口：
```rust
pub struct SmallDimResult {
    pub rows: Vec<DimRow>,
}
pub struct DimRow {
    pub points: Vec<Vec3V2>,
    pub pos: Vec3V2,
    pub cheight: f32,
    pub level: u16,
    pub texts: Vec<String>,
}
pub fn solve_small_dims(input: &SmallDimInput) -> SmallDimResult;
```

### Step 3: LeaderLineRouter

**目标**：复制 `mlabel.addleadline` — 选最近文字框角作为引线起点。

算法：
1. 给定 text_anchor、text_width、text_height、orientation/up → 算四角
2. 选距 dimpos 最近的角作为引线端点
3. 输出引线折线点

接口：
```rust
pub fn route_leader_line(
    dim_pos: Vec3V2,
    text_anchor: Vec3V2,
    text_width_mm: f32,
    text_height_mm: f32,
    orientation: Vec3V2,
    up: Vec3V2,
) -> Vec<Vec3V2>;
```

### Step 4: PrimitiveAssembler

**目标**：把 V1 `LayoutResult` 的各种 `PlacedXxx` 转换成 V2 `MbdPrimitive` 列表。

映射表：
| V1 类型 | V2 Primitive | 需要补充的字段 |
|---------|-------------|--------------|
| `PlacedLinearDim` | `LinearDimPrimitive` | extension_1/2, dim_line, arrows, TextBlock |
| `PlacedAngle` (in PlacedBend) | `AngleDimPrimitive` | ArcGeometry, arrows |
| `PlacedWeld` | `WeldMarkPrimitive` + optional `LabelPrimitive` | — |
| `PlacedSlope` | `SlopeMarkPrimitive` | TextBlock |
| `PlacedTag` | `LabelPrimitive` | orientation/up |
| `PlacedFitting` | `LabelPrimitive` | orientation/up |

接口：
```rust
pub fn assemble_v2_primitives(
    layout: &LayoutResult,
    context: &AssemblerContext,
) -> (Vec<MbdPrimitive>, Vec<MbdV2Issue>);
```

### Step 5: 基础验证 ✅ 代码级检查用例已就位

- 每个模块保留代码级检查用例，供需要时定向运行
- TextMeasurement: 逐字符宽度 vs PDMS 表
- SmallDimSolver: 直管、短段、多段的 DimRow 输出
- Assembler: V1 fixture → V2 primitive 快照
- 按仓库约定，默认不把 `cargo test` 作为交付验收动作

进入 **Phase 3 Step 1** 再补 `pipeline.rs`（`LayoutResult → MbdV2PipeData`）单测，
详见 [`MBD-V2-Phase3-Step1-执行计划.md`](./MBD-V2-Phase3-Step1-执行计划.md)。

更大规模的 `tests/mbd_v2_fixtures/` snapshot 集合留到 Phase 3 Step 2 以后补齐。

---

## 阶段验证标准

1. 默认不运行 `cargo test`；如需确认可编译，优先使用最小范围 `cargo check`。
2. TextMeasurement 与 PDMS `mbdtextlen` 对每个字符宽度保持一致。
3. SmallDimSolver 对 3 种典型输入（正常段、短段需缩字高、短段需错层）产出正确 DimRow。
4. PrimitiveAssembler 对 V1 LayoutResult fixture 产出正确 MbdPrimitive 列表。
5. 后端阶段验证优先使用 CLI / HTTP JSON；最终完成标准必须走 plant3d-web 真实页面：`http://localhost:3101/?output_project=AvevaMarineSample&mbd_refno=24381_145712`。

---

## Phase 2 收尾 · 后续衔接

Phase 2 的四个底层模块 + 组装器已具备代码级检查用例；默认不运行 `cargo test`，后续以真实接口和真实页面验收为准。
下一步 Phase 3 分阶段推进：

| Step | 内容 | 状态 |
|------|------|------|
| Phase 3 Step 1 | `pipeline::build_mbd_v2_pipe_data` — V1 LayoutResult → V2 MbdV2PipeData | ✅ 完成 |
| Phase 3 Step 2 | `assembler::expand_linear_dim_chain` — SmallDimSolver 原子集成（手动分组） | ✅ 完成 |
| Phase 3 Step 2.5 | Chain 自动聚类 + pipeline `enable_small_dim_stacking` 开关 | ✅ 完成 |
| Phase 3 Step 3 | AvoidanceEngine 骨架（label–label 2D 避让 + pipeline `enable_avoidance`） | ✅ 完成 |
| Phase 3 Step 3.1 | `detect_leader_line_label_conflicts` — leader–label 2D 相交探测 | ✅ 完成 |
| Phase 3 Step 3.2 | `reroute_leader_lines_around_labels` — 2 点 leader → L 形折线 | ✅ 完成 |
| Phase 3 Step 3.3 | 3D AABB 真求交 + LinearDim 跨 chain 全局避让 | 待开始 |
| Phase 3 Step 3.4 | ≥3 点 leader 重路由 + 多 via 点 beam search | 待开始 |
| Phase 3 Step 4 | PolarSystem（或推迟至 Phase 2.5） | 待开始 |
| Phase 3 Step 5 | BranchCalculator v2（直接吃 MbdPipeData 产 MbdV2PipeData） | 待开始 |
