# MBD V2 Phase 3 · Step 3.2 执行计划

> **状态**：进行中
> **开始日期**：2026-04-21
> **前置**：Phase 3 Step 3.1（leader–label 探测）已完成 ✅

---

## 一、背景

Step 3.1 把穿过其他 label 的 leader line 以 `MbdV2Issue` 的形式报出了，但并未修改几何。本 step 把"探测"升级为"修复"：对满足条件的 2 点 leader，尝试插入 1 个折点变成 L 形折线，使其绕开相交的 label bbox。

完成后 pipeline 的避让链条变成：

```
resolve_label_label_conflicts   (Step 3)
      ↓
reroute_leader_lines_around_labels  (Step 3.2 · 本次)
      ↓
detect_leader_line_label_conflicts  (Step 3.1，残留的报 Issue)
```

---

## 二、范围

### 2.1 新增 API（`avoidance.rs`）

```rust
/// 对 primitives 里 2 点的 `LeaderLinePrimitive` 做重路由，把穿过其他
/// `LabelPrimitive` bbox 的 leader 改成 3 点 L 形折线。
///
/// 修改 primitives；返回因路径上依然相交、超过 `max_leader_reroute_attempts`
/// 而发出的 Issue（category = Avoidance）。
pub fn reroute_leader_lines_around_labels(
    primitives: &mut [MbdPrimitive],
    config: &AvoidanceConfig,
) -> Vec<MbdV2Issue>;
```

### 2.2 算法（MVP）

对每条 leader line `L`：

1. 若 `L.points.len() != 2`：跳过（MVP 只处理直线 leader）
2. 遍历所有 label：若 L 与该 label bbox 相交，记录为"待绕开"
3. 对每个待绕开 label：
   - 在 label 的 2D 平面上，把 bbox 的 4 个角 + 中点候选作为 `via` 点
   - 对每个候选 via：构造 `L' = [L.points[0], via_world, L.points[1]]`
     - via_world = text_anchor + orientation * via_u + up * via_v
   - 若 L' 的两段都**不穿过任何 label bbox**（包括刚才遍历过的其他 label），接受该 reroute
   - 否则尝试下一个候选，直到用尽
4. 重复以上流程至多 `max_leader_reroute_attempts` 次（默认 3）
5. 超出后仍有相交：写入 Issue（`id = "leader-reroute-failed-{leader.id}"`）

**候选 via 点**（按顺序试，选第一个成功的）：
- 4 个 bbox 角，沿 label 平面推出一个小 `margin`（默认 0.2mm），共 8 个候选
- 再加上"L 形折点"：两条 leader 端点在 label 平面 2D 坐标下 (u0,v0) 和 (u1,v1) 的两个拐点 `(u0, v1)` / `(u1, v0)`（也同样推出 margin）

**margin 字段**：`AvoidanceConfig.leader_reroute_margin_mm`（默认 0.2）。

### 2.3 `AvoidanceConfig` 扩展

```rust
pub struct AvoidanceConfig {
    ...
    /// 最大 leader 重路由尝试次数。默认 3。
    pub max_leader_reroute_attempts: u16,
    /// 重路由 via 点相对 bbox 的外扩量（mm）。默认 0.2。
    pub leader_reroute_margin_mm: f32,
}
```

### 2.4 Pipeline 集成

`build_mbd_v2_pipe_data` 在 `enable_avoidance = true` 分支：

```rust
issues.extend(resolve_label_label_conflicts(&mut primitives, &ctx.avoidance_config));
issues.extend(reroute_leader_lines_around_labels(&mut primitives, &ctx.avoidance_config));
issues.extend(detect_leader_line_label_conflicts(&primitives, &ctx.avoidance_config));
```

### 2.5 不做

| 项 | 后续 step |
|----|-----------|
| ≥3 点 leader 的复杂重路由 | Step 3.3 |
| 3D 真 AABB 求交 | Step 3.3 |
| LinearDim 跨 chain 避让 | Step 3.3 |
| 基于最短路径 / A* 的最优 reroute | 暂不做；MVP 走贪心 |

---

## 三、测试计划（`avoidance.rs::tests`）

1. `leader_crossing_single_label_is_rerouted_to_3_points`
   - leader 穿过 1 个 label → points.len() 从 2 → 3，且新 leader 不再穿 bbox
2. `leader_not_crossing_is_untouched`
3. `leader_with_3_points_already_is_skipped`
4. `leader_crossing_multiple_labels_retries`
5. `unrouteable_leader_emits_issue`
   - 多个重叠 label 完全挡住 → 超过 max_attempts 发 Issue
6. 还有 pipeline 单测：`pipeline_avoidance_reroutes_leader_before_detection`
   - enable_avoidance=true，构造 leader+label 相交 → pipeline 完成后 leader.points.len() == 3，且 issues 无 leader-crosses-\*（因为已重路由）

---

## 四、验收标准

1. `cargo check -p aios_core --lib --tests` 成功
2. `mbd/v2/avoidance.rs` / `pipeline.rs` clippy 干净
3. Step 表更新：Step 3.2 ✅

---

## 五、风险

| 风险 | 缓解 |
|------|------|
| 候选 via 点的选择顺序可能导致次优解 | MVP 用贪心；后续 Step 3.3 引入代价函数（总长度 + 弯折数） |
| margin 太小/太大 | 默认 0.2mm 经验值；单测用 1.0mm 的 margin 做边界检查 |
| 3 点 leader 处理器漏覆盖本应修复的场景 | 本 step 只改 2 点 leader；已有 3 点 leader 的后续处理在 Step 3.3 |
