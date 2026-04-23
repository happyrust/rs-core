# MBD V2 Phase 3 · Step 3.1 执行计划

> **状态**：进行中
> **开始日期**：2026-04-21
> **前置**：Phase 3 Step 3（label–label 2D 避让）已完成 ✅

---

## 一、背景

Step 3 解决了 **label vs label** 的 2D 重叠。但是 `LeaderLinePrimitive` 的直线引线仍可能穿过其它 `LabelPrimitive` 的文字 bbox，视觉观感差。

Step 3.1 做**检测 + 诊断**：扫描所有 leader line 与 label 的几何相交，把问题以 `MbdV2Issue`（`category = Avoidance`）报出，供前端或上游系统做降级处理。真正的 **重路由（reroute）** 留到 Step 3.2。

---

## 二、范围

### 2.1 新增 API（`avoidance.rs`）

```rust
/// 扫描 primitives 中每条 `LeaderLinePrimitive` 与每个 `LabelPrimitive`
/// 文字 bbox 的 2D 相交关系，返回 Issue 列表（不修改 primitives）。
///
/// 共面假设：leader line 点和 label 的 `text_anchor` 需在 label 平面
/// 法线方向上的偏差 ≤ `config.coplanar_offset_tolerance`。
pub fn detect_leader_line_label_conflicts(
    primitives: &[MbdPrimitive],
    config: &AvoidanceConfig,
) -> Vec<MbdV2Issue>;
```

### 2.2 关键算法

对每条 leader line `L = [p_0, p_1, ..., p_k]`：
- 扫描所有相邻 segment `[p_i, p_{i+1}]`；
- 对每个 label，检查 segment 是否与 label bbox 相交：
  1. 若 leader segment 在 label 平面法线方向的两端点偏差都 > `coplanar_offset_tolerance`：视为不共面，跳过
  2. 投影 segment 两端到 label 的 `(orientation, up)` 2D 坐标系（以 `text_anchor` 为原点）
  3. 用 **2D 线段 vs AABB 求交**（Cohen–Sutherland 或参数法）判定相交
  4. 相交：记 Issue `id = "leader-crosses-{leader.id}-vs-{label.id}-seg{i}"`

`AABB = [0, width_mm] × [0, height_mm]`，`width_mm = mbd_text_width(label.content, label.height_mm)`。

### 2.3 线段–AABB 相交判定

采用**参数法**：
- 计算 segment 端点 `(u0,v0) → (u1,v1)`；
- 若两端点都落在 AABB 某侧外（u<0 且 u<0；u>w 且 u>w；v<0 且 v<0；v>h 且 v>h），则不相交；
- 若任一端点落在 AABB 内，则相交；
- 否则计算 segment 与 AABB 四条边的参数 t，若 `[t_enter, t_exit] ∩ [0, 1]` 非空，则相交。

（此算法简单鲁棒，足够 MVP。）

### 2.4 pipeline 集成

`build_mbd_v2_pipe_data` 在 `enable_avoidance = true` 分支内，于 `resolve_label_label_conflicts` 之后调用 `detect_leader_line_label_conflicts`，合并 Issue。

复用已有 `enable_avoidance` 开关；不引入新字段。

### 2.5 不做

| 项 | 后续 step |
|----|----------|
| Leader line 重路由（调整 points 绕开 label） | Step 3.2 |
| Leader line vs dim_line / extension line 相交 | Step 3.2 |
| 3D 真 AABB 求交（考虑 label 厚度） | Step 3.3 |

---

## 三、测试计划

`avoidance.rs::tests` 新增：

1. `leader_crossing_label_bbox_produces_issue`
   - leader 从 label 中心穿过 → 1 条 Issue
2. `leader_not_crossing_label_bbox_yields_no_issue`
   - leader 远离 label → 0 Issue
3. `leader_touching_label_corner_counts_as_conflict`
   - leader 端点就是 label anchor → 相交（endpoint inside AABB）
4. `leader_on_different_plane_is_ignored`
   - leader 在法线方向偏离 label 过大 → 忽略
5. `multi_segment_leader_each_segment_checked`
   - 折线 leader 的第 2 段穿过 label → Issue id 含 `seg1`

`pipeline.rs::tests`（1 个新增）：

- `pipeline_avoidance_surfaces_leader_label_conflicts`
  - enable_avoidance + 构造"tag 正好挨在 leader 路径上"的 layout → issues 里有 leader-crosses-\* 项

---

## 四、验收标准

1. `cargo check -p aios_core --lib --tests` 成功
2. `mbd/v2/avoidance.rs` / `pipeline.rs` 无新 clippy 告警
3. 更新 `MBD-V2-Phase2-执行计划.md` step 表：Step 3.1 ✅

---

## 五、风险

| 风险 | 缓解 |
|------|------|
| 线段–AABB 判交有浮点误差 | 判交带 1e-6 的 epsilon；单测覆盖"端点在 bbox 边上"边界情况 |
| 共面判定过严导致漏报 | `coplanar_offset_tolerance` 默认 0.5mm；3D 真求交留 Step 3.3 |
| Issue 膨胀（N leader × M label = NM 条） | 遇到 > 100 条时只保留前 100 条 + 总数摘要（MVP 不做；后续可加） |
