# Phase 9：V2 Primitive 渲染层设计

> **日期**：2026-05-02
> **目标**：让 Bevy 渲染器直接消费 `MbdV2PipeData`，不再需要 `BranchAnnotationData` 中间层

---

## 一、当前渲染架构

```
[rs-core] MbdV2PipeData { primitives: Vec<MbdPrimitive> }
    │
    ▼ (当前不直接消费)
[rs-plant3-d] BranchAnnotationData { measurements, welds }
    │
    ▼
[Bevy] AnnotationRenderer::render() → Bevy Entity
```

### 问题
- `BranchAnnotationData` 是 V1 风格，与 V2 `MbdPrimitive` 不兼容
- 前端渲染需要自己计算偏移/方向，违背 V2 "后端已排版" 的设计目标
- 新增 primitive 类型需要同时修改 `MeasurementCommand` 和渲染器

---

## 二、目标架构

```
[rs-core] MbdV2PipeData { primitives: Vec<MbdPrimitive> }
    │
    ▼ (V2PrimitiveRenderer 直接消费)
[rs-plant3-d] V2PrimitiveRenderer::render_all(primitives)
    │
    ├── LinearDim → DimensionLine3D (extension lines + dim line + arrows + text)
    ├── Label → TextSprite3D (text + optional box)
    ├── LeaderLine → PolyLine3D (折线 + arrow)
    ├── WeldMark → WeldCrossSymbol3D (× 或 + 符号)
    ├── SlopeMark → SlopeIndicator3D (坡度线 + 文字)
    ├── AngleDim → ArcDimension3D (弧 + 箭头 + 文字)
    ├── AidLine → HelperLine3D (实线/虚线/点划线)
    ├── AidArc → HelperArc3D
    ├── AidCircle → HelperCircle3D
    ├── AidPoint → HelperPoint3D
    └── AidText → HelperText3D
```

---

## 三、实现计划

### 3.1 V2 Primitive 类型对接

`rs-core` 的 `MbdV2PipeData` 已经通过 `serde` 可序列化为 JSON。在 Bevy 侧：

1. 在 `rs-plant3-d/Cargo.toml` 添加 `aios_core` 依赖（已存在）
2. 直接 `use aios_core::mbd::v2::*` 使用 V2 类型

### 3.2 V2PrimitiveRenderer 模块

新建 `rs-plant3-d/src/plugins/mbd_annotation/v2_renderer.rs`：

```rust
use aios_core::mbd::v2::{MbdPrimitive, MbdV2PipeData};
use bevy::prelude::*;

pub struct V2PrimitiveRenderer;

impl V2PrimitiveRenderer {
    pub fn render_all(
        commands: &mut Commands,
        data: &MbdV2PipeData,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
    ) -> usize {
        let mut count = 0;
        for prim in &data.primitives {
            if !prim.visible() { continue; }
            match prim {
                MbdPrimitive::LinearDim(d) => { render_linear_dim(commands, d, meshes, materials); count += 1; }
                MbdPrimitive::Label(l) => { render_label(commands, l); count += 1; }
                MbdPrimitive::LeaderLine(l) => { render_leader_line(commands, l, meshes, materials); count += 1; }
                MbdPrimitive::WeldMark(w) => { render_weld_mark(commands, w, meshes, materials); count += 1; }
                MbdPrimitive::SlopeMark(s) => { render_slope_mark(commands, s, meshes, materials); count += 1; }
                MbdPrimitive::AngleDim(a) => { render_angle_dim(commands, a, meshes, materials); count += 1; }
                MbdPrimitive::AidLine(a) => { render_aid_line(commands, a, meshes, materials); count += 1; }
                MbdPrimitive::AidArc(a) => { render_aid_arc(commands, a, meshes, materials); count += 1; }
                MbdPrimitive::AidCircle(c) => { render_aid_circle(commands, c, meshes, materials); count += 1; }
                MbdPrimitive::AidPoint(p) => { render_aid_point(commands, p, meshes, materials); count += 1; }
                MbdPrimitive::AidText(t) => { render_aid_text(commands, t); count += 1; }
            }
        }
        count
    }
}
```

### 3.3 各 Primitive 渲染实现

| Primitive | 渲染方式 | 所需 Bevy 能力 |
|---|---|---|
| LinearDim | 5 条线段 (2 extension + 1 dim_line + 2 arrow) + 1 文字 | Mesh (Line), Text3D |
| Label | 文字精灵 + 可选边框 | Text3D, optional BoxMesh |
| LeaderLine | 折线 + 箭头 | Mesh (Line), Triangle |
| WeldMark | ×/+ 符号 | Mesh (Line) |
| SlopeMark | 斜线 + 文字 | Mesh (Line), Text3D |
| AngleDim | 弧线 + 2 箭头 + 文字 | ArcMesh, Text3D |
| AidLine | 实线/虚线/点划线 | Mesh (Line/DashedLine) |
| AidArc | 弧线 | ArcMesh |
| AidCircle | 圆 | CircleMesh |
| AidPoint | 圆点 | SphereMesh |
| AidText | 文字 | Text3D |

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

### 方案 A：HTTP API（当前）

```
Bevy Plugin → reqwest::get("/api/mbd/v2/pipe/{refno}")
    → serde_json::from_str::<MbdV2Response>()
    → V2PrimitiveRenderer::render_all()
```

### 方案 B：直接 crate 调用（可选优化）

```
Bevy Plugin → aios_core::mbd::v2::build_mbd_v2_pipe_data_direct(&qr, &ctx)
    → V2PrimitiveRenderer::render_all()
```

推荐先用方案 A（与现有 API 一致），稳定后可选迁移到方案 B。

---

## 五、退役清单

Phase 9 完成后可删除：

| 文件 | 行数 | 条件 |
|---|---|---|
| `rs-core/src/mbd/v2/assembler.rs` | ~1800 | V2 直算覆盖所有场景 |
| `json_export/data.rs` 的 `MeasurementCommand` | ~200 | V2 Primitive 完全替代 |
| `json_export/renderer.rs` 的旧渲染器 | ~180 | V2PrimitiveRenderer 稳定 |
| V1 API handler `get_mbd_pipe` | ~100 | 前端全切 V2 |
