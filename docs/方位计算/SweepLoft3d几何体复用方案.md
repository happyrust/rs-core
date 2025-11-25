# SweepLoft3d 几何体复用方案

## 问题描述

当前 `SweepLoft3d`（`SweepSolid`）在生成几何体时，每个实例都会重新生成网格，即使它们使用相同的 profile 和路径参数。对于大量使用相同 profile 的 GENSEC，这会导致：

1. **内存浪费**：重复存储相同的几何数据
2. **性能问题**：重复计算相同的网格
3. **加载时间延长**：大量重复的几何生成操作

## 当前实现状态

### 已有的基础设施

`SweepSolid` 已经实现了复用所需的基础方法：

1. **`is_reuse_unit()`**：返回 `true`，表示支持复用
2. **`hash_unit_mesh_params()`**：计算影响几何的参数哈希值
   - 包含：profile、归一化路径、端面倾斜、镜像标记、plax、bangle
   - 排除：位置、缩放（通过 transform 处理）
3. **`gen_unit_shape()`**：生成归一化的单位几何体
   - 单段直线路径归一化为沿 Z 轴的单位长度
   - 清除 `segment_transforms` 和 `spine_segments`

### 当前问题

在 `src/prim_geo/profile.rs` 第 454 行有注释：
```rust
//先暂时不做几何体共享
```

这意味着虽然基础设施已准备好，但实际使用时还没有实现复用逻辑。

## 解决方案

### 方案 1：基于哈希的全局缓存（推荐）

在生成 `SweepSolid` 几何体时，使用 `hash_unit_mesh_params()` 作为缓存键，复用已生成的网格。

#### 实现步骤

1. **创建全局缓存结构**：
   ```rust
   // 在 src/prim_geo/basic.rs 或新建 src/prim_geo/mesh_cache.rs
   use dashmap::DashMap;
   use std::sync::Arc;
   use crate::shape::pdms_shape::PlantMesh;
   
   pub type SweepMeshCache = Arc<DashMap<u64, Arc<PlantMesh>>>;
   
   // 全局缓存实例（线程安全）
   lazy_static::lazy_static! {
       pub static ref SWEEP_MESH_CACHE: SweepMeshCache = Arc::new(DashMap::new());
   }
   ```

2. **修改 `SweepSolid::gen_csg_shape()`**：
   ```rust
   fn gen_csg_shape(&self) -> anyhow::Result<crate::prim_geo::basic::CsgSharedMesh> {
       use crate::geometry::sweep_mesh::generate_sweep_solid_mesh;
       use crate::mesh_precision::LodMeshSettings;
       use crate::prim_geo::basic::SWEEP_MESH_CACHE;
       
       // 1. 计算哈希值
       let hash = self.hash_unit_mesh_params();
       
       // 2. 检查缓存
       if let Some(cached_mesh) = SWEEP_MESH_CACHE.get(&hash) {
           return Ok(crate::prim_geo::basic::CsgSharedMesh::new(
               (*cached_mesh.value()).clone()
           ));
       }
       
       // 3. 生成新网格
       let settings = LodMeshSettings::default();
       let mesh = generate_sweep_solid_mesh(self, &settings, None)
           .ok_or_else(|| anyhow::anyhow!("SweepSolid 网格生成失败"))?;
       
       // 4. 存入缓存
       let mesh_arc = Arc::new(mesh.clone());
       SWEEP_MESH_CACHE.insert(hash, mesh_arc.clone());
       
       Ok(crate::prim_geo::basic::CsgSharedMesh::new(mesh))
   }
   ```

3. **优化：使用单位几何体生成**：
   ```rust
   fn gen_csg_shape(&self) -> anyhow::Result<crate::prim_geo::basic::CsgSharedMesh> {
       use crate::geometry::sweep_mesh::generate_sweep_solid_mesh;
       use crate::mesh_precision::LodMeshSettings;
       use crate::prim_geo::basic::SWEEP_MESH_CACHE;
       
       // 1. 计算哈希值（基于单位几何体）
       let hash = self.hash_unit_mesh_params();
       
       // 2. 检查缓存
       if let Some(cached_mesh) = SWEEP_MESH_CACHE.get(&hash) {
           // 直接返回缓存的网格（位置和缩放通过 transform 处理）
           return Ok(crate::prim_geo::basic::CsgSharedMesh::new(
               (*cached_mesh.value()).clone()
           ));
       }
       
       // 3. 使用单位几何体生成网格（避免位置/缩放影响）
       let unit_shape = self.gen_unit_shape();
       let settings = LodMeshSettings::default();
       
       // 需要将 unit_shape 转换为 SweepSolid
       let unit_sweep = unit_shape.downcast_ref::<SweepSolid>()
           .ok_or_else(|| anyhow::anyhow!("无法转换为 SweepSolid"))?;
       
       let mesh = generate_sweep_solid_mesh(unit_sweep, &settings, None)
           .ok_or_else(|| anyhow::anyhow!("SweepSolid 单位网格生成失败"))?;
       
       // 4. 存入缓存
       let mesh_arc = Arc::new(mesh.clone());
       SWEEP_MESH_CACHE.insert(hash, mesh_arc.clone());
       
       Ok(crate::prim_geo::basic::CsgSharedMesh::new(mesh))
   }
   ```

### 方案 2：在 profile.rs 中实现复用

在 `normalize_spine_segments()` 生成 `SweepSolid` 时，检查是否已有相同哈希的几何体。

#### 实现步骤

1. **在 `profile.rs` 中添加缓存检查**：
   ```rust
   use crate::prim_geo::basic::SWEEP_MESH_CACHE;
   
   // 在生成 loft 后
   let loft = SweepSolid { ... };
   
   // 计算哈希
   let mesh_hash = loft.hash_unit_mesh_params();
   
   // 检查缓存（可选：提前检查，避免创建 SweepSolid）
   // 如果缓存命中，可以标记为复用实例
   let is_cached = SWEEP_MESH_CACHE.contains_key(&mesh_hash);
   
   csg_shapes_map
       .entry(refno)
       .or_insert(Vec::new())
       .push(CateCsgShape {
           refno: RefU64(hash).into(),
           csg_shape: Box::new(loft),
           transform,
           visible: true,
           is_tubi: false,
           shape_err: None,
           pts: vec![],
           is_ngmr: false,
       });
   ```

2. **在 `gen_csg_shape()` 中实现缓存逻辑**（同方案 1）

### 方案 3：延迟生成 + 缓存

在需要时才生成网格，并缓存结果。

#### 优势

- 避免不必要的网格生成
- 更好的内存管理
- 支持按需加载

#### 实现

在 `gen_csg_shape()` 中实现（同方案 1），但可以添加：
- 缓存大小限制（LRU）
- 缓存统计信息
- 缓存清理机制

## 推荐方案

**推荐使用方案 1**，原因：

1. **集中管理**：缓存逻辑集中在 `gen_csg_shape()` 中，易于维护
2. **透明性**：调用方无需关心缓存细节
3. **性能优化**：使用 `DashMap` 实现线程安全的并发访问
4. **内存效率**：使用 `Arc` 共享网格数据，避免重复存储

## 实现细节

### 1. 哈希计算的关键点

`hash_unit_mesh_params()` 已经正确实现了：
- ✅ 包含影响几何的参数（profile、path、drns/drne、lmirror、plax、bangle）
- ✅ 排除位置和缩放（通过 transform 处理）
- ✅ 单段直线路径的特殊优化

### 2. 单位几何体的处理

`gen_unit_shape()` 已经实现了归一化：
- 单段直线路径归一化为沿 Z 轴的单位长度
- 清除 `segment_transforms` 和 `spine_segments`

**注意**：如果使用单位几何体生成网格，需要确保：
- 网格生成函数能正确处理归一化的路径
- Transform 能正确应用位置、旋转和缩放

### 3. 缓存生命周期

- **缓存时机**：在 `gen_csg_shape()` 首次调用时生成并缓存
- **缓存清理**：可以考虑添加：
  - 最大缓存大小限制
  - LRU 淘汰策略
  - 手动清理接口

### 4. 线程安全

使用 `DashMap` 和 `Arc` 确保：
- 多线程并发访问安全
- 网格数据共享（避免复制）

## 验证方法

1. **功能验证**：
   - 创建多个使用相同 profile 的 GENSEC
   - 验证它们共享相同的网格数据
   - 验证 transform 正确应用

2. **性能验证**：
   - 对比复用前后的内存使用
   - 对比复用前后的生成时间
   - 统计缓存命中率

3. **正确性验证**：
   - 验证不同 transform 的实例显示正确
   - 验证不同 profile 的实例不共享
   - 验证路径参数变化时缓存失效

## 潜在问题和注意事项

1. **内存增长**：缓存会占用内存，需要考虑：
   - 设置最大缓存大小
   - 实现 LRU 淘汰
   - 提供清理接口

2. **哈希冲突**：虽然概率很低，但需要考虑：
   - 使用更强的哈希算法
   - 添加冲突检测机制

3. **单位几何体兼容性**：确保 `generate_sweep_solid_mesh()` 能正确处理单位几何体

4. **segment_transforms 的影响**：
   - 当前 `hash_unit_mesh_params()` 不包含 `segment_transforms`
   - 但 `gen_unit_shape()` 会清除 `segment_transforms`
   - 需要确保这不会影响网格生成

## 相关代码位置

- `src/prim_geo/sweep_solid.rs`：`SweepSolid` 定义和 `hash_unit_mesh_params()`、`gen_unit_shape()`
- `src/prim_geo/profile.rs`：`normalize_spine_segments()` 生成 `SweepSolid`
- `src/geometry/sweep_mesh.rs`：`generate_sweep_solid_mesh()` 生成网格
- `src/shape/pdms_shape.rs`：`BrepShapeTrait` 定义

## 后续优化

1. **缓存统计**：添加缓存命中率、内存使用等统计信息
2. **缓存管理**：实现 LRU 淘汰、最大大小限制等
3. **批量预加载**：在后台预加载常用 profile 的网格
4. **序列化支持**：支持将缓存序列化到磁盘，加速后续加载

---

**创建时间**: 2025-01-XX  
**状态**: 🟡 待实现  
**优先级**: 中（性能优化）
