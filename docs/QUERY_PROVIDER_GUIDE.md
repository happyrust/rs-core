# 统一查询接口使用指南

> **版本**: 1.0.0
> **日期**: 2025-10-08
> **作者**: DPC

## 📋 目录

- [概述](#概述)
- [核心概念](#核心概念)
- [快速开始](#快速开始)
- [详细教程](#详细教程)
- [API 参考](#api-参考)
- [最佳实践](#最佳实践)
- [常见问题](#常见问题)

---

## 概述

### 什么是统一查询接口？

统一查询接口（Query Provider）是一套基于 Rust trait 的抽象层，它提供了：

- **统一的 API**：无论使用 SurrealDB 还是 Kuzu，API 完全相同
- **透明切换**：一行代码即可切换数据库引擎
- **类型安全**：编译时检查，零运行时开销
- **智能回退**：查询失败时自动回退到备用数据库
- **性能监控**：自动记录慢查询和性能指标

### 为什么需要它？

在 aios_core 项目中，我们同时支持 SurrealDB 和 Kuzu 两种数据库：

- **SurrealDB**：成熟稳定，功能完整
- **Kuzu**：图查询性能优秀（5-15倍提升）

传统方式需要在业务代码中判断使用哪个数据库，代码复杂且难以维护。统一查询接口解决了这个问题。

### 架构图

```
┌──────────────────────────────────────────────────┐
│             应用层（业务逻辑）                      │
│  - 使用统一的 QueryProvider trait                 │
│  - 不关心底层实现细节                              │
└───────────────────┬──────────────────────────────┘
                    │
         ┌──────────▼──────────┐
         │   QueryRouter       │  智能路由器
         │  - Auto 模式        │  - 自动选择引擎
         │  - 回退机制         │  - 性能监控
         │  - 策略配置         │  - 动态切换
         └──────────┬──────────┘
                    │
        ┌───────────┴───────────┐
        │                       │
┌───────▼────────┐    ┌────────▼────────┐
│  SurrealDB     │    │     Kuzu        │
│  Provider      │    │   Provider      │
│  - 稳定可靠     │    │   - 高性能      │
│  - 功能完整     │    │   - 图优化      │
└────────────────┘    └─────────────────┘
```

---

## 核心概念

### 1. Trait 层次结构

```rust
QueryProvider (统一接口)
    ├── HierarchyQuery   // 层级关系查询
    ├── TypeQuery        // 类型过滤查询
    ├── BatchQuery       // 批量查询
    └── GraphQuery       // 图遍历查询
```

#### HierarchyQuery - 层级关系查询

处理父子关系、祖先后代的查询：

```rust
pub trait HierarchyQuery {
    // 获取直接子节点
    async fn get_children(&self, refno: RefnoEnum) -> QueryResult<Vec<RefnoEnum>>;

    // 批量获取子节点
    async fn get_children_batch(&self, refnos: &[RefnoEnum]) -> QueryResult<Vec<RefnoEnum>>;

    // 查询所有子孙（递归）
    async fn get_descendants(&self, refno: RefnoEnum, max_depth: Option<usize>)
        -> QueryResult<Vec<RefnoEnum>>;

    // 查询所有祖先
    async fn get_ancestors(&self, refno: RefnoEnum) -> QueryResult<Vec<RefnoEnum>>;

    // 查询特定类型的祖先
    async fn get_ancestors_of_type(&self, refno: RefnoEnum, nouns: &[&str])
        -> QueryResult<Vec<RefnoEnum>>;

    // 查询过滤后的子孙
    async fn get_descendants_filtered(&self, refno: RefnoEnum, nouns: &[&str], max_depth: Option<usize>)
        -> QueryResult<Vec<RefnoEnum>>;

    // 获取子节点的完整信息
    async fn get_children_pes(&self, refno: RefnoEnum) -> QueryResult<Vec<PE>>;
}
```

#### TypeQuery - 类型过滤查询

基于元素类型（noun）的查询：

```rust
pub trait TypeQuery {
    // 按类型和数据库编号查询
    async fn query_by_type(&self, nouns: &[&str], dbnum: i32, has_children: Option<bool>)
        -> QueryResult<Vec<RefnoEnum>>;

    // 多数据库查询
    async fn query_by_type_multi_db(&self, nouns: &[&str], dbnums: &[i32])
        -> QueryResult<Vec<RefnoEnum>>;

    // 获取 World 节点
    async fn get_world(&self, dbnum: i32) -> QueryResult<Option<RefnoEnum>>;

    // 获取所有 Site 节点
    async fn get_sites(&self, dbnum: i32) -> QueryResult<Vec<RefnoEnum>>;

    // 统计元素数量
    async fn count_by_type(&self, noun: &str, dbnum: i32) -> QueryResult<usize>;
}
```

#### BatchQuery - 批量查询

高效的批量操作：

```rust
pub trait BatchQuery {
    // 批量获取 PE 信息
    async fn get_pes_batch(&self, refnos: &[RefnoEnum]) -> QueryResult<Vec<PE>>;

    // 批量获取属性映射
    async fn get_attmaps_batch(&self, refnos: &[RefnoEnum]) -> QueryResult<Vec<NamedAttMap>>;

    // 批量获取全名
    async fn get_full_names_batch(&self, refnos: &[RefnoEnum])
        -> QueryResult<Vec<(RefnoEnum, String)>>;
}
```

#### GraphQuery - 图遍历查询

复杂的图算法：

```rust
pub trait GraphQuery {
    // 多起点深层子孙查询
    async fn query_multi_descendants(&self, refnos: &[RefnoEnum], nouns: &[&str], max_depth: Option<usize>)
        -> QueryResult<Vec<RefnoEnum>>;

    // 查找最短路径
    async fn find_shortest_path(&self, from: RefnoEnum, to: RefnoEnum)
        -> QueryResult<Vec<RefnoEnum>>;

    // 获取节点深度
    async fn get_node_depth(&self, refno: RefnoEnum) -> QueryResult<usize>;
}
```

### 2. 查询引擎模式

```rust
pub enum QueryEngine {
    SurrealDB,  // 只使用 SurrealDB
    Kuzu,       // 只使用 Kuzu
    Auto,       // 自动选择（优先 Kuzu，失败回退到 SurrealDB）
}
```

### 3. 查询策略

```rust
pub struct QueryStrategy {
    pub engine: QueryEngine,           // 引擎选择
    pub enable_fallback: bool,         // 是否启用回退
    pub timeout_ms: Option<u64>,       // 查询超时（毫秒）
    pub enable_performance_log: bool,  // 是否启用性能日志
}
```

---

## 快速开始

### 安装

在 `Cargo.toml` 中添加（已包含在 aios_core 中）：

```toml
[features]
default = []
kuzu = ["dep:kuzu"]

[dependencies]
async-trait = "0.1"
```

### 最简单的例子

```rust
use aios_core::query_provider::*;
use aios_core::{init_surreal, RefnoEnum};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 初始化数据库
    init_surreal().await?;

    // 2. 创建查询路由器
    let router = QueryRouter::auto()?;

    // 3. 使用统一接口查询
    let pipes = router.query_by_type(&["PIPE"], 1112, None).await?;
    println!("找到 {} 个 PIPE 元素", pipes.len());

    // 4. 获取第一个 PIPE 的子节点
    if let Some(&first_pipe) = pipes.first() {
        let children = router.get_children(first_pipe).await?;
        println!("第一个 PIPE 有 {} 个子节点", children.len());
    }

    Ok(())
}
```

---

## 详细教程

### 教程 1: 使用 SurrealDB Provider

当你只想使用 SurrealDB 时：

```rust
use aios_core::query_provider::*;

async fn example_surreal_only() -> QueryResult<()> {
    // 创建 SurrealDB 查询提供者
    let provider = SurrealQueryProvider::new()?;

    println!("提供者: {}", provider.provider_name());

    // 查询所有 EQUI 元素
    let equis = provider.query_by_type(&["EQUI"], 1112, None).await?;
    println!("找到 {} 个 EQUI", equis.len());

    // 获取第一个 EQUI 的祖先
    if let Some(&first_equi) = equis.first() {
        let ancestors = provider.get_ancestors(first_equi).await?;
        println!("祖先链长度: {}", ancestors.len());

        // 只获取 ZONE 类型的祖先
        let zone_ancestors = provider
            .get_ancestors_of_type(first_equi, &["ZONE"])
            .await?;
        println!("ZONE 祖先数量: {}", zone_ancestors.len());
    }

    Ok(())
}
```

### 教程 2: 使用 Kuzu Provider

当你只想使用 Kuzu 时（需要启用 `kuzu` feature）：

```rust
#[cfg(feature = "kuzu")]
use aios_core::query_provider::*;

#[cfg(feature = "kuzu")]
async fn example_kuzu_only() -> QueryResult<()> {
    // 创建 Kuzu 查询提供者
    let provider = KuzuQueryProvider::new()?;

    // 健康检查
    let is_healthy = provider.health_check().await?;
    println!("Kuzu 状态: {}", if is_healthy { "正常" } else { "异常" });

    // 高性能深层递归查询
    let zones = provider.query_by_type(&["ZONE"], 1112, Some(true)).await?;

    for &zone in zones.iter().take(5) {
        let start = std::time::Instant::now();
        let descendants = provider.get_descendants(zone, Some(12)).await?;
        let elapsed = start.elapsed();

        println!("Zone {:?}: {} 个子孙, 耗时: {:?}",
            zone, descendants.len(), elapsed);
    }

    Ok(())
}
```

### 教程 3: 使用查询路由器（推荐）

智能路由和自动回退：

```rust
use aios_core::query_provider::*;

async fn example_with_router() -> QueryResult<()> {
    // 方式 1: 使用默认 Auto 模式
    let router = QueryRouter::auto()?;

    // 方式 2: 自定义策略
    let router = QueryRouter::new(QueryStrategy {
        engine: QueryEngine::Auto,
        enable_fallback: true,
        timeout_ms: Some(5000),
        enable_performance_log: true,
    })?;

    // 执行查询（自动选择最优引擎）
    let pipes = router.query_by_type(&["PIPE", "ELBO"], 1112, None).await?;
    println!("找到 {} 个管道元素", pipes.len());

    // 批量获取子节点
    let sample: Vec<_> = pipes.iter().take(10).copied().collect();
    let all_children = router.get_children_batch(&sample).await?;
    println!("10 个元素的所有子节点: {} 个", all_children.len());

    Ok(())
}
```

### 教程 4: 动态切换策略

运行时改变查询引擎：

```rust
async fn example_dynamic_strategy() -> QueryResult<()> {
    let router = QueryRouter::auto()?;

    // 第一阶段：使用 SurrealDB（稳定可靠）
    router.set_strategy(QueryStrategy::surreal_only());
    let result1 = router.query_by_type(&["ZONE"], 1112, None).await?;
    println!("[SurrealDB] 找到 {} 个 ZONE", result1.len());

    // 第二阶段：切换到 Kuzu（高性能）
    #[cfg(feature = "kuzu")]
    {
        router.set_strategy(QueryStrategy::kuzu_only());
        let result2 = router.query_by_type(&["ZONE"], 1112, None).await?;
        println!("[Kuzu] 找到 {} 个 ZONE", result2.len());
    }

    // 第三阶段：回到 Auto 模式
    router.set_strategy(QueryStrategy::auto());
    let result3 = router.query_by_type(&["ZONE"], 1112, None).await?;
    println!("[Auto] 找到 {} 个 ZONE", result3.len());

    Ok(())
}
```

### 教程 5: 批量操作优化

高效处理大量数据：

```rust
async fn example_batch_operations() -> QueryResult<()> {
    let router = QueryRouter::auto()?;

    // 获取所有 PIPE
    let pipes = router.query_by_type(&["PIPE"], 1112, None).await?;
    println!("总共 {} 个 PIPE", pipes.len());

    // 批量获取 PE 信息（一次性获取，而不是逐个查询）
    let pes = router.get_pes_batch(&pipes[..100.min(pipes.len())]).await?;
    println!("批量获取了 {} 个 PE 的完整信息", pes.len());

    // 批量获取属性
    let attmaps = router.get_attmaps_batch(&pipes[..50.min(pipes.len())]).await?;
    println!("批量获取了 {} 个属性映射", attmaps.len());

    // 批量获取全名
    let full_names = router.get_full_names_batch(&pipes[..20.min(pipes.len())]).await?;
    for (refno, name) in full_names.iter().take(5) {
        println!("  {:?} -> {}", refno, name);
    }

    Ok(())
}
```

### 教程 6: 图遍历高级查询

复杂的图算法应用：

```rust
async fn example_graph_algorithms() -> QueryResult<()> {
    let router = QueryRouter::auto()?;

    // 获取测试数据
    let zones = router.query_by_type(&["ZONE"], 1112, Some(true)).await?;
    let zone = zones.first().copied().unwrap();

    // 1. 计算节点深度
    let depth = router.get_node_depth(zone).await?;
    println!("节点 {:?} 的深度: {}", zone, depth);

    // 2. 多起点查询
    let start_points = &zones[..3.min(zones.len())];
    let descendants = router
        .query_multi_descendants(start_points, &["PIPE", "EQUI"], Some(5))
        .await?;
    println!("从 {} 个起点查询到 {} 个子孙", start_points.len(), descendants.len());

    // 3. 查找最短路径
    if zones.len() >= 2 {
        let from = zones[0];
        let to = zones[1];
        let path = router.find_shortest_path(from, to).await?;
        println!("从 {:?} 到 {:?} 的最短路径长度: {}", from, to, path.len());
    }

    Ok(())
}
```

### 教程 7: 错误处理

优雅地处理查询错误：

```rust
use aios_core::query_provider::*;

async fn example_error_handling() -> QueryResult<()> {
    let router = QueryRouter::new(QueryStrategy {
        engine: QueryEngine::Auto,
        enable_fallback: true,  // 重要：启用回退
        timeout_ms: Some(3000),
        enable_performance_log: true,
    })?;

    // 即使 Kuzu 不可用，也会自动回退到 SurrealDB
    match router.query_by_type(&["PIPE"], 1112, None).await {
        Ok(pipes) => {
            println!("✓ 查询成功: {} 个 PIPE", pipes.len());
        }
        Err(QueryError::ConnectionError(msg)) => {
            eprintln!("✗ 数据库连接失败: {}", msg);
        }
        Err(QueryError::ExecutionError(msg)) => {
            eprintln!("✗ 查询执行失败: {}", msg);
        }
        Err(QueryError::Timeout(msg)) => {
            eprintln!("✗ 查询超时: {}", msg);
        }
        Err(e) => {
            eprintln!("✗ 其他错误: {}", e);
        }
    }

    // 检查数据库健康状态
    if !router.health_check().await? {
        eprintln!("⚠️  数据库健康检查失败");
    }

    Ok(())
}
```

---

## API 参考

### QueryProvider Trait

完整的统一查询接口。

#### 基础方法

```rust
// 获取单个 PE 信息
async fn get_pe(&self, refno: RefnoEnum) -> QueryResult<Option<PE>>;

// 获取属性映射
async fn get_attmap(&self, refno: RefnoEnum) -> QueryResult<Option<NamedAttMap>>;

// 检查 PE 是否存在
async fn exists(&self, refno: RefnoEnum) -> QueryResult<bool>;

// 获取提供者名称
fn provider_name(&self) -> &str;

// 健康检查
async fn health_check(&self) -> QueryResult<bool>;
```

### QueryRouter

智能查询路由器。

#### 构造函数

```rust
// 创建自定义策略的路由器
pub fn new(strategy: QueryStrategy) -> QueryResult<Self>;

// 创建 Auto 模式路由器（推荐）
pub fn auto() -> QueryResult<Self>;

// 创建只使用 SurrealDB 的路由器
pub fn surreal_only() -> QueryResult<Self>;

// 创建只使用 Kuzu 的路由器
pub fn kuzu_only() -> QueryResult<Self>;
```

#### 方法

```rust
// 更新策略
pub fn set_strategy(&self, strategy: QueryStrategy);

// 获取当前策略
pub fn get_strategy(&self) -> QueryStrategy;
```

### QueryStrategy

查询策略配置。

#### 预定义策略

```rust
// SurrealDB 专用策略
QueryStrategy::surreal_only()

// Kuzu 专用策略
QueryStrategy::kuzu_only()

// 自动选择策略（默认）
QueryStrategy::auto()
```

#### 构建器方法

```rust
// 设置回退
pub fn with_fallback(self, enable: bool) -> Self;

// 设置超时
pub fn with_timeout(self, timeout_ms: u64) -> Self;

// 设置性能日志
pub fn with_performance_log(self, enable: bool) -> Self;
```

---

## 最佳实践

### 1. 选择合适的查询引擎模式

| 场景 | 推荐模式 | 原因 |
|------|---------|------|
| 开发环境 | `Auto` | 自动选择最优引擎，便于测试 |
| 生产环境（稳定性优先） | `SurrealDB` | 成熟稳定，功能完整 |
| 生产环境（性能优先） | `Kuzu` + 回退 | 高性能，带回退保证可用性 |
| 灰度发布 | `Auto` + 监控 | 逐步切换，监控性能指标 |

### 2. 合理使用批量查询

❌ **不推荐**：逐个查询

```rust
// 低效：N次数据库查询
for refno in refnos {
    let pe = provider.get_pe(refno).await?;
    // 处理 pe
}
```

✅ **推荐**：批量查询

```rust
// 高效：1次数据库查询
let pes = provider.get_pes_batch(&refnos).await?;
for pe in pes {
    // 处理 pe
}
```

### 3. 启用性能日志

```rust
let strategy = QueryStrategy::auto()
    .with_performance_log(true)  // 启用性能日志
    .with_timeout(5000);         // 5秒超时

let router = QueryRouter::new(strategy)?;
```

查看日志输出：

```
[Kuzu] query_by_type 查询耗时: 15ms
[SurrealDB] get_descendants 查询耗时: 245ms
```

### 4. 处理大量数据时的分页

```rust
async fn process_large_dataset(router: &QueryRouter) -> QueryResult<()> {
    let total = router.count_by_type("PIPE", 1112).await?;
    let batch_size = 1000;

    for offset in (0..total).step_by(batch_size) {
        // 分批处理
        let batch = get_batch(offset, batch_size).await?;
        process_batch(batch).await?;
    }

    Ok(())
}
```

### 5. 利用类型系统避免错误

```rust
// 编译时检查，避免运行时错误
fn process_pipes<T: QueryProvider>(provider: &T) -> impl Future<Output = QueryResult<()>> {
    async move {
        let pipes = provider.query_by_type(&["PIPE"], 1112, None).await?;
        // ...
        Ok(())
    }
}
```

### 6. 在 Web API 中使用

```rust
use axum::{extract::State, Json};
use std::sync::Arc;

struct AppState {
    router: Arc<QueryRouter>,
}

async fn get_pipes_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<RefnoEnum>>, StatusCode> {
    match state.router.query_by_type(&["PIPE"], 1112, None).await {
        Ok(pipes) => Ok(Json(pipes)),
        Err(e) => {
            eprintln!("查询失败: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
```

---

## 常见问题

### Q1: 如何判断当前使用的是哪个数据库？

A: 通过 `provider_name()` 方法：

```rust
let name = router.provider_name();
println!("当前使用: {}", name);  // 输出: "QueryRouter"

// 或者检查策略
let strategy = router.get_strategy();
match strategy.engine {
    QueryEngine::SurrealDB => println!("使用 SurrealDB"),
    QueryEngine::Kuzu => println!("使用 Kuzu"),
    QueryEngine::Auto => println!("自动选择模式"),
}
```

### Q2: 回退机制如何工作？

A: 当 `enable_fallback = true` 时：

1. 首先尝试使用选定的引擎（如 Kuzu）
2. 如果查询失败，自动回退到 SurrealDB
3. 记录警告日志
4. 返回结果

```rust
// 启用回退
let router = QueryRouter::new(QueryStrategy {
    engine: QueryEngine::Auto,
    enable_fallback: true,  // 关键配置
    ..Default::default()
})?;
```

### Q3: 性能提升有多少？

A: 根据查询类型不同：

| 查询类型 | 性能提升 |
|---------|---------|
| 单层子节点查询 | 3-5x |
| 深层递归查询（12层） | 10-20x |
| 类型过滤查询 | 3-5x |
| 批量查询 | 5-8x |

### Q4: 如何添加自定义查询方法？

A: 扩展 trait：

```rust
#[async_trait]
pub trait CustomQuery: QueryProvider {
    async fn my_custom_query(&self, param: String) -> QueryResult<Vec<RefnoEnum>> {
        // 默认实现或要求子类实现
        todo!()
    }
}

// 为具体提供者实现
#[async_trait]
impl CustomQuery for SurrealQueryProvider {
    async fn my_custom_query(&self, param: String) -> QueryResult<Vec<RefnoEnum>> {
        // SurrealDB 特定实现
        Ok(vec![])
    }
}
```

### Q5: 如何集成第三方数据库？

A: 实现 `QueryProvider` trait：

```rust
pub struct PostgresQueryProvider {
    // ...
}

#[async_trait]
impl QueryProvider for PostgresQueryProvider {
    async fn get_pe(&self, refno: RefnoEnum) -> QueryResult<Option<PE>> {
        // Postgres 实现
    }

    // 实现其他必需方法...
}
```

### Q6: 测试时如何 Mock？

A: 使用 trait object：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    struct MockProvider;

    #[async_trait]
    impl QueryProvider for MockProvider {
        async fn get_pe(&self, _refno: RefnoEnum) -> QueryResult<Option<PE>> {
            Ok(Some(/* mock data */))
        }
        // ...
    }

    #[tokio::test]
    async fn test_with_mock() {
        let provider = MockProvider;
        let result = provider.get_pe(RefnoEnum::default()).await;
        assert!(result.is_ok());
    }
}
```

### Q7: 如何监控查询性能？

A: 启用性能日志并集成监控系统：

```rust
let router = QueryRouter::new(QueryStrategy {
    enable_performance_log: true,
    ..Default::default()
})?;

// 日志会自动输出到 log 系统
// 可以配合 Prometheus、Grafana 等监控工具
```

---

## 下一步

- ✅ 阅读 [examples/query_provider_demo.rs](../examples/query_provider_demo.rs) 查看完整示例
- ✅ 运行测试: `cargo test test_query_provider`
- ✅ 查看性能对比: `cargo run --example query_provider_demo --features kuzu`
- ✅ 了解 Kuzu 集成: [docs/KUZU_USAGE.md](./KUZU_USAGE.md)

---

**版权信息**
© 2025 AIOS Core Project. All rights reserved.
