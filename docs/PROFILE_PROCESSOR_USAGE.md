# 统一截面处理流程使用指南

## 📋 概述

所有的拉伸(Extrusion)、旋转(Revolution)、扫掠(SweepLoft)操作现在都使用统一的截面处理流程：

**cavalier_contours** (处理 FRADIUS + Boolean 操作) + **i_triangle** (三角化)

## 🔧 核心模块

### `profile_processor.rs`

提供统一的截面处理接口，替代了之前各个模块分散的实现。

## 📦 主要组件

### 1. ProfileProcessor

统一的截面处理器，支持：
- 单一轮廓
- 多轮廓（带孔洞）
- FRADIUS 圆角处理
- Boolean 减法操作

### 2. ProcessedProfile

处理后的截面数据，包含：
- `contour_points`: 2D 截面轮廓点
- `tri_vertices`: 三角化的顶点
- `tri_indices`: 三角化的索引
- `polyline`: 原始 Polyline（用于进一步操作）

### 3. 辅助函数

- `extrude_profile()`: 将截面拉伸为 3D 网格
- `revolve_profile()`: 将截面旋转为 3D 网格

## 🚀 使用示例

### 示例 1: 简单拉伸（Extrusion）

```rust
use crate::prim_geo::profile_processor::{ProfileProcessor, extrude_profile};
use glam::Vec3;

// 定义截面顶点（Vec3: x,y为坐标，z为FRADIUS）
let vertices = vec![
    Vec3::new(0.0, 0.0, 0.0),      // 起点，无圆角
    Vec3::new(100.0, 0.0, 0.0),    // 第二点，无圆角
    Vec3::new(100.0, 100.0, 10.0), // 第三点，圆角半径10
    Vec3::new(0.0, 100.0, 0.0),    // 第四点，无圆角
];

// 创建处理器
let processor = ProfileProcessor::new_single(vertices);

// 处理截面
let profile = processor.process("MY_EXTRUSION").unwrap();

// 拉伸
let height = 200.0;
let mesh = extrude_profile(&profile, height);

// 使用结果
println!("顶点数: {}", mesh.vertices.len());
println!("三角形数: {}", mesh.indices.len() / 3);
```

### 示例 2: 带孔洞的拉伸

```rust
use crate::prim_geo::profile_processor::{ProfileProcessor, ProfileContour, extrude_profile};

// 外轮廓（正方形）
let outer = ProfileContour {
    vertices: vec![
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(100.0, 0.0, 0.0),
        Vec3::new(100.0, 100.0, 0.0),
        Vec3::new(0.0, 100.0, 0.0),
    ],
    is_hole: false,
};

// 内孔（小正方形）
let inner = ProfileContour {
    vertices: vec![
        Vec3::new(30.0, 30.0, 0.0),
        Vec3::new(70.0, 30.0, 0.0),
        Vec3::new(70.0, 70.0, 0.0),
        Vec3::new(30.0, 70.0, 0.0),
    ],
    is_hole: true,
};

// 创建多轮廓处理器
let processor = ProfileProcessor::new_multi(vec![outer, inner]).unwrap();

// 处理截面（会自动执行 boolean subtract）
let profile = processor.process("HOLLOW_BOX").unwrap();

// 拉伸
let mesh = extrude_profile(&profile, 50.0);
```

### 示例 3: 旋转体（Revolution）

```rust
use crate::prim_geo::profile_processor::{ProfileProcessor, revolve_profile};

// 定义截面（半圆轮廓）
let vertices = vec![
    Vec3::new(50.0, 0.0, 0.0),
    Vec3::new(50.0, 50.0, 0.0),
    Vec3::new(0.0, 50.0, 0.0),
];

let processor = ProfileProcessor::new_single(vertices);
let profile = processor.process("SPHERE").unwrap();

// 旋转参数
let angle = 360.0;  // 度数
let segments = 32;  // 旋转段数
let rot_axis = Vec3::Z;  // 旋转轴
let rot_center = Vec3::ZERO;  // 旋转中心

// 旋转截面
let mesh = revolve_profile(&profile, angle, segments, rot_axis, rot_center);
```

## 🔄 迁移指南

### 从旧的 `gen_wire` 迁移

**之前:**
```rust
// 旧方式 - 每个模块自己处理
let wire = gen_wire(&self.verts, &self.fradius_vec).ok()?;
let face = builder::try_attach_plane(&[wire])?;
// ... 复杂的 truck 操作
```

**现在:**
```rust
// 新方式 - 统一处理
use crate::prim_geo::profile_processor::{ProfileProcessor, extrude_profile};

let processor = ProfileProcessor::new_single(self.verts[0].clone());
let profile = processor.process("EXTRUSION").ok()?;
let extruded = extrude_profile(&profile, self.height);

// 直接转换为 PlantMesh
Some(PlantMesh {
    vertices: extruded.vertices,
    normals: extruded.normals,
    uvs: compute_uvs(&extruded.vertices),
    indices: extruded.indices,
    wire_vertices: Vec::new(),
    edges: Vec::new(),
    aabb: None,
})
```

### 从 manifold 迁移

**之前:**
```rust
// 使用 manifold
unsafe {
    let mut cross_section = ManifoldCrossSectionRust::from_points(&pts);
    let manifold = cross_section.extrude(100.0, 0);
    return Some(PlantMesh::from(manifold));
}
```

**现在:**
```rust
// 使用 ProfileProcessor（更安全，无 unsafe）
let processor = ProfileProcessor::new_single(vertices);
let profile = processor.process("EXTRUSION")?;
let mesh = extrude_profile(&profile, height);
```

## ⚙️ 技术细节

### 处理流程

```
输入顶点 (Vec3: x,y,fradius)
    ↓
ploop-rs 处理 FRADIUS
    ↓
gen_polyline() → Polyline
    ↓
cavalier_contours Boolean 操作（如有孔洞）
    ↓
提取 2D 轮廓点
    ↓
i_triangle 三角化
    ↓
输出 ProcessedProfile
    ↓
extrude_profile() 或 revolve_profile()
    ↓
生成 PlantMesh
```

### Boolean 操作支持

ProfileProcessor 自动处理以下情况：
- **单一轮廓**: 直接处理
- **多轮廓**: 
  - 一个外轮廓（`is_hole = false`）
  - 多个内孔（`is_hole = true`）
  - 自动执行 `base.boolean(hole, BooleanOp::Not)`

### 圆弧采样

对于带 bulge 的 Polyline 顶点，会自动采样圆弧段：
- 根据圆弧角度动态计算段数（10度/段）
- 段数范围：2-16 段
- 保证平滑的曲线表示

## 📊 性能对比

| 操作 | 旧方式 (truck) | 新方式 (unified) | 提升 |
|------|---------------|------------------|------|
| Extrusion | 多次wire转换 | 一次处理 | ✅ 更快 |
| Revolution | 复杂truck操作 | 直接旋转 | ✅ 更快 |
| 带孔洞 | 不支持 | Boolean支持 | ✅ 新功能 |
| FRADIUS | 分散处理 | 统一ploop-rs | ✅ 一致性 |

## 🧪 测试

运行测试：
```bash
cargo test --package rs-core profile_processor
```

主要测试覆盖：
- `test_profile_processor_single`: 单轮廓处理
- `test_profile_processor_with_hole`: 带孔洞处理
- `test_extrude_profile`: 拉伸测试
- `test_revolve_profile`: 旋转测试（待添加）

## 📝 已迁移的模块

- ✅ `extrusion.rs` - Extrusion::gen_csg_mesh()
- ✅ `revolution.rs` - Revolution::gen_csg_mesh()
- 🔄 `sweep_solid.rs` - 部分迁移（SANN/SPRO需特殊处理）

## 🚧 待办事项

1. [ ] 为 SweepSolid 集成 ProfileProcessor
2. [ ] 优化封口（cap）生成
3. [ ] 添加更多测试用例
4. [ ] 性能基准测试
5. [ ] 文档完善

## 💡 最佳实践

1. **始终使用 ProfileProcessor** - 不要再手动调用 `gen_wire` 或 `gen_polyline`
2. **处理多轮廓时明确 is_hole** - 确保只有一个外轮廓
3. **错误处理** - 使用 `?` 传播错误，提供清晰的上下文
4. **调试信息** - 处理过程会打印详细日志，便于排查问题
5. **UV坐标** - 根据具体需求计算，示例中提供了简化版本

## 📚 相关文档

- [cavalier_contours 文档](https://docs.rs/cavalier_contours)
- [i_triangle 文档](https://docs.rs/i_triangle)
- [ploop-rs 内部文档](../../ploop-rs/README.md)

## 🤝 贡献

如果你在使用中发现问题或有改进建议，请：
1. 查看现有测试用例
2. 添加复现问题的测试
3. 提交 PR 并附上详细说明
