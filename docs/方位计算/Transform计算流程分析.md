# Transform 计算流程分析

## 概述

Transform 用于将归一化的单位几何体（mesh）实例化到实际位置和尺寸。本文档详细分析 Transform 的计算流程。

## 计算流程

### 1. 多段 SPINE 路径（GENSEC/WALL/STWALL）

#### 1.1 调用 `normalize_spine_segments`

**位置**: `profile.rs` 第 459 行

```rust
match normalize_spine_segments(spine_paths.clone(), first_plax, bangle).await {
    Ok((normalized_paths, segment_transforms)) => {
        // ...
    }
}
```

**参数**:
- `spine_paths`: 原始 Spine3D 段列表（包含实际世界坐标）
- `first_plax`: 第一个 profile 的参考方向（用于计算 Frenet 标架）
- `bangle`: 绕路径方向的旋转角度（度数）

**返回**:
- `normalized_paths`: 归一化路径段（LINE: 10.0单位，ARC: 1.0半径）
- `segment_transforms`: 每段的完整 Transform（包含 translation、rotation、scale）

#### 1.2 `normalize_spine_segments` 内部计算

**函数签名**: `profile.rs` 第 46-50 行

```rust
async fn normalize_spine_segments(
    segments: Vec<Spine3D>,
    plax: Vec3,
    bangle: f32,
) -> anyhow::Result<(Vec<SegmentPath>, Vec<Transform>)>
```

##### LINE 段的 Transform 计算（第 84-120 行）

```rust
// 1. 计算实际方向和长度
let direction = (spine.pt1 - spine.pt0).normalize_or_zero();
let length = spine.pt0.distance(spine.pt1);

// 2. 创建归一化路径（固定 10.0 单位长度）
normalized_segments.push(SegmentPath::Line(Line3D {
    start: Vec3::ZERO,
    end: Vec3::Z * 10.0,
    is_spine: true,
}));

// 3. 计算 Frenet 标架旋转
let ref_up = plax.normalize_or_zero();
let right = ref_up.cross(direction).normalize_or_zero();
let up = direction.cross(right).normalize_or_zero();
let frenet_rotation = Quat::from_mat3(&Mat3::from_cols(right, up, direction));

// 4. 计算 bangle 旋转（绕路径方向）
let bangle_rotation = Quat::from_axis_angle(direction, bangle.to_radians());

// 5. 组合旋转
let final_rotation = frenet_rotation * bangle_rotation;

// 6. 创建 Transform
transforms.push(Transform {
    translation: spine.pt0,                    // ✅ 起点位置（实际世界坐标）
    rotation: final_rotation,                  // ✅ Frenet 标架旋转 × bangle 旋转
    scale: Vec3::new(1.0, 1.0, length / 10.0), // ✅ Z 方向缩放：实际长度/10.0
});
```

**Transform 组成**:
- `translation`: `spine.pt0`（实际起点位置）
- `rotation`: `frenet_rotation * bangle_rotation`（Frenet 标架旋转 × bangle 旋转）
- `scale`: `Vec3::new(1.0, 1.0, length / 10.0)`（Z 方向缩放系数）

##### ARC 段的 Transform 计算（第 122-177 行，THRU 和 CENT 类似）

```rust
// 1. 计算圆心和半径
let center = circum_center(spine.pt0, spine.pt1, spine.thru_pt); // 或 spine.center_pt
let radius = center.distance(spine.pt0);

// 2. 创建归一化圆弧（单位半径）
normalized_segments.push(SegmentPath::Arc(Arc3D {
    center: Vec3::ZERO,
    radius: 1.0,
    // ...
}));

// 3. 计算切线方向
let radial = (spine.pt0 - center).normalize_or_zero();
let tangent = axis.cross(radial).normalize_or_zero(); // 或取反（根据 clockwise）

// 4. 计算 Frenet 标架旋转
let ref_dir = spine.preferred_dir.normalize_or_zero(); // 或 plax
let right = ref_dir.cross(tangent).normalize_or_zero();
let up = tangent.cross(right).normalize_or_zero();
let frenet_rotation = Quat::from_mat3(&Mat3::from_cols(right, up, tangent));

// 5. 计算 bangle 旋转
let bangle_rotation = Quat::from_axis_angle(tangent, bangle.to_radians());

// 6. 组合旋转
let final_rotation = frenet_rotation * bangle_rotation;

// 7. 创建 Transform
transforms.push(Transform {
    translation: center,        // ✅ 圆心位置（实际世界坐标）
    rotation: final_rotation,   // ✅ Frenet 标架旋转 × bangle 旋转
    scale: Vec3::splat(radius), // ✅ 统一缩放到实际半径
});
```

**Transform 组成**:
- `translation`: `center`（实际圆心位置）
- `rotation`: `frenet_rotation * bangle_rotation`（Frenet 标架旋转 × bangle 旋转）
- `scale`: `Vec3::splat(radius)`（统一缩放到实际半径）

#### 1.3 获取第一段的 Transform 用于实例化

**位置**: `profile.rs` 第 511-528 行

```rust
// 获取第一段的完整变换用于实例化
let first_transform = segment_transforms
    .first()
    .cloned()
    .unwrap_or(Transform::IDENTITY);

// 实例化 Transform：使用 translation、rotation 和 scale
// mesh 是基于归一化路径生成的，所以实例化时需要应用 scale 来缩放回实际尺寸
let transform = first_transform;
```

**最终 Transform**:
- `translation`: 第一段的起点位置（LINE）或圆心位置（ARC）
- `rotation`: 第一段的 Frenet 标架旋转 × bangle 旋转
- `scale`: 第一段的缩放系数（LINE: `Vec3::new(1.0, 1.0, length/10.0)`，ARC: `Vec3::splat(radius)`）

### 2. SCTN 路径（无 SPINE，只有 POSS/POSE）

#### 2.1 计算实际高度

**位置**: `profile.rs` 第 384-387 行

```rust
if let Some(poss) = att.get_poss()
    && let Some(pose) = att.get_pose()
{
    let height = pose.distance(poss);  // ✅ 实际高度
```

#### 2.2 创建归一化路径和 SweepSolid

**位置**: `profile.rs` 第 396-416 行

```rust
// 创建归一化路径（固定 10.0 单位长度）
let path = Line3D {
    start: Default::default(),
    end: Vec3::Z * 10.0,
    is_spine: false,
};

let solid = SweepSolid {
    profile: profile.clone(),
    // ...
    height: 1.0,  // 归一化高度
    path: SweepPath3D::from_line(path),
    // ...
    segment_transforms: vec![], // SCTN 无需局部变换
};
```

#### 2.3 计算 Transform

**位置**: `profile.rs` 第 418-428 行

```rust
// SCTN 类型：使用 POSS 位置、缩放和 bangle 旋转
let bangle = att.get_f32("BANG").unwrap_or_default();
let bangle_rotation = Quat::from_axis_angle(Vec3::Z, bangle.to_radians());

// scale: 归一化路径长度为 10.0，实际高度为 height，所以 Z 方向缩放为 height / 10.0
let scale = Vec3::new(1.0, 1.0, height / 10.0);

let transform = Transform {
    rotation: bangle_rotation,  // ✅ 应用 bangle 旋转（绕 Z 轴）
    scale,                        // ✅ Z 方向缩放：height / 10.0
    translation: poss,            // ✅ POSS 位置（实际起点）
};
```

**最终 Transform**:
- `translation`: `poss`（POSS 位置，实际起点）
- `rotation`: `bangle_rotation`（绕 Z 轴的 bangle 旋转）
- `scale`: `Vec3::new(1.0, 1.0, height / 10.0)`（Z 方向缩放系数）

## Transform 组成总结

### 多段 SPINE 路径

| 组件 | LINE 段 | ARC 段 |
|------|---------|--------|
| `translation` | `spine.pt0`（起点） | `center`（圆心） |
| `rotation` | `frenet_rotation * bangle_rotation` | `frenet_rotation * bangle_rotation` |
| `scale` | `Vec3::new(1.0, 1.0, length/10.0)` | `Vec3::splat(radius)` |

### SCTN 路径

| 组件 | 值 |
|------|-----|
| `translation` | `poss`（POSS 位置） |
| `rotation` | `bangle_rotation`（绕 Z 轴） |
| `scale` | `Vec3::new(1.0, 1.0, height/10.0)` |

## 关键点

1. **归一化路径**: 所有路径都被归一化到标准尺寸（LINE: 10.0单位，ARC: 1.0半径）
2. **Transform 的作用**: 将归一化几何体实例化到实际位置和尺寸
3. **Scale 的计算**:
   - LINE: `length / 10.0`（实际长度 / 归一化长度）
   - ARC: `radius`（实际半径 / 归一化半径 1.0）
   - SCTN: `height / 10.0`（实际高度 / 归一化高度）
4. **Rotation 的组成**:
   - SPINE: `frenet_rotation * bangle_rotation`（Frenet 标架旋转 × bangle 旋转）
   - SCTN: `bangle_rotation`（仅 bangle 旋转）
5. **Translation**: 实际世界坐标位置（起点或圆心）

## 使用场景

Transform 在以下场景中使用：
1. **Mesh 生成时**: `segment_transforms` 用于将归一化路径变换为实际尺寸路径（`sweep_mesh.rs` 第 384-395 行）
2. **实例化时**: `transform` 用于将归一化 mesh 实例化到实际位置和尺寸（`profile.rs` 第 537 行）

---

**创建时间**: 2025-01-XX  
**相关文件**: 
- `src/prim_geo/profile.rs` (normalize_spine_segments, create_profile_geos)
- `src/geometry/sweep_mesh.rs` (sample_path_frames_sync)





