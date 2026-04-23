# MBD V2 Phase 3 · Step 1 执行计划

> **状态**：进行中
> **开始日期**：2026-04-21
> **前置**：Phase 1（契约）+ Phase 2（四底层模块）已完成 ✅
> **目标 Phase**：Phase 3 · 后端 AvoidanceEngine + V2 API（本 step 仅覆盖 V2 API 的 rs-core 入口）

---

## 一、定位与动机

Phase 2 收尾后，rs-core 侧已经有了：

- `mbd::v2::{primitive, text_measurement, small_dim, leader_router, assembler}` 五个模块
- `assemble_v2_primitives(layout, ctx) -> (Vec<MbdPrimitive>, Vec<MbdV2Issue>)`

但是 **缺一个"从 V1 `LayoutResult` 到顶层 `MbdV2PipeData` 的入口"**，上游 `plant-model-gen` 要接入就得手工拼装 `version` / `meta` / `issues`，不利于契约稳定。

本 step 就是把这层"最薄胶水"做出来：

```
LayoutResult ──► build_mbd_v2_pipe_data(layout, ctx) ──► MbdV2PipeData
                                                              │
                                                              ▼
                                                   V2 API 直接 serde 成 JSON
```

---

## 二、范围

### 2.1 新增文件

| 文件 | 作用 |
|------|------|
| `src/mbd/v2/pipeline.rs` | 定义 pipeline 入口 + 上下文 + meta 汇总 |

### 2.2 新增公开项

在 `mbd::v2::pipeline`：

- `pub struct MbdV2PipelineContext`
  - `pub input_refno: String`
  - `pub branch_refno: String`
  - `pub branch_attrs: BTreeMap<String, String>`
  - `pub assembler: AssemblerContext`
  - `pub generated_at_override: Option<String>`（测试用，传空走 `chrono::Utc::now()`）

- `pub fn build_mbd_v2_pipe_data(layout: &LayoutResult, ctx: &MbdV2PipelineContext) -> MbdV2PipeData`

内部：

- `fn compute_meta(layout, branch_attrs, generated_at) -> MbdV2Meta`
  - `segments_count = layout.linear_dims.len() as u32`
  - `welds_count = layout.welds.len() as u32`
  - `dims_by_kind`：对 `linear_dims + cut_tubis + bends.size_dims` 按 `PlacedLinearDim.kind` 汇总
  - `branch_attrs` 透传
  - `generated_at` 使用 override 或 `chrono::Utc::now().to_rfc3339()`

- `fn collect_suppression_issues(layout) -> Vec<MbdV2Issue>`
  - 把 `LayoutResult.suppressed_items` 里的项翻译成 `MbdV2Issue`（`severity = Warning`, `category = Layout`, `message` 格式 `"{kind}:{reason}"`）
  - primitive 级别的 issue 来自 `assemble_v2_primitives`，两组合并

### 2.3 在 `v2/mod.rs` 重导出

```rust
pub mod pipeline;

pub use pipeline::{MbdV2PipelineContext, build_mbd_v2_pipe_data};
```

### 2.4 更新文档

更新 `MBD-V2-Phase2-执行计划.md` Step 5 状态（golden test fixtures 留给后续 step）。

---

## 三、接口草案

```rust
pub struct MbdV2PipelineContext {
    pub input_refno: String,
    pub branch_refno: String,
    pub branch_attrs: BTreeMap<String, String>,
    pub assembler: AssemblerContext,
    pub generated_at_override: Option<String>,
}

impl Default for MbdV2PipelineContext { /* AssemblerContext::default + 空串 */ }

pub fn build_mbd_v2_pipe_data(
    layout: &LayoutResult,
    ctx: &MbdV2PipelineContext,
) -> MbdV2PipeData {
    let (primitives, mut issues) = assemble_v2_primitives(layout, &ctx.assembler);
    issues.extend(collect_suppression_issues(layout));

    let generated_at = ctx
        .generated_at_override
        .clone()
        .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());
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
```

---

## 四、不做（留给后续 step）

| 项 | 后续 step |
|----|-----------|
| AvoidanceEngine | Phase 3 Step 2 |
| SmallDimSolver 集成进 assembler（把 PlacedLinearDim 链式尺寸按 `DimRow` 展开） | Phase 3 Step 3 |
| PolarSystem | Phase 3 Step 4 或 Phase 2.5 |
| BranchCalculator v2（直接吃 MbdPipeData） | Phase 3 Step 5 |
| V2 HTTP API handler（在 `plant-model-gen`） | 交给 plant-model-gen 仓 |
| Fixture snapshot 测试 | Phase 3 Step 2 后补；本 step 先用合成 LayoutResult |

---

## 五、测试计划

在 `pipeline.rs` 内加 `#[cfg(test)] mod tests`：

1. `empty_layout_produces_zeroed_meta`
   - 空 LayoutResult → primitives 空，meta.segments_count=0，dims_by_kind 空，issues 空
2. `mixed_layout_counts_correctly`
   - 3 linear_dims + 2 welds + 1 slope + 1 tag → primitives 个数 = 3 + 2 + 2 + 1 + 1 = 9（weld 产出 weld_mark + label）
   - meta.segments_count = 3，welds_count = 2，dims_by_kind["segment"] = 3
3. `suppressed_items_emit_issues`
   - LayoutResult 含 2 条 suppressed_items → issues 长度 ≥ 2，category 均为 Layout
4. `dims_by_kind_aggregates_bend_size_dims`
   - bend.size_dims 里的 PlacedLinearDim.kind 也要算进 dims_by_kind
5. `generated_at_override_is_used`
   - override 非空时直接使用该字符串
6. `refnos_are_forwarded`
   - input_refno / branch_refno 原样透传到 MbdV2PipeData

---

## 六、验收标准

1. `cargo check -p aios_core --lib --tests` 成功、无 warning 增量
2. 新单测在代码级别自洽（AGENTS.md 规定默认不跑 `cargo test`；若用户后续要求再跑）
3. `MbdV2PipeData` 能序列化成 JSON 且 `kind` 字段正确出现在每个 primitive 上（复用 Phase 1 已有的 roundtrip 单测模式）
4. `MBD-V2-Phase2-执行计划.md` 的 Step 5 标注「Step 1 pipeline 入口已完成」

---

## 七、风险与备注

- `chrono::Utc::now()` 依赖系统时钟；单测用 `generated_at_override` 绕开
- `suppressed_items` 目前 message 里不带 refno；Phase 3 Step 2 做 avoidance 时再扩充结构化字段
- `MbdV2Meta.branch_attrs` 在本 step 只透传；真正提取 PDMS 属性放在 `plant-model-gen` 侧的 context builder
