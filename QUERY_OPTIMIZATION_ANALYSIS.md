# query_deep_children_filter_inst 性能优化分析

## 当前实现分析

### 函数签名
```rust
async fn query_deep_children_filter_inst(
    refno: RefU64,
    nouns: &[&str],
    filter: bool,
) -> anyhow::Result<Vec<RefU64>>
```

### 当前执行流程

1. **第一步：收集候选节点** (1 次数据库查询)
   ```rust
   let candidates = collect_descendant_refnos(refno.into(), nouns, true, false).await?;
   ```
   - 调用 `fn::collect_descendants_by_children` 获取所有子孙节点
   - 返回可能包含数千个节点

2. **第二步：分块过滤** (N 次数据库查询, N = candidates.len() / 200)
   ```rust
   for chunk in candidates.chunks(200) {
       let sql = format!("select value id from [{pe_keys}] where array::len(->inst_relate) = 0 and array::len(->tubi_relate) = 0");
       // 执行查询...
   }
   ```

### 性能瓶颈

#### 1. 往返次数过多 ⚠️ **最严重**
- **问题**: 如果有 2000 个候选节点，需要执行 11 次数据库查询
  - 1 次收集候选节点
  - 10 次分块过滤 (2000 / 200 = 10)
- **影响**: 每次往返约 1-5ms，总延迟可达 55ms+
- **优化潜力**: 减少到 1 次查询可节省 90% 的往返时间

#### 2. array::len 性能差 ⚠️
- **问题**: `array::len(->inst_relate)` 需要：
  1. 遍历所有 inst_relate 关系边
  2. 计数
  3. 比较
- **影响**: 对于有大量关系的节点，计数操作很慢
- **替代方案**: 使用 `count(->inst_relate)` 或检查第一个元素

#### 3. 分块大小固定 ⚠️
- **问题**: 固定 200 个一组，不考虑实际情况
  - 节点少时（< 200）：浪费一次往返
  - 节点多时（> 5000）：查询次数过多
- **影响**: 无法根据数据量动态调整
- **优化**: 自适应分块大小或完全避免分块

#### 4. 串行执行 ⚠️
- **问题**: 分块查询是串行执行的
- **影响**: 总时间 = 单次查询时间 × 查询次数
- **优化**: 可以并行执行多个查询

## 优化方案

### 方案 1: 数据库端统一处理 ⭐ **推荐**

#### 优势
- ✅ 往返次数: 从 N+1 次减少到 1 次
- ✅ 减少 90%+ 的网络延迟
- ✅ 充分利用数据库端优化
- ✅ 代码更简洁易维护

#### 实现
在 SurrealDB 创建函数 `fn::collect_descendants_filter_inst`:

```surql
-- 收集子孙节点并过滤 inst_relate 和 tubi_relate
DEFINE FUNCTION fn::collect_descendants_filter_inst(
    $root: record,
    $types: array<string>,
    $filter_inst: bool,
    $include_self: bool
) {
    -- 1. 收集所有子孙节点
    LET $descendants = fn::collect_descendants_by_children($root, $types, $include_self, true);
    
    -- 2. 如果不需要过滤，直接返回
    IF !$filter_inst {
        RETURN $descendants;
    };
    
    -- 3. 过滤掉有 inst_relate 或 tubi_relate 的节点
    RETURN array::filter($descendants, |$node| {
        count(SELECT VALUE id FROM $node->inst_relate LIMIT 1) = 0 AND
        count(SELECT VALUE id FROM $node->tubi_relate LIMIT 1) = 0
    });
};
```

Rust 代码优化为:
```rust
async fn query_deep_children_filter_inst(
    refno: RefU64,
    nouns: &[&str],
    filter: bool,
) -> anyhow::Result<Vec<RefU64>> {
    let nouns_str = rs_surreal::convert_to_sql_str_array(nouns);
    let types_expr = if nouns.is_empty() {
        "[]".to_string()
    } else {
        format!("[{}]", nouns_str)
    };
    let filter_str = if filter { "true" } else { "false" };
    let pe_key = refno.to_pe_key();
    
    let sql = format!(
        "SELECT VALUE fn::collect_descendants_filter_inst({}, {}, {}, true);",
        pe_key, types_expr, filter_str
    );
    
    let mut response = SUL_DB.query(&sql).await?;
    let refnos: Vec<RefnoEnum> = response.take(0)?;
    Ok(refnos.into_iter().map(|r| r.refno()).collect())
}
```

**性能提升预估**:
- 2000 节点: 从 ~55ms 降到 ~5ms (提升 91%)
- 10000 节点: 从 ~255ms 降到 ~15ms (提升 94%)

### 方案 2: 优化关系检查 ⭐

#### 问题
`array::len(->inst_relate) = 0` 需要遍历所有关系

#### 优化
使用更高效的检查方式:

```surql
-- 方式 1: 使用 count + LIMIT (推荐)
count(SELECT VALUE id FROM $node->inst_relate LIMIT 1) = 0

-- 方式 2: 检查第一个元素
($node->inst_relate)[0] = NONE

-- 方式 3: 使用 EXISTS (如果支持)
NOT EXISTS (SELECT VALUE id FROM $node->inst_relate LIMIT 1)
```

**性能提升**: 对每个节点节省 50-80% 的检查时间

### 方案 3: 并行分块查询

如果无法使用方案 1，可以并行执行分块查询:

```rust
async fn query_deep_children_filter_inst(
    refno: RefU64,
    nouns: &[&str],
    filter: bool,
) -> anyhow::Result<Vec<RefU64>> {
    let candidates = collect_descendant_refnos(refno.into(), nouns, true, false).await?;
    if candidates.is_empty() {
        return Ok(vec![]);
    }
    
    // 并行执行所有分块查询
    let futures: Vec<_> = candidates
        .chunks(200)
        .map(|chunk| {
            let pe_keys = chunk.iter().map(|x| x.to_pe_key()).join(",");
            let filter_clause = if filter {
                " where count(SELECT VALUE id FROM ->inst_relate LIMIT 1) = 0 and count(SELECT VALUE id FROM ->tubi_relate LIMIT 1) = 0"
            } else {
                ""
            };
            let sql = format!("select value id from [{}]{};", pe_keys, filter_clause);
            
            async move {
                let mut response = SUL_DB.query(&sql).await?;
                response.take::<Vec<RefnoEnum>>(0)
            }
        })
        .collect();
    
    // 等待所有查询完成
    let results = futures::future::try_join_all(futures).await?;
    
    let result: Vec<RefU64> = results
        .into_iter()
        .flatten()
        .map(|r| r.refno())
        .collect();
    
    Ok(result)
}
```

**性能提升**: 总时间 ≈ 最慢的单次查询时间

### 方案 4: 自适应分块大小

根据节点数量动态调整分块大小:

```rust
let chunk_size = match candidates.len() {
    0..=500 => 500,        // 少量节点，一次查询
    501..=2000 => 400,     // 中等数量，2-5 次查询
    2001..=10000 => 500,   // 大量节点，20-50 次查询
    _ => 1000,             // 超大量，减少批次数
};

for chunk in candidates.chunks(chunk_size) {
    // ...
}
```

## 推荐实施方案

### 阶段 1: 立即实施 (方案 1 + 方案 2)
1. 创建 SurrealDB 函数 `fn::collect_descendants_filter_inst`
2. 使用优化的关系检查 `count(...LIMIT 1) = 0`
3. 更新 Rust 代码使用新函数

**预期收益**: 性能提升 90%+

### 阶段 2: 进一步优化 (可选)
1. 添加结果缓存 (使用 `#[cached]`)
2. 添加索引到 inst_relate 和 tubi_relate
3. 考虑物化视图存储无关系的节点

## 性能对比测试计划

```rust
#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;
    
    #[tokio::test]
    async fn test_query_performance_comparison() {
        let refno = RefU64::from_two_nums(100, 200);
        let nouns = &["BOX", "CYLI", "CONE"];
        
        // 测试旧实现
        let start = Instant::now();
        let result_old = query_deep_children_filter_inst_old(refno, nouns, true).await.unwrap();
        let time_old = start.elapsed();
        
        // 测试新实现
        let start = Instant::now();
        let result_new = query_deep_children_filter_inst(refno, nouns, true).await.unwrap();
        let time_new = start.elapsed();
        
        println!("旧实现: {:?}, 结果数: {}", time_old, result_old.len());
        println!("新实现: {:?}, 结果数: {}", time_new, result_new.len());
        println!("性能提升: {:.2}%", (1.0 - time_new.as_secs_f64() / time_old.as_secs_f64()) * 100.0);
        
        assert_eq!(result_old.len(), result_new.len());
    }
}
```

## 额外优化建议

1. **添加索引**
   ```surql
   DEFINE INDEX inst_relate_idx ON TABLE inst_relate FIELDS in, out;
   DEFINE INDEX tubi_relate_idx ON TABLE tubi_relate FIELDS in, out;
   ```

2. **使用缓存**
   ```rust
   #[cached(time = 300, result = true)]  // 缓存 5 分钟
   pub async fn query_deep_children_filter_inst(...)
   ```

3. **批量预取**
   - 如果经常查询相同的 refno，可以预取并缓存结果

4. **监控和日志**
   - 添加查询时间监控
   - 记录慢查询用于进一步优化
