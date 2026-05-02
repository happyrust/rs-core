# Phase 9：V2 Primitive 渲染层设计

> **日期**：2026-05-02
> **目标**：让 Bevy 渲染器直接消费 `MbdV2PipeData`，不再需要 `BranchAnnotationData` 中间层

---

## 一、当前渲染架构

```
[rs-core] MbdV2PipeData { primitives: Vec<MbdPrimitive> }
    │
    ▼ (plant3d-web 通过 HTTP API 获取)
[plant3d-web] useMbdPipeAnnotationThree.ts
    │ (当前使用 V1 MbdPipeResponse 适配层)
    ▼
[three.js] LinearDimension3D / TextSprite / Line 等
```

### 已有前端基础
- `plant3d-web/src/types/mbdV2.ts` — V2 类型定义完整（与后端镜像）
- `plant3d-web/src/api/mbdPipeApi.ts` — HTTP API 调用，**已有 `getMbdPipeV2Annotations()`**
- `plant3d-web/src/api/mbdPipeApi.ts` — **已有 `adaptMbdV2ResponseToPipeResponse()`** 适配层（~220行）
- `plant3d-web/src/composables/useMbdPipeAnnotationThree.ts` — 现有标注渲染
- `plant3d-web/src/composables/mbd/` — branchLayoutEngine、mbdDimensionMode、mbdRequestSync
- `plant3d-web/src/utils/three/annotation/annotations/LinearDimension3D.ts` — 尺寸线 3D

### 当前状态
**前端已在消费 V2 API**（`getMbdPipeV2Annotations()`），但通过 `adaptMbdV2ResponseToPipeResponse()` 将 V2 primitive 翻译回 V1 `MbdPipeResponse` 格式给现有渲染器。这意味着：
- V2 primitive 的精确排版信息（extension_line、dim_line、arrows、text_anchor）被丢弃
- 前端仍用 V1 逻辑重新计算偏移/方向
- 适配层增加了维护成本（每加一种 primitive 需要同步修改适配层）

### Phase 9 目标
移除适配层，直接用 V2 primitive 的精确坐标渲染——**关键差异是不再做前端二次排版**

---

## 二、目标架构

```
[rs-core] MbdV2PipeData { primitives: Vec<MbdPrimitive> }
    │
    ▼ GET /api/mbd/v2/pipe/{refno}
[plant3d-web] renderV2Primitives(data.primitives)
    │
    ├── LinearDim → LinearDimension3D (extension + dim_line + arrows + text)
    ├── Label → CSS2DObject / TextSprite (text + optional box)
    ├── LeaderLine → THREE.Line (折线 + arrow)
    ├── WeldMark → 交叉线段 (× 或 + 符号)
    ├── SlopeMark → THREE.Line + TextSprite (坡度线 + 文字)
    ├── AngleDim → THREE.RingGeometry / ArcMesh (弧 + 箭头 + 文字)
    ├── AidLine → THREE.Line (solid/dashed/dash_dot)
    ├── AidArc → THREE.Line (圆弧采样)
    ├── AidCircle → THREE.RingGeometry
    ├── AidPoint → THREE.SphereGeometry
    └── AidText → CSS2DObject / TextSprite
```

---

## 三、实现计划

### 3.1 V2 Primitive 类型对接

`plant3d-web/src/types/mbdV2.ts` 已完整定义所有 V2 primitive 类型，与后端 `aios_core::mbd::v2::primitive` 完全镜像。无需额外类型工作。

### 3.2 V2 渲染分发器

在 `plant3d-web/src/composables/useMbdPipeAnnotationThree.ts` 中添加：

```typescript
import type { MbdPrimitive, MbdV2PipeData } from '@/types/mbdV2'

function renderV2Primitives(data: MbdV2PipeData, scene: THREE.Scene): number {
  let count = 0
  for (const prim of data.primitives) {
    if (!prim.visible) continue
    switch (prim.kind) {
      case 'linear_dim': renderLinearDim(prim, scene); count++; break
      case 'label': renderLabel(prim, scene); count++; break
      case 'leader_line': renderLeaderLine(prim, scene); count++; break
      case 'weld_mark': renderWeldMark(prim, scene); count++; break
      case 'slope_mark': renderSlopeMark(prim, scene); count++; break
      case 'angle_dim': renderAngleDim(prim, scene); count++; break
      case 'aid_line': renderAidLine(prim, scene); count++; break
      case 'aid_arc': renderAidArc(prim, scene); count++; break
      case 'aid_circle': renderAidCircle(prim, scene); count++; break
      case 'aid_point': renderAidPoint(prim, scene); count++; break
      case 'aid_text': renderAidText(prim, scene); count++; break
    }
  }
  return count
}
```

### 3.3 各 Primitive 渲染实现

| Primitive | 渲染方式 | 所需 three.js 能力 |
|---|---|---|
| LinearDim | 5 条线段 (2 extension + 1 dim_line + 2 arrow) + 1 文字 | THREE.Line, CSS2DObject |
| Label | 文字标签 + 可选边框 | CSS2DObject / TextSprite |
| LeaderLine | 折线 + 箭头 | THREE.Line, THREE.ConeGeometry |
| WeldMark | ×/+ 符号 | THREE.Line |
| SlopeMark | 斜线 + 文字 | THREE.Line, CSS2DObject |
| AngleDim | 弧线 + 2 箭头 + 文字 | THREE.Line (弧采样), CSS2DObject |
| AidLine | 实线/虚线/点划线 | THREE.Line, THREE.LineDashedMaterial |
| AidArc | 弧线 | THREE.Line (弧采样) |
| AidCircle | 圆 | THREE.RingGeometry / THREE.Line |
| AidPoint | 圆点 | THREE.SphereGeometry |
| AidText | 文字 | CSS2DObject |

### 3.4 分阶段交付

**Phase 9a（优先）**：LinearDim + Label + LeaderLine
- 这 3 种覆盖 90% 的标注内容
- 使用 Bevy 原生 `Mesh` + `Line` 渲染尺寸线和引线
- Text3D 使用现有项目的文字渲染方案

**Phase 9b**：WeldMark + SlopeMark
- 焊缝 ×/+ 符号：2 条交叉线段
- 坡度指示线 + 角度文字

**Phase 9c**：AngleDim + AidLine/Arc/Circle/Point/Text
- 弧线网格生成
- 虚线/点划线样式
- 辅助图元（调试可见性可选）

---

## 四、数据获取路径

### 方案 A：HTTP API（推荐）

```
plant3d-web → fetch("/api/mbd/v2/pipe/{refno}")
    → response.json() as MbdV2Response
    → renderV2Primitives(data.primitives, scene)
```

### 方案 B：v2_direct 直算

```
plant3d-web → fetch("/api/mbd/v2/pipe/{refno}?v2_direct=true")
    → response.json() as MbdV2Response
    → renderV2Primitives(data.primitives, scene)
```

方案 A 和 B 对前端透明——响应格式完全相同，区别仅在后端数据路径。

---

## 五、退役清单

Phase 9 完成后可删除：

| 文件 | 行数 | 条件 |
|---|---|---|
| `rs-core/src/mbd/v2/assembler.rs` | ~1800 | V2 直算覆盖所有场景 |
| `plant3d-web` V1 适配层代码 | ~500 | V2 primitive 原生渲染稳定 |
| V1 API handler `get_mbd_pipe` | ~100 | 前端全切 V2 |
| `plant-model-gen` V1 generate_mbd_data 的 MBD 分支 | ~1000 | V2 直算替代 |
