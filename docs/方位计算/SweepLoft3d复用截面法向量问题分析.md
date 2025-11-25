# SweepLoft3d 复用场景下的截面法向量问题分析

## 问题核心

用户指出：**截面的法向量是 POINSP 考虑不到的**。

这意味着在几何体复用场景下，仅使用 POINSP 的 transform 可能无法正确设定截面的法向量（Frenet 标架）。

## 当前实现分析

### 1. 截面法向量（Frenet 标架）的计算

**位置**: `src/geometry/sweep_mesh.rs` 的 `sample_path_frames_sync()` 函数（第 445-520 行）

**计算过程**：

```rust
// 1. 获取路径的切线方向
let first_tan = raw_samples[0].1;  // 路径的切线方向

// 2. 根据路径类型选择合适的参考方向
let ref_up = match segments.first() {
    Some(SegmentPath::Arc(arc)) => {
        arc.pref_axis  // 使用 pref_axis (YDIR) 作为 Y 轴
    }
    Some(SegmentPath::Line(line)) if line.is_spine => {
        // 从 segments 中查找 pref_axis，或使用 plax
        segments.iter()
            .find_map(|seg| {
                if let SegmentPath::Arc(arc) = seg {
                    Some(arc.pref_axis)
                } else {
                    None
                }
            })
            .unwrap_or(plax)
    }
    _ => {
        plax  // 使用 plax 作为参考方向
    }
};

// 3. 构建 Frenet 标架
let first_right = ref_up.cross(first_tan).normalize();
let first_up = first_tan.cross(first_right).normalize();
let first_rot = Mat3::from_cols(first_right, first_up, first_tan);
```

**关键点**：
- 截面法向量基于**路径的几何特性**（切线方向）和**参考方向**（`plax`、`pref_axis`）计算
- 这些信息**不在 POINSP 的 transform 中**

### 2. POINSP Transform 的构成

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
    translation: spine.pt0,                    // POINSP 的位置
    rotation: local_rotation,                  // POINSP 相对于 GENSEC 的局部旋转
    scale: Vec3::new(1.0, 1.0, length / 10.0), // 路径长度的缩放
});
```

**关键点**：
- `get_local_transform(poinsp_refno)` 返回的是 POINSP **相对于 GENSEC** 的局部 transform
- 这个 transform **不包含**截面法向量的信息（因为截面法向量是基于路径几何计算的）

### 3. 问题的根源

**问题**：在复用场景下，如果只使用 POINSP 的 transform，无法正确设定截面的法向量，因为：

1. **截面法向量需要的信息**：
   - 路径的切线方向（`tangent`）
   - 参考方向（`plax`、`pref_axis`/`YDIR`）
   - 这些信息在 `SweepSolid` 的 `path` 和 `plax` 字段中

2. **POINSP Transform 包含的信息**：
   - POINSP 的位置（`spine.pt0`）
   - POINSP 相对于 GENSEC 的旋转（`get_local_transform(poinsp_refno).rotation`）
   - **不包含**截面法向量的信息

3. **复用场景下的问题**：
   - 如果多个 GENSEC 使用相同的 profile 和路径形状，但 `plax` 或 `pref_axis` 不同
   - 它们的截面法向量应该不同
   - 但如果只使用 POINSP 的 transform，可能无法区分

## 解决方案

### 方案 1：Transform 应该包含截面法向量信息（推荐）

**思路**：在 `normalize_spine_segments()` 中，计算包含截面法向量的完整 transform。

**实现步骤**：

1. **计算路径的 Frenet 标架**（基于路径几何和参考方向）：
   ```rust
   // 对于 LINE 类型
   let direction = (spine.pt1 - spine.pt0).normalize_or_zero();
   let ref_up = spine.preferred_dir.normalize_or_zero();
   let right = ref_up.cross(direction).normalize_or_zero();
   let up = direction.cross(right).normalize_or_zero();
   let path_frenet_rotation = Quat::from_mat3(&Mat3::from_cols(right, up, direction));
   ```

2. **获取 POINSP 的局部旋转**（相对于 Frenet 标架）：
   ```rust
   let poinsp_local_rotation = crate::transform::get_local_transform(spine.refno)
       .await
       .ok()
       .flatten()
       .map(|t| t.rotation)
       .unwrap_or(Quat::IDENTITY);
   ```

3. **组合得到最终旋转**：
   ```rust
   // 最终旋转 = 路径 Frenet 标架旋转 × POINSP 局部旋转
   let final_rotation = path_frenet_rotation * poinsp_local_rotation;
   ```

4. **构建完整 Transform**：
   ```rust
   transforms.push(Transform {
       translation: spine.pt0,
       rotation: final_rotation,  // 包含截面法向量信息
       scale: Vec3::new(1.0, 1.0, length / 10.0),
   });
   ```

**优点**：
- ✅ Transform 包含完整的截面方位信息（包括法向量）
- ✅ 复用场景下，不同的 `plax`/`pref_axis` 会产生不同的 transform
- ✅ 与 `sample_path_frames_sync()` 的计算逻辑一致

**缺点**：
- ❌ 需要修改 `normalize_spine_segments()` 的实现
- ❌ 需要确保与 `sample_path_frames_sync()` 的计算逻辑一致

### 方案 2：在 `gen_csg_shape()` 中处理截面法向量

**思路**：保持 `segment_transforms` 不变，在 `gen_csg_shape()` 中基于 `SweepSolid` 的 `path` 和 `plax` 计算截面法向量。

**实现步骤**：

1. **在 `gen_csg_shape()` 中**：
   ```rust
   fn gen_csg_shape(&self) -> anyhow::Result<crate::prim_geo::basic::CsgSharedMesh> {
       // 1. 计算几何体哈希（基于 profile、路径形状、plax 等）
       let mesh_hash = self.hash_unit_mesh_params();
       
       // 2. 检查缓存
       if let Some(cached_mesh) = SWEEP_MESH_CACHE.get(&mesh_hash) {
           return Ok(CsgSharedMesh::new((*cached_mesh.value()).clone()));
       }
       
       // 3. 生成新网格（基于单位几何体）
       // generate_sweep_solid_mesh 内部会基于 path 和 plax 计算截面法向量
       let unit_shape = self.gen_unit_shape();
       let mesh = generate_sweep_solid_mesh(unit_shape, &settings, None)?;
       
       // 4. 缓存网格
       SWEEP_MESH_CACHE.insert(mesh_hash, Arc::new(mesh.clone()));
       
       Ok(CsgSharedMesh::new(mesh))
   }
   ```

2. **确保 `hash_unit_mesh_params()` 包含 `plax`**：
   ```rust
   // 在 SweepSolid::hash_unit_mesh_params() 中
   struct Hashable<'a> {
       profile: &'a CateProfileParam,
       path: &'a SweepPath3D,
       plax: Vec3,  // ✅ 已包含
       // ...
   }
   ```

**优点**：
- ✅ 不需要修改 `normalize_spine_segments()`
- ✅ 截面法向量在网格生成时计算，逻辑集中

**缺点**：
- ❌ Transform 仍然不包含截面法向量信息
- ❌ 如果 `sample_path_frames_sync()` 使用 `segment_transforms` 的旋转，可能不一致

### 方案 3：分离路径变换和截面方位

**思路**：在 `normalize_spine_segments()` 中，分离路径变换（位置+缩放）和截面方位（旋转）。

**实现步骤**：

1. **路径变换**（只包含位置和缩放）：
   ```rust
   let path_transform = Transform {
       translation: spine.pt0,
       rotation: Quat::IDENTITY,  // 不包含旋转
       scale: Vec3::new(1.0, 1.0, length / 10.0),
   };
   ```

2. **截面方位**（基于路径几何和参考方向计算）：
   ```rust
   // 在 generate_sweep_solid_mesh 中，基于 path 和 plax 计算截面法向量
   // 然后叠加 POINSP 的局部旋转
   ```

**优点**：
- ✅ 路径几何不被旋转影响
- ✅ 截面方位基于路径几何计算

**缺点**：
- ❌ 需要大幅修改现有代码
- ❌ Transform 结构需要调整

## 推荐方案

**推荐使用方案 1**，原因：

1. **完整性**：Transform 包含完整的截面方位信息（包括法向量）
2. **一致性**：与 `sample_path_frames_sync()` 的计算逻辑一致
3. **复用友好**：不同的 `plax`/`pref_axis` 会产生不同的 transform，正确区分不同的实例

## 关键修改点

### 1. 修改 `normalize_spine_segments()`

在计算 `segment_transforms` 时，需要：

1. **计算路径的 Frenet 标架旋转**（基于路径几何和参考方向）
2. **获取 POINSP 的局部旋转**（相对于 Frenet 标架）
3. **组合得到最终旋转**（`path_frenet_rotation * poinsp_local_rotation`）

### 2. 确保 `hash_unit_mesh_params()` 包含所有影响截面法向量的参数

```rust
struct Hashable<'a> {
    profile: &'a CateProfileParam,
    path: &'a SweepPath3D,
    plax: Vec3,  // ✅ 影响截面法向量
    // pref_axis 也应该包含（如果存在）
    // ...
}
```

### 3. 验证 `sample_path_frames_sync()` 的使用

确保 `sample_path_frames_sync()` 正确使用 `segment_transforms` 的旋转，或者基于 `path` 和 `plax` 重新计算。

## 验证方法

1. **功能验证**：
   - 创建多个使用相同 profile 但不同 `plax` 的 GENSEC
   - 验证它们的截面法向量不同
   - 验证 transform 正确反映截面方位

2. **复用验证**：
   - 创建多个使用相同 profile、相同路径形状、相同 `plax` 的 GENSEC
   - 验证它们共享相同的几何体
   - 验证 transform 正确设定

3. **正确性验证**：
   - 验证截面法向量与 `sample_path_frames_sync()` 的计算结果一致
   - 验证 POINSP 的局部旋转正确叠加在 Frenet 标架上

---

**创建时间**: 2025-01-XX  
**状态**: 🟡 待实现  
**关键问题**: 如何在复用场景下正确设定截面的法向量？
