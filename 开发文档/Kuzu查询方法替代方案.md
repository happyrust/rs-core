# Kuzu 查询方法替代方案

> **目标**: 将 SurrealDB 的层级查询和类型过滤查询迁移到 Kuzu 图数据库，提升查询性能 5-15 倍

---

## 📋 需要替代的方法清单

### 一、层级查询方法 (Hierarchy Queries) 🌳

#### 1. **祖先查询 (Ancestors)**
**文件**: `src/rs_surreal/queries/hierarchy.rs`

| 方法名 | 功能 | SurrealDB 模式 | Kuzu 模式 |
|--------|------|---------------|-----------|
| `query_ancestor_refnos` | 查询所有祖先 | `->pe_owner->...` | `MATCH (c)<-[:OWNS*]-(a)` |
| `query_ancestor_of_type` | 查询特定类型祖先 | `->pe_owner->... WHERE noun=X` | `MATCH (c)<-[:OWNS*]-(a) WHERE a.noun=X` |
| `get_ancestor_types` | 获取祖先类型列表 | `->pe_owner->... RETURN noun` | `MATCH (c)<-[:OWNS*]-(a) RETURN DISTINCT a.noun` |
| `get_ancestor_attmaps` | 获取祖先属性映射 | `->pe_owner->... RETURN refno.*` | `MATCH (c)<-[:OWNS*]-(a) RETURN a` |

#### 2. **子节点查询 (Children)**
**文件**: `src/rs_surreal/queries/hierarchy.rs`, `src/rs_surreal/query.rs`

| 方法名 | 功能 | SurrealDB 模式 | Kuzu 模式 |
|--------|------|---------------|-----------|
| `get_children_refnos` | 获取直接子节点 | `<-pe_owner` | `MATCH (p)-[:OWNS]->(c)` |
| `get_children_pes` | 获取子节点完整信息 | `<-pe_owner WHERE !deleted` | `MATCH (p)-[:OWNS]->(c) WHERE c.deleted=false` |
| `get_children_named_attmaps` | 获取子节点属性 | `<-pe_owner SELECT refno.*` | `MATCH (p)-[:OWNS]->(c) RETURN c` |
| `get_all_children_refnos` | 批量获取子节点 | `[{keys}]<-pe_owner` | `MATCH (p)-[:OWNS]->(c) WHERE p.refno IN [...]` |
| `query_children_full_names_map` | 获取子节点全名映射 | `<-pe_owner fn::default_full_name` | 需要递归拼接祖先 name |

#### 3. **深层子孙查询 (Deep Children)**
**文件**: `src/rs_surreal/graph.rs`

| 方法名 | 功能 | 递归深度 | SurrealDB 模式 | Kuzu 模式 |
|--------|------|---------|---------------|-----------|
| `query_deep_children_refnos` | 查询所有子孙 | 12层 | `<-pe_owner<-...(12次)` | `MATCH (p)-[:OWNS*1..12]->(d)` |
| `query_filter_deep_children` | 按类型过滤子孙 | 12层 | `<-pe_owner<-... WHERE noun IN` | `MATCH (p)-[:OWNS*1..12]->(d) WHERE d.noun IN [...]` |
| `query_filter_deep_children_atts` | 过滤子孙+属性 | 12层 | `<-pe_owner<-... SELECT refno.*` | `MATCH (p)-[:OWNS*1..12]->(d) WHERE d.noun IN [...] RETURN d` |
| `query_deep_children_refnos_pbs` | PBS系统深层查询 | 12层 | `<-pbs_owner<-...(12次)` | `MATCH (p)-[:PBS_OWNS*1..12]->(d)` |
| `query_filter_all_bran_hangs` | 查询BRAN/HANG类型 | 12层 | `<-pe_owner<-... WHERE noun IN ['BRAN','HANG']` | `MATCH (p)-[:OWNS*1..12]->(d) WHERE d.noun IN ['BRAN','HANG']` |

#### 4. **过滤子节点查询 (Filtered Children)**
**文件**: `src/rs_surreal/query.rs`

| 方法名 | 功能 | SurrealDB 模式 | Kuzu 模式 |
|--------|------|---------------|-----------|
| `query_filter_children` | 按类型过滤直接子节点 | `<-pe_owner WHERE in.noun IN [...]` | `MATCH (p)-[:OWNS]->(c) WHERE c.noun IN [...]` |
| `query_filter_children_atts` | 过滤子节点+属性 | `<-pe_owner WHERE in.noun IN [...] SELECT refno.*` | `MATCH (p)-[:OWNS]->(c) WHERE c.noun IN [...] RETURN c` |

#### 5. **祖先过滤查询 (Filtered Ancestors)**
**文件**: `src/rs_surreal/graph.rs`

| 方法名 | 功能 | SurrealDB 模式 | Kuzu 模式 |
|--------|------|---------------|-----------|
| `query_filter_ancestors` | 按类型过滤祖先 | `->pe_owner->... WHERE noun IN [...]` | `MATCH (c)<-[:OWNS*]-(a) WHERE a.noun IN [...]` |

---

### 二、类型过滤查询方法 (Type Filtering Queries) 🔍

#### 1. **dbnum + noun 过滤查询**
**文件**: `src/rs_surreal/mdb.rs`

| 方法名 | 功能 | 额外过滤条件 | SurrealDB 模式 | Kuzu 模式 |
|--------|------|-------------|---------------|-----------|
| `query_type_refnos_by_dbnum` | 按dbnum+noun查询 | `has_children`, `only_history` | `SELECT FROM {noun} WHERE dbnum={X}` | `MATCH (p:PE) WHERE p.dbnum={X} AND p.noun IN [...]` |
| `query_type_refnos_by_dbnums` | 多dbnum查询 | - | `SELECT FROM {noun} WHERE dbnum IN [...]` | `MATCH (p:PE) WHERE p.dbnum IN [...]` |
| `query_use_cate_refnos_by_dbnum` | 带类别过滤 | `only_history` | `SELECT FROM {noun} WHERE dbnum={X} AND ...` | `MATCH (p:PE) WHERE p.dbnum={X} AND p.noun IN [...]` |

**重要**: `has_children` 过滤条件的实现:
```sql
-- SurrealDB
WHERE (REFNO<-pe_owner.in)[0] != none

-- Kuzu
WHERE EXISTS { MATCH (p)-[:OWNS]->() }
```

#### 2. **world/site 查询**
**文件**: `src/rs_surreal/queries/basic.rs`

| 方法名 | 功能 | SurrealDB 模式 | Kuzu 模式 |
|--------|------|---------------|-----------|
| `get_world_by_dbnum` | 获取world节点 | `SELECT FROM WORLD WHERE dbnum={X}` | `MATCH (w:PE) WHERE w.dbnum={X} AND w.noun='WORLD' LIMIT 1` |
| `get_sites_of_dbnum` | 获取site列表 | `SELECT FROM SITE WHERE dbnum={X}` | `MATCH (s:PE) WHERE s.dbnum={X} AND s.noun='SITE'` |

---

### 三、批量查询方法 (Batch Queries) 📦

**文件**: `src/rs_surreal/queries/batch.rs`

| 方法名 | 功能 | SurrealDB 模式 | Kuzu 模式 |
|--------|------|---------------|-----------|
| `query_full_names` | 批量查询全名 | `SELECT [in, fn::default_full_name(in)] FROM {refno}<-pe_owner` | 需要递归拼接祖先 name |
| `query_full_names_map` | 批量查询全名映射 | 同上 | 同上 |

**实现策略**:
- 查询每个 refno 的祖先路径: `MATCH path = (c)<-[:OWNS*]-(a) RETURN [node IN nodes(path) | node.name]`
- 按层级排序后拼接 name 字段

---

### 四、多条件组合查询 (Multi-filter Queries) 🔧

**文件**: `src/rs_surreal/graph.rs`

| 方法名 | 功能 | 条件组合 | 递归深度 |
|--------|------|---------|---------|
| `query_multi_filter_deep_children` | 多refno+类型过滤 | 多parent + noun过滤 | 12层 |
| `query_multi_deep_children_filter_inst` | 实例化过滤 | 多parent + noun + 实例化 | 12层 |
| `query_multi_deep_children_filter_spre` | SPRE过滤 | 多parent + noun + SPRE | 可变深度 |
| `query_deep_children_refnos_filter_spre` | 单refno SPRE过滤 | 单parent + SPRE | 可变深度 |
| `query_filter_deep_children_by_path` | 路径前缀过滤 | 单parent + path前缀 | 12层 |

**特征**: 复杂的多条件组合 + 深层递归 + 类型过滤
**Kuzu 优势**: 图遍历性能更优，Cypher 的 MATCH 模式更清晰

---

### 五、时间线查询 (Timeline Queries) ⏱️

**文件**: `src/rs_surreal/queries/timeline.rs`

| 方法名 | 功能 | 优先级 |
|--------|------|-------|
| `query_ses_time_range_by_dbnum` | 查询session时间范围 | 低 |
| `query_ses_records_at_time` | 查询特定时间记录 | 低 |
| `get_latest_ses_records` | 获取最新session记录 | 低 |

**说明**: 这些方法主要是时间过滤，不涉及层级遍历，**优先级较低**

---

### 六、属性查询 (Attribute Queries) 📋

**文件**: `src/rs_surreal/queries/attributes.rs`

| 方法名 | 功能 | 说明 |
|--------|------|------|
| `get_named_attmap` | 获取PE属性映射 | 查询属性表，非层级关系 |
| `get_named_attmap_with_uda` | 获取PE+UDA属性 | 同上 |

**说明**: 如果后续 Kuzu 也存储了属性关系 (`TO_EQUI`, `TO_PIPE` 等)，可以用图查询优化

---

## 📊 统计总览

| 查询类型 | 方法数量 | 优先级 |
|---------|---------|-------|
| 层级查询 | 18个 | 🔴 高 |
| 类型过滤 | 5个 | 🔴 高 |
| 批量查询 | 3个 | 🟡 中 |
| 多条件查询 | 6个 | 🟡 中 |
| 时间线查询 | 3个 | 🟢 低 |
| **总计** | **35+ 个** | - |

---

## 🎯 实施方案

### 阶段一：基础架构搭建 (第1周)

#### 1. 创建 Kuzu 查询服务模块
```
src/rs_kuzu/
├── queries/
│   ├── mod.rs                  # 查询模块入口
│   ├── hierarchy.rs            # 层级查询服务
│   ├── type_filter.rs          # 类型过滤查询服务
│   ├── batch.rs                # 批量查询服务
│   └── multi_filter.rs         # 多条件组合查询
├── query_builder.rs            # Kuzu Cypher 查询构建器
├── cache.rs                    # Kuzu 查询缓存层
└── converter.rs                # SurrealDB ↔ Kuzu 数据转换
```

#### 2. 核心基础设施
- **连接池管理**: 复用现有的 `create_kuzu_connection()`
- **查询构建器**: `KuzuQueryBuilder` trait
- **错误处理**: `KuzuQueryError` 类型
- **缓存层**: 集成到现有 `QUERY_CACHE`

---

### 阶段二：高优先级方法实现 (第2-3周) 🔴

#### 1. 基础层级查询 - `src/rs_kuzu/queries/hierarchy.rs`

```rust
// 获取直接子节点
pub async fn kuzu_get_children_refnos(refno: RefnoEnum) -> Result<Vec<RefnoEnum>> {
    let query = format!(
        "MATCH (parent:PE {{refno: {}}})-[:OWNS]->(child:PE)
         WHERE child.deleted = false
         RETURN child.refno",
        refno.refno().0
    );
    // 执行查询并返回
}

// 查询所有祖先
pub async fn kuzu_query_ancestor_refnos(refno: RefnoEnum) -> Result<Vec<RefnoEnum>> {
    let query = format!(
        "MATCH (child:PE {{refno: {}}})<-[:OWNS*]-(ancestor:PE)
         WHERE ancestor.deleted = false
         RETURN ancestor.refno",
        refno.refno().0
    );
    // 执行查询
}

// 深层子孙查询
pub async fn kuzu_query_deep_children_refnos(refno: RefnoEnum) -> Result<Vec<RefnoEnum>> {
    let query = format!(
        "MATCH (parent:PE {{refno: {}}})-[:OWNS*1..12]->(descendant:PE)
         WHERE descendant.deleted = false
         RETURN DISTINCT descendant.refno",
        refno.refno().0
    );
    // 执行查询
}
```

**关键点**:
- 使用 `[:OWNS*1..12]` 限制递归深度 (对应 SurrealDB 的 12 层递归)
- 过滤 `deleted = false`
- `DISTINCT` 去重

#### 2. 类型过滤查询 - `src/rs_kuzu/queries/type_filter.rs`

```rust
// 按 dbnum + noun 查询
pub async fn kuzu_query_type_refnos_by_dbnum(
    nouns: &[&str],
    dbnum: u32,
    has_children: Option<bool>,
) -> Result<Vec<RefnoEnum>> {
    let nouns_str = nouns.iter().map(|n| format!("'{}'", n)).join(", ");

    let child_filter = match has_children {
        Some(true) => "AND EXISTS { MATCH (p)-[:OWNS]->() }",
        Some(false) => "AND NOT EXISTS { MATCH (p)-[:OWNS]->() }",
        None => "",
    };

    let query = format!(
        "MATCH (p:PE)
         WHERE p.dbnum = {} AND p.noun IN [{}] AND p.deleted = false {}
         RETURN p.refno",
        dbnum, nouns_str, child_filter
    );
    // 执行查询
}

// 获取 world 节点
pub async fn kuzu_get_world_by_dbnum(dbnum: u32) -> Result<Option<RefnoEnum>> {
    let query = format!(
        "MATCH (w:PE)
         WHERE w.dbnum = {} AND w.noun = 'WORLD'
         RETURN w.refno
         LIMIT 1",
        dbnum
    );
    // 执行查询
}
```

#### 3. 过滤深层查询 - `src/rs_kuzu/queries/hierarchy.rs`

```rust
// 按类型过滤深层子孙
pub async fn kuzu_query_filter_deep_children(
    refno: RefnoEnum,
    nouns: &[&str],
) -> Result<Vec<RefnoEnum>> {
    let noun_filter = if nouns.is_empty() {
        String::new()
    } else {
        let nouns_str = nouns.iter().map(|n| format!("'{}'", n)).join(", ");
        format!("AND descendant.noun IN [{}]", nouns_str)
    };

    let query = format!(
        "MATCH (parent:PE {{refno: {}}})-[:OWNS*1..12]->(descendant:PE)
         WHERE descendant.deleted = false {}
         RETURN DISTINCT descendant.refno",
        refno.refno().0,
        noun_filter
    );
    // 执行查询
}

// 按类型过滤祖先
pub async fn kuzu_query_filter_ancestors(
    refno: RefnoEnum,
    nouns: &[&str],
) -> Result<Vec<RefnoEnum>> {
    let nouns_str = nouns.iter().map(|n| format!("'{}'", n)).join(", ");

    let query = format!(
        "MATCH (child:PE {{refno: {}}})<-[:OWNS*]-(ancestor:PE)
         WHERE ancestor.noun IN [{}] AND ancestor.deleted = false
         RETURN ancestor.refno",
        refno.refno().0,
        nouns_str
    );
    // 执行查询
}
```

---

### 阶段三：中优先级方法实现 (第4周) 🟡

#### 4. 批量查询 - `src/rs_kuzu/queries/batch.rs`

```rust
// 批量获取子节点
pub async fn kuzu_get_all_children_refnos(
    refnos: &[RefnoEnum],
) -> Result<Vec<RefnoEnum>> {
    let refno_list = refnos.iter().map(|r| r.refno().0).join(", ");

    let query = format!(
        "MATCH (parent:PE)-[:OWNS]->(child:PE)
         WHERE parent.refno IN [{}] AND child.deleted = false
         RETURN DISTINCT child.refno",
        refno_list
    );
    // 执行查询
}

// 查询全名 (需要递归拼接祖先 name)
pub async fn kuzu_query_full_names(
    refnos: &[RefnoEnum],
) -> Result<Vec<String>> {
    // 实现方案:
    // 1. 查询每个 refno 的祖先路径
    // 2. 按层级排序
    // 3. 拼接 name 字段

    let refno_list = refnos.iter().map(|r| r.refno().0).join(", ");

    let query = format!(
        "MATCH path = (child:PE)<-[:OWNS*]-(ancestor:PE)
         WHERE child.refno IN [{}]
         RETURN child.refno,
                [node IN nodes(path) | node.name] AS names
         ORDER BY length(path) DESC",
        refno_list
    );
    // 执行查询并拼接全名
}
```

#### 5. 多条件查询 - `src/rs_kuzu/queries/multi_filter.rs`

```rust
// 多 refno + 类型过滤深层查询
pub async fn kuzu_query_multi_filter_deep_children(
    refnos: &[RefnoEnum],
    nouns: &[&str],
) -> Result<Vec<RefnoEnum>> {
    let refno_list = refnos.iter().map(|r| r.refno().0).join(", ");
    let nouns_str = nouns.iter().map(|n| format!("'{}'", n)).join(", ");

    let query = format!(
        "MATCH (parent:PE)-[:OWNS*1..12]->(descendant:PE)
         WHERE parent.refno IN [{}]
               AND descendant.noun IN [{}]
               AND descendant.deleted = false
         RETURN DISTINCT descendant.refno",
        refno_list, nouns_str
    );
    // 执行查询
}

// SPRE 过滤查询 (需要检查是否实例化)
pub async fn kuzu_query_deep_children_filter_spre(
    refno: RefnoEnum,
    max_level: Option<usize>,
) -> Result<Vec<RefnoEnum>> {
    let depth_limit = max_level.unwrap_or(12);

    let query = format!(
        "MATCH (parent:PE {{refno: {}}})-[:OWNS*1..{}]->(descendant:PE)
         WHERE descendant.deleted = false
               AND NOT EXISTS {{ MATCH (descendant)-[:TO_SPRE]->() }}
         RETURN DISTINCT descendant.refno",
        refno.refno().0,
        depth_limit
    );
    // 注意: 这里假设 TO_SPRE 关系已经在 Kuzu 中创建
}
```

---

### 阶段四：查询路由与兼容层 (第5周) 🔄

#### 创建统一查询路由器

```rust
// src/rs_kuzu/query_router.rs

/// 查询引擎选择策略
#[derive(Debug, Clone, Copy)]
pub enum QueryEngine {
    SurrealDB,      // 使用 SurrealDB
    Kuzu,           // 使用 Kuzu
    Auto,           // 自动选择 (根据性能和数据完整性)
}

/// 统一查询路由器
pub struct QueryRouter {
    strategy: QueryEngine,
}

impl QueryRouter {
    pub async fn get_children_refnos(&self, refno: RefnoEnum) -> Result<Vec<RefnoEnum>> {
        match self.strategy {
            QueryEngine::SurrealDB => {
                rs_surreal::queries::hierarchy::get_children_refnos(refno).await
            }
            QueryEngine::Kuzu => {
                rs_kuzu::queries::hierarchy::kuzu_get_children_refnos(refno).await
            }
            QueryEngine::Auto => {
                // 自动选择策略:
                // 1. 检查 Kuzu 数据是否完整
                // 2. 对比 SurrealDB 和 Kuzu 的性能
                // 3. 回退到 SurrealDB 如果 Kuzu 失败
                self.auto_select_get_children(refno).await
            }
        }
    }

    async fn auto_select_get_children(&self, refno: RefnoEnum) -> Result<Vec<RefnoEnum>> {
        // 尝试 Kuzu
        match rs_kuzu::queries::hierarchy::kuzu_get_children_refnos(refno).await {
            Ok(result) => Ok(result),
            Err(e) => {
                log::warn!("Kuzu query failed, fallback to SurrealDB: {}", e);
                rs_surreal::queries::hierarchy::get_children_refnos(refno).await
            }
        }
    }
}
```

---

### 阶段五：性能优化与测试 (第6周) ⚡

#### 1. 缓存优化
```rust
// 集成到现有缓存层
impl QUERY_CACHE {
    pub async fn get_kuzu_children(&self, refno: &RefnoEnum) -> Option<Vec<RefnoEnum>> {
        // 检查缓存
    }

    pub async fn set_kuzu_children(&self, refno: RefnoEnum, children: Vec<RefnoEnum>) {
        // 设置缓存
    }
}
```

#### 2. 性能基准测试
```rust
// examples/benchmark_kuzu_vs_surreal_queries.rs

async fn benchmark_hierarchy_queries() {
    // 测试场景:
    // 1. 单层子节点查询 (100次)
    // 2. 深层递归查询 (100次)
    // 3. 类型过滤查询 (100次)
    // 4. 多条件组合查询 (100次)

    // 对比 SurrealDB vs Kuzu 性能
}
```

#### 3. 集成测试
```rust
// src/test/test_kuzu_queries.rs

#[tokio::test]
async fn test_kuzu_get_children_consistency() {
    // 验证 Kuzu 查询结果与 SurrealDB 一致
}

#[tokio::test]
async fn test_kuzu_deep_children_performance() {
    // 验证深层查询性能
}
```

---

## 💡 实施策略建议

### 渐进式迁移:
1. ✅ **第1阶段**: 仅在新功能中使用 Kuzu 查询
2. ✅ **第2阶段**: 在非关键路径启用 `QueryEngine::Auto` 模式
3. ✅ **第3阶段**: 在关键路径使用 Kuzu + SurrealDB 双写双读验证
4. ✅ **第4阶段**: 逐步切换到 `QueryEngine::Kuzu` 模式

### 风险控制:
- 始终保留 SurrealDB 查询作为 fallback
- 通过配置文件控制查询引擎选择 (`DbOption.toml` 添加 `query_engine = "auto"`)
- 监控 Kuzu 查询失败率和性能指标

---

## 📈 预期性能提升

根据之前的性能测试 (Kuzu 保存速度是 SurrealDB 的 **18.99倍**):

| 查询类型 | 预期提升 | 原因 |
|---------|---------|------|
| 层级查询 | **5-10倍** | 图遍历 vs 表连接 |
| 深层递归 | **10-20倍** | 原生递归 vs 12层嵌套 |
| 类型过滤 | **3-5倍** | 索引优化 + 图扫描 |

**总体目标**: 使查询性能提升 **5-15 倍**

---

## 📊 性能对比矩阵

| SurrealDB 模式 | Kuzu 模式 | 性能提升 | 可读性 |
|---------------|----------|---------|--------|
| `<-pe_owner` 单层 | `MATCH (p)-[:OWNS]->(c)` | 3-5x | ✅ 更清晰 |
| `<-pe_owner<-...<-` 12层嵌套 | `MATCH (p)-[:OWNS*1..12]->(c)` | 10-20x | ✅✅ 显著提升 |
| `where REFNO.dbnum={dbnum}` | `WHERE p.dbnum = {dbnum}` | 3-5x | ✅ 相当 |
| `where noun in [...]` | `WHERE p.noun IN [...]` | 2-3x | ✅ 相当 |
| 嵌套子查询 | `EXISTS { MATCH ... }` | 5-8x | ✅✅ 更简洁 |

---

## 🔧 配置文件扩展

在 `DbOption.toml` 中添加查询引擎配置:

```toml
# 查询引擎选择: "surrealdb" | "kuzu" | "auto"
query_engine = "auto"

# Kuzu 数据库路径
kuzu_db_path = "./data/kuzu_db"

# 查询超时时间 (毫秒)
query_timeout_ms = 5000

# 是否启用查询缓存
enable_query_cache = true

# 缓存过期时间 (秒)
cache_expire_secs = 300
```

---

## ✅ 检查清单

### 第1周 - 基础架构
- [ ] 创建 `src/rs_kuzu/queries/` 模块结构
- [ ] 实现 `KuzuQueryBuilder` trait
- [ ] 实现 `KuzuQueryError` 错误类型
- [ ] 集成查询缓存层

### 第2-3周 - 高优先级方法
- [ ] 实现基础层级查询 (5个方法)
- [ ] 实现类型过滤查询 (5个方法)
- [ ] 实现过滤深层查询 (5个方法)
- [ ] 编写单元测试

### 第4周 - 中优先级方法
- [ ] 实现批量查询 (3个方法)
- [ ] 实现多条件查询 (6个方法)
- [ ] 编写集成测试

### 第5周 - 查询路由
- [ ] 实现 `QueryRouter` 和 `QueryEngine` 枚举
- [ ] 实现自动选择和 fallback 机制
- [ ] 配置文件扩展

### 第6周 - 性能优化
- [ ] 性能基准测试
- [ ] 缓存优化
- [ ] 双写双读验证
- [ ] 文档完善

---

## 📚 参考资料

- [Kuzu 官方文档](https://kuzudb.com/)
- [Cypher 查询语言](https://neo4j.com/docs/cypher-manual/current/)
- [SurrealDB 查询文档](https://surrealdb.com/docs/surrealql)

---

**最后更新**: 2025-10-07
**负责人**: DPC
**状态**: 📝 方案设计完成，待实施
