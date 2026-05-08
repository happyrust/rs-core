# MBD V2 直算重写计划

> **日期**：2026-05-05
> **目标**：彻底跳过 V1 LayoutResult，从 `BranchQueryResult` 直接产出 `MbdV2PipeData`
> **前置**：Phase 1–9a 已完成（154 测试通过，8918 行代码）

---

## 一、决策记录

| # | 决策点 | 结论 | 理由 |
|---|---|---|---|
| 1 | V1 兼容 | **不兼容** — 重写直算路径，不维护 V1 bridge | 减少维护成本，消除中间类型 |
| 2 | 几何函数 | **提取复用** — assembler 核心几何搬到 layout_engine.rs | text_frame/leader/chain 算法已验证 |
| 3 | 产出架构 | **单一 pipeline** — BranchQueryResult → MbdPrimitive 直出 | 不引入新中间类型 |
| 4 | PolarSystem | **dim_direction 主导 + PolarSystem 微调** | 渐进式引入，避免单点故障 |
| 5 | chain stacking | **算法复用 + V2 类型适配** | SmallDimSolver 零改动 |
| 6 | avoidance | **100% 复用**（1293 行零改动） | 纯 V2 类型输入 |
| 7 | 数据查询 | **复用 generate_mbd_data 子查询** | 已验证的焊缝/坡度/弯头逻辑 |
| 8 | 前端 | **后端先行，前端分阶段** | 后端可独立验收 |
| 9 | V1 处置 | **保留 2 周观察后删除** | fallback 安全网 |

---

## 二、模块复用分析

### 可 100% 复用（零改动）

| 模块 | 行数 | 说明 |
|---|---|---|
| `avoidance.rs` | 1293 | label/dim/leader 避让，纯 V2 类型 |
| `dim_direction.rs` | 347 | isoGetDimDir/isoGetBestDir |
| `iso_ori.rs` | 281 | pipedir/dimdir/chardir 坐标系 |
| `small_dim.rs` | 378 | SmallDimSolver 错层/字高自适应 |
| `text_measurement.rs` | 349 | PDMS 字符宽度表 |
| `polar_system.rs` | 913 | 柱坐标空间搜索 |
| `used_dir.rs` | 224 | 已用方向注册/冲突计数 |
| `member_positions.rs` | 206 | 管件位置收集/排序 |
| `leader_router.rs` | 131 | 引线路由 |
| `primitive.rs` | 692 | 11 种 primitive 类型定义 |
| `data_source.rs` | 298 | V2 数据源 + InMemoryDataSource |
| **小计** | **5112** | |

### 需要重写/替换

| 模块 | 当前行数 | 处置 | 新代码估算 |
|---|---|---|---|
| `assembler.rs` | 1838 | 标记 deprecated → 2 周后删 | 0（核心函数提取到 layout_engine） |
| `pipeline.rs` | 1339 | 重写直算入口 | ~100 行改动 |
| `branch_calculator.rs` | 555 | 重写为直算版 | ~300 行改动 |

### 新建

| 文件 | 估算行数 | 职责 |
|---|---|---|
| `layout_engine.rs` | ~800 | 直算核心：BranchQueryResult → Vec\<MbdPrimitive\> |

---

## 三、`layout_engine.rs` 设计

### 3.1 公共入口

```rust
pub fn compute_v2_primitives(
    qr: &BranchQueryResult,
    ctx: &LayoutEngineContext,
) -> (Vec<MbdPrimitive>, Vec<MbdV2Issue>)
```

### 3.2 内部流程

```
BranchQueryResult
  ├─ compute_segment_dims(members, bbox_center)
  │    └─ resolve_dim_direction → build_linear_dim_geometry → LinearDimPrimitive
  ├─ compute_port_dims(members)
  │    └─ arrive_axis/leave_axis → LinearDimPrimitive
  ├─ compute_weld_marks(welds)
  │    └─ WeldMarkPrimitive + LabelPrimitive + LeaderLinePrimitive
  ├─ compute_slope_marks(slopes)
  │    └─ SlopeMarkPrimitive + AidLinePrimitive + AidTextPrimitive
  ├─ compute_bend_marks(bends)
  │    └─ AngleDimPrimitive + AidLinePrimitive + AidArcPrimitive
  ├─ compute_tag_labels(tags)
  │    └─ LabelPrimitive + LeaderLinePrimitive
  ├─ group_and_stack_chains(linear_dims)
  │    └─ chain 分组 → SmallDimSolver 展开 → 替换原 dim
  ├─ PolarSystem 微调（可选）
  │    └─ extract_isolines → build_polar_system → adjust_offsets
  └─ avoidance（复用 avoidance.rs）
       ├─ resolve_linear_dim_text_conflicts
       ├─ resolve_label_label_conflicts
       ├─ reroute_leader_lines_around_labels
       └─ detect_leader_line_label_conflicts
```

### 3.3 从 assembler.rs 提取的函数

| 函数 | 原位置 | 新签名变化 |
|---|---|---|
| `text_frame_from_linear_dim` | L738 | 不变（已是纯几何） |
| `build_leader_for_label` | L786 | 不变（已接受 V2 类型） |
| `build_primitive_for_row_segment` | L1226 | 输入从 PlacedLinearDim 改为 (start,end,dir,text,...) |
| `assemble_weld` | L244 | 输入从 PlacedWeld 改为 WeldData |
| `assemble_slope` | L318 | 输入从 PlacedSlope 改为 SlopeData |
| `assemble_tag` | L478 | 输入从 PlacedTag 改为 TagData |
| `assemble_bend` | L527 | 输入从 PlacedBend 改为 BendData |
| `group_dims_into_chains` | L1105 | 输入从 &[PlacedLinearDim] 改为 &[LinearDimPrimitive] |
| 几何工具函数 | L658-720 | 不变（midpoint/dot/cross/normalize 等） |

### 3.4 LayoutEngineContext

```rust
pub struct LayoutEngineContext {
    pub cheight: f32,                    // 字高 mm（生产 100.0）
    pub pipe_od: f32,                    // 管段外径 mm
    pub bbox_center: Option<Vec3V2>,     // 分支包围盒中心
    pub default_orientation: Vec3V2,     // [1,0,0]
    pub default_up: Vec3V2,              // [0,0,1]
    pub arrow_len: f32,                  // 箭头长度
    pub lane_step_multiplier: f32,       // 偏移步进系数（1.2）
    pub enable_polar: bool,              // PolarSystem 微调
    pub enable_chain_stacking: bool,     // chain 分组+错层
    pub enable_avoidance: bool,          // 避让引擎
    pub avoidance_config: AvoidanceConfig,
    pub chain_tolerance: ChainTolerance,
    pub small_dim_params: SmallDimChainParams,
}
```

---

## 四、plant-model-gen 改造

### 4.1 `get_mbd_pipe_v2_direct` 数据补全

当前只填了 `members`，需要补充：

| 数据 | 来源 | 复用函数 |
|---|---|---|
| `welds` | `generate_mbd_data` 内部的焊缝探测 | 抽取为 `detect_welds_from_segments()` |
| `slopes` | `generate_mbd_data` 内部的坡度计算 | 抽取为 `compute_slopes_from_segments()` |
| `tags` | `generate_mbd_data` 内部的管件标签 | 抽取为 `extract_fitting_tags()` |
| `bends` | `generate_mbd_data` 内部的弯头识别 | 抽取为 `detect_bends_from_segments()` |
| `attrs` | `try_fill_branch_name_and_attrs` | 已实现 |
| `arrive_noun` | `CacheTubiSeg.arrive_noun` | 需确认字段是否存在 |

### 4.2 API 路由

```
GET /api/mbd/v2/pipe/{refno}
  └─ v2_direct=false → V1 bridge（保留 2 周）
  └─ v2_direct=true  → 新直算路径
```

直算验收通过后：
- `v2_direct` 默认值从 `false` 改为 `true`
- 2 周后删除 V1 bridge 路径

---

## 五、执行计划

### Phase A：layout_engine 骨架 + 管段标注（3 天）

- [ ] **A.1** 新建 `rs-core/src/mbd/v2/layout_engine.rs`
  - `LayoutEngineContext` 配置结构体
  - `compute_v2_primitives()` 入口
  - `compute_segment_dims()` — 从 `BranchMember` 产出 `LinearDimPrimitive`
  - `compute_port_dims()` — 从 arrive_axis/leave_axis 产出 `LinearDimPrimitive`
  - 从 assembler 提取：`build_linear_dim_geometry` + 几何工具函数
- [ ] **A.2** `pipeline.rs` 新增 `build_mbd_v2_pipe_data_v2()` 入口调 layout_engine
- [ ] **A.3** 8+ 单元测试覆盖 segment/port dim 产出
- [ ] **A.4** `cargo check --no-default-features` 编译通过

### Phase B：全类型标注 + 数据查询补全（2 天）

- [ ] **B.1** layout_engine 新增 `compute_weld_marks()`（从 assembler.assemble_weld 提取适配）
- [ ] **B.2** layout_engine 新增 `compute_slope_marks()`（从 assembler.assemble_slope 提取适配）
- [ ] **B.3** layout_engine 新增 `compute_tag_labels()`（从 assembler.assemble_tag 提取适配）
- [ ] **B.4** layout_engine 新增 `compute_bend_marks()`（从 assembler.assemble_bend 提取适配）
- [ ] **B.5** plant-model-gen: 从 `generate_mbd_data` 抽取 welds/slopes/tags/bends 查询函数
- [ ] **B.6** `get_mbd_pipe_v2_direct` 填充完整的 `BranchQueryResult`
- [ ] **B.7** 6+ 单元测试覆盖每种标注类型

### Phase C：chain stacking + PolarSystem（1 天）

- [ ] **C.1** layout_engine 新增 `group_and_stack_chains()`（V2 类型版 chain 分组）
- [ ] **C.2** SmallDimSolver 集成（零改动，调用接口不变）
- [ ] **C.3** PolarSystem 微调步骤集成（可选开关）
- [ ] **C.4** 4+ 单元测试

### Phase D：验收 + 基线锁定（1 天）

- [ ] **D.1** 本地启动 plant-model-gen，`?v2_direct=true` 测试主样本 `24381_145712`
- [ ] **D.2** 运行 `batch-validate-v2-extended.sh` 100 条 BRAN
- [ ] **D.3** 建立直算基线 JSON（存入 `test_data/mbd_v2_direct_baseline.json`）
- [ ] **D.4** 全量回归：`cargo test -p aios_core -- mbd::`

---

## 六、验收标准

### 结构正确性

- [ ] 每条 BRAN：primitives 非空
- [ ] `linear_dim` 数量 ≥ V1 bridge 的 80%
- [ ] 每种 primitive kind 至少出现一次（如果 V1 有）
- [ ] 零 error issues
- [ ] 无 NaN/Infinity 坐标
- [ ] direction 不全为零

### 几何合理性

- [ ] 所有尺寸 `text_height > 0`
- [ ] extension_line 起终点与管段端点距离 < 容差
- [ ] label 和 leader_line 的 anchor 匹配

### 回归基线

- [ ] 100 条 BRAN 直算基线 JSON 建立
- [ ] 后续改动回归检查通过

---

## 七、风险缓解

| 风险 | 概率 | 影响 | 缓解 |
|---|---|---|---|
| 提取几何函数遗漏边界条件 | 中 | 标注位置偏移 | 逐函数对比 assembler 原实现 + 测试 |
| chain stacking V2 类型适配引入 bug | 中 | 尺寸分组错误 | 保留 assembler 原测试用例适配 |
| plant-model-gen 查询抽取不完整 | 中 | 焊缝/坡度遗漏 | 对比 V1 bridge 同一 BRAN 的产出 |
| PolarSystem 微调与直算产出不匹配 | 低 | 方向异常 | 微调开关可关闭 |
| 2 周观察期内发现 V1 退化 | 低 | 需要回滚 | `v2_direct=false` 立即回退 |

---

## 八、文件变更预览

```
新增  rs-core/src/mbd/v2/layout_engine.rs          (~800 行)
修改  rs-core/src/mbd/v2/pipeline.rs               (~100 行改动)
修改  rs-core/src/mbd/v2/branch_calculator.rs       (~300 行改动)
修改  rs-core/src/mbd/v2/mod.rs                     (注册 layout_engine)
修改  plant-model-gen/src/web_api/mbd_pipe_api.rs   (~200 行改动)
标记  rs-core/src/mbd/v2/assembler.rs               (deprecated, 2 周后删)
新增  MBD-V2-开发规划/2026-05-05-mbd-v2-rewrite-plan.md (本文件)
```

---

> **上次更新**：2026-05-05 — 决策收敛，计划就绪
> **下次更新**：Phase A 完成后
