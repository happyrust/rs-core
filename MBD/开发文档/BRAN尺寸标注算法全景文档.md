# BRAN 尺寸标注算法全景文档

> **范围**：BRAN（管道分支）尺寸标注从原始 PDMS PML 实现到 Rust V1/V2 移植的完整算法说明  
> **生成时间**：2026-05-04  
> **生成方式**：基于 ace-tool 语义检索 + 源码逐文件分析  
> **最终验收入口**：`http://localhost:3101/?output_project=AvevaMarineSample&mbd_refno=24381_145712`

---

## 目录

- [一、三代代码并存的现状](#一三代代码并存的现状)
- [二、PDMS PML 主链路（业务真值）](#二pdms-pml-主链路业务真值)
- [三、核心算法组件详解](#三核心算法组件详解)
- [四、Rust V2 模块矩阵](#四rust-v2-模块矩阵)
- [五、前后端双路径与渲染](#五前后端双路径与渲染)
- [六、默认参数集中表](#六默认参数集中表)
- [七、五个关键问题点](#七五个关键问题点)
- [八、推荐的下一步](#八推荐的下一步)
- [九、关键文件索引](#九关键文件索引)

---

## 一、三代代码并存的现状

BRAN 尺寸标注当前**不是双轨而是三代同在**：

| 代 | 位置 | 角色 | 状态 |
|---|---|---|---|
| **PDMS PML 原版** | `rs-core/MBD/markpipe/object/*.pmlobj` | 原始业务真值，在 PDMS 中运行 | 在用，事实标准 |
| **Rust V1 1:1 移植** | `rs-core/src/mbd/iso_dim.rs` + `iso_branch.rs` + `iso_extras.rs` | 严格对齐 PML 的 Rust MVP solver | 在用，作为 V2 bridge 的输入 |
| **Rust V1 workspace-split** | `rs-core-ws-split/crates/aios-mbd/` | V1 的 crate 拆分版（同源 fork） | 重构进行中 |
| **Rust V2 过渡** | `rs-core/src/mbd/v2/` | 后端排版 + 前端 1:1 渲染 | 当前线上版，Phase 9 进行中 |

**最权威的对比文档**：

```text
plant3d-web/开发文档/MBD标注开发文档/rs-core-MBD对比与前端MBD标注开发文档.md
```

本文档与上述对比文档互补：本文档侧重「算法本身」，对比文档侧重「前后端契约对齐进度」。

---

## 二、PDMS PML 主链路（业务真值）

### 2.1 isobran 总控器：状态机 + 编排器

`rs-core/MBD/markpipe/object/isobran.pmlobj`（6441 行）不是「纯绘制对象」，而是 branch 级状态机：

**初始化阶段**：
- 记录 branch 名称、是否 `INST` 管道
- 保存 `minslope` / `maxslope` / `considerprenextdir` / `lookangle` / `drawtype`
- 创建 `mbdstru`（MBD 结构容器）
- 准备首层/第二层尺寸倍率：`firstdimtimes=0.5`、`seconddimtimes=1.7`

**中间状态**（`clearcontent()` 重置）：
- `isolines` — 直管段切片
- `materials` — 材料汇总
- `tags` — 管件标签
- `welds` — 焊缝
- `zyjc` — 在役检查（中文缩写）
- `benchmarks` — 基准点
- `wronglines` — 错误日志

**触发动作**：
- `drawall()` — 触发全标注
- `draw(type)` — 按类型分别绘制（`ISODIM` / `ISOSLOPE` / `ISOTAG` / `ISOMATERIALTEXT` / `ISOWELDTEXT` / `ISODMF`）
- `generatejsonattrvm()` — 导出 JSON/RVM/ATT
- `totplant()` — 上传 TPlant

### 2.2 分支拆解：`getisolines()`

**关键算法之一**，把一条 branch 拆成多个 `isoline`（直管段）：

```
1. 遍历 branch members（含 Head/Tail）
2. 收集每段候选成员数组 allarr
3. 收集每段方向 ldirs
4. 收集转折点 poss
5. 根据数组构造 isoline
```

**嵌入的业务规则**（不是简单的「按方向变化切」）：
- `PCOM` 的水平/垂直布置分支逻辑
- `BEND / ELBO` 等拐弯元件的拆线规则
- special element（如三通的支管）单独切段
- branch 建模方向错误时记录 `wronglines` 并返回失败
- 把前一个 isoline 的 `prehoridir` 传给下一个 isoline，形成「前后段约束」

**Rust V2 的简化点**：`branch_calculator.rs::extract_isolines` 当前用 5° 方向阈值分割，会把 170° 弯头误判为同一 isoline。Phase 8.2 后续直算应改用管件类型（ELBO/BEND）切割。

### 2.3 对象采集与优先级：`getobjects()` + `putIntoIsoLine()`

**采集顺序即语义**（`isobran.getobjects`）：

```
1. getattadatas()         — atta 属性
2. getadjustwelds()       — 调整焊点（优先级高）
3. getinstallationangles()— 安装角度
4. getslopes()            — 坡度（优先级高于其他「其他」）
5. getmaterials()         — 材料
6. gettags()              — 管件标签（tee/olet/href/tref/atta/elbo）
7. getwelds()             — 焊缝
8. getcutparts()          — 切管段
9. getbranmlabel()        — 管道名 + 管径
```

**`putIntoIsoLine()` 划入规则**：
- 有名字的对象优先按成员归属分配
- 没名字的对象（cut pipe/text/辅助对象）按投影距离找最近 isoline
- 焊缝文字、弯头 pad/leg 等特殊对象有单独判断
- 无法找到合适 isoline 时：留调试痕迹 + 进入错误路径，**不静默硬画**

**与前端 fallback 的差异**：old PML「先归线后绘制」；plant3d-web fallback「直接按对象类型绘制」。这是前端有时出现「个别对象位置不对」的主要原因。

### 2.4 isobran.draw() 顶层流程

```text
draw()
  ├─ split()
  │   ├─ getisolines()
  │   └─ putIntoIsoLine()
  ├─ 权限检查（删除旧 DIM/AID、切换到 MBD site/group）
  ├─ drawdmf()            处理弯头/定位辅助项
  ├─ 遍历 isolines:
  │   ├─ isoline.draw()
  │   │   └─ isoDim.draw()
  │   │       ├─ CalculateDimChardirs()  优选方向
  │   │       ├─ dimOneMemSlope()        坡管直线
  │   │       ├─ isoori 计算最终 (pipedir, dimdir, chardir, ori)
  │   │       └─ drawDim(dimtimes)
  │   │           └─ dimOneMem(offdis)
  │   │               └─ 生成 lindim 对象
  │   ├─ addtopreornextpolarsystem      告诉相邻 isoline 已用方向
  │   └─ addUsedDirToPre/Next
  ├─ 收集 wronglines
  └─ 按需 generatejsonattrvm() → totplant() 上传
```

### 2.5 BranAttarr 属性表

`rs-core/MBD/markpipe/branAttlist.txt`：

```
BranAttarr: Duty/介质, Pspec/管道等级, RCCM/RCCM, CLEAN/清洁度,
            TEMP/设计温度, ISPEC/保温, INSUTHICK/保温厚度,
            TSPEC/伴热, SWGD/室外, DRAWNUM/图号, REV/版本, status/状态
```

---

## 三、核心算法组件详解

### 3.1 方向决策三件套（PML ↔ Rust V1 ↔ Rust V2 三点一线）

#### 3.1.1 isoGetDimDir — 管段偏移方向决策

```
输入：pipedir（管段方向），dimdirs[1..3]（3 个优选方向）
输出：dimdir（标注偏移方向）

算法：
1. dimdir = pipedir.cross(dimdirs[3])         # 1-based dimdirs[3] = U
2. 若 |cross| < eps（管段与 dimdirs[3] 平行）:
     dimdir = pipedir.cross(dimdirs[2])
3. dimdir = isoGetBestDir(dimdir, dimdirs)
```

**Rust 等价**：`glam::Vec3::cross()` + 方向翻转。

#### 3.1.2 isoGetBestDir — 方向一致性校正（2023-05-10 新版）

```
输入：inputdir（初步方向），dirs[1..3]（优选方向列表）
输出：调整后的方向

算法：
1. 分别计算 inputdir 与 dirs[1], dirs[2], dirs[3] 的夹角
2. 对每个夹角取 min(angle, 180 - angle) → 绝对夹角
3. 排序找最小绝对夹角对应的方向索引 num
4. 如果 inputdir 与 dirs[num] 的原始夹角 > 90°，则取反
```

**本质**：确保输出方向与「最接近的优选方向」同侧（< 90°）。

#### 3.1.3 CalculateDimChardirs — 从包围盒推优选方向

```
输入：segment_midpoint（标注管段中点），bran_bbox_center（分支包围盒中心）

算法：
1. dir = segment_midpoint.direction(bran_bbox_center)
2. horiz = [E, N, W, S]
3. 按 angle(horiz[i], dir) 排序
4. dim_dirs = [-horiz[i0], -horiz[i1], U]    # 远离包围盒中心
5. char_dirs = [U, horiz[i0], horiz[i1]]      # 字符优选 U 为首
```

#### 3.1.4 三代实现对照

| 代 | 文件 | 函数 |
|---|---|---|
| PML | `markpipe/function/isoGetDimDir.pmlfnc`、`isoGetBestDir.pmlfnc`、`isoDim.CalculateDimChardirs` | — |
| Rust V1 | `rs-core/src/mbd/iso_dim.rs` | `compute_linear_dim_layout` / `calculate_dim_chardirs` / `select_dim_dir` / `pick_best_dir` |
| Rust V2 | `rs-core/src/mbd/v2/dim_direction.rs` + `iso_ori.rs` | `resolve_dim_direction` / `iso_get_dim_dir` / `iso_get_best_dir` / `calculate_dim_char_dirs` / `compute_iso_ori` |
| workspace-split | `rs-core-ws-split/crates/aios-mbd/src/iso_dim.rs` | 与 V1 同源 |

### 3.2 lane 分配：UsedDirRegistry

`rs-core/src/mbd/iso_branch.rs` 包含 V1 的完整实现，与常被忽视的事实是 **V1 已经有这套机制**，V2 在其基础上加了 PolarSystem 集成。

**关键常量**（来自 `rs-core-ws-split/crates/aios-mbd/src/iso_branch.rs`）：

```rust
const DIRECTION_ALIGN_COS_THRESHOLD: f32 = 0.94;       // ~20° 内视为同方向（含反向）
const DISTANCE_OVERLAP_TOLERANCE_MM: f32 = 0.5;        // 距离区间重叠容差
const SPATIAL_PROXIMITY_FACTOR: f32 = 0.25;            // 中点距离阈值因子
const SPATIAL_PROXIMITY_FLOOR_MM: f32 = 300.0;         // 中点距离下限（300mm）
```

**`solve_linear_dim_series` 算法**：

```
for each input segment:
    1. compute_linear_dim_layout → 得到初始 direction
    2. 查 UsedDirRegistry：
       - 同方向（cos ≥ 0.94）+ 同距离区间（overlap > -0.5mm）+ 空间邻近（中点距离 ≤ max(0.25*dim_len, 300mm)）→ 冲突
    3. 冲突则 dim_times++ 重新求解
    4. 直至 lane 数足够
返回 (Vec<PlacedLinearDim>, UsedDirRegistry)
```

V2 的 `used_dir.rs::UsedDirRegistry` 在此基础上加入了 PolarSystem 的角度+半径维度。

### 3.3 小尺寸拆分：small_dim.rs 详细公式

#### 3.3.1 get_good_cheight（PDMS `getgoodcheight` 等价）

```rust
cheight = (投影总跨度 × bili) / (各段文字 em 宽度之和)
// bili 默认 0.8（文字占段长 80%）
// 下限为 cheight × change_cheight_auto_bili (默认 0.5)
```

#### 3.3.2 solve_small_dims 决策树

```text
for each segment:
    if text_width ≤ dis:
        累积到当前 row
    else:
        if change_cheight_auto:
            尝试缩字高 → 满足则单独一行（level=0）
        if sep_small_dim and 仍不够:
            level++，单独一行错层
        else:
            强制缩字高收为 level=0
```

**输出**：`Vec<DimRow { points, pos, cheight, level, texts }>`

#### 3.3.3 与 PML 的差异

PML 原版 `lindim` 在一个对象里同时处理；Rust 拆为：
- `SmallDimSolver`（`small_dim.rs`）— 求解
- `expand_linear_dim_chain`（`assembler.rs`）— 把组转成 primitive

### 3.4 chain stacking：expand_linear_dim_chain

`rs-core/src/mbd/v2/assembler.rs` 中的 chain stacking：

```rust
pub struct LinearDimChain<'a> {
    pub dims: &'a [PlacedLinearDim],    // 组内须 direction/offset 近似 + 端点可串
    pub small_dim_params: SmallDimChainParams,
}

pub fn expand_linear_dim_chain(
    chain: &LinearDimChain<'_>,
    ctx: &AssemblerContext,
    next_id: &mut dyn FnMut(&str) -> String,
) -> Vec<MbdPrimitive>;
```

**流程**：「一组同基线方向、端点相邻的 segment PlacedLinearDim」→ SmallDimInput → DimRow → 多个 LinearDimPrimitive。`DimRow.level` 直接写回 `LinearDimPrimitive.level`，`DimRow.cheight` 可能小于原始 cheight。

### 3.5 PolarSystem：柱坐标空间优化器

#### 3.5.1 本质

围绕管段轴线的 **3D 空间放置优化器**，三维量纲：
- **dis**：轴向距离（沿管段方向距起点的距离）
- **angle**：绕轴线的角度（0–360°）
- **radius**：径向距离（到轴线的距离）

#### 3.5.2 PML 代码规模

| 文件 | 行数 | 职责 |
|---|---|---|
| `polarsystem.pmlobj` | ~3177 | 主系统：初始化、add 元素、getBestPosAndOri、方向决策 |
| `getpolarelement.pmlobj` | ~2420 | 元素投影：lindim/mlabel/aidline/arc/box/cyli → 柱坐标 |
| `polarelement.pmlobj` | 31 | 数据结构：startdis/enddis/startangle/endangle/startradius/endradius |
| `polarbox.pmlobj` | 29 | 辅助：box 形状 |
| `polarcyli.pmlobj` | 29 | 辅助：cylinder 形状 |
| **PML 合计** | **~5600** | |
| **Rust 合计** | **~700** | `polar_system.rs` |

#### 3.5.3 核心 9 步（`get_best_pos_and_ori`）

```text
1. 初始化：管段起止点 → 柱坐标系（pos, dir, ori, basicRadius）
2. getDetail：从中心点推 horidir/showdir/mainDimDir
3. add 元素：将已有标注/管件投影到柱坐标 → PolarElement
4. splitrange：dis/angle/radius 范围按 best 值分割成优先级单元
5. balance：对每个轴向单元算最佳角度、方向、障碍元素
6. getDirAndCha：在已用角度中找最佳间隙方向 + 偏差值
7. weightedweight：角度偏差 × 径向距离 × 引线系数 = 加权评分
8. 选最优：遍历所有 (dis, radius) 候选，取加权最小的位置
9. getresult：(gooddis, goodangle, goodradius) → (goodpos, textori)
```

#### 3.5.4 当前 V2 集成方式

`branch_calculator.rs::enhance_layout_with_polar_directions` 是**增量模式**：

- 在 assembler 之前执行
- 用 PolarSystem 覆盖 V1 的硬编码方向
- 完全脱离 V1 的 BranchCalculatorV2 直算尚未完成（Phase 8.2）

#### 3.5.5 与现有 V2 模块的覆盖关系

| PolarSystem 概念 | 现有 V2 模块 | 覆盖程度 |
|---|---|---|
| horidir/showdir/mainDimDir | `dim_direction.rs` `iso_ori.rs` | 部分覆盖 |
| 柱坐标投影 | `polar_system.rs` | 已实现（500+ 行） |
| 空间占用记录 | `used_dir.rs` + `polar_system.rs` | 已实现 |
| 最佳位置搜索 | `polar_system.rs::get_best_pos_and_ori` | 已实现 |
| 加权评分 | `polar_system.rs::weighted_weight` | 已实现，PML 分段权重表精确复刻 |
| 元素几何投影（box/cyli/arc） | `polar_system.rs::axis_distance` 等 | 部分覆盖 |

### 3.6 避让引擎：四步流水线

`rs-core/src/mbd/v2/avoidance.rs`：

```rust
// pipeline.rs 中依次调用
resolve_linear_dim_text_conflicts(...)     // 尺寸文字互相遮挡
resolve_label_label_conflicts(...)         // label 二维 AABB 碰撞 + lane bump
reroute_leader_lines_around_labels(...)    // leader 绕行 label 的 AABB
detect_leader_line_label_conflicts(&prims) // 检测遗留冲突 → Issue
```

#### 3.6.1 AvoidanceConfig 默认值

```rust
lane_step_multiplier: 1.2,                 // 每次 lane bump = height_mm × 1.2
max_lanes: 6,                              // 超过发 Warning Issue
min_gap_mm: 0.5,                           // 两个文字 bbox 间最小间距
coplanar_dir_tolerance: 1e-3,              // orientation/up 近似容差
coplanar_offset_tolerance: 0.5,            // 法线方向偏离容差
max_leader_reroute_attempts: 3,
leader_reroute_margin_mm: 0.2,
```

#### 3.6.2 label-label 算法

```
1. 在每个 label 的平面投影（orientation × up）上算 AABB
2. 按 anchor 沿 orientation 方向排序、扫描
3. 与已放置 labels 求交：若相撞，沿 up 抬高一个 lane
4. 超过 max_lanes 停在最后一层并发 Issue
```

**写回**：修改 `LabelPrimitive.text_anchor`（沿 up 加偏移）。

#### 3.6.3 leader 重路由策略

- 只处理 2 点 leader（≥3 点跳过）
- 用 label bbox 的 4 个外扩角（向外推 0.2mm）作为 candidate via
- 只尝试一次，多个 label 同时挡住的复杂场景发 Issue

#### 3.6.4 明确不做的事

| 项 | 后续 step |
|---|---|
| ≥3 点 leader 复杂重路由 | Step 3.3 |
| 3D 真 AABB 求交 | Step 3.3 |
| LinearDim 跨 chain 避让 | Step 3.3 |
| 最短路径/A* 最优 reroute | 不做 |

### 3.7 多层偏移公式【三代差异】

| 代 | 公式 | 说明 |
|---|---|---|
| **PML 原版** `isoDim.drawDim` | `offset = od/2 + od/2 + cheight*1.2*(dimtimes-1)` = `od + cheight*1.2*(dimtimes-1)` | 严格原版 |
| **Rust V1** `iso_dim.rs` | `offset = od + 1.2*cheight*(dimtimes-1)` | 1:1 PDMS |
| **Rust V2 默认** | `offset = od/2 + cheight + cheight*1.2*(dimtimes-1)` | 小管径不压管线 |
| **Rust V2 PDMS 兼容** | `offset = od + 1.2*cheight*(dimtimes-1)` | `use_pdms_offset_formula=true` |

**差值** = `od/2 - cheight`：
- 大管径（od > 2×cheight）：PDMS 偏移大
- 小管径（od < 2×cheight）：V2 默认偏移大，避免文字压管线

### 3.8 LeaderLineRouter

`leader_router.rs` 复刻 `mlabel.addleadline` — 选最近文字框角作为引线起点。

### 3.9 TextMeasurement

`text_measurement.rs` 逐字符复刻自 PML `mbdtextlen.pmlfnc`，**保留了 PML 原文的 `x/X` 笔误**（查不到表则走 `DEFAULT_CHAR_WIDTH=1.02`）。

```rust
mbd_text_len(text) -> f32              // em 总和、无量纲
mbd_text_width(text, cheight) -> f32   // mm 宽度
format_dim_value(value) -> String       // 数值格式化
```

---

## 四、Rust V2 模块矩阵

`rs-core/src/mbd/v2/`（13 个模块）：

| 模块 | 行数 | 职责 | 对应 PML |
|---|---|---|---|
| `primitive.rs` | 693 | 11 种图元类型定义 + 序列化测试 | `json.pmlobj` / `jsonmem.pmlobj` |
| `pipeline.rs` | ~1100 | V2 顶层入口 + production_defaults + 双路径分支 | — |
| `assembler.rs` | ~1800 | V1 LayoutResult → V2 primitive + chain stacking | — |
| `branch_calculator.rs` | ~470 | extract_isolines + enhance_layout + path_total_length | `isobran.draw` |
| `polar_system.rs` | ~700 | 柱坐标空间放置 + 加权评分 | `polarsystem.pmlobj` 5600 行 → ~500 行 |
| `dim_direction.rs` | ~350 | isoGetDimDir / isoGetBestDir / calculate_dim_char_dirs | `isoDim.CalculateDimChardirs` |
| `iso_ori.rs` | ~280 | (pipedir, dimdir, chardir) → ori | `isoori.pmlobj` |
| `small_dim.rs` | ~380 | sepSmallDim 错层 + changeCheightAuto 缩字高 | `lindim.sepSmallDim` |
| `text_measurement.rs` | ~350 | 字符宽度表逐字符复刻（保留 PML 的 x/X 笔误） | `mbdtextlen.pmlfnc` |
| `avoidance.rs` | ~1150 | label-label / leader-label / dim-text 避让 | — |
| `leader_router.rs` | ~130 | label 引线选最近角点 | `mlabel.addleadline` |
| `member_positions.rs` | ~210 | 管件位置收集/排序/去重 | `isoDim.addmem` |
| `used_dir.rs` | ~225 | 已用方向注册 + count_overlaps → dimtimes | `isoUsedDir` + `addUsedDirToPre/Next` |
| `data_source.rs` | ~260 | V2 BranchQueryResult 数据源 trait + InMemory mock | — |

### 4.1 V2 数据契约（11 种 primitive，已冻结）

```ts
type MbdPrimitive =
  | LinearDimPrimitive   // sub_kind: segment | chain | overall | port
  | AngleDimPrimitive
  | LabelPrimitive
  | LeaderLinePrimitive
  | AidLinePrimitive     // style: solid | dashed | dash_dot
  | AidArcPrimitive
  | AidCirclePrimitive
  | AidPointPrimitive
  | AidTextPrimitive
  | WeldMarkPrimitive    // weld_type: shop | field
  | SlopeMarkPrimitive;
```

顶层结构：

```ts
type MbdV2PipeData = {
  version: "v2";
  input_refno: string;
  branch_refno: string;
  primitives: MbdPrimitive[];
  meta: MbdV2Meta;
  issues: MbdV2Issue[];     // 对标 PML wronglines
};
```

### 4.2 当前 pipeline 数据流

```text
前端 GET /api/mbd/v2/pipe/{refno}
  ├─ resolve_effective_branch_refno（HANG → BRAN）
  ├─ mbd_v2_layout_query 强制 LayoutFirst + 全标注开
  ├─ generate_mbd_data → V1 BranchCalculator::solve_branch → LayoutResult
  └─ build_mbd_v2_pipe_data:
      ├─ enhance_layout_with_polar_directions       (PolarSystem 覆盖方向 + 多层 offset)
      ├─ assemble_v2_primitives_with_chain_stacking (V1→V2 翻译 + small_dim 错层)
      ├─ resolve_linear_dim_text_conflicts
      ├─ resolve_label_label_conflicts (max_lanes=6)
      ├─ reroute_leader_lines_around_labels
      ├─ detect_leader_line_label_conflicts
      └─ collect_suppression_issues
  → MbdV2PipeData
```

### 4.3 v2_direct 直算路径（Phase 8.3）

```
GET /api/mbd/v2/pipe/{refno}?v2_direct=true
  ├─ fetch_tubi_segments_from_surreal_with_debug
  ├─ CacheTubiSeg → BranchMember
  └─ build_mbd_v2_pipe_data_direct
```

过渡期内部仍转 LayoutResult，正在脱钩中。

---

## 五、前后端双路径与渲染

### 5.1 后端 API 三入口

```
GET /api/mbd/pipe/{refno}                    — V1 API（mode=layout_first 返回 layout_result）
GET /api/mbd/v2/pipe/{refno}                 — V2 默认（V1 bridge 路径）
GET /api/mbd/v2/pipe/{refno}?v2_direct=true  — V2 直算路径（脱离 V1）
```

### 5.2 前端双路径

plant3d-web 现阶段并行维护：
- **layout_first**：优先消费后端已排布好的 `layout_result`
- **construction / fallback**：前端用 `branchLayoutEngine` / `computePipeAlignedOffsetDirs` / `computeMbdDimOffset` 做近似重建

**已对齐核心抽象**：`mode` / `layout_result` / `suppressed_reason` / `placement_lane` / `offset_level` / `direction` / `offset` / `label_t`。

**未完全对齐**：
- 完整 isoline 对象优先级
- old PML 特殊元件拆线规则
- 导出链路
- 「前后段占位/极坐标系统」全量语义

### 5.3 前端渲染器状态

- `useMbdPipeAnnotationThree.ts` — V1 时代产物，~3000 行，仍在使用
- `MbdV2Renderer` 骨架已在 `rs-plant3-d/src/plugins/mbd_annotation/v2_renderer.rs`（~280 行 Bevy）
- TypeScript V2 类型在 `plant3d-web/src/types/mbdV2.ts` + `ModelRevPlatform/src/types/mbd-v2-primitives.d.ts`
- Phase 9b-c：WeldMark / AngleDim / AidLine 的 three.js 渲染未接入

### 5.4 前端 Primitive 渲染映射表

| Primitive `kind` | Three.js 实体 |
|---|---|
| `linear_dim` | `LinearDimension3D` |
| `angle_dim` | `AngleDimension3D` |
| `label` | `CSS2DObject` + 可选方框 |
| `leader_line` | `Line2` |
| `aid_line` | `Line2`（按 style） |
| `aid_arc` | `THREE.Line` 圆弧 curve |
| `aid_circle` | `THREE.LineLoop` |
| `aid_point` | `THREE.Points` |
| `aid_text` | `SolveSpaceBillboardVectorText` |
| `weld_mark` | `WeldAnnotation3D` |
| `slope_mark` | `SlopeAnnotation3D` |

---

## 六、默认参数集中表

### 6.1 生产环境（`production_defaults`）

```text
cheight: 100mm                                # 文字高度
firstdimtimes: 0.5                            # 首层标注离管壁距离系数
seconddimtimes: 1.7                           # 二层
minslope/maxslope: 0.001 / 0.1                # 坡管阈值
lane_step_multiplier: 1.2                     # 多层 lane 偏移系数
max_lanes: 6                                  # 避让最大层数
change_cheight_auto_bili: 0.5                 # 字高缩小下限
DIRECTION_ALIGN_COS_THRESHOLD: 0.94           # ~20° 内同方向（含反向）
DISTANCE_OVERLAP_TOLERANCE_MM: 0.5
SPATIAL_PROXIMITY_FACTOR: 0.25
SPATIAL_PROXIMITY_FLOOR_MM: 300mm             # 空间邻近下限
look_angle: 30°                               # PolarSystem 观察角度
bili (good_cheight): 0.8                      # 文字占比
default_od: 229.0mm                           # 默认管段外径

enable_polar_direction: true                  # 默认启用 PolarSystem 增强
enable_avoidance: true
enable_small_dim_stacking: true
include_port_dims: true                       # 04-28 后启用
include_weld_nouns: true                      # weld_type A/M 区分
```

### 6.2 测试环境（`Default`）

```text
cheight: 2.5mm   ⚠️ 与生产相差 40×
enable_avoidance: false
enable_small_dim_stacking: false
```

40× 字高差异是 Phase 7.4 已补的 6 个生产字高测试要测的重点，避免避让逻辑在测试中看着正常但生产中失效。

### 6.3 Avoidance 配置

```text
lane_step_multiplier: 1.2
max_lanes: 6
min_gap_mm: 0.5
coplanar_dir_tolerance: 1e-3
coplanar_offset_tolerance: 0.5
max_leader_reroute_attempts: 3
leader_reroute_margin_mm: 0.2
```

---

## 七、五个关键问题点

1. **isoline 拆分语义丢失**  
   V2 `extract_isolines` 用 5° 阈值，丢了 PML 的 `PCOM` 水平/垂直布置、`BEND/ELBO` 拐弯拆线、`prehoridir` 前后段约束。需要 Phase 8.2 按管件类型重写。

2. **getobjects 优先级顺序未完全复刻**  
   PML 的「adjustweld > installation_angles > slopes > others」优先级在 V1/V2 里混在一起，需要在进一步重构时始终保持。

3. **putIntoIsoLine 划入规则缺失**  
   old PML「先归线后绘制」未在 V2 里完整复刻，是个别对象位置坐标偏差的根源。

4. **PolarSystem 覆盖度未全量验证**  
   horidir/showdir/mainDimDir 完成 60%；柱坐标投影、加权评分、最佳位置搜索、元素几何投影（box/cyli/arc）在 V2 里已加，但与 PDMS 输出的 diff 需要量化验证。

5. **多层偏移公式不一致**  
   PML 原版、V1、V2 三套公式，需要在实际样本上验证 V2 默认公式与 PDMS 的视觉差异是否可接受。

---

## 八、推荐的下一步

### P0 验收闭环

- 跑 `MBD-V2-开发规划/scripts/build-sample-registry.sh` 拿到全量 BRAN 基准快照
- 跑 `MBD-V2-开发规划/scripts/batch-validate-v2-extended.sh` 运行 100+ 样本
- 10 条代表性 BRAN 手工验收（`mbd_refno=24381_145712` 为主）

### P0 架构脱钩

- Phase 8.2 BranchCalculatorV2 完整脱离 V1 LayoutResult
- 重点：isoline 按管件类型（ELBO/BEND）切割、putIntoIsoLine 划入规则复刻

### P1 量化对齐 PDMS

- PolarSystem 在 ≥5 样本上跟 PDMS 原版输出做坐标 1mm / 角度 0.1° diff
- 记录字符宽度表与 PDMS 的一致性（包括 x/X 笔误）

### P1 前端刷骨架

- plant3d-web V2 primitive 原生渲染，退退 V1 翻译层
- Phase 9b-c：WeldMark/AngleDim/AidLine 的 three.js 渲染

### P2 V1 退役

- assembler.rs 1800 行 → 大幅精简
- V1 API 标 deprecated，2 周观察期后删除

---

## 九、关键文件索引

### PML 参考（业务真值）

```
rs-core/MBD/markpipe/object/isobran.pmlobj           — 6441 行，分支总控
rs-core/MBD/markpipe/object/isoDim.pmlobj            — 1537 行，线性尺寸
rs-core/MBD/markpipe/object/isoline.pmlobj           — 4940 行，直管段
rs-core/MBD/markpipe/object/isoteeline.pmlobj        — 2156 行，三通段
rs-core/MBD/markpipe/object/isoori.pmlobj            — 字符方向求解
rs-core/MBD/markpipe/function/isoGetDimDir.pmlfnc    — 标注偏移方向
rs-core/MBD/markpipe/function/isoGetBestDir.pmlfnc   — 方向一致性校正
rs-core/MBD/markpipe/markpipeform.pmlfrm             — UI 入口表单
rs-core/MBD/markpipe/branAttlist.txt                 — BranAttarr 属性表
rs-core/MBD/object/polarsystem/polarsystem.pmlobj    — 极坐标避让
rs-core/MBD/object/mbd/lindim.pmlobj                 — sepSmallDim/changeCheightAuto
rs-core/MBD/object/mbd/mlabel.pmlobj                 — 引线
rs-core/MBD/object/mbd/json.pmlobj / jsonmem.pmlobj  — JSON 导出契约
rs-core/MBD/function/draw/mbdtextlen.pmlfnc          — 字符宽度表
```

### Rust V1（1:1 PML 移植）

```
rs-core/src/mbd/iso_dim.rs                  — compute_linear_dim_layout
rs-core/src/mbd/iso_branch.rs               — UsedDirRegistry + solve_linear_dim_series
rs-core/src/mbd/iso_extras.rs               — solve_bend / solve_slope / solve_tag / solve_weld
rs-core/src/mbd/iso_params.rs               — BranchContext / IsoParams / SegmentInput
rs-core/src/mbd/mod.rs                      — BranchCalculator::solve_branch + LayoutResult
```

### Rust V1 workspace-split（同源 fork）

```
rs-core-ws-split/crates/aios-mbd/src/iso_dim.rs
rs-core-ws-split/crates/aios-mbd/src/iso_branch.rs
rs-core-ws-split/crates/aios-mbd/src/iso_extras.rs
rs-core-ws-split/crates/aios-mbd/src/iso_params.rs
rs-core-ws-split/crates/aios-mbd/src/lib.rs
```

### Rust V2

```
rs-core/src/mbd/v2/mod.rs                   — 模块入口 + 公共 re-export
rs-core/src/mbd/v2/primitive.rs             — 11 种图元类型
rs-core/src/mbd/v2/pipeline.rs              — V2 顶层入口
rs-core/src/mbd/v2/assembler.rs             — V1→V2 翻译
rs-core/src/mbd/v2/branch_calculator.rs     — extract_isolines + enhance_layout
rs-core/src/mbd/v2/polar_system.rs          — PolarSystem 移植
rs-core/src/mbd/v2/dim_direction.rs         — 方向决策
rs-core/src/mbd/v2/iso_ori.rs               — 标注坐标系
rs-core/src/mbd/v2/small_dim.rs             — 小尺寸求解
rs-core/src/mbd/v2/avoidance.rs             — 避让引擎
rs-core/src/mbd/v2/leader_router.rs         — 引线
rs-core/src/mbd/v2/text_measurement.rs      — 字符宽度
rs-core/src/mbd/v2/used_dir.rs              — UsedDir 注册
rs-core/src/mbd/v2/member_positions.rs      — 管件位置
rs-core/src/mbd/v2/data_source.rs           — V2 数据源 trait
```

### API 层

```
plant-model-gen/src/web_api/mbd_pipe_api.rs            — V1/V2 双路由
plant-model-gen/src/web_api/mod.rs                     — 路由注册
```

### 前端

```
plant3d-web/src/composables/useMbdPipeAnnotationThree.ts  — 现有渲染（~3000 行）
plant3d-web/src/composables/mbd/branchLayoutEngine.ts     — V1 fallback
plant3d-web/src/composables/mbd/computePipeAlignedOffsetDirs.ts
plant3d-web/src/composables/mbd/computeMbdDimOffset.ts
plant3d-web/src/api/mbdPipeApi.ts                         — API 调用
plant3d-web/src/types/mbdV2.ts                            — V2 类型定义
plant3d-web/src/components/dock_panels/ViewerPanel.vue    — mbd_refno 入口
ModelRevPlatform/src/types/mbd-v2-primitives.d.ts         — TS 类型
rs-plant3-d/src/plugins/mbd_annotation/v2_renderer.rs     — Bevy V2 渲染器骨架
```

### 开发文档

```
rs-core/MBD/开发文档/MBD-V2-开发计划.md                       — 总规划（615 行）
rs-core/MBD/开发文档/MBD-V2-下一步开发计划.md
rs-core/MBD/开发文档/MBD-V2-Phase2-执行计划.md                — 后端底层模块
rs-core/MBD/开发文档/MBD-V2-Phase3-Step1-执行计划.md          — pipeline 顶层
rs-core/MBD/开发文档/MBD-V2-Phase3-Step2-执行计划.md          — chain stacking
rs-core/MBD/开发文档/MBD-V2-Phase3-Step2.5-执行计划.md
rs-core/MBD/开发文档/MBD-V2-Phase3-Step3-执行计划.md          — label-label 避让
rs-core/MBD/开发文档/MBD-V2-Phase3-Step3.1-执行计划.md        — leader-label 检测
rs-core/MBD/开发文档/MBD-V2-Phase3-Step3.2-执行计划.md        — leader 重路由
rs-core/MBD/开发文档/MBD-V2-Phase6-验收计划.md
rs-core/MBD/开发文档/MBD-V2-开发规划/2026-05-02-mbd-v2-next-phase-plan.md  — 当前迭代
rs-core/MBD/开发文档/MBD-V2-开发规划/findings.md             — 架构发现
rs-core/MBD/开发文档/MBD-V2-开发规划/progress.md             — 进度日志
rs-core/MBD/开发文档/MBD-V2-开发规划/task_plan.md            — 任务计划
rs-core/MBD/开发文档/MBD-V2-开发规划/phase9-renderer-design.md
rs-core/MBD/开发文档/MBD-标注方向移植/findings.md            — 方向算法分析
rs-core/MBD/开发文档/MBD-标注方向移植/progress.md
rs-core/MBD/开发文档/MBD-标注方向移植/task_plan.md
rs-core/MBD/开发文档/管道标注绘制流程.md                     — PML 绘制流程
rs-core/MBD/开发文档/MBD模块架构与数据接口.md                — PML 整体分层
rs-core/MBD/开发文档/标注重构开发计划.md                     — 早期重构方向
plant3d-web/开发文档/MBD标注开发文档/rs-core-MBD对比与前端MBD标注开发文档.md  — 三代对比文档
```

### 验收脚本

```
rs-core/MBD/开发文档/MBD-V2-开发规划/scripts/build-sample-registry.sh    — 样本注册
rs-core/MBD/开发文档/MBD-V2-开发规划/scripts/batch-validate-v2.sh        — 批量验收
rs-core/MBD/开发文档/MBD-V2-开发规划/scripts/batch-validate-v2-extended.sh
rs-core/MBD/开发文档/MBD-V2-开发规划/scripts/validate-v2-response.sh     — 单样本验证
```
