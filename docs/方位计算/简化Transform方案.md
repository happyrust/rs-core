# 简化 Transform 方案

## 用户提出的方案

**核心思路**：
- POINSP 的局部变换，只使用它的第一个点的 **translation（位置）**
- 旋转使用 **Frenet 标架旋转**，不使用 POINSP 的局部旋转

## 方案分析

### 1. 当前实现的问题

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
    rotation: local_rotation,                  // ❌ 使用 POINSP 的局部旋转
    scale: Vec3::new(1.0, 1.0, length / 10.0),
});
```

**问题**：
- POINSP 的局部旋转可能不完全匹配 Frenet 标架的计算
- 因为参考方向（`spine_ydir`/`YDIR`）可能与 `plax`/`pref_axis` 不一致

### 2. 简化方案的优势

**优势**：
1. **简化实现**：不需要提取 POINSP 的额外旋转
2. **确保一致性**：Frenet 标架的计算与 `sample_path_frames_sync()` 一致
3. **正确性**：截面法向量基于 Frenet 标架，确保正确

### 3. 关键问题：`bangle` 如何处理？

**问题**：`bangle` 是绕路径方向（Z 轴）旋转截面。如果 Transform 的旋转不使用 POINSP 的局部旋转，那么 `bangle` 如何应用？

**分析**：

1. **`bangle` 在 `hash_unit_mesh_params()` 中被包含**（第 158 行）：
   ```rust
   struct Hashable<'a> {
       // ...
       bangle: f32,
   }
   ```
   这意味着不同的 `bangle` 会产生不同的单位几何体。

2. **`bangle` 在 `apply_profile_transform()` 中被设置为 0.0**（第 216 行）：
   ```rust
   // bangle 交由 get_local_transform 处理，截面阶段只做平移和镜像
   let mat = build_profile_transform_matrix(plin_pos, 0.0, lmirror);
   ```
   这说明 `bangle` 原本是通过 `get_local_transform` 的旋转应用的。

3. **如果 Transform 不使用 POINSP 的旋转，`bangle` 需要在哪里应用？**

**解决方案**：

**方案 A：`bangle` 在单位几何体生成时应用**（推荐）

- 修改 `apply_profile_transform()` 或 `get_profile_data()`，在生成单位几何体时应用 `bangle`
- Transform 的旋转只包含 Frenet 标架旋转
- 这样，`bangle` 的效果会被保留在单位几何体中

**方案 B：`bangle` 在 Frenet 标架计算后应用**

- 在 `sample_path_frames_sync()` 中，计算 Frenet 标架后，应用 `bangle` 旋转
- 这样，`bangle` 的效果会被保留在路径采样帧中

**推荐方案 A**，因为：
- `bangle` 已经在 `hash_unit_mesh_params()` 中被包含，说明它应该影响单位几何体
- 在单位几何体生成时应用 `bangle`，逻辑更清晰

## 推荐实现方案

### 1. 修改 `normalize_spine_segments()` 计算 Transform

```rust
// 对于 LINE 类型
let direction = (spine.pt1 - spine.pt0).normalize_or_zero();
let length = spine.pt0.distance(spine.pt1);

// 1. 计算 Frenet 标架旋转
let ref_up = spine.preferred_dir.normalize_or_zero();  // 或使用 plax
let right = ref_up.cross(direction).normalize_or_zero();
let up = direction.cross(right).normalize_or_zero();
let frenet_rotation = Quat::from_mat3(&Mat3::from_cols(right, up, direction));

// 2. 获取 POINSP 的位置（只使用 translation）
let poinsp_translation = crate::transform::get_local_transform(spine.refno)
    .await
    .ok()
    .flatten()
    .map(|t| t.translation)
    .unwrap_or(Vec3::ZERO);

// 3. 构建 Transform（只使用 Frenet 标架旋转）
transforms.push(Transform {
    translation: spine.pt0,  // 或使用 poinsp_translation，取决于坐标系
    rotation: frenet_rotation,  // ✅ 只使用 Frenet 标架旋转
    scale: Vec3::new(1.0, 1.0, length / 10.0),
});
```

### 2. 修改 `apply_profile_transform()` 应用 `bangle`

```rust
/// 对截面应用 plin_pos/bangle/lmirror 变换
fn apply_profile_transform(
    mut profile: ProfileData, 
    plin_pos: Vec2, 
    bangle: f32,  // ✅ 新增 bangle 参数
    lmirror: bool
) -> ProfileData {
    // ✅ 现在 bangle 在这里应用，而不是通过 get_local_transform
    let mat = build_profile_transform_matrix(plin_pos, bangle, lmirror);

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

### 3. 修改 `generate_sweep_solid_mesh()` 传递 `bangle`

```rust
pub fn generate_sweep_solid_mesh(
    sweep: &SweepSolid,
    settings: &LodMeshSettings,
    refno: Option<RefU64>,
) -> Option<PlantMesh> {
    // ✅ 传递 bangle 到 apply_profile_transform
    let profile = get_profile_data(&sweep.profile, refno)?;
    let profile = apply_profile_transform(
        profile, 
        sweep.profile.get_plin_pos(), 
        sweep.bangle,  // ✅ 传递 bangle
        sweep.lmirror
    );

    // ... 其余代码不变
}
```

## 验证逻辑

### 单位几何体
- 路径：`(0, 0, 0)` → `(0, 0, 10)`（沿 Z 方向）
- 截面：在 XY 平面，**已应用 `bangle` 旋转**
- 坐标系：`(X, Y, Z)`

### 应用 Transform 后
- 路径：`spine.pt0` → `spine.pt0 + direction * length`
- 截面：在垂直于 `direction` 的平面上，**保持 `bangle` 旋转**
- 坐标系：`(right, up, direction)`（Frenet 标架）

### `sample_path_frames_sync()` 的处理
- 使用 `transform_line` 变换路径段（应用 Transform 的旋转和缩放）
- 基于变换后的路径计算 Frenet 标架
- 由于 Transform 的旋转已经是 Frenet 标架旋转，结果应该一致

## 潜在问题

### 问题 1：POINSP 的位置 vs `spine.pt0`

**问题**：应该使用 POINSP 的位置（`poinsp_translation`）还是 `spine.pt0`？

**分析**：
- `spine.pt0` 是路径段的起点位置
- POINSP 的位置可能包含额外的偏移（`NPOS` 属性）

**建议**：使用 `spine.pt0`，因为：
- 路径段的起点位置更准确
- POINSP 的位置可能包含相对于路径的偏移，不应该直接使用

### 问题 2：`bangle` 的坐标系

**问题**：`bangle` 是绕哪个轴旋转的？

**分析**：
- `bangle` 是绕路径方向（Z 轴）旋转截面
- 在单位几何体中，路径方向是 Z 轴
- 在应用 Transform 后，路径方向是 `direction`

**验证**：如果 `bangle` 在单位几何体生成时应用（绕 Z 轴），然后应用 Transform（包含 Frenet 标架旋转），`bangle` 的效果应该被保留。

## 总结

### 简化方案的优势

1. **简化实现**：不需要提取 POINSP 的额外旋转
2. **确保一致性**：Frenet 标架的计算与 `sample_path_frames_sync()` 一致
3. **正确性**：截面法向量基于 Frenet 标架，确保正确
4. **复用友好**：如果两个 GENSEC 有相同的 `bangle` 和 `plax`，可以共享单位几何体

### 需要修改的地方

1. **`normalize_spine_segments()`**：只使用 POINSP 的 translation，旋转使用 Frenet 标架
2. **`apply_profile_transform()`**：应用 `bangle` 旋转
3. **`generate_sweep_solid_mesh()`**：传递 `bangle` 到 `apply_profile_transform()`

---

**创建时间**: 2025-01-XX  
**状态**: 🟡 待实现  
**关键问题**: `bangle` 应该在单位几何体生成时应用，还是在 Transform 中应用？




