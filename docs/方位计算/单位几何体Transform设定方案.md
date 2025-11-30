# 单位几何体 Transform 设定方案

## 问题描述

对于单位几何体（比如沿 Z 方向的直线扫描体），如何通过 local transform 来共享这个单位几何体，才能保证结果的正确性？

**关键问题**：
- 单位几何体：沿 Z 方向的单位长度扫描体（`Vec3::Z * 10.0`）
- 实际路径：任意方向的直线（比如从 `spine.pt0` 到 `spine.pt1`）
- Transform 需要：将单位几何体变换到实际路径的方向和位置

## 当前实现分析

### 1. 单位几何体的生成

**位置**: `src/prim_geo/sweep_solid.rs` 的 `gen_unit_shape()` 函数（第 195-209 行）

```rust
fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
    let mut unit = self.clone();
    if unit.path.as_single_line().is_some() && !self.is_sloped() {
        unit.extrude_dir = DVec3::Z;
        unit.path = SweepPath3D::from_line(Line3D {
            start: Default::default(),      // (0, 0, 0)
            end: Vec3::Z * 10.0,            // (0, 0, 10) - 沿 Z 方向
            is_spine: false,
        });
    }
    // 单位体不应携带原始的段变换，避免重复应用位移/缩放
    unit.segment_transforms = vec![Transform::IDENTITY];
    unit.spine_segments.clear();
    Box::new(unit)
}
```

**关键点**：
- 单位几何体的路径是沿 Z 方向的：`(0, 0, 0)` → `(0, 0, 10)`
- `segment_transforms` 被设置为 `Transform::IDENTITY`
- 截面在 XY 平面，法向量是 Z 方向

### 2. 实际路径的 Transform 设定

**位置**: `src/prim_geo/profile.rs` 的 `normalize_spine_segments()` 函数（第 89-102 行）

```rust
// 获取该段起点 POINSP 的局部旋转
let local_rotation = crate::transform::get_local_transform(spine.refno)
    .await
    .ok()
    .flatten()
    .map(|t| t.rotation)
    .unwrap_or(Quat::IDENTITY);

// 完整变换：包含位置、旋转和缩放
transforms.push(Transform {
    translation: spine.pt0,                    // 起点位置
    rotation: local_rotation,                  // POINSP 的局部旋转
    scale: Vec3::new(1.0, 1.0, length / 10.0), // Z 方向缩放：实际长度/10.0
});
```

**问题**：
- `rotation` 只包含 POINSP 的局部旋转
- **不包含**从 Z 方向到实际路径方向的旋转
- **不包含**截面法向量的旋转（基于 plax/pref_axis）

### 3. 网格生成的流程

**位置**: `src/geometry/sweep_mesh.rs` 的 `generate_sweep_solid_mesh()` 函数（第 933-965 行）

```rust
pub fn generate_sweep_solid_mesh(
    sweep: &SweepSolid,
    settings: &LodMeshSettings,
    refno: Option<RefU64>,
) -> Option<PlantMesh> {
    // ...
    let frames = sample_path_frames_sync(
        &sweep.path.segments,        // 路径段（归一化的）
        arc_segments,
        sweep.plax,                  // 参考方向
        &sweep.segment_transforms,   // 每段的 transform
    )?;
    
    let mesh = generate_mesh_from_frames(&profile, &frames, sweep.drns, sweep.drne);
    Some(mesh)
}
```

**`sample_path_frames_sync()` 的处理**（第 383-394 行）：

```rust
// 1. 变换所有段
let mut transformed_segments = Vec::new();
for (i, segment) in segments.iter().enumerate() {
    let transform = segment_transforms.get(i).unwrap_or(&Transform::IDENTITY);
    
    let transformed_segment = match segment {
        SegmentPath::Line(line) => SegmentPath::Line(transform_line(line, transform)),
        SegmentPath::Arc(arc) => transform_arc(arc, transform),
    };
    transformed_segments.push(transformed_segment);
}

// 2. 基于变换后的路径计算 Frenet 标架
// ...
```

**关键点**：
- `transform_line` 和 `transform_arc` 会应用 transform 的**所有分量**（位置、旋转、缩放）
- 然后基于变换后的路径计算 Frenet 标架
- Frenet 标架还会考虑 `plax` 和 `pref_axis`

## 正确的 Transform 设定方案

### 方案：组合三个旋转分量

Transform 的 `rotation` 应该包含三个旋转分量的组合：

1. **路径方向旋转**：从 `Vec3::Z` 旋转到实际路径方向
2. **截面法向量旋转**：基于 plax/pref_axis 计算 Frenet 标架的旋转
3. **POINSP 局部旋转**：叠加 POINSP 的局部旋转

### 实现步骤

#### 1. 计算路径方向旋转

```rust
// 对于 LINE 类型
let direction = (spine.pt1 - spine.pt0).normalize_or_zero();
let path_direction_rotation = Quat::from_rotation_arc(Vec3::Z, direction);
```

**作用**：将单位几何体的 Z 方向旋转到实际路径方向

#### 2. 计算截面法向量旋转（Frenet 标架）

```rust
// 基于路径方向和参考方向计算 Frenet 标架
let ref_up = spine.preferred_dir.normalize_or_zero();  // 或使用 plax
let right = ref_up.cross(direction).normalize_or_zero();
let up = direction.cross(right).normalize_or_zero();

// 构建 Frenet 标架的旋转矩阵
let frenet_rotation = Quat::from_mat3(&Mat3::from_cols(right, up, direction));

// 相对于 Z 方向的旋转
// 默认情况下，单位几何体的截面在 XY 平面，法向量是 Z
// 我们需要将 (X, Y, Z) 旋转到 (right, up, direction)
let frenet_relative_rotation = frenet_rotation * path_direction_rotation.inverse();
```

**注意**：这里需要仔细考虑旋转的组合顺序。

#### 3. 获取 POINSP 的局部旋转

```rust
let poinsp_local_rotation = crate::transform::get_local_transform(spine.refno)
    .await
    .ok()
    .flatten()
    .map(|t| t.rotation)
    .unwrap_or(Quat::IDENTITY);
```

#### 4. 组合最终旋转

```rust
// 最终旋转 = 路径方向旋转 × 截面法向量旋转 × POINSP 局部旋转
let final_rotation = path_direction_rotation * frenet_relative_rotation * poinsp_local_rotation;
```

**或者更简单的方式**：

```rust
// 直接使用 Frenet 标架的旋转，然后叠加 POINSP 的局部旋转
let final_rotation = frenet_rotation * poinsp_local_rotation;
```

### 简化方案：直接使用 Frenet 标架旋转

**更简单的方式**：直接计算 Frenet 标架的旋转，然后叠加 POINSP 的局部旋转。

```rust
// 对于 LINE 类型
let direction = (spine.pt1 - spine.pt0).normalize_or_zero();
let ref_up = spine.preferred_dir.normalize_or_zero();  // 或使用 plax

// 构建 Frenet 标架
let right = ref_up.cross(direction).normalize_or_zero();
let up = direction.cross(right).normalize_or_zero();
let frenet_rotation = Quat::from_mat3(&Mat3::from_cols(right, up, direction));

// 获取 POINSP 的局部旋转
let poinsp_local_rotation = crate::transform::get_local_transform(spine.refno)
    .await
    .ok()
    .flatten()
    .map(|t| t.rotation)
    .unwrap_or(Quat::IDENTITY);

// 组合：Frenet 标架旋转 × POINSP 局部旋转
let final_rotation = frenet_rotation * poinsp_local_rotation;

// 构建 Transform
transforms.push(Transform {
    translation: spine.pt0,
    rotation: final_rotation,
    scale: Vec3::new(1.0, 1.0, length / 10.0),
});
```

**关键点**：
- `frenet_rotation` 将单位几何体的坐标系 `(X, Y, Z)` 旋转到 `(right, up, direction)`
- `poinsp_local_rotation` 是 POINSP 相对于 GENSEC 的局部旋转，叠加在 Frenet 标架上

### 验证逻辑

**单位几何体**：
- 路径：`(0, 0, 0)` → `(0, 0, 10)`（沿 Z 方向）
- 截面：在 XY 平面
- 坐标系：`(X, Y, Z)`

**应用 Transform 后**：
- 路径：`spine.pt0` → `spine.pt0 + direction * length`
- 截面：在垂直于 `direction` 的平面上
- 坐标系：`(right, up, direction)`，然后叠加 POINSP 的局部旋转

**`sample_path_frames_sync()` 的处理**：
- 使用 `transform_line` 变换路径段
- 基于变换后的路径计算 Frenet 标架
- 但由于路径已经被正确旋转，Frenet 标架应该与 transform 的旋转一致（除了 POINSP 的局部旋转）

## 潜在问题

### 问题 1：双重旋转

如果 `sample_path_frames_sync()` 基于变换后的路径重新计算 Frenet 标架，可能会导致：
- Transform 已经旋转了路径
- `sample_path_frames_sync()` 又基于旋转后的路径计算 Frenet 标架
- 结果可能不一致

**解决方案**：
- 确保 `sample_path_frames_sync()` 使用的 `plax` 与计算 transform 时使用的 `pref_axis` 一致
- 或者，修改 `sample_path_frames_sync()` 直接使用 `segment_transforms` 的旋转，而不是重新计算

### 问题 2：POINSP 局部旋转的含义

`get_local_transform(poinsp_refno)` 返回的是 POINSP 相对于 GENSEC 的局部旋转。

**问题**：这个旋转是相对于什么坐标系的？
- 如果是相对于世界坐标系，那么应该直接叠加
- 如果是相对于路径的 Frenet 标架，那么需要先转换到 Frenet 标架

**需要验证**：`get_local_transform` 返回的旋转的含义。

## 推荐实现方案

### 方案 A：在 `normalize_spine_segments()` 中计算完整 Transform（推荐）

**优点**：
- Transform 包含完整的方位信息
- 复用场景下，不同的路径方向和 plax 会产生不同的 transform
- 逻辑集中，易于维护

**实现**：

```rust
// 对于 LINE 类型
let direction = (spine.pt1 - spine.pt0).normalize_or_zero();
let length = spine.pt0.distance(spine.pt1);

// 1. 计算 Frenet 标架旋转
let ref_up = spine.preferred_dir.normalize_or_zero();  // 或使用 plax
let right = ref_up.cross(direction).normalize_or_zero();
let up = direction.cross(right).normalize_or_zero();
let frenet_rotation = Quat::from_mat3(&Mat3::from_cols(right, up, direction));

// 2. 获取 POINSP 的局部旋转
let poinsp_local_rotation = crate::transform::get_local_transform(spine.refno)
    .await
    .ok()
    .flatten()
    .map(|t| t.rotation)
    .unwrap_or(Quat::IDENTITY);

// 3. 组合最终旋转
let final_rotation = frenet_rotation * poinsp_local_rotation;

// 4. 构建 Transform
transforms.push(Transform {
    translation: spine.pt0,
    rotation: final_rotation,
    scale: Vec3::new(1.0, 1.0, length / 10.0),
});
```

### 方案 B：修改 `sample_path_frames_sync()` 直接使用 Transform 的旋转

**优点**：
- 避免双重计算
- 确保一致性

**缺点**：
- 需要修改 `sample_path_frames_sync()` 的实现
- 可能影响其他使用场景

## 验证方法

1. **创建单位几何体**：
   - 沿 Z 方向的直线扫描体
   - 验证路径是 `(0, 0, 0)` → `(0, 0, 10)`

2. **应用 Transform**：
   - 将单位几何体变换到实际路径
   - 验证路径方向正确
   - 验证截面法向量正确

3. **复用验证**：
   - 创建多个使用相同 profile 但不同路径方向的 GENSEC
   - 验证它们共享相同的单位几何体
   - 验证 transform 正确设定

4. **正确性验证**：
   - 对比复用前后的几何体
   - 验证位置、旋转、缩放都正确

---

**创建时间**: 2025-01-XX  
**状态**: 🟡 待实现  
**关键问题**: 如何确保 Transform 正确地将单位几何体变换到实际路径？




