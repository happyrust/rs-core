# Range 参数功能文档

## 概述

为 `query_filter_deep_children_atts` 添加了带 `range` 参数的变体函数，用于控制图遍历的层级深度。

## 实现方式

**简化设计**：直接在 Rust 层面将 range 字符串拼接到 SQL 中，而不是在 SurrealDB 函数中传递参数。这样更简洁、更灵活。

## 功能说明

### SurrealDB 函数（保持不变）

现有的 SurrealDB 函数保持不变，不需要修改：

- `fn::collect_descendant_infos` - 使用固定的 `..` 范围（无限递归）
- `fn::collect_descendant_ids_by_types` - 调用上述函数
- `fn::collect_descendants_with_attrs` - 调用上述函数

这些函数继续使用默认的无限递归行为。

### Rust API - 主要功能

#### 新增函数：`query_filter_deep_children_atts_with_range`

查询子孙节点的属性（带层级范围控制）

**签名：**

```rust
pub async fn query_filter_deep_children_atts_with_range(
    refno: RefnoEnum,
    nouns: &[&str],
    range: Option<&str>,
) -> anyhow::Result<Vec<NamedAttrMap>>
```

**参数：**
- `refno` - 根节点引用
- `nouns` - 要筛选的类型数组
- `range` - 层级范围字符串，**直接拼接到 SQL 中的 `@.{range+collect+inclusive}.children`**
  - `Some("..")` - 无限递归（默认，最多256层）
  - `Some("1")` - 仅查询1层子节点
  - `Some("2")` - 仅查询2层子节点
  - `Some("1..5")` - 查询1到5层
  - `Some("3")` - 固定3层
  - `Some("1..")` - 从1层开始到最大深度
  - `None` - 使用默认值 `".."`（无限递归）

**实现方式：**

函数内部生成如下 SQL（range 直接拼接）：

```surql
LET $root = pe:17496_171099;
LET $descendants = (SELECT VALUE array::flatten(@.{1..5+collect+inclusive}.children).{ id, noun } FROM ONLY $root LIMIT 1) ?: [];
LET $filtered = array::filter($descendants, |$node| true && noun IN ['ZONE', 'EQUI']);
LET $pes = array::filter($filtered, |$info| $info.id != NONE && record::exists($info.id));
SELECT VALUE $pes.id.refno.* FROM $pes;
```

**使用示例：**

```rust
use aios_core::{RefnoEnum, query_filter_deep_children_atts_with_range};

// 查询所有层级（默认）
let atts = query_filter_deep_children_atts_with_range(
    refno,
    &["ZONE", "EQUI"],
    None  // 等同于 Some("..")
).await?;

// 仅查询直接子节点（1层）
let atts = query_filter_deep_children_atts_with_range(
    refno,
    &["ZONE", "EQUI"],
    Some("1")
).await?;

// 查询1到3层
let atts = query_filter_deep_children_atts_with_range(
    refno,
    &["ZONE", "EQUI"],
    Some("1..3")
).await?;

// 查询固定5层
let atts = query_filter_deep_children_atts_with_range(
    refno,
    &["ZONE", "EQUI"],
    Some("5")
).await?;

// 从第2层开始到最大深度
let atts = query_filter_deep_children_atts_with_range(
    refno,
    &["ZONE", "EQUI"],
    Some("2..")
).await?;
```

#### 原有函数保持向后兼容

`query_filter_deep_children_atts` 函数保持不变，内部调用新函数并传递 `None` 作为 range 参数：

```rust
pub async fn query_filter_deep_children_atts(
    refno: RefnoEnum,
    nouns: &[&str],
) -> anyhow::Result<Vec<NamedAttrMap>> {
    query_filter_deep_children_atts_with_range(refno, nouns, None).await
}
```

## 性能建议

1. **使用固定层级避免过度递归**：如果只需要查询几层子节点，使用固定层级（如 `"3"` 或 `"1..5"`）可以显著提高性能。

2. **优先使用范围而非无限递归**：在已知层级深度的情况下，使用范围参数可以减少数据库的扫描量。

3. **注意开放式范围的性能影响**：`".."` 会递归最多256层，对于深度较大的树结构可能影响性能。

## 实现细节

### 直接 SQL 拼接方式

在 `query_filter_deep_children_atts_with_range` 中，range 字符串直接拼接到 SQL 的 `@.{range+collect+inclusive}.children` 位置：

```rust
let range_str = range.unwrap_or("..");
let sql = format!(
    r#"
    LET $root = {};
    LET $descendants = (SELECT VALUE array::flatten(@.{{{}+collect+inclusive}}.children).{{ id, noun }} FROM ONLY $root LIMIT 1) ?: [];
    LET $filtered = array::filter($descendants, |$node| true{});
    LET $pes = array::filter($filtered, |$info| $info.id != NONE && record::exists($info.id));
    SELECT VALUE $pes.id.refno.* FROM $pes;
    "#,
    pe_key, range_str, type_filter
);
```

### 优点

1. **灵活性**：支持任意 SurrealDB 图遍历语法，不局限于预定义的几种范围
2. **简洁性**：不需要在 SurrealDB 函数中添加复杂的 if-else 分支
3. **可扩展性**：用户可以传入任何合法的 range 表达式，如 `"2..10"`, `"5"`, `"3.."` 等

## Range 语法说明

根据 SurrealDB 文档，`@.{range}` 支持以下语法：

- `n` - 固定深度 n（如 `"3"` 表示递归3次）
- `..` - 无限递归（最多256层）
- `n..m` - 从 n 到 m 层（如 `"1..5"` 表示1到5层）
- `n..` - 从 n 层开始到最大深度（如 `"2.."` 从第2层开始）
- `..m` - 从当前到 m 层（如 `"..10"` 最多10层）

## 注意事项

1. Range 字符串会直接拼接到 SQL 中，请确保传入合法的语法
2. 传入非法的 range 会导致 SurrealDB 查询错误
3. `None` 会使用默认值 `".."`（无限递归）
4. 建议使用常见的范围值以避免错误

## 参考文档

- SurrealDB Graph Traversal: `/Volumes/DPC/work/gen-model/external/rs-core/docs/surrealdb/src/content/doc-surrealql/statements/relate.mdx`
- Graph Relations Reference: `/Volumes/DPC/work/gen-model/external/rs-core/docs/surrealdb/src/content/doc-surrealdb/reference-guide/graph-relations.mdx`
