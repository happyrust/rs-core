# Phase 1 测试指南

## 🧪 测试 Kuzu 集成基础功能

### 环境要求

1. **系统依赖**
   - CMake (用于编译 Kuzu C++ 库)
   - C++ 编译器 (GCC 或 Clang)
   - 足够的磁盘空间 (~500MB for Kuzu build)

2. **Rust 环境**
   - Rust nightly toolchain
   - 项目已配置的依赖

### 快速测试

#### 1. 编译检查（不需要运行数据库）

```bash
# 检查代码是否能编译（需要较长时间，首次约 5-10 分钟）
cargo check --features kuzu

# 如果遇到超时，可以增加超时时间
CARGO_BUILD_JOBS=4 cargo check --features kuzu
```

#### 2. 运行单元测试（不需要实际数据库）

```bash
# 测试连接配置
cargo test --features kuzu test_connection_config --lib

# 测试连接统计
cargo test --features kuzu test_connection_stats --lib

# 测试类型转换
cargo test --features kuzu test_attr_to_kuzu_value --lib
cargo test --features kuzu test_kuzu_to_attr --lib
cargo test --features kuzu test_vec3_round_trip --lib
cargo test --features kuzu test_array_conversion --lib
```

#### 3. 运行集成测试（需要初始化实际数据库）

```bash
# 完整工作流测试
cargo test --features kuzu test_kuzu_full_workflow -- --nocapture

# 数据库初始化测试
cargo test --features kuzu test_kuzu_init -- --nocapture

# 模式初始化测试
cargo test --features kuzu test_schema_initialization -- --nocapture
```

### 详细测试步骤

#### Step 1: 验证 Kuzu 库已编译

```bash
# 查找 Kuzu 库文件
find target -name "libkuzu*" 2>/dev/null

# 应该看到类似输出：
# target/debug/deps/libkuzu-xxx.rlib
# target/debug/deps/libkuzu-xxx.rmeta
# target/debug/build/kuzu-xxx/out/libkuzu_rs.a
```

✅ 如果看到上述文件，说明 Kuzu 已成功编译

#### Step 2: 运行配置测试

```bash
cargo test --features kuzu test_connection_config --lib -- --nocapture
```

**期望输出**:
```
running 1 test
✓ 连接配置测试通过
test rs_kuzu::connection::tests::test_connection_config ... ok
```

#### Step 3: 运行类型转换测试

```bash
cargo test --features kuzu test_attr_to_kuzu_value --lib -- --nocapture
```

**期望输出**:
```
running 1 test
✓ 整数类型转换成功
✓ 字符串类型转换成功
✓ 浮点类型转换成功
...
test rs_kuzu::types::tests::test_attr_to_kuzu_value ... ok
```

#### Step 4: 运行完整工作流测试

```bash
cargo test --features kuzu test_kuzu_full_workflow -- --nocapture
```

**期望输出**:
```
running 1 test
✓ 步骤 1: 数据库初始化成功
✓ 步骤 2: 连接获取成功
✓ 步骤 3: 模式初始化成功
✓ 步骤 4: 模式验证成功
✓ 步骤 5: 统计查询成功
  PE 节点数: 0
  属性节点数: 0

🎉 Kuzu 完整工作流测试成功！
test kuzu_tests::test_kuzu_full_workflow ... ok
```

### 手动测试示例

创建一个测试文件 `test_kuzu_manual.rs`:

```rust
use aios_core::rs_kuzu::*;
use kuzu::SystemConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🚀 开始测试 Kuzu 集成...\n");

    // 1. 初始化数据库
    println!("📦 Step 1: 初始化数据库");
    init_kuzu("./manual_test_db", SystemConfig::default()).await?;
    println!("   ✅ 数据库初始化成功\n");

    // 2. 检查状态
    println!("🔍 Step 2: 检查数据库状态");
    let is_init = is_kuzu_initialized();
    println!("   ✅ 数据库已初始化: {}\n", is_init);

    // 3. 获取连接
    println!("🔗 Step 3: 获取数据库连接");
    let conn = get_kuzu_connection()?;
    println!("   ✅ 连接获取成功\n");

    // 4. 初始化模式
    println!("🏗️  Step 4: 初始化图模式");
    init_kuzu_schema().await?;
    println!("   ✅ 图模式初始化成功\n");

    // 5. 验证模式
    println!("✔️  Step 5: 验证模式");
    let schema_init = is_schema_initialized().await?;
    println!("   ✅ 模式已初始化: {}\n", schema_init);

    // 6. 查询统计
    println!("📊 Step 6: 查询统计信息");
    let stats = SchemaStats::query().await?;
    println!("   PE 节点数: {}", stats.pe_count);
    println!("   属性节点数: {}", stats.attribute_count);
    println!("   UDA 节点数: {}", stats.uda_count);
    println!("   ✅ 统计查询成功\n");

    println!("🎉 所有测试通过！Kuzu 集成工作正常。");

    Ok(())
}
```

运行：
```bash
cargo run --features kuzu --example test_kuzu_manual
```

### 性能测试

#### 连接性能测试

```bash
cargo test --features kuzu bench_connection --release -- --nocapture
```

#### 类型转换性能测试

```bash
cargo test --features kuzu bench_type_conversion --release -- --nocapture
```

### 故障排查

#### 问题 1: 编译超时

**解决方案**:
```bash
# 减少并行任务数
CARGO_BUILD_JOBS=2 cargo build --features kuzu

# 或者使用 release 模式（更快）
cargo build --features kuzu --release
```

#### 问题 2: CMake 未找到

**解决方案**:
```bash
# macOS
brew install cmake

# Ubuntu/Debian
sudo apt-get install cmake

# Windows
# 从 https://cmake.org/download/ 下载安装
```

#### 问题 3: C++ 编译器错误

**解决方案**:
```bash
# macOS
xcode-select --install

# Ubuntu/Debian
sudo apt-get install build-essential

# Windows
# 安装 Visual Studio Build Tools
```

#### 问题 4: 测试数据库文件冲突

**解决方案**:
```bash
# 清理测试数据
rm -rf ./test_data/*
rm -rf ./manual_test_db

# 重新运行测试
cargo test --features kuzu
```

### 测试覆盖率

| 模块 | 测试类型 | 状态 |
|------|---------|------|
| `connection.rs` | 单元测试 | ✅ |
| `schema.rs` | 集成测试 | ✅ |
| `types.rs` | 单元测试 | ✅ |
| `queries/*` | 占位实现 | 🚧 Phase 2 |
| `operations/*` | 占位实现 | 🚧 Phase 2 |

### 下一步

完成 Phase 1 测试后，可以继续：
1. Phase 2: 实现数据库适配器
2. Phase 2: 实现混合数据库管理器
3. Phase 2: 实现完整的查询和操作功能

---

**提示**: 如果所有测试都通过，说明 Phase 1 基础设施已成功搭建！ 🎉