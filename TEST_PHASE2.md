# Phase 2 测试指南

## 测试状态

### 已创建的测试文件
1. ✅ `src/test/test_db_adapter/mod.rs` - 测试模块入口
2. ✅ `src/test/test_db_adapter/test_surreal_adapter.rs` - SurrealAdapter 单元测试 (8个测试)
3. ✅ `src/test/test_db_adapter/test_kuzu_adapter.rs` - KuzuAdapter 单元测试 (8个测试)
4. ✅ `src/test/test_db_adapter/test_hybrid_manager.rs` - HybridDatabaseManager 单元测试 (11个测试)
5. ✅ `tests/db_adapter_integration_test.rs` - 集成测试 (5个测试)

### 当前修复问题
正在修复以下编译问题：
1. ✅ 缺少 `SPdmsElement` 类型导入 - 已通过在 `types/mod.rs` 中添加 `pub use pe::*;` 解决
2. 🔧 `SurrealAdapter::new()` 不接受参数 - 正在修复
3. 🔧 部分方法需要 `Option<QueryContext>` 而非 `QueryContext` - 正在修复
4. 🔧 `HybridDatabaseManager::new()` 不需要 name 参数 - 正在修复

### API 变更说明

#### Adapter 构造函数
```rust
// 正确用法
let surreal = SurrealAdapter::new();  // 无参数，名称固定为 "SurrealDB"
let kuzu = KuzuAdapter::new();        // 无参数，名称固定为 "KuzuDB"

// HybridDatabaseManager 自动生成名称
let manager = HybridDatabaseManager::new(
    Arc::new(surreal),
    Some(Arc::new(kuzu)),
    config  // 无需提供 name
);
// 名称自动生成为: "Hybrid<SurrealDB,KuzuDB>"
```

#### 查询上下文参数
某些方法接受 `Option<QueryContext>` 而非直接的 `QueryContext`:
```rust
// query_children, get_attmap 等方法
manager.query_children(refno, Some(ctx)).await?;  // 需要 Some()
manager.query_children(refno, None).await?;       // 或 None

// 其他方法仍然直接接受 QueryContext
manager.get_pe(refno, ctx).await?;
```

## 下一步

1. 完成编译错误修复
2. 运行不带 kuzu feature 的测试：`cargo test test_db_adapter --lib`
3. 运行带 kuzu feature 的测试：`cargo test test_db_adapter --lib --features kuzu`
4. 运行集成测试：`cargo test --test db_adapter_integration_test`
5. 生成测试报告

## 测试覆盖范围

### SurrealAdapter 测试
- ✅ 适配器名称验证
- ✅ 数据库能力检查
- ✅ 健康检查
- ✅ PE 数据获取
- ✅ 子元素查询
- ✅ 所有者查询
- ✅ 属性映射获取
- ✅ 按名称查询

### KuzuAdapter 测试 (需要 kuzu feature)
- ✅ 适配器名称验证
- ✅ 数据库能力检查
- ✅ 健康检查
- ✅ PE 数据获取
- ✅ 子元素查询
- ✅ 最短路径查询（图遍历）
- ✅ 子树查询（图遍历）
- ✅ 属性映射获取

### HybridDatabaseManager 测试
- ✅ 管理器名称验证
- ✅ 综合能力检查
- ✅ 健康检查
- ✅ PE 数据获取（单库模式）
- ✅ 子元素查询（单库模式）
- ✅ 属性映射获取（单库模式）
- ✅ 双库回退机制（需要 kuzu feature）
- ✅ 图查询智能路由（需要 kuzu feature）
- ✅ 所有混合模式验证（需要 kuzu feature）

### 集成测试
- ✅ SurrealAdapter 完整工作流
- ✅ KuzuAdapter 完整工作流（需要 kuzu feature）
- ✅ 单库混合管理器工作流
- ✅ 双库混合管理器工作流（需要 kuzu feature）
- ✅ 所有混合模式测试（需要 kuzu feature）

## 预期测试结果

### 不带 kuzu feature
- 应通过约 15 个测试（SurrealAdapter + HybridDatabaseManager单库模式）

### 带 kuzu feature
- 应通过约 32 个测试（所有适配器 + 所有混合模式）

## 注意事项

1. **数据库初始化**: 测试假设 SurrealDB 全局实例已初始化，Kuzu 则在测试中初始化
2. **测试数据**: 测试使用 refno=1 的虚拟数据，实际测试时请确保测试数据存在
3. **编译时间**: 首次编译 kuzu feature 可能需要 5-10 分钟
4. **并发**: 某些测试可能因数据库锁而需要串行运行