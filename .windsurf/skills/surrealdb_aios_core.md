# SurrealDB 查询技能 (aios-core)

本技能指南涵盖 aios-core 项目中 SurrealDB 的完整查询模式，结合官方 SurrealQL 语法。

---

## 1. 核心连接与查询模式

### 1.1 全局 DB 实例

```rust
use aios_core::{SUL_DB, SurrealQueryExt};

// 单结果查询 - 提取第 N 条语句结果
let result: Vec<T> = SUL_DB.query_take(sql, 0).await?;

// 多结果查询 - 执行多条语句并分别提取
let mut resp = SUL_DB.query_response(sql).await?;
let data1: Vec<T1> = resp.take(0)?;
let data2: Vec<T2> = resp.take(1)?;
```

### 1.2 类型安全规范

**必须使用 `SurrealValue` trait，禁止 `serde_json::Value`**

```rust
use surrealdb::types::{self as surrealdb_types, SurrealValue};

// ✅ 正确
#[derive(Serialize, Deserialize, Debug, SurrealValue)]
pub struct QueryResult {
    pub refno: RefnoEnum,
    pub name: String,
}
let result: Vec<QueryResult> = SUL_DB.query_take(sql, 0).await?;

// ❌ 禁止
let result: Vec<serde_json::Value> = SUL_DB.query_take(sql, 0).await?;
```

---

## 2. Record ID 格式

### 2.1 标准格式

```sql
-- 简单 ID
pe:⟨12345_67890⟩
inst_geo:⟨abc123hash⟩

-- 复合 ID（关系表）
tubi_relate:[pe:⟨21491_10000⟩, 0]
pe:⟨["12345_67890", 880]⟩  -- 历史版本
```

### 2.2 Rust 转换

```rust
let pe_key = refno.to_pe_key();           // "pe:⟨12345_67890⟩"
let table_key = refno.to_table_key("pe_transform");  // "pe_transform:⟨12345_67890⟩"

// 动态构建
let sql = "let $pe_id = type::record('pe', '12345_67890');";
```

---

## 3. SELECT 语句详解

### 3.1 基础查询

```sql
-- 基础 SELECT
SELECT * FROM pe WHERE noun = 'BOX';

-- 选择特定字段
SELECT refno, name, noun FROM pe WHERE deleted = false;

-- VALUE 返回单列
SELECT VALUE id FROM pe WHERE noun IN ['BOX', 'CYLI'];

-- 从记录数组查询
SELECT * FROM [pe:⟨123⟩, pe:⟨456⟩];

-- ONLY 限定单条记录
SELECT * FROM ONLY pe:⟨123⟩ LIMIT 1;
```

### 3.2 条件过滤

```sql
-- IN 操作符
SELECT * FROM pe WHERE noun IN ['BOX', 'CYLI', 'CONE'];

-- 范围查询
SELECT * FROM pe WHERE sesno >= 100 AND sesno <= 200;

-- 嵌套属性访问
SELECT * FROM pe WHERE refno.TYPE = 'EQUI' OR refno.TYPEX = 'EQUI';

-- 空值判断
SELECT * FROM inst_relate WHERE aabb.d != NONE;
```

### 3.3 去重查询

**SurrealDB 不支持 `SELECT DISTINCT`，必须使用 `array::distinct()`**

```sql
-- ❌ 错误
SELECT DISTINCT field FROM table;

-- ✅ 正确
SELECT array::distinct((SELECT VALUE field FROM table)) AS unique_values;

-- 或使用 GROUP BY
SELECT field FROM table GROUP BY field;
```

### 3.4 数量统计

```sql
-- 表总数
SELECT value count() FROM ONLY pe GROUP ALL LIMIT 1;

-- 条件统计
SELECT value count() FROM ONLY pe WHERE noun = 'PIPE' GROUP ALL LIMIT 1;
```

---

## 4. 图遍历语法

### 4.1 正向遍历 (->)

```sql
-- 遍历单个关系
SELECT * FROM pe:⟨123⟩->inst_relate;

-- 链式遍历
SELECT * FROM pe:⟨123⟩->LSTU->CATR;

-- 获取关系终点
SELECT VALUE out FROM pe:⟨123⟩->tubi_relate;
```

### 4.2 反向遍历 (<-)

```sql
-- 获取指向当前节点的关系
SELECT * FROM pe:⟨123⟩<-pe_owner;

-- 获取父节点
SELECT VALUE in FROM pe:⟨123⟩->pe_owner;

-- 获取所有子节点
SELECT VALUE in FROM pe:⟨123⟩<-pe_owner;
```

### 4.3 条件过滤遍历

```sql
-- 过滤已删除节点
SELECT * FROM pe:⟨123⟩<-pe_owner[? !in.deleted];

-- 按类型过滤
SELECT VALUE in FROM pe:⟨123⟩<-pe_owner WHERE in.noun = 'BOX';
```

---

## 5. 递归路径查询 (Recursive Path)

### 5.1 基础语法

```sql
-- 格式: @.{range+options}.field
-- @ = 当前记录, range = 递归深度, options = 控制选项, field = 递归字段
```

### 5.2 递归范围 (Range)

| 格式 | 说明 | 示例 |
|------|------|------|
| `..` | 无限递归 | `@.{..}.children` |
| `3` | 精确 N 层 | `@.{3}.children` |
| `1..5` | 从 1 到 5 层 | `@.{1..5}.children` |
| `..5` | 从 0 到 5 层 | `@.{..5}.children` |
| `3..` | 从 3 层到无限 | `@.{3..}.children` |

### 5.3 递归选项

| 选项 | 说明 |
|------|------|
| `collect` | 收集所有层级结果到数组 |
| `inclusive` | 包含起始节点本身 |

### 5.4 完整示例

```sql
-- 获取所有子孙节点
SELECT VALUE array::flatten(@.{..+collect}.children)
FROM ONLY pe:⟨12345⟩ LIMIT 1;

-- 获取子孙节点的特定字段（包含自身）
SELECT VALUE array::flatten(@.{..+collect+inclusive}.children).{ id, noun }
FROM ONLY $root LIMIT 1;

-- 获取祖先链
SELECT VALUE @.{..+collect+inclusive}.owner
FROM ONLY pe:⟨12345⟩ LIMIT 1;

-- 限制深度 1-5 层
SELECT VALUE array::flatten(@.{1..5+collect}.children)
FROM ONLY $root LIMIT 1;
```

---

## 6. ID Range 查询

对于复合 ID 表，推荐使用 ID Range 查询：

### 6.1 语法

```sql
-- [start_id]..[end_id]
-- .. 表示无穷大

-- 查询 BRAN 下所有 tubi_relate
SELECT * FROM tubi_relate:[pe:⟨bran_refno⟩, 0]..[pe:⟨bran_refno⟩, ..]
```

### 6.2 字段提取

```sql
SELECT
    id[0] as refno,           -- 复合 ID 第一个元素
    id[0].old_pe as old_refno, -- 访问嵌套字段
    id[0].owner.noun as generic,
    in as leave,
    out as arrive,
    aabb.d as world_aabb,
    record::id(geo) as geo_hash
FROM tubi_relate:[pe_key, 0]..[pe_key, ..]
WHERE aabb.d != NONE
```

### 6.3 Rust 示例

```rust
// ✅ 推荐 - ID Range
let sql = format!("SELECT * FROM tubi_relate:[{}, 0]..[{}, ..]", pe_key, pe_key);

// ❌ 避免 - WHERE 过滤
let sql = format!("SELECT * FROM tubi_relate WHERE record::id(id)[0] = {}", pe_key);
```

---

## 7. 层级查询 API

### 7.1 aios-core 层级查询 (SurrealDB)

**这些函数底层调用 SurrealDB 的 `fn::collect_descendant_ids_by_types` 数据库函数**

```rust
use aios_core::collect_descendant_filter_ids;
use aios_core::collect_children_filter_ids;
use aios_core::query_filter_ancestors;

// 子孙节点查询（无限深度）
let ids = collect_descendant_filter_ids(&[refno], &["EQUI"], None).await?;

// 子孙节点查询（限制深度 1-5 层）
let ids = collect_descendant_filter_ids(&[refno], &["EQUI"], Some("1..5")).await?;

// 批量查询多个根节点
let ids = collect_descendant_filter_ids(&[r1, r2, r3], &["BOX"], None).await?;

// 直接子节点查询
let children = collect_children_filter_ids(refno, &["EQUI", "PIPE"]).await?;

// 祖先查询
let zones = query_filter_ancestors(refno, &["ZONE"]).await?;
```

### 7.2 gen_model-dev 层级查询 (TreeIndex)

**gen_model-dev 项目提供基于内存 indextree 的高性能层级查询**

#### 前置条件

1. **生成 `.tree` 文件**：需要先导出场景树索引文件到 `output/scene_tree/` 目录
2. **文件格式**：`{dbnum}.tree`（如 `1112.tree`）

#### 初始化 TreeIndexQueryProvider

```rust
// 位置: rs-core/src/query_provider/tree_index_provider.rs
use aios_core::query_provider::TreeIndexQueryProvider;

// 方式1：从目录加载所有 .tree 文件（推荐）
// 注意：Windows 上需要使用大栈线程避免栈溢出
let handle = std::thread::Builder::new()
    .name("tree-index-loader".to_string())
    .stack_size(64 * 1024 * 1024)  // 64MB 栈
    .spawn(|| TreeIndexQueryProvider::from_tree_dir("output/scene_tree"))
    .context("创建线程失败")?;

let provider = handle.join()
    .map_err(|_| anyhow::anyhow!("线程 panic"))??;
```

#### 使用全局 Provider (gen_model-dev)

```rust
// 位置: gen_model-dev/src/fast_model/query_provider.rs
use crate::fast_model::query_provider::get_model_query_provider;

// 获取全局 Provider（首次调用会自动初始化）
let provider = get_model_query_provider().await?;
```

#### HierarchyQuery trait 方法

```rust
use aios_core::query_provider::HierarchyQuery;

// 获取直接子节点
let children = provider.get_children(refno).await?;

// 获取所有子孙节点（可限制深度）
let descendants = provider.get_descendants(refno, Some(5)).await?;  // 最多5层
let all_descendants = provider.get_descendants(refno, None).await?; // 无限深度

// 获取过滤后的子孙节点（按类型）
let equi_descendants = provider.get_descendants_filtered(refno, &["EQUI"], None).await?;

// 获取祖先节点
let ancestors = provider.get_ancestors(refno).await?;

// 获取特定类型的祖先
let zones = provider.get_ancestors_of_type(refno, &["ZONE"]).await?;

// 获取子节点的完整 PE 信息
let children_pes = provider.get_children_pes(refno).await?;
```

#### gen_model-dev 便捷函数

```rust
// 位置: gen_model-dev/src/fast_model/query_provider.rs
use crate::fast_model::query_provider::*;

// 子节点
let children = get_children(refno).await?;

// 祖先
let ancestors = get_ancestors(refno).await?;
let zones = get_ancestors_of_type(refno, &["ZONE"]).await?;

// PE 查询（委托 SurrealDB）
let pe = get_pe(refno).await?;
let pes = get_pes_batch(&refnos).await?;

// 属性查询（委托 SurrealDB）
let attmaps = get_attmaps_batch(&refnos).await?;
```

#### TreeIndex vs SurrealDB 性能对比

| 场景 | SurrealDB | TreeIndex | 说明 |
|------|-----------|-----------|------|
| 1000 节点子孙查询 | ~500ms | ~5ms | TreeIndex 快 100 倍 |
| 数据来源 | 数据库实时查询 | 内存索引 (`.tree` 文件) | |
| 使用位置 | aios-core | gen_model-dev | |
| 层级查询 | ❌ 较慢 | ✅ 推荐 | |
| PE/属性查询 | ✅ 必须 | 委托 SurrealDB | |

#### 架构说明

```
TreeIndexQueryProvider
├── 层级查询 (HierarchyQuery) → TreeIndex (内存 indextree)
│   ├── get_children()
│   ├── get_descendants()
│   ├── get_ancestors()
│   └── get_descendants_filtered()
│
└── 其他查询 → 委托 SurrealQueryProvider
    ├── get_pe() / get_pes_batch()
    ├── get_attmaps_batch()
    └── query_by_type()
```

### 7.3 泛型查询函数 (SurrealDB)

```rust
use aios_core::collect_descendant_with_expr;

// 查询 ID 列表
let ids: Vec<RefnoEnum> = collect_descendant_with_expr(
    &[refno], &["EQUI"], None, "VALUE id"
).await?;

// 查询完整元素
let elements: Vec<SPdmsElement> = collect_descendant_with_expr(
    &[refno], &["EQUI"], None, "*"
).await?;

// 查询属性映射
let attrs: Vec<NamedAttrMap> = collect_descendant_with_expr(
    &[refno], &["ZONE"], Some("1..5"), "VALUE id.refno.*"
).await?;
```

---

## 8. 数据库端自定义函数

### 8.1 层级函数

```sql
-- 祖先查询
fn::ancestor($pe)
fn::ancestor($pe)[WHERE noun = 'ZONE']

-- 子节点
fn::children($pe)
fn::collect_children($root, $types)

-- 子孙收集
fn::collect_descendant_ids_by_types($pe, ['EQUI', 'PIPE'], none, "..")
fn::collect_descendant_ids_by_types($pe, ['EQUI'], none, "1..5")

-- 几何子孙
fn::visible_geo_descendants($root, $include_self, $range_str)
fn::negative_geo_descendants($root, $include_self, $range_str)
```

### 8.2 过滤函数

```sql
-- 过滤 SPRE/CATR 节点
fn::collect_descendants_filter_spre($pe, [], true, none, "..")

-- 过滤已生成实例节点
fn::collect_descendants_filter_inst($pe, $types, $filter, true, false)
```

### 8.3 拓扑函数

```sql
-- 连接节点导航
fn::prev_connect_pe($pe)
fn::next_connect_pe($pe)

-- 管道关系
fn::query_tubi_to($pe)
fn::query_tubi_from($pe)
fn::has_leave_tubi($pe)
fn::has_arrive_tubi($pe)
```

### 8.4 辅助函数

```sql
fn::default_name($pe)       -- 默认名称
fn::ses_date($pe)           -- 会话日期
fn::query_negative_entities($pe)  -- 查询负实体
```

---

## 9. RELATE 关系语句

### 9.1 基本语法

```sql
RELATE $in->relation_table:[$key1, $key2]->$out
SET field1 = value1, field2 = value2;
```

### 9.2 示例

```sql
-- 创建 tubi_relate
RELATE pe:⟨leave⟩->tubi_relate:[pe:⟨bran⟩, index]->pe:⟨arrive⟩
SET
    geo = inst_geo:⟨geo_hash⟩,
    aabb = aabb:⟨aabb_hash⟩,
    world_trans = trans:⟨trans_hash⟩,
    bore_size = 'DN100',
    bad = false,
    system = pe:⟨system_refno⟩,
    dt = fn::ses_date(pe:⟨leave⟩);

-- 创建 inst_relate
RELATE $pe->inst_relate:[$pe_id]->$inst_geo
SET world_trans = $trans, aabb = $aabb;
```

---

## 10. 批量查询优化

### 10.1 array::map + flatten + distinct 模式

```rust
let sql = format!(
    r#"
    array::distinct(array::filter(array::flatten(array::map([{}], |$refno|
        fn::collect_descendants_filter_inst($refno, {}, {}, true, false)
    )), |$v| $v != none))
    "#,
    refno_list, types_expr, filter_str
);
```

### 10.2 批量 IN 查询

```rust
// ✅ 推荐
let sql = format!("SELECT * FROM pe WHERE id IN [{}]", pe_keys.join(","));

// ❌ 避免
for refno in refnos {
    let result = SUL_DB.query_take(&format!("SELECT * FROM pe:{}", refno), 0).await?;
}
```

---

## 11. 核心表结构速查

| 表名 | 类型 | ID 格式 | 说明 |
|------|------|---------|------|
| `pe` | 节点 | `pe:⟨dbnum_refno⟩` | PDMS 元素主表 |
| `inst_relate` | 关系 | `inst_relate:⟨refno⟩` | PE→几何实例关联 |
| `inst_info` | 节点 | `inst_info:⟨refno_info⟩` | 实例信息 |
| `inst_geo` | 节点 | `inst_geo:⟨geo_hash⟩` | 几何数据 |
| `geo_relate` | 关系 | `geo_relate:⟨hash⟩` | 实例→几何关联 |
| `tubi_relate` | 关系 | `tubi_relate:[pe:⟨bran⟩, idx]` | 管道直段 |
| `neg_relate` | 关系 | `neg_relate:[neg, idx]` | 负实体关系 |
| `ngmr_relate` | 关系 | `ngmr_relate:[ele, target, ngmr]` | NGMR 负实体 |
| `pe_transform` | 节点 | `pe_transform:⟨refno⟩` | 变换缓存 |

---

## 12. 最佳实践总结

### ✅ 推荐做法

1. **使用 SurrealValue trait** 替代 serde_json::Value
2. **使用 TreeIndex** 替代 SurrealDB 图遍历进行层级查询
3. **使用 ID Range** 替代 WHERE 条件查询复合 ID 表
4. **使用 `id[0]`** 直接访问复合 ID，避免 `record::id()`
5. **批量查询** 优于循环单条查询
6. **使用 `array::distinct()`** 替代 SELECT DISTINCT
7. **利用数据库端函数** 减少网络往返

### ❌ 避免做法

1. 使用 `serde_json::Value` 作为返回类型
2. 循环单条查询数据库
3. 使用 `record::id(id)` 解析复合 ID
4. 使用 WHERE 条件过滤复合 ID 表
5. 在 gen_model-dev 中使用 SurrealDB 图遍历（应使用 TreeIndex QueryProvider）

---

## 13. 代码位置索引

| 模块 | 路径 |
|------|------|
| 查询扩展 | `src/rs_surreal/query_ext.rs` |
| 层级查询 | `src/rs_surreal/graph.rs` |
| 实例查询 | `src/rs_surreal/inst.rs` |
| PE 查询 | `src/rs_surreal/query.rs` |
| 变换查询 | `src/rs_surreal/pe_transform.rs` |
| 布尔查询 | `src/rs_surreal/boolean_query.rs` |
| 空间查询 | `src/rs_surreal/spatial.rs` |
| 数据库函数定义 | `resource/surreal/*.surql` |
