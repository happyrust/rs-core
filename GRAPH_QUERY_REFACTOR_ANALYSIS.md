# graph.rs 子孙节点查询函数重构分析

## 当前状态

### 数据库端函数（SurrealDB）

在 `src/rs_surreal/schemas/functions/common.surql` 中存在以下子孙节点收集函数：

1. **fn::collect_descendants_by_children** - 通过 children 关系收集子孙节点
2. **fn::collect_descendant_ids_by_types** - 按类型收集子孙节点 ID（推荐统一使用）
3. **fn::collect_descendant_infos** - 收集子孙节点详细信息
4. **fn::collect_descendants_filter_inst** - 收集并过滤 inst_relate/tubi_relate（新增优化）

### Rust 端查询函数（graph.rs）

#### 核心内部函数

1. **`collect_descendant_refnos`** ⚠️ 核心函数，被多处调用
   - 参数：`refno`, `nouns`, `include_self`, `skip_deleted`
   - 调用：`fn::collect_descendants_by_children`（最新版本）
   - 调用：`fn::collect_descendant_infos`（历史版本）
   - 问题：使用不同的数据库函数，不统一

#### 公开查询函数（按功能分类）

**A. 基础子孙节点查询**

1. `query_deep_children_refnos(refno)` 
   - 调用：`collect_descendant_refnos(refno, &[], true, true)`
   - 用途：查询所有子孙节点（包含自身，跳过已删除）
   - 状态：✅ 有缓存

2. `query_filter_deep_children(refno, nouns)`
   - 调用：`collect_descendant_refnos(refno, nouns, true, true)`
   - 用途：按类型过滤子孙节点
   - 状态：无缓存

3. `query_filter_all_bran_hangs(refno)`
   - 调用：`query_filter_deep_children(refno, &["BRAN", "HANG"])`
   - 用途：专门查询 BRAN 和 HANG 类型
   - 状态：✅ 有缓存

**B. PBS 相关查询**

4. `query_deep_children_refnos_pbs(refno)` ⚠️ **特殊实现**
   - 直接使用 SQL 手写递归查询（12层）
   - 用途：PBS 节点递归查询
   - 问题：不使用标准函数，性能可能较差

5. `query_ele_filter_deep_children_pbs(refno, nouns)`
   - 调用：`query_deep_children_refnos_pbs`
   - 用途：查询 PBS 元素并按类型过滤

**C. 属性和元素查询**

6. `query_filter_deep_children_atts(refno, nouns)`
   - 调用：`collect_descendant_refnos` → 分块查询属性
   - 用途：查询子孙节点属性
   - 问题：分块处理，多次往返

7. `query_ele_filter_deep_children(refno, nouns)`
   - 调用：`collect_descendant_refnos` → 一次性查询元素
   - 用途：查询子孙元素完整信息

**D. 路径查询**

8. `query_filter_deep_children_by_path(refno, nouns)` ⚠️ **特殊实现**
   - 使用 `gen_noun_incoming_relate_sql` 生成路径查询
   - 用途：基于 noun 路径关系查询
   - 问题：不使用标准子孙节点函数

**E. 带条件过滤的查询**

9. `query_deep_children_refnos_filter_spre(refno, filter)` ⚠️ **需要重构**
   - 调用：`collect_descendant_refnos` → 分块过滤 SPRE/CATR
   - 问题：分块处理，应该使用数据库端函数

10. `query_versioned_deep_children_filter_inst(refno, nouns, filter)` ✅ **已优化**
    - 最新版本：调用 `fn::collect_descendants_filter_inst`
    - 历史版本：使用分块查询
    - 用途：过滤 inst_relate/tubi_relate

11. `query_deep_children_filter_inst(refno, nouns, filter)` ✅ **已优化**
    - 调用：`fn::collect_descendants_filter_inst`
    - 用途：过滤 inst_relate/tubi_relate（RefU64 版本）

**F. 批量查询**

12. `query_multi_filter_deep_children(refnos, nouns)` ✅ **使用推荐函数**
    - 调用：`fn::collect_descendant_ids_by_types`
    - 用途：批量查询多个起点的子孙节点
    - 状态：**最佳实践示例**

13. `query_multi_deep_versioned_children_filter_inst(refnos, nouns, filter)`
    - 循环调用：`query_versioned_deep_children_filter_inst`
    - 用途：批量查询并过滤（支持版本）

14. `query_multi_deep_children_filter_inst(refnos, nouns, filter)`
    - 循环调用：`query_deep_children_filter_inst`
    - 用途：批量查询并过滤

15. `query_multi_deep_children_filter_spre(refnos, filter)`
    - 循环调用：`query_deep_children_refnos_filter_spre`
    - 用途：批量查询并过滤 SPRE

## 问题总结

### 🔴 严重问题

1. **函数调用不统一**
   - `collect_descendant_refnos` 在最新版本使用 `fn::collect_descendants_by_children`
   - `query_multi_filter_deep_children` 使用 `fn::collect_descendant_ids_by_types`
   - **推荐**：统一使用 `fn::collect_descendant_ids_by_types`

2. **PBS 查询使用手写 SQL**
   - `query_deep_children_refnos_pbs` 手写 12 层递归查询
   - 性能差，难以维护
   - **应该**：使用标准数据库函数

3. **分块查询过多**
   - `query_filter_deep_children_atts` - 分块 200
   - `query_deep_children_refnos_filter_spre` - 分块 200
   - **应该**：在数据库端完成过滤

### 🟡 中等问题

4. **路径查询独立实现**
   - `query_filter_deep_children_by_path` 使用特殊的路径生成逻辑
   - 可能无法统一，但应该评估是否必要

5. **缺少缓存**
   - 多个高频查询函数没有 `#[cached]`
   - 应该考虑添加缓存

### 🟢 已优化

6. **inst_relate 过滤已优化**
   - `query_deep_children_filter_inst` 系列已使用数据库端函数
   - 性能提升 90%+

## 重构方案

### 阶段 1: 统一核心函数调用 ⭐ **高优先级**

#### 目标
将 `collect_descendant_refnos` 改为统一使用 `fn::collect_descendant_ids_by_types`

#### 优势
- 统一接口，易于维护
- 性能可能更优（需要测试验证）
- 减少数据库端函数维护负担

#### 实施
```rust
async fn collect_descendant_refnos(
    refno: RefnoEnum,
    nouns: &[&str],
    include_self: bool,
    skip_deleted: bool,
) -> anyhow::Result<Vec<RefnoEnum>> {
    let nouns_str = rs_surreal::convert_to_sql_str_array(nouns);
    let types_expr = if nouns.is_empty() {
        "[]".to_string()
    } else {
        format!("[{}]", nouns_str)
    };
    
    let pe_key = refno.to_pe_key();
    let include_param = if include_self { "none" } else { "true" };  // none 表示包含，true 表示排除自身
    
    // 统一使用 fn::collect_descendant_ids_by_types
    let sql = format!(
        "SELECT VALUE fn::collect_descendant_ids_by_types({}, {}, {});",
        pe_key, types_expr, include_param
    );
    
    // ... 查询执行逻辑
}
```

**注意**：需要验证 `skip_deleted` 参数如何传递到 `fn::collect_descendant_ids_by_types`

### 阶段 2: 创建数据库端过滤函数 ⭐ **高优先级**

#### 2.1 SPRE/CATR 过滤函数

```surql
DEFINE FUNCTION fn::collect_descendants_filter_spre(
    $root: record,
    $types: array<string>,
    $filter_inst: bool,
    $include_self: bool
) {
    -- 1. 收集所有子孙节点
    let $descendants = fn::collect_descendant_ids_by_types($root, $types, $include_self);
    
    -- 2. 过滤 SPRE 和 CATR
    let $with_spre = array::filter($descendants, |$node| {
        let $pe = type::thing('pe', $node);
        return SELECT VALUE id FROM $pe WHERE (refno.SPRE.id != none OR refno.CATR.id != none);
    });
    
    -- 3. 如果需要过滤 inst_relate
    if $filter_inst {
        return array::filter($with_spre, |$node| {
            let $pe = type::thing('pe', $node);
            count(SELECT VALUE id FROM $pe->inst_relate LIMIT 1) = 0 AND
            count(SELECT VALUE id FROM $pe->tubi_relate LIMIT 1) = 0
        });
    };
    
    return $with_spre;
};
```

#### 2.2 属性批量查询函数

```surql
DEFINE FUNCTION fn::collect_descendants_with_attrs(
    $root: record,
    $types: array<string>,
    $include_self: bool
) {
    let $ids = fn::collect_descendant_ids_by_types($root, $types, $include_self);
    return array::map($ids, |$id| {
        let $pe = type::thing('pe', $id);
        return SELECT VALUE refno.* FROM $pe;
    });
};
```

### 阶段 3: 重构 Rust 端函数 ⭐ **中优先级**

#### 3.1 简化 `query_deep_children_refnos_filter_spre`

```rust
pub async fn query_deep_children_refnos_filter_spre(
    refno: RefnoEnum,
    filter: bool,
) -> anyhow::Result<Vec<RefnoEnum>> {
    let pe_key = refno.to_pe_key();
    let filter_str = if filter { "true" } else { "false" };
    
    let sql = format!(
        "SELECT VALUE fn::collect_descendants_filter_spre({}, [], {}, true);",
        pe_key, filter_str
    );
    
    let mut response = SUL_DB.query(&sql).await?;
    Ok(response.take(0)?)
}
```

#### 3.2 优化 `query_filter_deep_children_atts`

```rust
pub async fn query_filter_deep_children_atts(
    refno: RefnoEnum,
    nouns: &[&str],
) -> anyhow::Result<Vec<NamedAttrMap>> {
    let nouns_str = rs_surreal::convert_to_sql_str_array(nouns);
    let types_expr = if nouns.is_empty() {
        "[]".to_string()
    } else {
        format!("[{}]", nouns_str)
    };
    let pe_key = refno.to_pe_key();
    
    let sql = format!(
        "SELECT VALUE fn::collect_descendants_with_attrs({}, {}, none);",
        pe_key, types_expr
    );
    
    let mut response = SUL_DB.query(&sql).await?;
    let atts: Vec<NamedAttrMap> = response.take(0)?;
    Ok(atts)
}
```

#### 3.3 重构 PBS 查询（如果可行）

需要评估 PBS 的 `pbs_owner` 关系是否也可以使用通用函数。如果 PBS 结构相同，可以创建：

```surql
DEFINE FUNCTION fn::collect_pbs_descendants(
    $root: record,
    $types: array<string>
) {
    -- 类似 collect_descendant_ids_by_types 但使用 pbs_owner 关系
    -- 实现细节需要根据 PBS 实际结构调整
};
```

### 阶段 4: 添加性能优化 🔧 **低优先级**

#### 4.1 添加缓存

为高频查询函数添加缓存：

```rust
#[cached(result = true, time = 300)]  // 缓存 5 分钟
pub async fn query_filter_deep_children(
    refno: RefnoEnum,
    nouns: &[&str],
) -> anyhow::Result<Vec<RefnoEnum>> {
    // ...
}
```

建议添加缓存的函数：
- `query_filter_deep_children`
- `query_ele_filter_deep_children`
- `query_deep_children_refnos_filter_spre`

#### 4.2 批量查询优化

`query_multi_*` 系列函数可以考虑：
- 减少循环调用，改为真正的批量查询
- 使用数据库端的批量处理能力

## 测试计划

### 单元测试

对每个重构的函数编写测试：

```rust
#[tokio::test]
async fn test_collect_descendant_refnos_unified() {
    let refno = RefU64::from_two_nums(100, 200);
    let result = collect_descendant_refnos(refno.into(), &["BOX"], true, true).await;
    assert!(result.is_ok());
    assert!(!result.unwrap().is_empty());
}
```

### 性能测试

对比重构前后的性能：

```rust
#[tokio::test]
#[ignore]
async fn benchmark_unified_query() {
    let start = Instant::now();
    let result_old = collect_descendant_refnos_old(...).await;
    let time_old = start.elapsed();
    
    let start = Instant::now();
    let result_new = collect_descendant_refnos(...).await;
    let time_new = start.elapsed();
    
    println!("性能对比:");
    println!("  旧实现: {:?}", time_old);
    println!("  新实现: {:?}", time_new);
    println!("  提升: {:.2}%", (1.0 - time_new.as_secs_f64() / time_old.as_secs_f64()) * 100.0);
}
```

### 回归测试

确保重构后所有现有功能正常：

```rust
#[tokio::test]
async fn test_regression_all_query_functions() {
    // 测试所有公开查询函数
    // 确保返回结果数量和内容一致
}
```

## 实施时间表

| 阶段 | 任务 | 预估时间 | 依赖 |
|------|------|---------|------|
| 1.1  | 分析 fn::collect_descendant_ids_by_types 接口 | 1 小时 | - |
| 1.2  | 重构 collect_descendant_refnos | 2 小时 | 1.1 |
| 1.3  | 测试核心函数 | 2 小时 | 1.2 |
| 2.1  | 创建 fn::collect_descendants_filter_spre | 1 小时 | - |
| 2.2  | 创建 fn::collect_descendants_with_attrs | 1 小时 | - |
| 3.1  | 重构 query_deep_children_refnos_filter_spre | 1 小时 | 2.1 |
| 3.2  | 重构 query_filter_deep_children_atts | 1 小时 | 2.2 |
| 3.3  | 评估 PBS 查询重构可行性 | 2 小时 | - |
| 4.1  | 添加缓存和性能优化 | 2 小时 | 3.1-3.3 |
| 4.2  | 性能测试和基准对比 | 3 小时 | 4.1 |

**总计**: 约 2-3 天工作量

## 风险评估

### 高风险 🔴

1. **fn::collect_descendant_ids_by_types 的参数含义**
   - 需要确认 `$exclude_self` 参数的确切含义
   - 需要确认是否支持 `skip_deleted` 功能

2. **历史版本查询兼容性**
   - 历史版本查询逻辑复杂，需要特别注意

### 中风险 🟡

3. **PBS 查询特殊性**
   - PBS 可能有特殊的关系结构
   - 手写查询可能有特殊原因

4. **性能回归**
   - 统一后可能某些场景性能下降
   - 需要充分的性能测试

### 低风险 🟢

5. **缓存失效**
   - 添加缓存后需要考虑数据更新时的失效策略

## 收益评估

### 代码质量

- ✅ 减少代码重复
- ✅ 统一接口，易于维护
- ✅ 减少数据库端函数维护成本

### 性能

- ✅ 减少网络往返（分块查询 → 单次查询）
- ✅ 数据库端过滤更高效
- ⚠️ 需要实际测试验证

### 可维护性

- ✅ 新功能更容易添加
- ✅ 问题排查更简单
- ✅ 文档更清晰

## 结论

**推荐立即实施阶段 1 和阶段 2**，优先统一核心函数调用和创建必要的数据库端过滤函数。这将带来最大的收益，风险可控。

PBS 查询和路径查询（阶段 3.3）需要额外评估，可以在后续版本中处理。
