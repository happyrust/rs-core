# Kuzu 启用指南

> **目标**: 启用 Kuzu 图数据库以获得 5-20x 的查询性能提升
> **当前状态**: ⏸️ Kuzu 代码已实现，等待数据库初始化
> **预计收益**: 深度查询 10-20x，批量查询 5-8x，类型查询 3-5x

---

## 📋 前置条件检查

### 1. 代码准备状态

| 检查项 | 状态 | 说明 |
|-------|------|------|
| Kuzu 依赖 | ✅ 已配置 | `Cargo.toml` 中 `kuzu = "0.11.2"` |
| Feature Gate | ✅ 已配置 | `features = ["kuzu"]` |
| KuzuQueryProvider | ✅ 已实现 | `src/query_provider/kuzu_provider.rs` |
| QueryRouter 集成 | ✅ 已实现 | 支持 Auto 模式自动切换 |
| 测试用例 | ✅ 已准备 | `tests/kuzu_integration_test.rs` |

**结论**: ✅ 代码层面已完全准备就绪

### 2. 数据库准备状态

| 检查项 | 状态 | 说明 |
|-------|------|------|
| Kuzu 数据库文件 | ❌ 未初始化 | 需要从 SurrealDB 导入数据 |
| 数据导入脚本 | ⚠️ 待确认 | 需要检查是否有现成脚本 |
| 数据库路径配置 | ⚠️ 待配置 | 默认路径或自定义路径 |

**结论**: ❌ 数据库需要初始化才能使用

---

## 🚀 启用步骤

### 步骤 1: 准备 Kuzu 数据库

#### 选项 A: 从现有备份恢复（推荐）

```bash
# 如果已有 Kuzu 数据库备份
cp -r /path/to/kuzu_db_backup ~/kuzu_db

# 或者从服务器下载
rsync -avz server:/data/kuzu_db ~/kuzu_db
```

#### 选项 B: 从 SurrealDB 导入数据

```bash
# 1. 运行数据导出脚本（假设存在）
cargo run --example export_to_kuzu --features kuzu

# 2. 或使用 Python 脚本
python scripts/export_surreal_to_kuzu.py \
  --surreal-db surrealdb://localhost:8000 \
  --kuzu-db ~/kuzu_db \
  --dbnum 1112
```

#### 选项 C: 创建测试数据库

```rust
// 用于测试的最小数据集
use aios_core::rs_kuzu::*;
use kuzu::SystemConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化 Kuzu
    init_kuzu("./test_kuzu_db", SystemConfig::default())
        .expect("Failed to init Kuzu");

    // 创建测试数据
    let conn = get_kuzu_connection()?;

    // 创建节点表
    conn.query("CREATE NODE TABLE IF NOT EXISTS PE(refno STRING, noun STRING, name STRING, PRIMARY KEY(refno))")?;

    // 创建关系表
    conn.query("CREATE REL TABLE IF NOT EXISTS PE_OWNER(FROM PE TO PE)")?;

    // 插入测试数据
    conn.query("CREATE (p:PE {refno: '17496_170578', noun: 'PIPE', name: 'PIPE-001'})")?;
    conn.query("CREATE (c:PE {refno: '17496_171101', noun: 'ELBO', name: 'ELBO-001'})")?;
    conn.query("MATCH (p:PE {refno: '17496_170578'}), (c:PE {refno: '17496_171101'}) CREATE (p)-[:PE_OWNER]->(c)")?;

    println!("✅ 测试数据库创建成功");
    Ok(())
}
```

### 步骤 2: 配置数据库路径

编辑配置文件或环境变量：

```toml
# DbOption.toml
[kuzu]
database_path = "~/kuzu_db"  # 或 "./data/kuzu_db"
buffer_pool_size = 1073741824  # 1GB
max_threads = 4
```

或使用环境变量：

```bash
export KUZU_DB_PATH="$HOME/kuzu_db"
export KUZU_BUFFER_SIZE=1073741824
export KUZU_MAX_THREADS=4
```

### 步骤 3: 编译启用 Kuzu

```bash
# 编译（首次编译会较慢，约 5-10 分钟）
cargo build --features kuzu

# 编译测试
cargo test --features kuzu

# 编译示例
cargo build --example test_unified_query --features kuzu
```

### 步骤 4: 验证 Kuzu 功能

```bash
# 运行集成测试
cargo test --features kuzu kuzu_integration_test

# 运行查询演示
cargo run --example kuzu_query_demo --features kuzu

# 运行统一查询测试
cargo run --example test_unified_query --features kuzu
```

### 步骤 5: 性能对比测试

创建性能测试脚本：

```bash
#!/bin/bash
# benchmark_kuzu.sh

echo "===== 性能对比测试 ====="

# 1. SurrealDB 基准测试
echo "📊 测试 SurrealDB 性能..."
cargo run --example test_unified_query 2>&1 | grep "查询耗时" > surreal_perf.log

# 2. Kuzu 性能测试
echo "📊 测试 Kuzu 性能..."
QUERY_ENGINE=kuzu cargo run --example test_unified_query --features kuzu 2>&1 | grep "查询耗时" > kuzu_perf.log

# 3. 对比结果
echo "📈 性能对比结果:"
paste surreal_perf.log kuzu_perf.log | awk '{print $1, $2, "vs", $4, $5}'
```

---

## 📊 预期性能提升

### 基准对比（基于设计目标）

| 查询类型 | SurrealDB | Kuzu (预期) | 提升倍数 | 测试场景 |
|---------|-----------|------------|---------|----------|
| **单层子节点** | ~50ms | ~10-15ms | **3-5x** | 查询 PIPE 的直接子节点 |
| **深度查询 (3层)** | 2006ms | ~100-200ms | **10-20x** | 查询 ZONE 的 3 层后代 |
| **深度查询 (12层)** | ~10s | ~500ms-1s | **10-20x** | 完整层级遍历 |
| **类型过滤** | 832ms | ~150-280ms | **3-5x** | 查询 PIPE 类型元素 |
| **批量查询** | ~100ms | ~12-20ms | **5-8x** | 批量获取 PE 信息 |
| **图遍历** | ~500ms | ~50-100ms | **5-10x** | 祖先/后代路径查询 |

### 实测数据（待验证）

运行测试后填写：

```
┌──────────────────┬──────────────┬──────────────┬──────────┐
│ 查询类型         │ SurrealDB    │ Kuzu         │ 实际提升 │
├──────────────────┼──────────────┼──────────────┼──────────┤
│ 单层子节点       │ _____ ms     │ _____ ms     │ ____ x   │
│ 深度查询 (3层)   │ _____ ms     │ _____ ms     │ ____ x   │
│ 类型过滤         │ _____ ms     │ _____ ms     │ ____ x   │
│ 批量查询         │ _____ ms     │ _____ ms     │ ____ x   │
└──────────────────┴──────────────┴──────────────┴──────────┘
```

---

## 🔧 使用方式

### 方式 1: 显式选择 Kuzu

```rust
use aios_core::query_provider::*;

// 创建 Kuzu Provider
let provider = KuzuQueryProvider::new()?;

// 执行查询
let pipes = provider.query_by_type(&["PIPE"], 1112, None).await?;
```

### 方式 2: 使用 Auto 模式（推荐）

```rust
use aios_core::query_provider::*;

// Auto 模式自动优先使用 Kuzu
let router = QueryRouter::auto()?;

// 如果 Kuzu 可用，自动使用 Kuzu
// 如果 Kuzu 失败，自动回退到 SurrealDB
let pipes = router.query_by_type(&["PIPE"], 1112, None).await?;
```

### 方式 3: 强制使用 Kuzu

```rust
let router = QueryRouter::new(QueryStrategy::kuzu_only())?;
let pipes = router.query_by_type(&["PIPE"], 1112, None).await?;
```

### 方式 4: 自定义策略

```rust
let router = QueryRouter::new(QueryStrategy {
    engine: QueryEngine::Auto,
    enable_fallback: true,
    timeout_ms: Some(5000),
    enable_performance_log: true,
})?;

// 运行时切换策略
router.set_strategy(QueryStrategy::kuzu_only());
```

---

## 🐛 故障排查

### 问题 1: Kuzu 编译失败

**症状**:
```
error: failed to compile `kuzu` ...
```

**解决方案**:
```bash
# 更新 Rust toolchain
rustup update

# 清理重新编译
cargo clean
cargo build --features kuzu
```

### 问题 2: 数据库连接失败

**症状**:
```
ConnectionError: Failed to connect to Kuzu database
```

**解决方案**:
```bash
# 检查数据库路径
ls -la ~/kuzu_db

# 检查权限
chmod -R 755 ~/kuzu_db

# 验证数据库完整性
cargo run --example kuzu_query_demo --features kuzu
```

### 问题 3: 查询返回空结果

**症状**:
```
query_by_type 返回空数组，但 SurrealDB 有数据
```

**解决方案**:
```bash
# 1. 检查 Kuzu 数据是否导入
cargo run --example kuzu_query_demo --features kuzu

# 2. 检查节点表是否存在
# 在 Kuzu CLI 中执行:
# CALL table_info() RETURN *;

# 3. 检查数据同步状态
# 对比 SurrealDB 和 Kuzu 的记录数
```

### 问题 4: 性能未提升

**症状**:
```
Kuzu 查询速度与 SurrealDB 相近
```

**排查步骤**:
1. 确认实际使用了 Kuzu（检查日志）
2. 检查 Kuzu 缓存配置（buffer_pool_size）
3. 验证查询是否命中索引
4. 查看 Kuzu 的查询计划

---

## 📈 监控和调优

### 启用性能日志

```rust
let router = QueryRouter::new(QueryStrategy {
    enable_performance_log: true,
    ..Default::default()
})?;

// 日志输出示例:
// [Kuzu] query_by_type 查询耗时: 145ms
// [SurrealDB] query_by_type 查询耗时: 832ms
```

### 查看统计信息

```rust
use aios_core::rs_kuzu::*;

let conn = get_kuzu_connection()?;
let stats = conn.get_stats();

println!("总查询数: {}", stats.total_queries);
println!("失败查询数: {}", stats.failed_queries);
println!("平均查询时间: {}ms", stats.avg_query_time_ms);
```

### 优化建议

1. **Buffer Pool 大小**
   - 默认: 1GB
   - 推荐: 系统内存的 50-70%
   - 设置: `SystemConfig::default().with_buffer_pool_size(4 * 1024 * 1024 * 1024)`

2. **线程数**
   - 默认: CPU 核心数
   - 推荐: CPU 核心数 - 1
   - 设置: `SystemConfig::default().with_max_threads(7)`

3. **查询优化**
   - 使用索引字段（refno, noun）
   - 避免全表扫描
   - 限制递归深度

---

## 🎯 成功验证标准

完成以下所有项即表示 Kuzu 已成功启用：

- [ ] Kuzu 数据库文件存在且可访问
- [ ] `cargo test --features kuzu` 所有测试通过
- [ ] `cargo run --example test_unified_query --features kuzu` 运行成功
- [ ] 查询返回正确的结果（与 SurrealDB 一致）
- [ ] 性能日志显示 Kuzu 被使用
- [ ] 深度查询速度提升 >5x
- [ ] Auto 模式下 Kuzu 优先被选择
- [ ] Kuzu 失败时自动回退到 SurrealDB

---

## 📝 下一步行动

### 立即行动（必需）

1. **初始化 Kuzu 数据库**
   - 确认是否有现成的数据库备份
   - 或运行数据导入脚本
   - 验证数据完整性

2. **运行验证测试**
   ```bash
   cargo test --features kuzu
   cargo run --example test_unified_query --features kuzu
   ```

3. **记录性能数据**
   - 运行基准测试
   - 对比 SurrealDB vs Kuzu
   - 更新性能报告

### 后续优化（可选）

1. **缓存优化**
   - 实现查询结果缓存
   - LRU 淘汰策略
   - 缓存预热

2. **连接池管理**
   - Kuzu 连接池
   - 连接复用
   - 负载均衡

3. **监控集成**
   - Prometheus 指标
   - Grafana 仪表板
   - 告警规则

---

## 📚 参考文档

- [Kuzu 官方文档](https://kuzudb.com/docs)
- [统一查询接口使用指南](docs/QUERY_PROVIDER_GUIDE.md)
- [实现总结文档](QUERY_PROVIDER_IMPLEMENTATION.md)
- [测试报告](TEST_REPORT.md)
- [Kuzu 使用指南](docs/KUZU_USAGE.md)

---

## 🙋 常见问题

**Q: Kuzu 编译需要多长时间？**
A: 首次编译约 5-10 分钟，取决于机器性能。

**Q: Kuzu 数据库有多大？**
A: 取决于数据量，通常是 SurrealDB 的 50-80%。

**Q: 能否同时使用 SurrealDB 和 Kuzu？**
A: 可以！QueryRouter 的 Auto 模式会自动选择最优引擎。

**Q: Kuzu 数据如何同步更新？**
A: 需要定期重新导入或实现增量同步机制。

**Q: 是否必须启用 Kuzu？**
A: 不是必需的。SurrealDB 完全可用，Kuzu 只是性能优化选项。

---

**文档版本**: v1.0
**创建日期**: 2025-10-08
**状态**: ⏸️ 等待数据库初始化

---

© 2025 AIOS Core Project
