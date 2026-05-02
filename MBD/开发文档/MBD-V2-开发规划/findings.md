# MBD V2 尺寸标注 — 架构发现与分析

> 最后更新：2026-05-01

---

## 1. 架构决策分析

### 1.1 V2 "后端已排版" 设计的核心价值

**发现**：V2 的设计目标是对齐 PDMS `json.getJson()` — 后端产出"已完成排版与避让的图元列表"，前端只做 `primitive.kind → three.js entity` 的 1:1 映射。

**影响**：
- 消除了 V1 时代前端 3000 行 `useMbdPipeAnnotationThree.ts` 的双算问题
- 前端不再需要 `computeDimensionOffsetDir`、`applyCutTubiLabelDeclutter` 等几何决策
- 新增标注类型只需后端产出新 primitive，前端加一个 renderer case

**陷阱**：当前是**过渡架构** — assembler.rs 仍然依赖 V1 `LayoutResult` 做翻译。真正的 V2 直算（BranchCalculatorV2）尚未实现。

### 1.2 过渡层（V1 bridge）的设计取舍

**发现**：`build_mbd_v2_pipe_data(layout: &LayoutResult, ctx: &MbdV2PipelineContext)` 是过渡入口，接收 V1 的 `LayoutResult` 并翻译为 V2 primitive。

**优点**：
- 不需要等 BranchCalculatorV2 完成就能让前端切 V2
- 所有后端排版算法（dim_direction、avoidance、small_dim stacking）已经在 V2 pipeline 中生效

**风险**：
- V1 `LayoutResult` 里的 `direction`、`offset` 等字段语义有限，assembler 做翻译时信息丢失
- V1 `PlacedLinearDim` 缺少 `extension_line_end`、`dim_line_start` 等精确点位，assembler 需要自己推算
- 链式尺寸的 chain 分组依赖端点连接关系，V1 数据的精度直接影响分组质量

### 1.3 production_defaults 的配置策略

**发现**：`MbdV2PipelineContext` 有两套默认值：
- `Default` — 保守兼容（stacking=false, avoidance=false, cheight=2.5）
- `production_defaults()` — Web 接口实际使用（stacking=true, avoidance=true, cheight=100.0）

**关注点**：`cheight=100.0` vs `Default` 的 `2.5` 差 40 倍。这意味着真实 Web 页面上文字高度是 100mm，而单元测试默认是 2.5mm。测试与生产的字高差异可能导致避让逻辑在测试中表现良好但生产中失效。

---

## 2. 模块级发现

### 2.1 text_measurement.rs — PDMS 精确复刻

**发现**：字符宽度表从 PML `mbdtextlen.pmlfnc` 逐字符翻译，保留了 PML 原文的 `x`/`X` 笔误（`s`/`S` 替代）。

**影响**：`'x'` 和 `'X'` 查不到表，走 DEFAULT_CHAR_WIDTH=1.02。这与 PDMS 行为一致。

**验证**：有完整单测覆盖，已确认对齐。

### 2.2 dim_direction.rs — 标注方向决策

**发现**：移植了 `isoGetDimDir`（管段方向叉积优选方向）和 `isoGetBestDir`（确保方向同侧校正）。

**当前局限**：`calculate_dim_char_dirs` 需要 `bran_bbox_center`（分支包围盒中心），当前通过 `infer_bbox_center_from_layout` 从 V1 layout 推导。V2 直算后需要从 SurrealDB 查询管段数据直接计算。

### 2.3 assembler.rs — V1→V2 翻译层

**发现**：assembler 是当前最大的单文件（~1800 行），负责：
1. `PlacedLinearDim → LinearDimPrimitive`（含 extension/dim_line/arrows/text 推算）
2. `PlacedWeld → WeldMarkPrimitive + LabelPrimitive + LeaderLinePrimitive`
3. `PlacedSlope → SlopeMarkPrimitive`
4. `PlacedTag → LabelPrimitive + LeaderLinePrimitive`
5. `PlacedBend → 弯头标注 primitive 组`
6. chain stacking（链式分组 + SmallDimSolver 展开）

**潜在问题**：
- `assemble_v2_primitives_with_chain_stacking` 和 `assemble_v2_primitives` 两条路径，chain stacking 路径较复杂
- chain 分组的 `ChainTolerance` 默认容差需要在更多样本上验证

### 2.4 avoidance.rs — 避让引擎

**发现**：三步避让流水线：
1. `resolve_linear_dim_text_conflicts` — 尺寸文字互相遮挡
2. `resolve_label_label_conflicts` — label 2D AABB 碰撞检测 + lane bump
3. `reroute_leader_lines_around_labels` — leader 绕行 label 的 AABB
4. `detect_leader_line_label_conflicts` — 检测遗留冲突，生成 Issue

**局限**：
- 仅做 2D 投影避让，不考虑 3D 遮挡
- `max_lanes=6`，超过后不再 bump，仅发 Warning Issue
- 相机旋转后可能产生新的遮挡（V2 不做相机相关避让是设计决策）

### 2.5 small_dim.rs — 小尺寸处理

**发现**：复刻 PDMS `lindim.sepSmallDim` + `changeCheightAuto`：
- 段长不够文字宽度时，先尝试缩字高（`change_cheight_auto_bili=0.5` 下限）
- 仍放不下则 `sep_small_dim` 错层（level 递增）
- 连续大段合并为一个 row，小段单独分行

**验证**：有单测覆盖 `solve_small_dims`，但生产字高 100mm 下的阈值行为需要更多样本验证。

---

## 3. 数据流关键路径

### 3.1 V2 API 请求链路

```
前端请求 → GET /api/mbd/v2/pipe/{refno}
    ↓
resolve_effective_branch_refno（HANG → BRAN 解析）
    ↓
mbd_v2_layout_query（强制 LayoutFirst + 开启全部标注类型）
    ↓
generate_mbd_data（V1 pipeline → LayoutResult）
    ├── SurrealDB 查询 tubi_relate
    ├── BranchCalculator V1 solve_branch
    └── LayoutResult { linear_dims, cut_tubis, welds, slopes, tags, bends }
    ↓
build_mbd_v2_pipe_data（V2 pipeline）
    ├── assemble_v2_primitives_with_chain_stacking
    │   ├── group_dims_into_chains → ChainGroup[]
    │   ├── expand_linear_dim_chain → SmallDimSolver → DimRow[]
    │   ├── assemble_weld / slope / tag / bend
    │   └── text_frame_from_linear_dim（dim_direction + iso_ori）
    ├── resolve_linear_dim_text_conflicts
    ├── resolve_label_label_conflicts
    ├── reroute_leader_lines_around_labels
    ├── detect_leader_line_label_conflicts
    └── collect_suppression_issues
    ↓
MbdV2PipeData { version:"v2", primitives[], meta, issues[] }
```

### 3.2 V1 数据依赖清单

当前 V2 依赖 V1 LayoutResult 的字段：

| V1 字段 | V2 使用方 | 去 V1 后替代方案 |
|---|---|---|
| `linear_dims` | assembler chain stacking | BranchCalculatorV2 直接产出 LinearDimPrimitive |
| `cut_tubis` | assembler CutTubi 子类型 | 同上 |
| `welds` | assembler WeldMark + Label | V2 直查焊缝数据 |
| `slopes` | assembler SlopeMark | V2 直算坡度 |
| `tags` | assembler Label + Leader | V2 直算管件标签 |
| `bends` | assembler 弯头标注 | V2 直算弯头角度/尺寸 |
| `suppressed_items` | pipeline issues | V2 自带 suppress 逻辑 |

---

## 4. PolarSystem 深度分析（2026-05-01 补充）

### 4.1 PolarSystem 的本质

PolarSystem 是一个**围绕管段轴线的 3D 空间放置优化器**，使用柱坐标系 `(dis, angle, radius)` 描述每个标注元素的空间占用：

- **dis**（轴向距离）：沿管段方向距起点的距离
- **angle**（角度）：绕管段轴线的角度（0–360°）
- **radius**（径向距离）：到管段轴线的距离

### 4.2 PML 代码规模

| 文件 | 行数 | 职责 |
|---|---|---|
| `polarsystem.pmlobj` | ~3177 | 主系统：初始化、add元素、getBestPosAndOri、方向决策 |
| `getpolarelement.pmlobj` | ~2420 | 元素投影：将 lindim/mlabel/aidline/arc/box/cyli 投影到柱坐标 |
| `polarelement.pmlobj` | 31 | 数据结构：startdis/enddis/startangle/endangle/startradius/endradius |
| `polarbox.pmlobj` | 29 | 辅助：box 形状 |
| `polarcyli.pmlobj` | 29 | 辅助：cylinder 形状 |

### 4.3 核心算法流程（getBestPosAndOri）

1. **初始化**：管段起止点 → 建立柱坐标系（pos, dir, ori, basicRadius）
2. **getDetail**：从管段中心点推导 horidir（水平方向）、showdir（观察方向）、mainDimDir
3. **add 元素**：将已有标注/管件投影到柱坐标 → `polarElement{startdis,enddis,startangle,endangle,startradius,endradius}`
4. **splitrange**：将 dis/angle/radius 范围按 best 值分割成优先级排序的候选单元
5. **balance**：对每个轴向单元计算最佳角度、方向、障碍元素
6. **getDirAndCha**：在已用角度中找最佳间隙方向 + 偏差值
7. **weightedweight**：角度偏差 × 径向距离 × 引线系数 = 加权评分
8. **选择最优**：遍历所有 (dis, radius) 候选单元，取加权最小的位置
9. **getresult**：(gooddis, goodangle, goodradius) → (goodpos, textori)

### 4.4 与现有 V2 模块的关系

| PolarSystem 概念 | 现有 V2 模块 | 覆盖程度 |
|---|---|---|
| horidir/showdir/mainDimDir | `dim_direction.rs` `iso_ori.rs` | 部分覆盖（优选方向已移植） |
| 柱坐标投影 | 无 | **未覆盖** |
| 空间占用记录 | `used_dir.rs` | 部分覆盖（只记方向+距离，缺角度+半径） |
| 最佳位置搜索 | 无 | **未覆盖** |
| 加权评分 | 无 | **未覆盖** |
| 元素几何投影（box/cyli/arc） | 无 | **未覆盖** |

**结论**：PolarSystem 的核心（柱坐标投影 + 空间搜索 + 加权评分）在 V2 中完全未实现。当前 V2 的方向决策（dim_direction/iso_ori）只是 PolarSystem 初始化阶段的部分语义。

### 4.5 移植策略建议

鉴于 PML 代码 ~5600 行，建议分 3 步：

1. **polar_system.rs** — 柱坐标系统核心（~800 行 Rust）
   - `PolarSystem::new(start, end, center, basic_radius, ...)` → 建立柱坐标系
   - `PolarElement { startdis, enddis, startangle, endangle, startradius, endradius }`
   - `PolarSystem::add(element)` → 注册已有占用
   - `PolarSystem::get_best_pos_and_ori(needs, best_diss, ...)` → 搜索最佳位置
   
2. **polar_projection.rs** — 几何投影（~400 行 Rust）
   - `project_linear_dim → PolarElement`
   - `project_label → PolarElement`
   - `project_cylinder → Vec<PolarElement>`
   - `project_box → Vec<PolarElement>`
   
3. **集成到 BranchCalculatorV2** — 替代 assembler 的方向硬编码

---

## 5. Phase 4 实现洞察（2026-05-01 补充）

### 5.1 增量集成策略的验证

**发现**：Phase 4 采用增量集成而非完全重写 — PolarSystem 作为 V2 pipeline 的一个可选增强步骤，在 assembler 之前执行。

**优点**：
- 不影响 V1 回退路径
- `enable_polar_direction=false` 完全退化为原有行为
- 所有现有测试保持通过
- 可以渐进式地用 PolarSystem 结果替代更多 V1 字段

**实现关键**：
- `enhance_layout_with_polar_directions` 修改 `LayoutResult` 的 `direction`、`offset`、`text_anchor` 字段
- 修改后的 `LayoutResult` 传给 assembler，后者不感知数据来源变化

### 5.2 isoline 提取的方向阈值

**发现**：`extract_isolines` 使用 5° 的方向变化阈值判定管段是否属于同一 isoline。

**风险**：如果管段在弯头处方向变化很小（如 170° 弯头，方向变化 10°），会被误判为同一 isoline。

**缓解**：当前只在 Phase 4 增量集成中使用，不影响最终的 BranchCalculatorV2 直算。后续直算会从管件类型（ELBO/BEND）而非角度阈值来分割 isoline。

### 5.3 dimtimes offset 公式

**发现**：当前实现的 offset 公式为：

```
offset = od/2 + cheight + lane_step_multiplier * (dimtimes - 1)
```

对比 PDMS 原文 `isoDim.draw`：
```
offset = od + cheight * dimtimes
```

差异在于：PDMS 用 od 而非 od/2（因为 PDMS 的 od 可能已经是半径），且用 cheight * dimtimes 而非独立的 lane_step。当前实现更灵活但需要在真实样本上验证数值对齐。

---

## 6. 验收执行结果（2026-05-01 补充）

### 6.1 主样本验收（24381_145712，服务 build 04-28）

```
✅ success = true
✅ version = v2
✅ primitives: 15 个
✅ linear_dim: 6 个
✅ error issues: 0
```

| Primitive 类型 | 数量 |
|---|---|
| linear_dim | 6 |
| label | 4 |
| leader_line | 4 |
| slope_mark | 1 |

**注意**：当前服务使用 04-28 build，不包含本轮新增代码。部署后应验证：
- port dim 出现（`include_port_dims=true`）
- weld_mark 区分 A/M（`include_weld_nouns=true`）
- 弯头出现 AidLine/AidArc
- PolarSystem 方向增强生效

---

## 7. 待确认问题

1. ~~**PolarSystem 移植范围**：PDMS `isoPolarSystem` 的完整语义是否已在 `dim_direction.rs` + `iso_ori.rs` 中覆盖？还是需要独立的极坐标模块？~~ **已确认**：需要独立模块，现有代码只覆盖了初始化阶段的部分语义。
2. **cheight 100mm vs 2.5mm**：生产和测试的字高差异是否影响避让结果的可靠性？
3. **overall dim 路径长度**：折线 BRAN 的 overall 应该是路径总长还是首尾直线距离？当前文档说"路径总长"，但 V2 pipeline 关闭了 overall。
4. **AngleDim 产出时机**：弯头当前只产出 LinearDim（size_dims），AngleDim 何时接入？需要弧线渲染支持吗？
5. **多 BRAN 样本覆盖**：除 `24381_145712` 外，还有哪些样本适合做回归验收？
