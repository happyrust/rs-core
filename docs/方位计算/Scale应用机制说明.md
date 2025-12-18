# Scale 应用机制说明

## 问题

"scale 已在 mesh 生成时应用到归一化路径" 是什么意思？

## 核心概念

### 1. 归一化路径（Normalized Path）

在 `normalize_spine_segments` 函数中，所有路径段都被归一化到标准尺寸：

**LINE 段**：
```rust
// 归一化路径：从原点沿 Z 轴10.0单位长度
normalized_segments.push(SegmentPath::Line(Line3D {
    start: Vec3::ZERO,
    end: Vec3::Z * 10.0,  // 固定 10.0 单位长度
    is_spine: true,
}));
```

**ARC 段**：
```rust
// 归一化圆弧：从原点开始，单位半径
normalized_segments.push(SegmentPath::Arc(Arc3D {
    center: Vec3::ZERO,
    radius: 1.0,  // 固定单位半径
    // ...
}));
```

**目的**：创建标准化的单位几何体，便于复用和缓存。

### 2. segment_transforms 中的 Scale

`segment_transforms` 存储了每段的完整变换，包括 `scale`：

**LINE 段**：
```rust
transforms.push(Transform {
    translation: spine.pt0,                    // 起点位置
    rotation: final_rotation,                  // Frenet 标架旋转 × bangle 旋转
    scale: Vec3::new(1.0, 1.0, length / 10.0), // Z 方向缩放：实际长度/10.0
});
```

**ARC 段**：
```rust
transforms.push(Transform {
    translation: center,        // 圆心位置
    rotation: final_rotation,   // Frenet 标架旋转 × bangle 旋转
    scale: Vec3::splat(radius), // 统一缩放到实际半径
});
```

**作用**：将归一化路径缩放回实际尺寸。

### 3. Mesh 生成时应用 Scale

在 `sample_path_frames_sync` 函数中（`sweep_mesh.rs` 第 384-395 行），使用 `segment_transforms` 中的 Transform（包含 scale）来变换归一化路径：

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
```

**`transform_line` 函数**（第 319-325 行）：
```rust
fn transform_line(line: &Line3D, transform: &Transform) -> Line3D {
    Line3D {
        start: transform.transform_point(line.start),  // 应用 translation + rotation + scale
        end: transform.transform_point(line.end),      // 应用 translation + rotation + scale
        is_spine: line.is_spine,
    }
}
```

**`transform_arc` 函数**（第 328-343 行）：
```rust
fn transform_arc(arc: &Arc3D, transform: &Transform) -> SegmentPath {
    let scale = transform.scale;
    // ...
    SegmentPath::Arc(Arc3D {
        center: transform.transform_point(arc.center),
        radius: arc.radius * scale.x,  // 应用 scale 缩放半径
        // ...
    })
}
```

**关键点**：`transform.transform_point()` 会同时应用 `translation`、`rotation` 和 `scale`，将归一化路径转换为实际尺寸的路径。

### 4. 生成的 Mesh 已经是实际尺寸

经过上述变换后，`transformed_segments` 中的路径已经是实际尺寸：
- LINE：从 `10.0` 单位长度 → `length`（实际长度）
- ARC：从 `1.0` 单位半径 → `radius`（实际半径）

然后基于这些实际尺寸的路径生成 mesh，所以生成的 mesh 已经是实际尺寸了。

### 5. 实例化时不需要 Scale

在 `create_profile_geos` 函数中（`profile.rs` 第 521-528 行），实例化 Transform 只使用 `translation` 和 `rotation`：

```rust
// 实例化 Transform：只使用 translation 和 rotation
// scale 已经在 mesh 生成时通过 segment_transforms 应用到归一化路径上了
// 实例化时不应该再次应用 scale，否则会导致双重缩放
let transform = Transform {
    translation: first_transform.translation,
    rotation: first_transform.rotation,
    scale: Vec3::ONE,  // 实例化时不需要 scale
};
```

**原因**：
- Mesh 已经是实际尺寸（在生成时已经应用了 scale）
- 实例化时只需要位置（translation）和方向（rotation）
- 如果再次应用 scale，会导致双重缩放，几何体会变大

## 完整流程图示

```
1. 归一化阶段（normalize_spine_segments）
   ┌─────────────────────────────────────┐
   │ 实际路径: LINE(0,0,0) → (0,0,50)    │  length = 50
   └─────────────────────────────────────┘
                    ↓
   ┌─────────────────────────────────────┐
   │ 归一化路径: LINE(0,0,0) → (0,0,10)   │  固定 10.0 单位
   └─────────────────────────────────────┘
                    ↓
   ┌─────────────────────────────────────┐
   │ segment_transforms[0].scale          │  scale.z = 50/10 = 5.0
   │   = Vec3(1.0, 1.0, 5.0)              │
   └─────────────────────────────────────┘

2. Mesh 生成阶段（sample_path_frames_sync）
   ┌─────────────────────────────────────┐
   │ 归一化路径: LINE(0,0,0) → (0,0,10)   │
   └─────────────────────────────────────┘
                    ↓ 应用 segment_transforms[0]
   ┌─────────────────────────────────────┐
   │ 实际路径: LINE(0,0,0) → (0,0,50)      │  scale 已应用
   └─────────────────────────────────────┘
                    ↓ 生成 mesh
   ┌─────────────────────────────────────┐
   │ Mesh（实际尺寸，长度 = 50）           │
   └─────────────────────────────────────┘

3. 实例化阶段（create_profile_geos）
   ┌─────────────────────────────────────┐
   │ Mesh（实际尺寸，长度 = 50）           │
   └─────────────────────────────────────┘
                    ↓ 应用实例化 Transform
   ┌─────────────────────────────────────┐
   │ Transform {                          │
   │   translation: spine.pt0,            │  位置
   │   rotation: final_rotation,          │  方向
   │   scale: Vec3::ONE,                  │  不需要缩放
   │ }                                    │
   └─────────────────────────────────────┘
                    ↓
   ┌─────────────────────────────────────┐
   │ 最终几何体（正确尺寸和位置）          │
   └─────────────────────────────────────┘
```

## 总结

1. **归一化路径**：标准尺寸（LINE: 10.0 单位，ARC: 1.0 半径）
2. **segment_transforms.scale**：将归一化路径缩放回实际尺寸的系数
3. **Mesh 生成时**：通过 `transform_line`/`transform_arc` 应用 scale，得到实际尺寸的路径和 mesh
4. **实例化时**：Mesh 已经是实际尺寸，只需要位置和旋转，scale 设为 `Vec3::ONE`

**关键点**：Scale 在 mesh 生成阶段应用一次，实例化阶段不再应用，避免双重缩放。

---

**创建时间**: 2025-01-XX  
**相关文件**: 
- `src/prim_geo/profile.rs` (normalize_spine_segments)
- `src/geometry/sweep_mesh.rs` (sample_path_frames_sync, transform_line, transform_arc)













