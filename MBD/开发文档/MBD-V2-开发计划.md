# MBD V2 开发计划（前端纯渲染版）

> **目标**：把 `plant-model-gen` 的 MBD 接口升级成"后端出已排版图元列表、前端只做 1:1 渲染"的 V2 架构，对齐 PDMS（`rs-core/MBD`）的职责分工。
>
> **关联文档**：
> - [MBD 模块架构与数据接口](./MBD模块架构与数据接口.md) — PDMS 现状
> - [管道标注绘制流程](./管道标注绘制流程.md) — PDMS 绘制流程
> - [标注重构开发计划](./标注重构开发计划.md) — rs-core 侧已有的方向性规划
>
> **状态**：分阶段实施中；最终验收以 plant3d-web 真实页面显示为准。
>
> **编写时间**：2026-04-21
> **当前最终验收入口**：`http://localhost:3101/?output_project=AvevaMarineSample&mbd_refno=24381_145712`

---

## 一、现状与 V2 设计目标

### 1.1 当前（V1）存在的问题

参考仓库：

- `plant-model-gen`：`src/web_api/mbd_pipe_api.rs`（API 层）
- `rs-core`：`src/mbd/mod.rs`（`BranchCalculator` MVP solver）
- `plant3d-web`：`src/composables/useMbdPipeAnnotationThree.ts`（~3000 行前端主体）

主要问题：

1. **职责双重**。前端即便在 `mode=layout_first` 模式下，仍然自己计算：
   - `pipe_clearances` 偏移（走 `computeMbdDimOffset` + 相机方向）
   - `applyCutTubiLabelDeclutter`、`applyTagLabelDeclutter`（label 避让）
   - 相机相关偏移方向（`computeDimensionOffsetDir`、`computeDimensionOffsetDirInLocal`）

2. **双路径 fallback 不一致**。`shouldUseLayoutFirstResult(mode, data)` 不真时回落到纯前端计算路径，与后端 PML 对齐算法不一致，产生视觉漂移。

3. **`BranchCalculator` 本身是 MVP**。
   
   ```319:333:/Volumes/DPC/work/plant-code/rs-core/src/mbd/mod.rs
   impl BranchCalculator {
       /// MVP solver：逐模块对齐 PML isoXxx 语义，产出完整 `LegacyPlacedLayoutSections`。
       ...
       /// iso_branch 在 MVP 里仅做"单 lane"处理；
       /// 多层 lane 分配 + `isoUsedDir` 已用方向惩罚留给后续迭代。
       pub fn solve_branch(input: SolveBranchInput<'_>) -> LegacyPlacedLayoutSections {
   ```

4. **前端维护成本高**。`useMbdPipeAnnotationThree.ts` 3000 行，每次后端新增语义都要联动改前端 fallback。

### 1.2 V2 核心设计

> **后端输出不再是"语义几何"，而是"已完成排版与避让的图元列表"。前端只做 primitive → three.js 的 1:1 映射，不做任何几何决策。**

对标 PDMS 的 `json.getJson()` / `jsonmem.getjsontext`：后端产出 `{ list: [ { type, nodeNames, geometry, text, ... } ] }`，前端只负责把每种 `type` 映射到对应的 three.js 实体。

```
V1 架构（双算）：
┌─ 后端 ─────────────┐      ┌─ 前端 ────────────────────────┐
│ MbdPipeData +      │─────>│ useMbdPipeAnnotationThree.ts │
│ layout_hint +      │      │ ├─ resolveBranchLayout       │
│ layout_result?     │      │ ├─ computeDimensionOffsetDir │
│                    │      │ ├─ applyCutTubiLabelDeclutter│
└────────────────────┘      │ └─ solvespaceLike (像素级)   │
                            └──────────────────────────────┘

V2 架构（单算）：
┌─ 后端 ─────────────────────────┐     ┌─ 前端 ───────────┐
│ MbdPipeData                    │     │ MbdV2Renderer    │
│  ↓                             │     │ ├─ primitive     │
│ BranchCalculator (upgraded)    │     │ │  → Line2       │
│  ↓                             │─────│ ├─ primitive     │
│ AvoidanceEngine                │     │ │  → Dim3D       │
│  ↓                             │     │ ├─ primitive     │
│ MbdPrimitiveList (contract)    │     │ │  → CSS2D label │
└────────────────────────────────┘     │ └─ 不做避让/布局 │
                                       └──────────────────┘
```

---

### 1.3 当前实现状态与最终验收边界

当前 V2 已具备数据合同、V1 `LayoutResult` 到 V2 primitive 的过渡组装、`SmallDimSolver` 基础集成、label 避让、leader 生成 / 重连 / reroute、leader-label 冲突检测、`plant-model-gen` 的真实 V2 后端 API，以及 plant3d-web 对 V2 primitive 的过渡渲染接入：

```text
GET /api/mbd/v2/pipe/{refno}
```

它还不是完整新版算法：`PolarSystem`、直接产出 V2 primitive 的 `BranchCalculatorV2` 仍需继续完成。plant3d-web 现阶段通过 `getMbdPipeV2Annotations()` 消费 `/api/mbd/v2/pipe/{refno}`，并把 V2 primitive 适配到现有三维标注渲染器。

后续开发不能只以 rs-core JSON 输出作为完成标准。阶段性 JSON 验证可以用于定位后端问题，但最终验收必须通过真实页面：

```text
http://localhost:3101/?output_project=AvevaMarineSample&mbd_refno=24381_145712
```

该页面必须自动进入 `AvevaMarineSample` 项目，自动触发 `mbd_refno=24381_145712` 的管道标注加载，并在三维视图中正确显示 BRAN 管道标注。`24381_145018` 仅保留为历史 / 辅助排查样本，不再作为主验收 BRAN。

当前前端入口事实：

- `output_project` 在 plant3d-web `App.vue` 中用于项目直达；项目真值来自后端 `/api/projects` 返回的真实项目。
- `mbd_refno` 在 plant3d-web `ViewerPanel.vue` 中作为 URL 预加载优先参数，会触发 `requestMbdPipeAnnotation(refno)`；`mbd_pipe` 只是兼容参数。
- 当前前端 MBD 请求入口在 layout_first 模式优先使用 `getMbdPipeV2Annotations()`；URL 加 `mbd_api=v1` / `mbd_version=v1` 可回滚到 `getMbdPipeAnnotations()`。
- 正式验收 URL 使用 `mbd_refno`；`mbd_pipe` 只作为历史兼容入口，不作为文档里的最终验收入口。
- V2 最终验收不能绕过上述 URL，不能只看后端 JSON。
- 当前 V1 `/api/mbd/pipe/{refno}` 随 `web_server` feature 默认启用 `mbd-iso`，`mode=layout_first` 会返回 `layout_result`，用于保证现有页面验收入口继续可用。


## 二、V2 数据契约（前后端单一真相源）

### 2.1 顶层结构

```ts
type MbdV2Response = {
  success: boolean;
  error_message?: string;
  data?: MbdV2PipeData;
}

type MbdV2PipeData = {
  version: "v2";
  input_refno: string;
  branch_refno: string;

  // 所有带语义的图元；前端按 type 分发渲染
  primitives: MbdPrimitive[];

  // 可选：聚合元数据（段信息、统计），只供 UI 面板展示，不参与渲染
  meta: MbdV2Meta;

  // 结构化错误，类似 PDMS wronglines
  issues: MbdV2Issue[];
}

type MbdV2Meta = {
  segments_count: number;
  welds_count: number;
  dims_by_kind: Record<string, number>;
  branch_attrs: Record<string, string>;
  generated_at: string; // ISO 8601
}
```

### 2.2 Primitive 类型（闭集合，扩展需 RFC）

对标 PDMS 的 7 种基本图元，**每一种前端都有对应的 1:1 three.js 实体**：

```ts
type MbdPrimitive =
  | LinearDimPrimitive
  | AngleDimPrimitive
  | LabelPrimitive
  | LeaderLinePrimitive
  | AidLinePrimitive
  | AidArcPrimitive
  | AidCirclePrimitive
  | AidPointPrimitive
  | AidTextPrimitive
  | WeldMarkPrimitive
  | SlopeMarkPrimitive;

type Vec3 = [number, number, number]; // 世界坐标，mm

type CommonFields = {
  id: string;                   // 稳定 ID，供 declutter / 追溯
  node_names: string[];         // PDMS nodeNames 同构
  function?: string;            // 业务语义（"长度" / "坡度" / "焊"），可空
  source_refno?: string;        // 产生这个 primitive 的对象 refno
  visible: boolean;
  suppressed_reason?: string;   // 被抑制但保留为 placeholder
}

type LinearDimPrimitive = CommonFields & {
  kind: "linear_dim";
  sub_kind: "segment" | "chain" | "overall" | "port";
  // 所有点位已经是最终世界坐标，无需前端再做 offset/direction 解算
  extension_1: { start: Vec3; end: Vec3 };
  extension_2: { start: Vec3; end: Vec3 };
  dim_line:    { start: Vec3; end: Vec3 };
  arrows: [
    { position: Vec3; direction: Vec3 },
    { position: Vec3; direction: Vec3 }
  ];
  text: {
    anchor: Vec3;               // 文字基线起点
    content: string;            // 已格式化（含单位，或 pureNumber + unit 字段由 UI 决定）
    height_mm: number;          // 与 PDMS cheight 一致
    orientation: Vec3;          // 文字阅读方向（右向量）
    up: Vec3;                   // 文字上方向
  };
  level: number;                // 分层序（> 0 表示已堆叠避让）
}

type AngleDimPrimitive = CommonFields & {
  kind: "angle_dim";
  vertex: Vec3;
  ray_1: Vec3;                  // 第一条参考射线方向（单位向量）
  ray_2: Vec3;                  // 第二条
  arc: {
    center: Vec3;
    radius_mm: number;
    start_angle_rad: number;
    sweep_rad: number;
    normal: Vec3;
  };
  arrows: [
    { position: Vec3; tangent: Vec3 },
    { position: Vec3; tangent: Vec3 }
  ];
  text: {
    anchor: Vec3;
    content: string;
    height_mm: number;
    orientation: Vec3;
    up: Vec3;
  };
}

type LabelPrimitive = CommonFields & {
  kind: "label";
  anchor: Vec3;                 // 指向点（图元关联的几何点）
  text_anchor: Vec3;            // 文字起点（已避让完）
  content: string;
  height_mm: number;
  orientation: Vec3;
  up: Vec3;
  box: { shape: "none" | "rect" | "circle"; padding_mm: number };
}

type LeaderLinePrimitive = CommonFields & {
  kind: "leader_line";
  points: Vec3[];               // 折线点，≥2 个
  arrow_at?: "start" | "end" | "both" | "none";
}

type AidLinePrimitive = CommonFields & {
  kind: "aid_line";
  points: Vec3[];               // ≥2 个，支持折线
  style: "solid" | "dashed" | "dash_dot";
}

type AidArcPrimitive = CommonFields & {
  kind: "aid_arc";
  center: Vec3;
  radius_mm: number;
  start_angle_rad: number;
  sweep_rad: number;
  normal: Vec3;
}

type AidCirclePrimitive = CommonFields & {
  kind: "aid_circle";
  center: Vec3;
  radius_mm: number;
  normal: Vec3;
}

type AidPointPrimitive = CommonFields & {
  kind: "aid_point";
  position: Vec3;
  diameter_mm: number;
}

type AidTextPrimitive = CommonFields & {
  kind: "aid_text";
  position: Vec3;
  content: string;
  height_mm: number;
  orientation: Vec3;
  up: Vec3;
}

type WeldMarkPrimitive = CommonFields & {
  kind: "weld_mark";
  position: Vec3;
  cross_size_mm: number;
  weld_type: "shop" | "field";
  label?: { linked_label_id: string };  // 引用 LabelPrimitive.id
}

type SlopeMarkPrimitive = CommonFields & {
  kind: "slope_mark";
  start: Vec3;
  end: Vec3;
  slope: number;                 // 有符号 (dy/horizontal)
  text: {
    anchor: Vec3;
    content: string;
    height_mm: number;
    orientation: Vec3;
    up: Vec3;
  };
}
```

### 2.3 Issue（对标 PDMS `wronglines`）

```ts
type MbdV2Issue = {
  id: string;
  severity: "info" | "warning" | "error";
  category: "geometry" | "data" | "layout" | "avoidance";
  message: string;
  related_refnos?: string[];
  related_primitive_ids?: string[];
}
```

### 2.4 前端渲染契约

前端只需要一张 dispatch 表：

| Primitive `kind` | Three.js 实体 |
|---|---|
| `linear_dim` | `LinearDimension3D`，直接喂 `extension_1/2/dim_line/arrows/text` |
| `angle_dim`  | `AngleDimension3D` |
| `label`      | `CSS2DObject` + 可选方框 |
| `leader_line`| `Line2` |
| `aid_line`   | `Line2`（样式按 `style`） |
| `aid_arc`    | `THREE.Line` 用圆弧 curve |
| `aid_circle` | `THREE.LineLoop` 或 `EllipseCurve` |
| `aid_point`  | `THREE.Points` 或小球 |
| `aid_text`   | `SolveSpaceBillboardVectorText` 或 `CSS2D` |
| `weld_mark`  | `WeldAnnotation3D`（保留现有类） |
| `slope_mark` | `SlopeAnnotation3D` |

**前端不再做**：

- 偏移方向 / offset 量级计算
- 相机相关位置计算
- `applyCutTubiLabelDeclutter` / `applyTagLabelDeclutter`
- chain / overall 显示级别选择

**前端仍保留**：

- 渲染实体本身（`Line2`、`CSS2D` 等）
- 缩放独立（`AnnotationBase.update`）
- SolveSpace 风格状态机（hovered / selected 颜色）
- 模型矩阵变换（局部 ↔ 世界）——如果必要

---

## 三、后端改造（`plant-model-gen` + `rs-core`）

### 3.1 新增 / 升级模块

| 模块 | 位置建议 | 职责 |
|---|---|---|
| `TextMeasurement` | `rs-core/src/mbd/text_measurement.rs` | 复刻 `mbdtextlen.pmlfnc` 的字符宽度查表，给定字符串 + 字高算视觉长度 |
| `SmallDimSolver` | `rs-core/src/mbd/small_dim.rs` | 复刻 `lindim.sepSmallDim` + `changeCheightAuto`：段长 vs 文字长 → 拆分或降字高或错层 |
| `PolarSystem` | `rs-core/src/mbd/polar_system.rs` | 复刻 `polarsystem.pmlobj`：管轴极坐标 + 区间搜索最佳 dis/angle/radius |
| `LeaderLineRouter` | `rs-core/src/mbd/leader_router.rs` | 复刻 `mlabel.addleadline`：选最近文字框角作为引线起点 |
| `AvoidanceEngine` | `rs-core/src/mbd/avoidance.rs` | 全局避让：label-label、label-line、primitive-primitive；多 lane 分配；`isoUsedDir` 惩罚 |
| `PrimitiveAssembler` | `rs-core/src/mbd/primitive.rs` | `LayoutResult` → `MbdPrimitiveList`：把 `PlacedLinearDim` 等展开成 primitive 数组 |
| `BranchCalculator v2` | `rs-core/src/mbd/branch_calculator_v2.rs` | 编排上述模块，完整替代现有 MVP solver |
| V2 API handler | `plant-model-gen/src/web_api/mbd_pipe_v2_api.rs` | 新 route `GET /api/mbd/v2/pipe/{refno}`；与 V1 并行共存 |

### 3.2 升级既有的 `BranchCalculator`

现有 MVP 流程：

```
compute_branch_layout_result → BranchCalculator::solve_branch → LayoutResult
```

V2 改为：

```
build_context(data)                          // 从 MbdPipeData 构造几何/属性/材料/焊缝包
  → PolarSystem::assign_lanes                // 轴测极坐标占位分配
  → SmallDimSolver::split_and_restack        // 小尺寸拆分 + 字高 + 错层
  → AvoidanceEngine::resolve_conflicts       // 全局避让
  → LeaderLineRouter::route_labels           // 每个标签的引线起点
  → PrimitiveAssembler::emit_primitives      // 产出 MbdPrimitiveList
  → V2 response
```

### 3.3 文字度量的一致性（关键决策点）

PDMS 用 `mbdtextlen.pmlfnc` 查表算字符宽。V2 有两个选项：

1. **复用同一张字符宽表**：把 PDMS `mbdtextlen.pmlfnc` 翻译成 Rust（~200 行 lookup）——保证后端与 PDMS 对齐
2. **用 HarfBuzz / rusttype 度量**：更通用，但与 PDMS 结果会有 0.5–2% 差异

**推荐方案 1**：与参考实现严格对齐，减少 diff 调试成本。后续如需做非 PDMS 字体支持再切方案 2。

---

## 四、前端改造（`plant3d-web`）

### 4.1 新增 composable

| 文件 | 职责 | 估算 |
|---|---|---|
| `src/composables/useMbdV2Renderer.ts` | 核心渲染 composable；input: `MbdV2PipeData`；output: `THREE.Object3D[]` | ~400 行 |
| `src/utils/three/annotation/v2/primitiveDispatch.ts` | primitive `kind` → 实体工厂，纯函数表 | ~150 行 |
| `src/api/mbdV2Api.ts` | `GET /api/mbd/v2/pipe/{refno}` 客户端 | ~50 行 |
| `src/types/mbdV2.ts` | TypeScript 类型定义（镜像 §2.2） | ~200 行 |

### 4.2 删除 / 大幅缩减

| 模块 | 现状 | V2 | 行动 |
|---|---|---|---|
| `useMbdPipeAnnotationThree.ts` | ~3000 行 | 废弃 | 保留到 V2 验收通过，然后移除 |
| `composables/mbd/branchLayoutEngine.ts` | 200+ 行 | 移到后端 | 删除 |
| `composables/mbd/computeMbdDimOffset.ts` | ~20 行 | 移到后端 | 删除 |
| `composables/mbd/computePipeAlignedOffsetDirs.ts` | 150+ 行 | 移到后端 | 删除 |
| `utils/three/annotation/utils/computeDimensionOffsetDir*.ts` | ~100 行 | 不需要 | 删除 |
| `utils/three/annotation/utils/solvespaceLike.ts` | 200+ 行 | 仅保留像素对齐部分 | 精简 |

### 4.3 保留

- `LinearDimension3D`、`AngleDimension3D`、`SlopeAnnotation3D`、`WeldAnnotation3D` 本体（实体类）
- `AnnotationBase` + `AnnotationMaterials` + `AnnotationInteractionController`
- `SolveSpaceBillboardVectorText`
- MBD 面板 UI（但移除"模式切换"开关，因为 V2 服务端返回的就是最终 layout）

### 4.4 新旧并行策略

- 新增 URL query 参数 `mbd_api_version=v2`（默认 `v1` 兼容）
- `useMbdPipeAnnotationThree.ts` 保留；新的 `useMbdV2Renderer.ts` 并行加载
- MBD 面板顶部加 "V1 / V2" 切换按钮（feature flag + `localStorage`）
- V2 通过验收后，默认切 V2；V1 进入 deprecation

---

## 五、分阶段 Roadmap（6 个阶段，约 12 周）

### Phase 1 · 数据契约冻结（1 周）

**目标**：定稿 §2 的 primitive 接口，避免后续返工。

- 交付：
  - `rs-core/MBD/开发文档/MBD-V2-CONTRACT.md`（JSON Schema + 字段语义）
  - `rs-core/src/mbd/v2/primitive.rs`（Rust 类型 + serde）
  - `plant3d-web/src/types/mbdV2.ts`（TypeScript 类型）
- 验收：
  - 后端 `MbdV2PipeData` 与前端 `MbdV2PipeData` 类型能相互反序列化测试用例通过
  - 覆盖 11 种 primitive 每一种都有 sample JSON

### Phase 2 · 后端底层模块（3–4 周）

**目标**：实现 `TextMeasurement` / `SmallDimSolver` / `PolarSystem` / `LeaderLineRouter`

- 优先级 1：`TextMeasurement` + `SmallDimSolver`（管道直段尺寸的核心）
- 优先级 2：`PolarSystem`（标签 / 管件避让的核心）
- 优先级 3：`LeaderLineRouter`
- 验收：
  - 每个模块有 golden test case，对齐 PDMS 相同输入下的输出（字符宽度逐字符比对；`sepSmallDim` 段长拆分逐项比对）
  - 10 条典型管道 fixtures 过

### Phase 3 · 后端 AvoidanceEngine + V2 API（2 周）

**目标**：把上述模块编排成端到端 V2 输出

- 实现 `AvoidanceEngine::resolve_conflicts`：多 lane 分配、已用方向惩罚
- 实现 `PrimitiveAssembler`
- 新 route `GET /api/mbd/v2/pipe/{refno}`
- 阶段验收：后端 JSON 可返回 `MbdV2PipeData`，且 `issues` 字段对 3 种故意构造的异常管道正确报出
- 最终验收：必须继续接入 plant3d-web，并通过 `localhost:3101/?output_project=AvevaMarineSample&mbd_refno=24381_145712` 真实页面显示验证

### Phase 4 · 前端 V2 Renderer（2 周）

**目标**：`useMbdV2Renderer.ts` + `primitiveDispatch.ts` 上线

- 实现 primitive → three.js 实体的完整 dispatch
- 保留 `AnnotationInteractionController`（选中/悬停仍在前端）
- UI 加 V1/V2 切换开关
- 验收：
  - 10 条 fixture 用 V2 渲染，与 V1 视觉对比 **不低于** V1 的正确率
  - `localhost:3101/?output_project=AvevaMarineSample&mbd_refno=24381_145712` 可自动加载并显示管道标注
  - V2 首屏渲染时间 ≤ V1（因为省去前端计算）

### Phase 5 · 并行运行 + 差异 QA（2 周）

**目标**：找 V2 与 PDMS / V1 的实际差异，迭代修补

- 批量跑 50–100 条管道，对比截图
- 建立 regression test harness：对每条 fixture 存 V2 输出快照，改动后 diff
- 验收：V2 在 95% fixtures 上视觉与 PDMS 对齐（或比 V1 更接近 PDMS）；主验收 BRAN `24381_145712` 必须在真实页面稳定显示

### Phase 6 · V1 Deprecation（1 周）

- 切默认到 V2
- 移除 §4.2 列的前端文件
- 保留 V1 API 一个版本（供线上回退）
- 更新 `MbdPipePanel.vue` 移除"版面优先 / 施工 / 校核"三模式（或改为前端样式开关，不再影响后端）
- 验收：
  - `plant3d-web` bundle 减少 ≥ 100KB（前端逻辑搬走）
  - `useMbdPipeAnnotationThree.ts` 删除
  - CHANGELOG 记入 break change（如果 V1 API 删除则是，否则只是 deprecation）

---

## 六、测试策略

### 6.1 Golden case fixtures

建立 `rs-core/tests/mbd_v2_fixtures/` 目录：

- 每个 fixture = 1 条真实管道的 refno + expected primitive list (JSON)
- 10 条初始覆盖：直管、单弯头、多弯头、三通、斜管、多坡度、带焊、带 tag、cut_tubi、管件密集
- 增加 fixtures 作为线上问题复现

### 6.2 对 PDMS 的 parity test

- 把 PDMS 对同一 refno 的 `json.getJson()` 输出（手动导出）作为参考
- 对 primitive 逐项 diff：坐标 tolerance 1mm、角度 tolerance 0.1°、文字完全相等
- 允许结构差异（V2 可以选择更简洁的 primitive 组织），但几何 footprint 必须一致

### 6.3 前端集成测试

- 用 Playwright / Vitest 截图对比 V1 / V2
- 渲染性能：`requestAnimationFrame` × 10 的时长 p95 不退化

### 6.4 性能 budget

- 后端 V2 接口 p95 延迟 ≤ V1 `mode=layout_first` 延迟 + 30%（因为做更多工作）
- 前端首屏渲染 p95 ≤ V1 （因为前端更轻）
- 内存：前端 mbd 相关 JS 堆使用 ≤ V1 的 80%

### 6.5 真实页面最终验收

最终验收只认以下入口：

```text
http://localhost:3101/?output_project=AvevaMarineSample&mbd_refno=24381_145712
```

必须满足：页面正常打开且不长期停在项目加载；当前项目为 `AvevaMarineSample`；`mbd_refno=24381_145712` 自动触发；后端成功返回该 BRAN 的管道标注数据；三维视图能看到该 BRAN 的管道模型和尺寸标注；MBD 面板显示当前 BRAN/HANG 为 `24381_145712`；浏览器控制台没有导致标注中断的 error；后端接口没有返回 error 级 `issues`。

尺寸显示必须满足：不缺失、不重复、方向不明显偏转、文字不堆叠到不可读，小尺寸能错层或缩字高。

后端阶段验证命令如下，但它只能证明后端数据可用，不能代表最终完成：

```bash
curl -s "http://127.0.0.1:3100/api/mbd/v2/pipe/24381_145712?debug=true" | jq .
```

---

## 七、风险与缓解

| 风险 | 概率 | 影响 | 缓解 |
|---|---|---|---|
| PDMS 避让算法复刻偏差 > 1% | 中 | 中 | Phase 2 每个子模块都有 golden case；发现 diff 立即拉 PDMS 开发者 pair review |
| 后端计算量过大，API 延迟过高 | 中 | 高 | `PolarSystem` 加缓存；`TextMeasurement` 结果 memoize；热点管道离线预生成 |
| 前端需要相机 / 分辨率相关微调 | 中 | 中 | 保留 `SolveSpaceBillboardVectorText` 的像素级 snap；primitive `height_mm` 是物理量，相机缩放独立由前端 `AnnotationBase.update` 负责，不影响后端 |
| Issue 反馈周期长 | 中 | 中 | Phase 5 并行 QA 至少 2 周；有回滚开关 `mbd_api_version=v1` |
| `pipe_clearances` 在 V1 未走 layout_result 路径，迁移后可能丢特性 | 低 | 中 | Phase 3 专门针对管道间距标注写 fixture；确保后端 V2 输出包含 `linear_dim` |
| 前端历史样式（blue/orange 配色、`materialSet.blue` 等）被硬编码 | 低 | 低 | Primitive 可携带 `style_preset: string` 字段，前端按预设取材质 |

---

## 八、里程碑与时间表

| 里程碑 | 周数 | 关键产物 |
|---|---|---|
| M1 契约冻结 | 1 | JSON Schema + TS/Rust 类型 |
| M2 后端模块 | 4 | `TextMeasurement` / `SmallDimSolver` / `PolarSystem` / `LeaderLineRouter` |
| M3 V2 API | 2 | `AvoidanceEngine` + `/api/mbd/v2/pipe/{refno}` |
| M4 前端 V2 渲染 | 2 | `useMbdV2Renderer.ts` |
| M5 并行 QA | 2 | 100 fixtures 通过；回归 harness |
| M6 V1 下线 | 1 | V1 删除、CHANGELOG、文档 |
| **合计** | **~12 周** | — |

---

## 九、附录：V1 关键代码参考

### 9.1 后端

- V1 API：`plant-model-gen/src/web_api/mbd_pipe_api.rs`
  - `get_mbd_pipe()`（`GET /api/mbd/pipe/{refno}`）
  - `compute_branch_layout_result`（~L2416：`mode=layout_first` 时调用）
  - `AnnotationLayoutPlanner`（~L706：启发式 `layout_hint`）
- V1 Solver：`rs-core/src/mbd/mod.rs`
  - `BranchCalculator::solve_branch`（~L319：MVP，待 V2 替换）
  - `LayoutResult`、`PlacedLinearDim`、`PlacedWeld` 等输出结构（~L72–L216）
- V1 算法：`rs-core/src/mbd/iso_dim.rs`
  - `compute_linear_dim_layout`（~L17：线性尺寸 PML 对齐求解）

### 9.2 前端

- V1 主 composable：`plant3d-web/src/composables/useMbdPipeAnnotationThree.ts`（~3000 行）
- V1 分支 layout：`plant3d-web/src/composables/mbd/branchLayoutEngine.ts`
- V1 offset 计算：`plant3d-web/src/composables/mbd/computePipeAlignedOffsetDirs.ts`
- V1 相机相关方向：`plant3d-web/src/utils/three/annotation/utils/computeDimensionOffsetDir*.ts`
- V1 像素避让：`plant3d-web/src/utils/three/annotation/utils/solvespaceLike.ts`

### 9.3 PDMS 参考（对齐来源）

- `rs-core/MBD/开发文档/MBD模块架构与数据接口.md` — 整体分层
- `rs-core/MBD/开发文档/管道标注绘制流程.md` — 绘制流程
- `rs-core/MBD/object/mbd/lindim.pmlobj` — `sepSmallDim` / `changeCheightAuto`
- `rs-core/MBD/object/polarsystem/polarsystem.pmlobj` — 极坐标避让
- `rs-core/MBD/object/mbd/mlabel.pmlobj` — 引线选点
- `rs-core/MBD/function/draw/mbdtextlen.pmlfnc` — 字符宽度表
- `rs-core/MBD/markpipe/object/isobran.pmlobj` — 分支标注总控
- `rs-core/MBD/object/mbd/json.pmlobj` / `jsonmem.pmlobj` — JSON 导出契约

---

## 十、评审决议记录（空，供评审后填写）

- [ ] 契约字段是否完备（§2.2）
- [ ] 阶段划分与时间估算是否合理（§五）
- [ ] 风险项是否还有遗漏（§七）
- [ ] 字符度量选项（方案 1 vs 方案 2，§3.3）
- [ ] 新旧并行机制（`mbd_api_version=v2`）是否通过
- [ ] V1 下线时机是否需要延后

**评审人**：_（待填）_  
**评审日期**：_（待填）_  
**最终决策**：_（待填）_
