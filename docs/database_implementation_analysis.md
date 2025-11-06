# 数据库实现架构分析

## 概述

本项目采用**多层抽象架构**设计，提供统一的数据库访问接口，目前主要基于 **SurrealDB** 实现。整体架构从高到低分为以下几个层次：

1. **应用层接口** (`aios_db_mgr`) - 提供 PDMS 领域特定的数据访问接口
2. **统一查询接口** (`query_provider`) - 提供跨数据库的统一查询抽象
3. **数据库适配器** (`db_adapter`) - 定义数据库能力抽象
4. **SurrealDB 实现层** (`rs_surreal`) - SurrealDB 的具体实现和查询函数

---

## 架构层次

### 1. 应用层接口 (`aios_db_mgr`)

**位置**: `src/aios_db_mgr/`

**核心组件**:
- `PdmsDataInterface` trait - 定义 PDMS 系统的数据访问接口
- `AiosDBMgr` - 实现该 trait，提供完整的数据库管理功能

**主要功能**:
```rust
// 核心接口定义
pub trait PdmsDataInterface {
    async fn get_world(&self, mdb_name: &str) -> Result<Option<PdmsElement>>;
    async fn get_pdms_element(&self, refno: RefU64) -> Result<Option<PdmsElement>>;
    async fn get_attr(&self, refno: RefU64) -> Result<NamedAttrMap>;
    async fn get_children(&self, refno: RefU64) -> Result<Vec<EleTreeNode>>;
    // ... 更多方法
}
```

**特点**:
- 领域特定：针对 PDMS（Plant Design Management System）系统设计
- 异步优先：所有操作都是异步的
- 支持多数据库：可连接主数据库和副机组数据库

---

### 2. 统一查询接口 (`query_provider`)

**位置**: `src/query_provider/`

**设计理念**:
- **统一接口**: 所有数据库实现相同的 trait
- **类型安全**: 使用 Rust 类型系统确保查询正确性
- **异步优先**: 全面支持 async/await
- **可扩展**: 易于添加新的数据库实现

**核心 Trait 分层**:

#### 2.1 HierarchyQuery - 层级查询
```rust
pub trait HierarchyQuery {
    async fn get_children(&self, refno: RefnoEnum) -> QueryResult<Vec<RefnoEnum>>;
    async fn get_descendants(&self, refno: RefnoEnum, max_depth: Option<usize>) -> QueryResult<Vec<RefnoEnum>>;
    async fn get_ancestors(&self, refno: RefnoEnum) -> QueryResult<Vec<RefnoEnum>>;
    async fn get_ancestors_of_type(&self, refno: RefnoEnum, nouns: &[&str]) -> QueryResult<Vec<RefnoEnum>>;
    async fn get_descendants_filtered(&self, refno: RefnoEnum, nouns: &[&str], max_depth: Option<usize>) -> QueryResult<Vec<RefnoEnum>>;
    async fn get_children_pes(&self, refno: RefnoEnum) -> QueryResult<Vec<PE>>;
}
```

#### 2.2 TypeQuery - 类型过滤查询
```rust
pub trait TypeQuery {
    async fn query_by_type(&self, nouns: &[&str], dbnum: i32, has_children: Option<bool>) -> QueryResult<Vec<RefnoEnum>>;
    async fn query_by_type_name_contains(&self, nouns: &[&str], dbnum: i32, keyword: &str, case_sensitive: bool) -> QueryResult<Vec<RefnoEnum>>;
    async fn query_by_type_multi_db(&self, nouns: &[&str], dbnums: &[i32]) -> QueryResult<Vec<RefnoEnum>>;
    async fn get_world(&self, dbnum: i32) -> QueryResult<Option<RefnoEnum>>;
    async fn get_sites(&self, dbnum: i32) -> QueryResult<Vec<RefnoEnum>>;
    async fn count_by_type(&self, noun: &str, dbnum: i32) -> QueryResult<usize>;
}
```

#### 2.3 BatchQuery - 批量查询
```rust
pub trait BatchQuery {
    async fn get_pes_batch(&self, refnos: &[RefnoEnum]) -> QueryResult<Vec<PE>>;
    async fn get_attmaps_batch(&self, refnos: &[RefnoEnum]) -> QueryResult<Vec<NamedAttMap>>;
    async fn get_full_names_batch(&self, refnos: &[RefnoEnum]) -> QueryResult<Vec<(RefnoEnum, String)>>;
}
```

#### 2.4 GraphQuery - 图遍历查询
```rust
pub trait GraphQuery {
    async fn query_multi_descendants(&self, refnos: &[RefnoEnum], nouns: &[&str]) -> QueryResult<Vec<RefnoEnum>>;
    async fn find_shortest_path(&self, from: RefnoEnum, to: RefnoEnum) -> QueryResult<Vec<RefnoEnum>>;
    async fn get_node_depth(&self, refno: RefnoEnum) -> QueryResult<usize>;
}
```

#### 2.5 QueryProvider - 统一接口
```rust
pub trait QueryProvider: HierarchyQuery + TypeQuery + BatchQuery + GraphQuery + Send + Sync {
    async fn get_pe(&self, refno: RefnoEnum) -> QueryResult<Option<PE>>;
    async fn get_attmap(&self, refno: RefnoEnum) -> QueryResult<Option<NamedAttMap>>;
    async fn exists(&self, refno: RefnoEnum) -> QueryResult<bool>;
    fn provider_name(&self) -> &str;
    async fn health_check(&self) -> QueryResult<bool>;
}
```

**查询路由器 (`QueryRouter`)**:
- 提供智能查询路由
- 支持回退机制（Fallback）
- 性能监控和日志记录
- 支持多种查询策略

---

### 3. 数据库适配器 (`db_adapter`)

**位置**: `src/db_adapter/`

**核心概念**:

#### 3.1 DatabaseCapabilities - 数据库能力标识
```rust
pub struct DatabaseCapabilities {
    pub supports_graph_traversal: bool,  // 图遍历
    pub supports_transactions: bool,      // 事务
    pub supports_versioning: bool,        // 版本控制
    pub supports_live_queries: bool,       // 实时查询
    pub supports_full_text_search: bool,   // 全文搜索
    pub supports_vector_index: bool,      // 向量索引
}
```

#### 3.2 DatabaseAdapter - 适配器接口
```rust
pub trait DatabaseAdapter {
    fn name(&self) -> &str;
    fn capabilities(&self) -> DatabaseCapabilities;
    async fn health_check(&self) -> Result<bool>;
    
    // PE 操作
    async fn get_pe(&self, refno: RefnoEnum, ctx: Option<QueryContext>) -> Result<Option<SPdmsElement>>;
    async fn save_pe(&self, pe: &SPdmsElement) -> Result<()>;
    async fn delete_pe(&self, refno: RefnoEnum) -> Result<()>;
    
    // 属性操作
    async fn get_attmap(&self, refno: RefnoEnum, ctx: Option<QueryContext>) -> Result<NamedAttrMap>;
    async fn save_attmap(&self, refno: RefnoEnum, attmap: &NamedAttrMap) -> Result<()>;
    
    // 关系操作
    async fn create_relation(&self, from: RefnoEnum, to: RefnoEnum, rel_type: &str) -> Result<()>;
    async fn query_related(&self, refno: RefnoEnum, rel_type: &str, ctx: Option<QueryContext>) -> Result<Vec<RefnoEnum>>;
    
    // 图遍历（可选）
    async fn shortest_path(&self, from: RefnoEnum, to: RefnoEnum, ctx: Option<QueryContext>) -> Result<Vec<RefnoEnum>>;
}
```

**特点**:
- 定义统一的数据库操作接口
- 支持查询上下文（超时、优先级等）
- 目前仅实现了 SurrealDB 适配器

---

### 4. SurrealDB 实现层 (`rs_surreal`)

**位置**: `src/rs_surreal/`

**核心组件**:

#### 4.1 全局数据库连接
```rust
pub static SUL_DB: Lazy<Surreal<Any>> = Lazy::new(Surreal::init);
pub static SECOND_SUL_DB: Lazy<Surreal<Any>> = Lazy::new(Surreal::init);  // 副机组
pub static KV_DB: Lazy<Surreal<Any>> = Lazy::new(Surreal::init);          // KV存储
#[cfg(feature = "mem-kv-save")]
pub static SUL_MEM_DB: Lazy<Surreal<Any>> = Lazy::new(Surreal::init);     // 内存KV（PE备份）
```

#### 4.2 连接初始化
```rust
pub async fn init_surreal() -> Result<()> {
    // 1. 加载配置
    let db_option: DbOption = load_from_config();
    
    // 2. 连接数据库
    SUL_DB.connect(db_option.get_version_db_conn_str())
        .with_capacity(1000)
        .await?;
    
    // 3. 设置命名空间和数据库
    SUL_DB.use_ns(&db_option.surreal_ns)
        .use_db(&db_option.project_name)
        .await?;
    
    // 4. 认证
    SUL_DB.signin(Root { username, password }).await?;
    
    // 5. 定义通用函数
    define_common_functions(...).await?;
    
    Ok(())
}
```

#### 4.3 查询模块 (`query.rs`)

**主要功能**:
- `get_pe(refno)` - 获取单个 PE 元素（带缓存）
- `query_ancestor_refnos(refno)` - 查询祖先节点
- `get_children_refnos(refno)` - 获取子节点
- `get_named_attmap(refno)` - 获取属性映射
- 批量查询操作

**缓存策略**:
- 使用 `#[cached]` 宏进行函数级缓存
- 常见查询结果自动缓存
- 减少重复数据库访问

#### 4.4 MDB 查询模块 (`mdb.rs`)

**核心功能**:
- `query_mdb_db_nums(mdb, module)` - 查询 MDB 的数据库编号列表
- `query_type_refnos_by_dbnum(nouns, dbnum, has_children)` - 按类型和数据库编号查询
- `query_type_refnos_by_dbnum_with_filter(...)` - 带名称过滤的类型查询
- `get_world_refno(mdb)` - 获取世界节点参考号
- `get_site_pes_by_dbnum(dbnum)` - 获取 SITE 节点列表

**查询优化**:
- 使用单表查询替代多表循环查询
- 支持 `has_children` 过滤减少不必要数据
- 支持名称模糊匹配（大小写敏感/不敏感）

#### 4.5 图查询模块 (`graph.rs`)

**功能**:
- 深层子节点查询（递归）
- 祖先节点查询
- 类型过滤的祖先/子孙查询
- 多起点查询

**SurrealDB 限制**:
- 最大递归深度为 12 层
- 深层查询性能需注意

#### 4.6 SurrealQueryProvider 实现

**位置**: `src/query_provider/surreal_provider.rs`

**实现方式**:
- 将 `QueryProvider` trait 的方法委托给 `rs_surreal` 模块的函数
- 提供统一的错误处理和日志记录
- 支持健康检查

---

## 数据库连接与配置

### 配置文件

**位置**: `DbOption.toml`, `DbOption_ABA.toml`, `DbOption_AMS.toml`

**配置结构**:
```toml
[surreal]
ns = "namespace"
project_name = "database_name"
version_db_conn_str = "ws://localhost:8000"
v_user = "root"
v_password = "root"

[mdb]
mdb_name = "/651YK"

[mesh_precision]
# 网格精度配置
```

### 连接初始化流程

1. **加载配置**: 从 `DbOption.toml` 读取配置
2. **建立连接**: 使用 WebSocket 或 RocksDB 后端连接
3. **设置命名空间**: `use_ns()` 和 `use_db()`
4. **认证**: `signin()` 使用 Root 用户
5. **定义函数**: 加载 SurrealQL 函数定义（`.surql` 文件）

### 支持的连接方式

- **WebSocket** (`ws` 特性): 连接远程 SurrealDB 服务器
- **RocksDB** (`local` 特性): 本地文件数据库
- **内存 KV** (`mem-kv-save` 特性): 内存数据库用于 PE 备份

---

## 数据模型

### PE (Plant Element) 结构

```rust
pub struct SPdmsElement {
    pub refno: RefnoEnum,      // 参考号（唯一标识）
    pub owner: RefnoEnum,      // 父节点参考号
    pub name: String,          // 名称
    pub noun: String,          // 类型（如 "PIPE", "ELBO", "ZONE"）
    pub children: Vec<RefnoEnum>, // 子节点列表
    pub deleted: bool,         // 是否已删除
    // ... 更多字段
}
```

### RefnoEnum - 参考号类型

```rust
pub enum RefnoEnum {
    RefNo(RefNo),        // 标准参考号 (dbnum, refno)
    RefU64(RefU64),      // U64 参考号
    // ...
}
```

### 属性映射 (NamedAttrMap)

```rust
pub struct NamedAttrMap {
    pub map: IndexMap<String, SurlValue>,  // 属性名 -> 值
}
```

---

## 查询性能优化

### 1. 缓存策略

- **函数级缓存**: 使用 `#[cached]` 宏
- **查询结果缓存**: 常见查询自动缓存
- **缓存失效**: 基于函数参数的缓存键

### 2. 批量操作

- `get_pes_batch()` - 批量获取 PE
- `get_attmaps_batch()` - 批量获取属性
- `query_children_batch()` - 批量查询子节点

### 3. 查询优化

- **单表查询**: 使用 `pe` 表替代多表循环
- **索引利用**: 利用 SurrealDB 的索引
- **过滤前置**: 在数据库层过滤，减少数据传输

### 4. 连接池

- 使用 `with_capacity(1000)` 设置连接池大小
- 支持多数据库连接（主库、副机组）

---

## 同步机制

**位置**: `src/sync/`

**组件**:
- `sync_manager.rs` - 同步管理器
- `sync_task.rs` - 同步任务定义
- `sync_strategy.rs` - 同步策略
- `batch_optimizer.rs` - 批量优化器
- `concurrent_executor.rs` - 并发执行器
- `cache_layer.rs` - 缓存层
- `performance_monitor.rs` - 性能监控

---

## 错误处理

### QueryError 类型

```rust
pub enum QueryError {
    NotFound(String),
    ExecutionError(String),
    Timeout(String),
    InvalidInput(String),
    DatabaseError(String),
}
```

### 错误传播

- 使用 `anyhow::Result` 进行错误传播
- 统一错误格式转换
- 提供详细的错误信息

---

## 特性标志 (Features)

- `local` - 使用本地 RocksDB 后端
- `ws` - 使用 WebSocket 连接远程数据库
- `mem-kv-save` - 启用内存 KV 数据库备份
- `sql` - 启用 MySQL 连接池支持
- `sea-orm` - 启用 SeaORM 支持
- `live` - 启用实时查询
- `test` - 测试相关功能

---

## 使用示例

### 基础查询

```rust
use aios_core::query_provider::{QueryProvider, SurrealQueryProvider};

async fn example() -> Result<()> {
    // 创建查询提供者
    let provider = SurrealQueryProvider::new()?;
    
    // 查询子节点
    let children = provider.get_children(refno).await?;
    
    // 按类型查询
    let pipes = provider.query_by_type(&["PIPE"], 1112, None).await?;
    
    // 批量查询
    let pes = provider.get_pes_batch(&refnos).await?;
    
    Ok(())
}
```

### 使用查询路由器

```rust
use aios_core::query_provider::{QueryRouter, QueryStrategy};

async fn example() -> Result<()> {
    // 创建带自动回退的路由器
    let router = QueryRouter::auto()?;
    
    // 执行查询（自动选择最佳提供者）
    let result = router.get_children(refno).await?;
    
    Ok(())
}
```

### 使用 AiosDBMgr

```rust
use aios_core::aios_db_mgr::AiosDBMgr;

async fn example() -> Result<()> {
    let mgr = AiosDBMgr::init_from_db_option().await?;
    
    // 获取 PE
    let pe = mgr.get_pdms_element(refno).await?;
    
    // 获取属性
    let attrs = mgr.get_attr(refno).await?;
    
    Ok(())
}
```

---

## 总结

### 优势

1. **清晰的层次结构**: 从应用层到实现层，职责分明
2. **类型安全**: 充分利用 Rust 类型系统
3. **统一接口**: 易于扩展新的数据库后端
4. **性能优化**: 缓存、批量操作、查询优化
5. **异步支持**: 全面支持 async/await

### 改进方向

1. **批量查询优化**: 当前批量查询是循环单个查询，可以优化为真正的批量查询
2. **缓存管理**: 需要更精细的缓存失效策略
3. **错误处理**: 可以进一步细化错误类型
4. **文档完善**: 增加更多使用示例和最佳实践
5. **性能监控**: 增强性能监控和诊断能力

### 架构建议

1. **考虑引入查询构建器**: 提供类型安全的查询构建
2. **统一日志系统**: 规范化日志输出
3. **指标收集**: 添加 Prometheus 等指标收集
4. **测试覆盖**: 增加集成测试和性能测试

---

**文档生成时间**: 2024年
**最后更新**: 基于当前代码库分析

