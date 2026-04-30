# MBD 标注方向移植 — 研究发现

## PML 方向算法核心逻辑

### isoGetDimDir（标注偏移方向决策）

```
输入：pipedir（管段方向），dimdirs[1..3]（3 个优选方向）
输出：dimdir（标注偏移方向）

算法：
1. dimdir = pipedir.orthogonal(dimdirs[3])
   → 管段方向与第 3 优选方向叉积，得到垂直于管段且偏向前两个优选方向的方向
2. 如果 pipedir 与 dimdirs[3] 平行（叉积为零），fallback 用 dimdirs[2]
3. 用 isoGetBestDir 微调方向一致性
```

**Rust 等价**：`glam::Vec3::cross()` + 方向翻转

### isoGetBestDir（方向一致性校正）

```
输入：inputdir（初步方向），dirs[1..3]（优选方向列表）
输出：调整后的方向

算法（2023-05-10 新版）：
1. 分别计算 inputdir 与 dirs[1], dirs[2], dirs[3] 的夹角
2. 对每个夹角取 min(angle, 180 - angle) → 绝对夹角
3. 排序找最小绝对夹角对应的方向索引 num
4. 如果 inputdir 与 dirs[num] 的原始夹角 > 90°，则取反

本质：确保输出方向与"最接近的优选方向"同侧（<90°）
```

**Rust 等价**：3 次 `Vec3::angle_between()` + 排序 + 条件翻转

### CalculateDimChardirs（优选方向推断）

```
输入：branname（分支名，用于获取包围盒）
输出：dimdirs[1..3]（优选标注方向），chardirs[1..3]（优选字符方向）

算法：
1. 获取 bran 包围盒 volume → 解析起止坐标 → 取中点 pos
2. 取标注线中点 linemidpos = poss[1].midpoint(poss.last())
3. 计算 linemidpos → pos 的方向 dir（从管线指向包围盒中心）
4. 在 E/N/W/S 四个方向中，按与 dir 的夹角排序
5. 优选标注方向 dimdirs = [最近方向.opposite(), 第二近.opposite(), U]
   → 标注向"远离包围盒中心"的方向偏移
6. 优选字符方向 chardirs = [U, 最近方向, 第二近方向]
```

**关键洞察**：标注方向的选择依赖于管道在工厂中的位置——管道在工厂内侧时标注向外偏移。

### isoUsedDir（已用方向占位）

```
结构：
  name: string      — 标识名（如 "ISODIM", "ISODIM-1"）
  direction: dir    — 占用的方向
  min_dis: real     — 沿管段方向的最小距离
  max_dis: real     — 沿管段方向的最大距离
  kind: string      — 类型（"MainDim", "SlopeDim-1" 等）

用途：
  每次标注放置后，记录该标注占用的方向+距离区间。
  后续标注在选择偏移方向和偏移量（dimtimes）时，
  检查是否与已用方向冲突，冲突则 dimtimes + 1（推到更远层）。
```

## 当前 V2 代码现状

### assembler.rs 中的方向处理

```rust
// 当前实现（简化版）：
let (text_orientation, text_up) =
    text_frame_from_linear_dim(dim_line_start, dim_line_end, start, end, dir, ctx);
```

`text_frame_from_linear_dim` 基于 dim_line 的 start-end 向量推导文字朝向，
但没有考虑"优选方向"和"已用方向惩罚"。

### PlacedLinearDim 携带的信息

```rust
pub struct PlacedLinearDim {
    pub start: [f32; 3],        // 管段起点
    pub end: [f32; 3],          // 管段终点
    pub direction: [f32; 3],    // 标注偏移方向（V1 solver 已计算）
    pub offset: f32,            // 偏移距离
    pub text: String,
    pub text_anchor: Option<[f32; 3]>,
    ...
}
```

**发现**：V1 的 `direction` 字段已经包含了标注方向信息，但它是由 V1 MVP solver 
简单计算的，没有走 PML 的优选方向逻辑。

## 关键差异

| 方面 | PDMS/PML | V2 当前 |
|------|----------|---------|
| 标注方向决策 | 3 优选方向 + 包围盒中心 + 已用方向惩罚 | 固定 default_orientation |
| 方向一致性 | isoGetBestDir 确保同侧 | 无 |
| 多层偏移 | dimtimes 基于 isoUsedDir 递增 | avoidance 的 lane_step |
| 斜管分解 | dimslope → 水平+垂直+直角标 | PlacedSlope 简化处理 |
| 字符方向 | isoOri.chardir 独立于管段方向 | 默认 up 方向 |

## 集成点分析（Phase 6 准备）

### plant-model-gen 中的 V2 调用位置

```rust
// plant-model-gen/src/web_api/mbd_pipe_api.rs L614-620
let ctx = MbdV2PipelineContext {
    input_refno: input_refno_enum.to_string(),
    branch_refno: data.branch_refno.clone(),
    branch_attrs: branch_attrs_to_mbd_v2_map(&data.branch_attrs),
    ..MbdV2PipelineContext::production_defaults()
};
let v2_data = build_mbd_v2_pipe_data(layout, &ctx);
```

**发现**：当前代码使用 `production_defaults()` 创建 ctx，
`bran_bbox_center` 默认为 `None`。要启用方向自动推断需要：

1. 在 `generate_mbd_data` 或 `get_mbd_pipe_v2` 中查询 bran 包围盒
2. 设置 `ctx.assembler.bran_bbox_center = Some([cx, cy, cz])`

### bran 包围盒查询方案

方案 A：从 SurrealDB 查询（推荐）
```sql
SELECT math::mean(poss) AS center FROM inst_geo WHERE pe = $bran_refno
```

方案 B：从 LayoutResult 的管段端点计算
```rust
let all_points: Vec<[f32;3]> = layout.linear_dims.iter()
    .flat_map(|d| [d.start, d.end])
    .collect();
let center = compute_aabb_center(&all_points);
```

方案 B 更简单且不需要额外数据库查询，推荐先用方案 B 快速集成。

## 移植风险

1. **CalculateDimChardirs 依赖 bran volume**：V2 pipeline 可能缺少包围盒信息，
   需要从上游（plant-model-gen）传入或计算
2. **addmem 逻辑高度依赖 PDMS 管件类型系统**：直接翻译会引入大量 PDMS 类型判断，
   需要在 Rust 侧做类型抽象
3. **isoOri 的 minangle 参数**：PML 代码中出现了 30° 和 60° 两种值，
   不同调用路径使用不同值，需要确认语义
