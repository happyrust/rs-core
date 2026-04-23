# rs-core Workspace 拆分 Phase 2: 解耦循环 + 提取 aios-types / aios-geo / aios-surreal

## 循环依赖现状

```
types/attmap.rs ──→ prim_geo::{CTorus,SCylinder,Dish,...}
types/named_attmap.rs ──→ prim_geo::*, shape::BrepShapeTrait, tool::*
types/hash.rs ──→ rs_surreal::PlantTransform
types/plant_aabb.rs ──→ shape::pdms_shape::RsVec3
types/db_info.rs ←── tool/db_tool.rs
                                     ↑
prim_geo::* ──→ types::{AttrMap, RefnoEnum, NamedAttrValue}
prim_geo/basic.rs ──→ geometry::csg::unit_*_mesh
prim_geo/profile.rs ──→ rs_surreal::query::*, rs_surreal::spatial::*
                                     ↑
geometry ──→ prim_geo + shape + types
shape ──→ geometry::PlantGeoData + types::RefnoEnum
tool/math_tool.rs ──→ shape::pdms_shape::{ANGLE_RAD_*}
```

关键环路：
1. **types ↔ prim_geo**（attmap/named_attmap 引用具体图元）
2. **types ↔ rs_surreal**（hash.rs 引用 PlantTransform）
3. **geometry ↔ prim_geo ↔ shape**（三方互引）
4. **prim_geo → rs_surreal**（profile.rs 直接调 DB 查询）
5. **tool ↔ types/shape**（小环路）

## 破环策略

### 原则

- **types 只下沉不上引**：types 不依赖 prim_geo/shape/tool/rs_surreal
- **用 trait 反转依赖**：types 定义 trait，prim_geo/shape 实现 trait
- **向上移动类型**：RsVec3、ANGLE_RAD_* 等小类型移入 types
- **DB 查询隔离**：prim_geo/profile 的 surreal 查询移到上层编排

### 目标 DAG

```
aios-types (纯类型 + trait 定义, 0 内部依赖)
    ↑
aios-geo (prim_geo + geometry + shape, 依赖 aios-types)
    ↑
aios-surreal (rs_surreal + query_provider, 依赖 aios-types)
    ↑
aios_core (剩余模块, 依赖以上三个 + 编排层)
```

## 实施步骤

### Step 1: 从 types 中剥离上引依赖（破环核心）

**目标**：让 `types/` 不再 `use crate::{prim_geo, shape, tool, rs_surreal}`

#### 1.1 types/attmap.rs — 解除对 prim_geo 具体类型的引用

当前问题：`attmap.rs` 导入 `CTorus, SCylinder, Dish, Pyramid, RTorus, SBox, LSnout, Sphere` 用于 AttrMap → 图元构建。

解法：**定义 trait `FromAttrMap`，由 prim_geo 侧实现**

```rust
// aios-types/src/attmap.rs（新版）
pub trait FromAttrMap: Sized {
    fn from_attr_map(map: &AttrMap) -> Option<Self>;
}
```

将 `attmap.rs` 中所有 `match noun { "CYLI" => SCylinder::from(...) ... }` 逻辑移到 `prim_geo/` 各模块的 `impl FromAttrMap for SCylinder` 中，或提取到 aios_core 的编排模块。

`attmap.rs` 只保留 `AttrMap` 数据结构定义和通用 get/set 方法。

#### 1.2 types/named_attmap.rs — 同上策略

将对 `prim_geo::*`、`shape::BrepShapeTrait`、`tool::*` 的引用移到调用侧。
- 方法：把 `named_attmap.rs` 中依赖外部的方法标记为 `#[cfg(feature = "geo")]` 或直接移到 prim_geo 的 extension trait 里

#### 1.3 types/hash.rs — 解除对 rs_surreal::PlantTransform

当前问题：为 `PlantTransform` 实现 Hash。

解法：
- 选项 A：把 `PlantTransform` 的定义移入 aios-types（如果它是纯数据类型）
- 选项 B：把 hash impl 移到 rs_surreal 侧（`impl Hash for PlantTransform` 放在定义侧）
- **推荐 A**：PlantTransform 是纯数据（translation + rotation），适合放在 types 中

#### 1.4 types/plant_aabb.rs — 解除对 shape::RsVec3

将 `RsVec3`（`type RsVec3 = glam::Vec3`）定义移入 aios-types，shape 侧改为 `use aios_types::RsVec3`。

#### 1.5 types/db_info.rs + types/pe.rs + types/whole_attmap.rs

这几个依赖 `tool::db_tool::{db1_hash, db1_dehash}`。

解法：将 `db1_hash` / `db1_dehash`（纯数学函数）移入 aios-types 的 `hash_util` 子模块。

### Step 2: 破解 geometry ↔ prim_geo ↔ shape 三角

#### 2.1 合并为一个 crate：aios-geo

geometry、prim_geo、shape 三者互引太紧密，不值得分三个 crate。合并为 `aios-geo`，对外只暴露稳定 API：

```
crates/aios-geo/
├── Cargo.toml        # 依赖 aios-types + glam + nalgebra + parry3d
├── src/
│   ├── lib.rs
│   ├── prim_geo/     # 原 prim_geo
│   ├── geometry/     # 原 geometry
│   └── shape/        # 原 shape
```

#### 2.2 切断 prim_geo/profile.rs → rs_surreal

将 `profile.rs` 中的 `get_owner_refno_by_type`、`get_owner_type_name` 等 DB 查询调用改为参数传入：
- 方法签名加一个 `owner_info: &OwnerInfo` 参数
- 上层编排（aios_core）负责从 surreal 查好再传下来
- 或定义 trait `GeoQueryProvider`，由 aios_core 实现

### Step 3: 提取 aios-surreal

#### 3.1 结构

```
crates/aios-surreal/
├── Cargo.toml        # 依赖 aios-types + surrealdb
├── src/
│   ├── lib.rs
│   ├── connection.rs # SUL_DB, KV_DB, 连接管理
│   ├── query/        # 原 rs_surreal/query
│   ├── inst.rs       # 原 rs_surreal/inst
│   ├── geom.rs       # 原 rs_surreal/geom
│   └── ...
```

#### 3.2 依赖处理

rs_surreal 大量使用 `crate::types::*`，提取后改为 `use aios_types::*`。
依赖 geometry/shape 的部分（inst.rs 的 `ShapeInstancesData`）改为 `use aios_geo::*` 或通过泛型/trait 隔离。

### Step 4: 创建 workspace + 收尾

#### 4.1 Cargo.toml 改造

```toml
[workspace]
members = [".", "crates/aios-mbd", "crates/aios-types", "crates/aios-geo", "crates/aios-surreal"]

[dependencies]
aios-types = { path = "crates/aios-types" }
aios-geo = { path = "crates/aios-geo" }
aios-surreal = { path = "crates/aios-surreal" }
```

#### 4.2 lib.rs re-export

```rust
pub use aios_types as types;
pub use aios_geo::{geometry, prim_geo, shape};
pub use aios_surreal as rs_surreal;
```

下游 `aios_core::types::*` / `aios_core::prim_geo::*` / `aios_core::rs_surreal::*` 路径不变，零改动。

## 工作量估计

| 步骤 | 预计时间 | 风险 |
|------|---------|------|
| Step 1.1-1.5: types 破环 | 2-3 天 | 高（attmap/named_attmap 改动面大） |
| Step 2.1-2.2: aios-geo 合并 | 1-2 天 | 中（文件移动 + import 路径） |
| Step 3: aios-surreal | 2-3 天 | 中（rs_surreal 模块多，import 散） |
| Step 4: workspace 集成 | 0.5 天 | 低 |
| 编译验证 + 下游适配 | 1-2 天 | 中 |
| **合计** | **7-11 天** | |

## 预期收益

- `rs_surreal` 改动可独立编译（当前改动 56 次全需重编整个 crate ~15s → 独立编 ~3s）
- types 改动（频繁）不再拖 geometry/surreal 重编
- geometry 改动不拖 surreal
- 为后续进一步拆分（parsed_data、spatial、transform 等）建立模式

## 执行进度

### Step 1 进度 (2026-04-21)

在 `rs-core-ws-split` worktree 上执行：

| 文件 | 状态 | 说明 |
|------|------|------|
| `types/attmap.rs` | ✅ 完成 | 8 prim_geo + BrepShapeTrait + tool 引用全部移除 |
| `types/hash.rs` | ✅ 完成 | rs_surreal::PlantTransform 替换为 gen_trs_hash |
| `types/db_info.rs` | ✅ 完成 | tool::db_tool → types::pdms_hash |
| `types/pe.rs` | ✅ 完成 | tool::db_tool → types::pdms_hash |
| `types/whole_attmap.rs` | ✅ 完成 | tool::db_tool → types::pdms_hash |
| `types/named_attvalue.rs` | ✅ 完成 | tool::float_tool → types::float_util |
| `types/plant_aabb.rs` | ⏸ 推迟 | shape::RsVec3 待整体迁移时处理 |
| `types/named_attmap.rs` | ⏳ 待拆 | 依赖 prim_geo/shape/tool/rs_surreal 全套 |

新建文件：
- `types/pdms_hash.rs` — db1_hash / db1_dehash 纯函数
- `types/float_util.rs` — hash_f32 / hash_f64_slice 纯函数
- `prim_geo/attmap_csg.rs` — AttrMapCsgExt 扩展 trait（AttrMap + NamedAttrMap 的 create_csg_shape）

types/ 外部依赖从 12 条降至 5 条。cargo check 通过。

消除的关键环路：
- ✅ types ↔ prim_geo（attmap + named_attmap 的 8 个图元导入 + BrepShapeTrait 全部移除）
- ✅ types → rs_surreal::PlantTransform（hash.rs 改用泛化 gen_trs_hash）
- ✅ types → tool::db_tool（attmap/db_info/pe/whole_attmap 全改用 types::pdms_hash）
- ✅ types → tool::float_tool（attmap 改用 types::float_util，named_attvalue 改用 types::float_util）

剩余 5 条（2 个文件）：
- named_attmap.rs → tool(float/math/dir) + rs_surreal(spatial) — 方向/变换计算方法内部
- plant_aabb.rs → shape::RsVec3 — 待 RsVec3 整体迁移

### Step 2 前置发现

aios-geo 不能简单提取为独立 crate，因为 geometry/prim_geo/shape 还依赖：
- `parsed_data`（CateAxisParam, PdmsGeoParam）
- `vec3_pool`
- `tool::hash_tool`
- `plant_transform::Transform`（应随 aios-types 下沉）
- `prim_geo/profile.rs` → `rs_surreal::spatial`（需先切断）

### prim_geo → rs_surreal 已消除 (2026-04-21)

`prim_geo/profile.rs` 中的 `rs_surreal::{query, spatial}` 导入实际**未使用**，已删除。
prim_geo/geometry/shape 不再依赖 rs_surreal，消除了计划中第 4 条环路。

### aios-geo 外部依赖清单（经完整扫描）

| 外部模块 | 依赖来源 | 处置方案 |
|---------|---------|---------|
| types (AttrMap, RefnoEnum, etc.) | 多处 | aios-types |
| plant_transform (Transform) | 多处 | 纳入 aios-types |
| pdms_types (noun 常量) | profile, tubing | 纳入 aios-types |
| parsed_data (PdmsGeoParam, CateAxisParam) | 多处 | 传入或提取 aios-parsed |
| tool (float/hash/dir/math) | 多处 | float/hash 已有 types 副本，其余提取或传入 |
| mesh_precision (LodMeshSettings) | csg, cylinder | 移入 aios-geo |
| debug_macros | csg, category | 移入 aios-geo 或 feature gate |
| vec3_pool | geometry/mod.rs | 移入 aios-geo 或提取 |
| transform (get_local_transform) | profile | 保留在 aios_core |
| csg::manifold | extrusion (feature-gated) | feature gate |

建议下一个工作会话：
1. 将 plant_transform + pdms_types + mesh_precision 纳入 aios-types
2. 将 parsed_data::geo_params_data 提取为 aios-geo 可用的 trait/接口
3. 做 aios-geo crate 物理提取

## 执行前置

1. 使用现有 `rs-core-ws-split` worktree（`feat/workspace-split` 分支）
2. Phase 1 (aios-mbd) 已完成（未提交）
3. Step 1 已在同一 worktree 完成，cargo check 通过
