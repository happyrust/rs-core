# FRADIUS 传递流程分析

## 概述

本文档分析在传递 FRADIUS（圆角半径）给 ploop-rs 时的完整处理流程。

## 数据格式说明

### 输入格式（Vec3）
- `Vec3::new(x, y, fradius)`
- **重要**：`z` 坐标存储的是 **FRADIUS（圆角半径）**，不是 z 坐标
- 如果 `z > 0.0`，表示该顶点有 FRADIUS
- 如果 `z == 0.0`，表示该顶点没有 FRADIUS（普通顶点）

### ploop-rs Vertex 格式
```rust
pub struct Vertex {
    pub x: f32,              // x 坐标
    pub y: f32,              // y 坐标
    pub z: f32,              // z 坐标（通常为 0.0）
    pub fradius: Option<f32>, // FRADIUS 值（可选）
}
```

## 完整流程

### 1. 输入阶段：Vec3 → Vertex

**位置**：`src/prim_geo/wire.rs::process_ploop_vertices` (line 1629-1641)

```rust
let ploop_vertices: Vec<Vertex> = vertices
    .iter()
    .map(|v| {
        if v.z > 0.0 {
            // 有 fradius 的顶点
            Vertex::with_fradius(v.x, v.y, 0.0, Some(v.z))
            //                           ^^^^  ^^^^^^^^^
            //                           z坐标   FRADIUS值
        } else {
            // 普通顶点
            Vertex::new(v.x, v.y)
        }
    })
    .collect();
```

**转换逻辑**：
- 如果 `Vec3.z > 0.0`：调用 `Vertex::with_fradius(x, y, 0.0, Some(z))`
  - `x, y`：直接传递坐标
  - `z`：设置为 `0.0`（因为我们只处理 2D 轮廓）
  - `fradius`：设置为 `Some(v.z)`，即 Vec3 的 z 值
- 如果 `Vec3.z == 0.0`：调用 `Vertex::new(x, y)`
  - `fradius`：自动为 `None`

**关键点**：
- ✅ **Vec3.z 的值被正确传递到 `Vertex.fradius`**
- ✅ **Vertex.z 被设置为 0.0**（因为我们处理的是 2D 轮廓）

### 2. ploop-rs 处理阶段

**位置**：`ploop-rs/src/processor.rs::process_ploop` (line 176-269)

#### 2.1 预处理：修复连续 FRADIUS 冲突
```rust
let vertices = self.detect_and_fix_consecutive_fradius_conflicts(vertices);
```
- 检测相邻顶点都有 FRADIUS 的情况
- 如果两个 FRADIUS 的和超过边长度，进行调整

#### 2.2 处理每个顶点
```rust
for i in 0..n {
    let prev_v = &vertices[prev_idx];
    let curr_v = &vertices[i];
    let next_v = &vertices[next_idx];
    
    if curr_v.has_fradius() {
        // 有 FRADIUS 的顶点
        if let Some(result) = self.fradius_processor
            .calculate_fillet_arc(prev_v, curr_v, next_v) {
            // 成功计算圆弧
            let tangent1 = result.tangent1.clone();
            let tangent2 = result.tangent2.clone();
            processed_vertices.push(tangent1);
            processed_vertices.push(tangent2);
        } else {
            // 计算失败，保留原顶点
            processed_vertices.push(curr_v.clone());
        }
    } else {
        // 普通顶点，直接添加
        processed_vertices.push(curr_v.clone());
    }
}
```

**处理逻辑**：
- **有 FRADIUS 的顶点**：
  - 调用 `calculate_fillet_arc` 计算圆弧
  - 如果成功：生成两个切点（`tangent1`, `tangent2`），**切点没有 FRADIUS**
  - 如果失败：保留原顶点（**仍然包含 FRADIUS**）
- **普通顶点**：直接添加到结果中

**关键点**：
- ✅ **处理后的切点（tangent1, tangent2）通常没有 FRADIUS**（`fradius: None`）
- ⚠️ **如果计算失败，原顶点会被保留，其 FRADIUS 仍然存在**

### 3. 输出阶段：Vertex → Vec3

**位置**：`src/prim_geo/wire.rs::process_ploop_vertices` (line 1648-1658)

```rust
let result: Vec<Vec3> = processed_vertices
    .iter()
    .map(|vertex| {
        Vec3::new(
            vertex.x,
            vertex.y,
            vertex.fradius.unwrap_or(0.0), // z 存储 fradius 值
        )
    })
    .collect();
```

**转换逻辑**：
- `x, y`：直接传递坐标
- `z`：从 `vertex.fradius` 获取，如果没有则为 `0.0`

**关键点**：
- ✅ **Vertex.fradius 被转换回 Vec3.z**
- ✅ **由于切点通常没有 FRADIUS，返回的 Vec3.z 通常为 0.0**

## 数据流图

```
输入 Vec3
  Vec3(x, y, fradius)  ← fradius 存储在 z 坐标
      ↓
  [转换检查]
      ↓
  if z > 0.0:
      Vertex::with_fradius(x, y, 0.0, Some(z))
      └─> Vertex { x, y, z: 0.0, fradius: Some(z) }
  else:
      Vertex::new(x, y)
      └─> Vertex { x, y, z: 0.0, fradius: None }
      ↓
  [ploop-rs 处理]
      ↓
  if has_fradius:
      calculate_fillet_arc() → 生成切点
      └─> tangent1: Vertex { x, y, z: 0.0, fradius: None }
      └─> tangent2: Vertex { x, y, z: 0.0, fradius: None }
  else:
      保留原顶点
      ↓
  [转换回 Vec3]
      ↓
  输出 Vec3
      Vec3(x, y, fradius.unwrap_or(0.0))
      └─> 切点的 z 通常为 0.0
```

## 特殊情况处理

### 1. FRADIUS 值过滤
**位置**：`ploop-rs/src/vertex.rs::with_fradius` (line 29-32)

```rust
pub fn with_fradius(x: f32, y: f32, z: f32, fradius: Option<f32>) -> Self {
    let fradius = fradius.filter(|&r| r > 0.0);
    Self { x, y, z, fradius }
}
```

- ✅ **负值会被过滤**：`fradius.filter(|&r| r > 0.0)`
- ✅ **0.0 值会被过滤**：`Some(0.0)` 会变成 `None`

### 2. 连续 FRADIUS 冲突
**位置**：`ploop-rs/src/processor.rs::detect_and_fix_consecutive_fradius_conflicts`

- 检测相邻顶点都有 FRADIUS 的情况
- 如果 `FRADIUS1 + FRADIUS2 > 边长度`，会进行调整

### 3. 圆弧计算失败
如果 `calculate_fillet_arc` 失败：
- 原顶点会被保留
- 原顶点的 FRADIUS 仍然存在
- 返回的 Vec3.z 可能仍然 > 0.0

## 测试验证

从测试结果可以看到：

### 测试 1：`test_process_ploop_vertices`
```
输入：4 个顶点，1 个有 FRADIUS (z=10.0)
输出：5 个顶点，所有顶点的 z 都是 0.0
```

### 测试 2：`test_process_ploop_from_content`
```
输入：4 个顶点，2 个有 FRADIUS (15.0 和 5.0)
输出：6 个顶点，所有顶点的 z 都是 0.0
```

**结论**：
- ✅ FRADIUS 被正确展开为切点
- ✅ 处理后的切点没有 FRADIUS（z = 0.0）
- ✅ 转换逻辑正确

## 潜在问题

### 问题 1：处理失败时的 FRADIUS 保留
如果 `calculate_fillet_arc` 失败，原顶点会被保留，其 FRADIUS 仍然存在。这可能导致：
- 返回的 Vec3.z > 0.0
- 后续处理可能期望所有顶点都没有 FRADIUS

### 问题 2：fradius 值的精度
- `Vertex.fradius` 是 `Option<f32>`
- `Vec3.z` 是 `f32`
- 转换时使用 `unwrap_or(0.0)`，丢失了 `None` 和 `Some(0.0)` 的区别

## 建议改进

1. **明确处理失败的情况**：
   - 如果圆弧计算失败，考虑是否应该移除 FRADIUS 或抛出错误

2. **保持数据一致性**：
   - 确保处理后的顶点要么全部展开，要么全部保留
   - 避免混合状态（部分顶点有 FRADIUS，部分没有）

3. **文档说明**：
   - 明确说明处理后的 Vec3.z 可能仍然 > 0.0 的情况（计算失败时）











