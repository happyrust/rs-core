# 修正 Transform 方案

## 用户纠正的理解

**关键点**：
1. **单位几何体应该是标准的**，不包含 `bangle`，用来被复用
2. **Transform 的旋转应该包含 `bangle`**，不应该放到几何体里

## 当前实现的问题

### 1. `hash_unit_mesh_params()` 包含了 `bangle`

**位置**: `src/prim_geo/sweep_solid.rs` 第 148-193 行

```rust
fn hash_unit_mesh_params(&self) -> u64 {
    struct Hashable<'a> {
        profile: &'a CateProfileParam,
        path: &'a SweepPath3D,
        // ...
        bangle: f32,  // ❌ 问题：bangle 不应该影响单位几何体的哈希
    }
    // ...
}
```

**问题**：
- 如果 `bangle` 在哈希中，不同的 `bangle` 会产生不同的单位几何体
- 但实际上，`bangle` 应该在 Transform 中应用，不应该影响单位几何体

### 2. `gen_unit_shape()` 保留了 `bangle`

**位置**: `src/prim_geo/sweep_solid.rs` 第 195-209 行

```rust
fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
    let mut unit = self.clone();  // ❌ 问题：保留了 bangle
    // ...
    unit.segment_transforms = vec![Transform::IDENTITY];
    unit.spine_segments.clear();
    Box::new(unit)
}
```

**问题**：
- `self.clone()` 会保留 `bangle`
- 单位几何体应该将 `bangle` 设置为 0.0

## 正确的复用逻辑

### 复用场景示例

**场景 1**：两个 GENSEC 有相同的 profile、path、plax，但不同的 `bangle`
- ✅ **应该共享**同一个单位几何体
- ✅ **Transform 不同**（因为 `bangle` 不同）

**场景 2**：两个 GENSEC 有相同的 profile、path、`bangle`，但不同的 `plax`
- ❌ **不应该共享**单位几何体（因为 `plax` 影响 Frenet 标架，进而影响单位几何体的生成）

### 正确的哈希逻辑

**应该包含在哈希中的参数**（影响单位几何体的形状）：
- `profile`：截面形状
- `path`：路径形状（归一化后）
- `drns`、`drne`：端面倾斜
- `lmirror`：镜像标记
- `plax`：参考方向（影响 Frenet 标架）

**不应该包含在哈希中的参数**（应该在 Transform 中应用）：
- `bangle`：绕路径方向的旋转（应该在 Transform 中）

## 修正后的实现方案

### 1. 修改 `hash_unit_mesh_params()` 移除 `bangle`

```rust
fn hash_unit_mesh_params(&self) -> u64 {
    // 仅对影响几何的参数取哈希：截面 + 归一化路径 + 端面倾斜/镜像
    // ✅ bangle 不在哈希中，因为它应该在 Transform 中应用
    #[derive(Serialize)]
    struct Hashable<'a> {
        profile: &'a CateProfileParam,
        path: &'a SweepPath3D,
        drns: &'a Option<DVec3>,
        drne: &'a Option<DVec3>,
        lmirror: bool,
        plax: Vec3,
        // ❌ 移除 bangle: f32,
    }

    let mut hasher = DefaultHasher::default();
    "SweepSolid".hash(&mut hasher);

    let target = /* ... */;
    
    if let Ok(bytes) = bincode::serialize(&target) {
        bytes.hash(&mut hasher);
    }

    hasher.finish()
}
```

### 2. 修改 `gen_unit_shape()` 将 `bangle` 设置为 0.0

```rust
fn gen_unit_shape(&self) -> Box<dyn BrepShapeTrait> {
    let mut unit = self.clone();
    if unit.path.as_single_line().is_some() && !self.is_sloped() {
        unit.extrude_dir = DVec3::Z;
        unit.path = SweepPath3D::from_line(Line3D {
            start: Default::default(),
            end: Vec3::Z * 10.0,
            is_spine: false,
        });
    }
    // ✅ 单位体不应携带原始的段变换，避免重复应用位移/缩放
    unit.segment_transforms = vec![Transform::IDENTITY];
    unit.spine_segments.clear();
    // ✅ 单位几何体应该是标准的，不包含 bangle
    unit.bangle = 0.0;
    Box::new(unit)
}
```

### 3. 修改 `normalize_spine_segments()` 计算包含 `bangle` 的 Transform

```rust
// 对于 LINE 类型
let direction = (spine.pt1 - spine.pt0).normalize_or_zero();
let length = spine.pt0.distance(spine.pt1);

// 1. 计算 Frenet 标架旋转
let ref_up = spine.preferred_dir.normalize_or_zero();  // 或使用 plax
let right = ref_up.cross(direction).normalize_or_zero();
let up = direction.cross(right).normalize_or_zero();
let frenet_rotation = Quat::from_mat3(&Mat3::from_cols(right, up, direction));

// 2. 计算 bangle 旋转（绕路径方向）
// bangle 是绕路径方向（Z 轴）旋转截面
// 在 Frenet 标架中，路径方向是 direction（第三个轴）
// 所以 bangle 旋转是绕 direction 轴旋转
let bangle_rotation = Quat::from_axis_angle(direction, bangle.to_radians());

// 3. 组合：Frenet 标架旋转 × bangle 旋转
// 注意：bangle 旋转应该在 Frenet 标架旋转之后应用
let final_rotation = frenet_rotation * bangle_rotation;

// 4. 获取 POINSP 的位置（只使用 translation）
let poinsp_transform = crate::transform::get_local_transform(spine.refno)
    .await
    .ok()
    .flatten()
    .unwrap_or(Transform::IDENTITY);

// 5. 构建 Transform
transforms.push(Transform {
    translation: spine.pt0,  // 使用路径起点位置
    rotation: final_rotation,  // ✅ 包含 Frenet 标架旋转 + bangle 旋转
    scale: Vec3::new(1.0, 1.0, length / 10.0),
});
```

**关键点**：
- `bangle_rotation` 是绕 `direction`（路径方向）旋转
- `final_rotation = frenet_rotation * bangle_rotation` 表示先应用 Frenet 标架旋转，然后绕路径方向旋转 `bangle`

### 4. 修改 `apply_profile_transform()` 不应用 `bangle`

```rust
/// 对截面应用 plin_pos/lmirror 变换（bangle 在 Transform 中应用，不在这里）
fn apply_profile_transform(mut profile: ProfileData, plin_pos: Vec2, lmirror: bool) -> ProfileData {
    // ✅ bangle 不在截面阶段应用，而是在 Transform 中应用
    let mat = build_profile_transform_matrix(plin_pos, 0.0, lmirror);

    for v in &mut profile.vertices {
        let p = mat.transform_point3(DVec3::new(v.pos.x as f64, v.pos.y as f64, 0.0));
        v.pos = Vec2::new(p.x as f32, p.y as f32);

        if v.normal.length_squared() > 0.0 {
            let n = mat.transform_vector3(DVec3::new(v.normal.x as f64, v.normal.y as f64, 0.0));
            v.normal = Vec2::new(n.x as f32, n.y as f32).normalize();
        }
    }

    profile
}
```

## 验证逻辑

### 单位几何体
- 路径：`(0, 0, 0)` → `(0, 0, 10)`（沿 Z 方向）
- 截面：在 XY 平面，**不包含 `bangle` 旋转**
- 坐标系：`(X, Y, Z)`
- `bangle = 0.0`

### 应用 Transform 后
- 路径：`spine.pt0` → `spine.pt0 + direction * length`
- 截面：在垂直于 `direction` 的平面上
- 坐标系：`(right, up, direction)`（Frenet 标架），然后绕 `direction` 旋转 `bangle`
- Transform 的旋转 = `frenet_rotation * bangle_rotation`

### 复用场景
- 如果两个 GENSEC 有相同的 profile、path、plax，但不同的 `bangle`：
  - ✅ 共享同一个单位几何体（因为哈希相同）
  - ✅ Transform 不同（因为 `bangle` 不同）

## 总结

### 修正后的方案

1. **单位几何体**：
   - 标准的，不包含 `bangle`
   - `hash_unit_mesh_params()` 不包含 `bangle`
   - `gen_unit_shape()` 将 `bangle` 设置为 0.0

2. **Transform**：
   - 旋转 = Frenet 标架旋转 × `bangle` 旋转（绕路径方向）
   - Translation = 路径起点位置
   - Scale = 路径长度缩放

3. **截面变换**：
   - `apply_profile_transform()` 不应用 `bangle`
   - `bangle` 在 Transform 的旋转中应用

### 优势

1. **复用友好**：相同 profile、path、plax 的 GENSEC 可以共享单位几何体
2. **逻辑清晰**：`bangle` 在 Transform 中应用，不影响单位几何体
3. **正确性**：截面法向量基于 Frenet 标架，`bangle` 正确应用

---

**创建时间**: 2025-01-XX  
**状态**: 🟡 待实现  
**关键修正**: `bangle` 应该在 Transform 中应用，不应该影响单位几何体的哈希










