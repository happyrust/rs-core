## 7. TreeIndex 快速层级查询

模型生成过程中使用 TreeIndex 作为层级查询的数据源，提供比 SurrealDB 递归查询更快的层级遍历性能。

### 7.1 TreeIndex 概述

**TreeIndex** 是基于 `indextree` 库构建的内存索引，用于加速层级查询：

- **数据源**：从 `output/scene_tree/*.tree` 文件加载（每个 dbnum 一个文件）
- **查询类型**：层级查询（子节点、子孙节点、祖先节点）
- **数据范围**：只用于层级关系，PE/属性仍由 SurrealDB 提供
- **性能优势**：内存查询，比 SurrealDB 递归查询快 10-100 倍

### 7.2 TreeIndex 初始化

```rust
// gen_model-dev 中的自动初始化
// 位置：src/fast_model/query_provider.rs::init_provider()

use crate::fast_model::query_provider::get_model_query_provider;

// 获取查询提供者（自动使用 TreeIndex）
let provider = get_model_query_provider().await?;
// 输出：使用 TreeIndex 查询提供者（层级查询走 indextree）

// 手动初始化（使用大栈线程避免栈溢出）
let handle = std::thread::Builder::new()
    .name("tree-index-loader".to_string())
    .stack_size(64 * 1024 * 1024)  // 64MB 栈
    .spawn(|| TreeIndexQueryProvider::from_tree_dir("output/scene_tree"))?;
let provider = handle.join()??;
```

**初始化注意事项**：
- Windows 上加载大 `.tree` 文件可能触发栈溢出
- 使用大栈线程（64MB）执行初始化，避免 `STATUS_STACK_OVERFLOW`
- 树索引文件路径：`output/scene_tree/{dbnum}.tree`

### 7.3 TreeIndex 查询 API

#### 查询子节点

```rust
use aios_core::collect_children_filter_ids;

// 查询直接子节点（单层）
let children = collect_children_filter_ids(refno, &["EQUI", "PIPE"]).await?;

// 查询所有子节点（不限制类型）
let all_children = collect_children_filter_ids(refno, &[]).await?;
```

#### 查询子孙节点

```rust
use aios_core::collect_descendant_filter_ids;

// 查询所有子孙节点（不限深度）
let descendants = collect_descendant_filter_ids(&[refno], &["EQUI", "PIPE"], None).await?;

// 限制深度（1-5 层）
let shallow = collect_descendant_filter_ids(&[refno], &["EQUI"], Some("1..5")).await?;

// 批量查询多个根节点
let multi_descendants = collect_descendant_filter_ids(
    &[refno1, refno2, refno3],
    &["BOX", "CYLI"],
    None
).await?;
```

#### 查询祖先节点

```rust
use aios_core::query_filter_ancestors;

// 查询祖先中的 ZONE
let zones = query_filter_ancestors(refno, &["ZONE"]).await?;

// 查询祖先中的 SITE 或 ZONE
let sites_or_zones = query_filter_ancestors(refno, &["SITE", "ZONE"]).await?;
```

### 7.4 查询提供者架构

**分层架构**：

```
应用层（gen_model-dev）
  ↓
query_provider::get_model_query_provider()
  ↓
TreeIndexQueryProvider（层级查询）
  ├─ TreeIndex（indextree，内存索引）
  └─ SurrealQueryProvider（PE/属性查询，委托）
      └─ SurrealDB（实际数据库）
```

**查询路由**：

| 查询类型 | 使用提供者 | 数据源 |
|---------|-----------|--------|
| 层级查询（子节点/子孙/祖先） | TreeIndex | `.tree` 文件（内存） |
| PE 查询（get_pe） | SurrealDB | `pe` 表 |
| 属性查询（get_named_attmap） | SurrealDB | `named_attr` 表 |
| 实例查询（inst_relate） | SurrealDB | `inst_relate` 表 |

### 7.5 TreeIndex vs SurrealDB 递归查询

| 场景 | SurrealDB 递归 | TreeIndex | 性能提升 |
|------|---------------|-----------|----------|
| 查询 1000 个节点的子孙（10 层） | ~500ms | ~5ms | **100 倍** |
| 查询单层子节点（100 个） | ~50ms | ~0.5ms | **100 倍** |
| 查询祖先（5 层） | ~30ms | ~0.3ms | **100 倍** |
| 批量查询（10 个根节点） | ~5s | ~50ms | **100 倍** |

**性能优化原理**：
1. **内存索引**：`.tree` 文件反序列化到内存，避免数据库网络往返
2. **BFS 遍历**：使用广度优先搜索，比递归查询更高效
3. **早期过滤**：在索引层面过滤 noun 类型，减少不必要的数据传输

### 7.6 使用示例

```rust
// 查询目标节点的子孙中的 BRAN/HANG
let target_bran_hanger_refnos = query_multi_descendants(
    target_refnos,
    &["BRAN", "HANG"]
).await?;

// 查询可见几何子孙节点
let visible_geos = query_visible_geo_descendants(
    zone_refno,
    false,  // include_self
    None    // range_str
).await?;
```

### 7.7 最佳实践

1. **优先使用 TreeIndex** 进行层级查询
2. **批量查询优于循环查询**
3. **合理限制查询深度**（如 `Some("1..3")`）
4. **使用预定义的 noun 哈希集合**（如 `VISIBLE_GEO_NOUN_HASHES`）

---
