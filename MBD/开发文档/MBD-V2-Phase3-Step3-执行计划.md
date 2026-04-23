# MBD V2 Phase 3 · Step 3 执行计划

> **状态**：进行中
> **开始日期**：2026-04-21
> **前置**：Phase 3 Step 2.5（chain 自动聚类 + stacking）已完成 ✅

---

## 一、背景

到 Step 2.5 为止，V2 pipeline 产出的 primitive 已经有"单段↔链式+level 错层"的能力，但 **label 之间仍可能重叠**：
- `LabelPrimitive`（管件 / 焊标签 / tag）的 `text_anchor` 是从 V1 `label_offset_world` 直接透传的，没有做多 label 之间的冲突检测。
- `LinearDimPrimitive.text` 之间，Step 2.5 的错层逻辑只对**同一 chain 内**起效；跨 chain 之间不避让。

本 step 做 **AvoidanceEngine 骨架**，首先实现 **label–label 2D 冲突检测 + lane 分配**；更高级的 label–line / 3D 真 AABB 避让留到 Step 3.x。

---

## 二、范围

### 2.1 新增模块

`src/mbd/v2/avoidance.rs`：

```rust
/// 避让引擎的全局配置。
#[derive(Debug, Clone)]
pub struct AvoidanceConfig {
    /// 每次 lane bump 的偏移系数：`offset = height_mm * lane_step_multiplier`。默认 1.2。
    pub lane_step_multiplier: f32,
    /// 最大 lane 数；超过则发 Issue 不再 bump。默认 6。
    pub max_lanes: u16,
    /// 两个文字 bbox 间的最小间距（mm），用于判"几乎相撞"。默认 0.5。
    pub min_gap_mm: f32,
}

impl Default for AvoidanceConfig { /* 1.2, 6, 0.5 */ }

/// 对一批 primitives 里的 [`LabelPrimitive`] 做 label–label 避让。
///
/// 算法：
/// 1. 在每个 label 的平面投影（由 `orientation` / `up` 张成）上算 AABB。
/// 2. 按 `anchor` 沿 `orientation` 方向排序、扫描。
/// 3. 与已放置 labels 的 AABB 求交：若相撞，把当前 label 沿 `up` 抬高一个 lane。
/// 4. 超过 `max_lanes` 的 label 停在最后一层并发 `IssueCategory::Avoidance`。
///
/// 写回：修改 `LabelPrimitive.text_anchor`（沿 `up` 加偏移）。
pub fn resolve_label_label_conflicts(
    primitives: &mut [MbdPrimitive],
    config: &AvoidanceConfig,
) -> Vec<MbdV2Issue>;
```

### 2.2 AABB 计算

对每个 label：

```text
width_mm  = mbd_text_width(content, height_mm)
corners = [
    text_anchor,
    text_anchor + orientation * width_mm,
    text_anchor + orientation * width_mm + up * height_mm,
    text_anchor + up * height_mm,
]
```

把 `corners` 投影到 `orientation` / `up` 的 2D 坐标系（以 `text_anchor` 为原点）：
- `u_min / u_max` = 沿 `orientation` 方向 → `[0, width_mm]`
- `v_min / v_max` = 沿 `up` 方向 → `[0, height_mm]`

### 2.3 2D 冲突检测

两个 label `A` / `B`：
- 必须先确认 **`A.orientation ≈ B.orientation`** 且 **`A.up ≈ B.up`** 且 **`A.anchor` 到 `B.anchor` 的向量在 `orientation × up` 方向上几乎为 0**（即共面），才做 2D 求交；否则视为不冲突（MVP 简化）。
- 共面时把两者 `anchor` 差投影到 `orientation`、`up` 得到相对 `(du, dv)`，再按 AABB 相交。

### 2.4 Lane 分配

```text
for label in labels_sorted_by_u:
    lane = 0
    loop:
        shifted_anchor = label.anchor + up * (lane * height_mm * lane_step_multiplier)
        if 不与已放置 labels 相撞（带 min_gap）:
            提交 shifted_anchor
            break
        else:
            lane += 1
            if lane > max_lanes:
                产 Issue；提交最终 lane 的 anchor（不再 bump）
                break
```

### 2.5 Pipeline 集成

`MbdV2PipelineContext` 加：

```rust
pub enable_avoidance: bool,          // 默认 false
pub avoidance_config: AvoidanceConfig,
```

`build_mbd_v2_pipe_data`：产出 primitives 后，若 `enable_avoidance = true`，先 `resolve_label_label_conflicts(&mut primitives, &config)`，把产生的 Issue 追加到 pipeline issues。

### 2.6 不做

| 项 | 后续 step |
|----|----------|
| Label–LeaderLine 相交避让 | Step 3.1 |
| LinearDim text 跨 chain 的 stacking | Step 3.2 |
| 3D AABB 真正求交 | Step 3.3 |
| WeldMark / AidPoint / AidText 参与避让 | Step 3.x |

---

## 三、测试计划（`avoidance.rs::tests` 与 `pipeline.rs::tests`）

1. `two_overlapping_labels_get_separated`
   - 同 anchor 的两条 label 都是 `"100"` + height 2.5 → 第二条被推到 lane 1（y 增加 3.0）
2. `three_stacked_labels_get_three_lanes`
3. `non_overlapping_labels_are_untouched`
4. `labels_on_different_orientations_do_not_interact`
5. `exceeding_max_lanes_produces_issue`
   - `max_lanes = 1`，3 条完全重叠 → 第 3 条产 Issue（category=Avoidance）
6. `pipeline_enable_avoidance_moves_labels_and_surfaces_issues`
   - pipeline 级别：enable_avoidance=true + 构造重叠输入，验证 primitives 被改、issues 收到 Avoidance

---

## 四、验收标准

1. `cargo check -p aios_core --lib --tests` 成功
2. 新文件 + 改动 clippy 干净
3. `MBD-V2-Phase2-执行计划.md` 的 step 表更新：Step 3 ✅

---

## 五、风险

| 风险 | 缓解 |
|------|------|
| 共面判定的 tolerance 选择可能太严/太松 | MVP 取 1e-3（单位向量差）+ 0.5mm（共面偏离）；单测覆盖两种场景 |
| Lane 分配贪心依赖排序稳定性 | 测试中固定输入顺序；真实场景记入 Step 3.1 改 beam search |
| 对 `LinearDimPrimitive.text` 不参与避让 | 本 step 只处理 LabelPrimitive；LinearDim 走 chain stacking（Step 2.5）已有错层 |
