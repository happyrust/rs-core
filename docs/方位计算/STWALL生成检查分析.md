# STWALL 25688/7958 生成检查分析

## 问题

参考 SweepLoft3d 的 Transform 修正方案，检查 STWALL 25688/7958 的生成是否合理，并可以导出 obj 模型。

## 发现的问题

### 1. STWALL 未包含在 `create_profile_geos` 的 SPINE 处理中

**位置**: `src/prim_geo/profile.rs` 第 310 行

**当前代码**:
```rust
let mut spine_paths = if type_name == "GENSEC" || type_name == "WALL" {
    // 处理 SPINE 路径
    // ...
} else {
    vec![]
};
```

**问题**:
- STWALL 在 `HAS_PLIN_TYPES` 中（`consts.rs` 第 272 行），说明它应该有 PLIN（profile）
- 但是 STWALL 没有包含在 SPINE 处理的条件中
- 这意味着 STWALL 如果有 SPINE，不会走多段路径的处理逻辑

### 2. `normalize_spine_segments` 函数签名已修改

**位置**: `src/prim_geo/profile.rs` 第 41-45 行

**当前状态**:
- 函数注释显示需要 `plax` 和 `bangle` 参数
- 函数实现（第 96-120 行）已经使用了 `plax` 和 `bangle`
- 调用处（第 454 行）已经传递了 `first_plax` 和 `bangle`

**但是**:
- 函数签名可能还没有更新（需要检查第 41 行的实际签名）

### 3. STWALL 的 Transform 计算

**如果 STWALL 走 SCTN 路径**（无 SPINE，只有 POSS/POSE）:
- 第 384-437 行处理 SCTN 类型
- 使用 `bangle: att.get_f32("BANG").unwrap_or_default()`（第 409 行）
- Transform 的 rotation 是 `Quat::IDENTITY`（第 358 行）
- **问题**: SCTN 类型的 Transform 没有应用 `bangle` 旋转

**如果 STWALL 走 GENSEC/WALL 路径**（有 SPINE）:
- 需要修改第 310 行，添加 `type_name == "STWALL"`

## 需要检查的事项

### 1. STWALL 25688/7958 的实际结构

需要检查：
- 是否有 SPINE？
- 是否有 PLIN（Profile）？
- 是否有 POSS/POSE？
- `bangle` 值是多少？
- `plax` 值是多少？

### 2. Transform 计算是否正确

如果 STWALL 有 SPINE：
- 需要确保 `normalize_spine_segments` 正确计算了 Frenet 标架旋转和 `bangle` 旋转
- 需要确保单位几何体不包含 `bangle`（`hash_unit_mesh_params` 和 `gen_unit_shape`）

如果 STWALL 只有 POSS/POSE（SCTN 路径）：
- 需要修正 Transform 的 rotation，应用 `bangle` 旋转

## 建议的修改

### 1. 修改 `create_profile_geos` 添加 STWALL 支持

```rust
// 第 310 行
let mut spine_paths = if type_name == "GENSEC" || type_name == "WALL" || type_name == "STWALL" {
    // 处理 SPINE 路径
    // ...
} else {
    vec![]
};
```

### 2. 修正 SCTN 类型的 Transform（如果 STWALL 走这个路径）

如果 STWALL 没有 SPINE，走 SCTN 路径，需要修正 Transform 的 rotation：

```rust
// 第 356-361 行
// SCTN 类型：需要应用 bangle 旋转
let bangle = att.get_f32("BANG").unwrap_or_default();
let bangle_rotation = Quat::from_axis_angle(Vec3::Z, bangle.to_radians());

let transform = Transform {
    rotation: bangle_rotation,  // ✅ 应用 bangle 旋转
    scale: solid.get_scaled_vec3(),
    translation: poss,
};
```

### 3. 验证 `normalize_spine_segments` 函数签名

检查函数签名是否已经更新为：
```rust
async fn normalize_spine_segments(
    segments: Vec<Spine3D>,
    plax: Vec3,        // 参考方向
    bangle: f32,       // 绕路径方向的旋转角度
) -> anyhow::Result<(Vec<SegmentPath>, Vec<Transform>)>
```

## 测试脚本

创建一个测试脚本来：
1. 检查 STWALL 25688/7958 的属性（SPINE、PLIN、POSS/POSE、BANG、PLAX）
2. 调用 `create_profile_geos` 生成几何
3. 检查生成的 Transform 是否正确
4. 导出 obj 模型

**测试脚本位置**: `src/test/test_stwall_25688_7958.rs`

## 下一步

1. **检查 STWALL 25688/7958 的实际结构**：运行测试脚本，查看它是否有 SPINE
2. **根据实际情况修改代码**：
   - 如果有 SPINE：添加 STWALL 到第 310 行的条件
   - 如果只有 POSS/POSE：修正 SCTN 路径的 Transform
3. **验证 Transform 计算**：确保 `bangle` 正确应用
4. **导出 obj 模型**：验证生成的几何是否正确

---

**创建时间**: 2025-01-XX  
**状态**: 🟡 待检查  
**关键问题**: STWALL 是否走 SPINE 路径？Transform 是否正确应用了 `bangle`？










