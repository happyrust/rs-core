# MBD V2 Phase 3 · Step 2.5 执行计划

> **状态**：进行中
> **开始日期**：2026-04-21
> **前置**：Phase 3 Step 2（`expand_linear_dim_chain`）已完成 ✅

---

## 一、背景

Step 2 只提供了 **原子工具** `expand_linear_dim_chain`，但调用方需要自己把 `PlacedLinearDim` 预分组。实际上，上游 `plant-model-gen` 拿到的是 `LayoutResult.linear_dims`/`cut_tubis` 两个大数组，手动分组工作量大也容易错。

本 step 让 `pipeline` 能**自动**把同基线的 segment 聚类成 chain，并在启用 `small_dim_stacking` 时走 `expand_linear_dim_chain` 替代单段路径。

---

## 二、范围

### 2.1 新增公开项（`assembler.rs`）

```rust
/// 链式聚类的容差。
#[derive(Debug, Clone)]
pub struct ChainTolerance {
    /// direction 单位向量的 per-axis 量化步长（越小越严）。默认 1e-3。
    pub direction_quant: f32,
    /// offset 量化步长（mm）。默认 0.1。
    pub offset_quant: f32,
    /// 端点连接的距离容差（mm）。默认 0.5。
    pub endpoint_tolerance: f32,
}

impl Default for ChainTolerance { /* 上述默认值 */ }

/// 一个聚类后的 chain 组；indices 指向原 dims slice 的下标。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainGroup {
    pub indices: Vec<usize>,
}

/// 把一批 `PlacedLinearDim` 自动聚类成 chain 组。
///
/// 算法（MVP）：
/// 1. 按 `(quantize(direction), quantize(offset))` 分桶（非法方向/offset NaN 落单桶）。
/// 2. 桶内按端点连接：任意 dim 的 end 与另一条 dim 的 start 距离 ≤ endpoint_tolerance。
/// 3. 无法成链的 dim 落单成一组（`indices.len() == 1`）。
///
/// 返回的组**按第一段 indices[0] 升序**，保证顺序稳定。
pub fn group_dims_into_chains(
    dims: &[PlacedLinearDim],
    tolerance: &ChainTolerance,
) -> Vec<ChainGroup>;
```

### 2.2 新增公开项（`pipeline.rs`）

`MbdV2PipelineContext` 增加：

```rust
pub enable_small_dim_stacking: bool,   // 默认 false，保持 Step 1 行为
pub chain_tolerance: ChainTolerance,   // 默认值见 ChainTolerance::default
pub small_dim_params: SmallDimChainParams, // 给 expand 用
```

`build_mbd_v2_pipe_data` 改造：

1. 若 `enable_small_dim_stacking = false`：走 Step 1 路径（`assemble_v2_primitives`）
2. 若为 true：
   - 对 `layout.linear_dims` / `layout.cut_tubis` 分别调 `group_dims_into_chains`
   - 每组 `indices.len() >= 2` 时走 `expand_linear_dim_chain`
   - `indices.len() == 1` 时走单段 `assemble_linear_dim`（复用现有 weld/slope/tag/bend 装配）
   - `welds` / `slopes` / `tags` / `bends` 行为不变

### 2.3 不做

- 跨 `linear_dims` / `cut_tubis` 的混合聚类
- 非直线（direction 实时计算）的链式识别
- 聚类失败时的显式 Issue（后续 Step 3 AvoidanceEngine 阶段再补）
- UI 可视化

---

## 三、测试计划

### 3.1 聚类单测（`assembler.rs::tests`）

1. `group_empty_dims_returns_empty`
2. `group_single_dim_forms_single_group`
3. `group_three_consecutive_dims_form_one_chain`
   - 3 段端点 0→500→800→1200，同方向同 offset → 1 组，indices = [0,1,2]
4. `group_two_unconnected_dims_form_two_groups`
   - 0→500 + 1000→1500，端点有间隙 → 2 组
5. `group_different_direction_goes_to_separate_buckets`
   - 0→500（dir=Y）+ 0→500（dir=X）→ 2 组
6. `group_tolerant_endpoint_snap`
   - 0→500 + 500.2→1000（容差 0.5 范围内）→ 1 组

### 3.2 pipeline 单测（`pipeline.rs::tests`）

1. `disabled_small_dim_stacking_matches_step1_behavior`
   - 3 条同基线 linear_dims，`enable_small_dim_stacking = false` → primitives 行为与原先一致（单段装配）
2. `enabled_small_dim_stacking_produces_chain_expansion`
   - 3 条同基线连续 linear_dims，enabled → 3 条 primitive 来自 chain expand，level=0 全一致
3. `enabled_small_dim_stacking_with_short_segment_triggers_level_bump`
   - 同 3.1 #6 的场景叠加短段 → 至少一条 primitive level>0

---

## 四、验收标准

1. `cargo check -p aios_core --lib --tests` 成功
2. `mbd/v2/assembler.rs` / `pipeline.rs` clippy 干净
3. 更新 `MBD-V2-Phase2-执行计划.md` 的 step 概览表：Step 2.5 标 ✅

---

## 五、风险

| 风险 | 缓解 |
|------|------|
| 容差选择不当导致假阳/假阴聚类 | MVP 取保守默认值；单测覆盖典型场景 |
| 端点匹配只做"end → start"单向 | 足够覆盖 PDMS 典型链式；反向 / 交叉在 Step 3 AvoidanceEngine 阶段处理 |
| 聚类后 `expand` 的 primitive id 规则 `{id}/r{level}s{i}` 与 V1 单段 id 风格不同 | 单段时回退到 `assemble_linear_dim`（保持 `source.id` 原样） |
