# SurrealDB 内存数据库测试迁移

## 概述

已成功将 SurrealDB RefnoEnum 测试从依赖外部数据库迁移到内存数据库，确保测试的独立性和可重复性。

## 主要改动

### 1. 新增内存数据库初始化函数

**文件**: `src/test/test_surreal/test_helpers.rs`

```rust
/// 初始化内存测试数据库
pub async fn init_memory_test_surreal() -> Result<()> {
    // 创建配置，启用 AST 格式
    let config = Config::default().ast_payload();

    // 连接到内存数据库
    SUL_DB
        .connect(("memory", config))
        .with_capacity(10) // 测试使用较小的连接池
        .await?;

    // 设置测试命名空间和数据库
    SUL_DB.use_ns("test").use_db("test").await?;

    // 以 Root 用户身份登录
    SUL_DB.signin(Root {
        username: "root",
        password: "root",
    }).await?;

    // 定义常用函数
    let _result = define_common_functions().await;

    Ok(())
}

/// 清理内存测试数据库
pub async fn cleanup_memory_test_surreal() -> Result<()> {
    // 删除所有测试表
    let tables = ["pe", "wosl", "site", "zone", "equi", "pipe"];
    
    for table in &tables {
        let _ = SUL_DB.query(&format!("REMOVE TABLE {}", table)).await;
    }
    
    Ok(())
}
```

### 2. 更新测试助手

将 `Test::new()` 方法从使用外部数据库改为使用内存数据库：

```rust
pub async fn new(sql: &str) -> Result<Self> {
    // 初始化内存测试数据库环境
    init_memory_test_surreal().await?;
    
    // 执行 SQL 查询
    let response = SUL_DB.query(sql).await?;
    
    Ok(Self { response })
}
```

### 3. 批量替换测试初始化

**文件**: `src/test/test_surreal/test_refno_enum.rs`

将所有测试中的：
```rust
crate::init_test_surreal().await;
```

替换为：
```rust
init_memory_test_surreal().await;
```

### 4. 添加测试清理

在每个测试结尾添加清理函数，确保测试之间的独立性：
```rust
// 清理测试数据
cleanup_memory_test_surreal().await?;
```

### 5. 新增专门的内存数据库测试

**文件**: `src/test/test_surreal/test_memory_db.rs`

包含以下测试：
- `test_memory_database_init()` - 验证内存数据库初始化
- `test_memory_database_pe_operations()` - 测试 PE 表操作
- `test_memory_database_refno_enum_direct()` - 测试直接 RefnoEnum 反序列化
- `test_memory_database_clean()` - 测试数据库清理功能

## 优势

### 1. 测试独立性
- 不依赖外部数据库的服务状态
- 每个测试都有独立的数据库环境
- 避免测试间的数据污染

### 2. 测试速度
- 内存数据库启动速度更快
- 网络开销为零
- 配置简化，无需读取外部配置文件

### 3. 测试可靠性
- 消除外部依赖带来的不确定性
- 测试结果更加稳定和可重现
- 便于 CI/CD 环境集成

### 4. 资源管理
- 自动清理机制
- 连接池大小限制（测试环境使用较小的池）
- 内存使用可控

## 使用示例

```rust
#[tokio::test]
async fn test_refno_enum_from_query() -> anyhow::Result<()> {
    // 初始化内存数据库
    let _db = init_memory_test_surreal().await;
    
    // 执行查询测试
    let sql = "SELECT VALUE 'pe:17496_123456'";
    let mut test = Test::new(sql).await?;
    
    // 验证结果
    let pe_id: String = test.response.take(0)?;
    assert_eq!(pe_id, "pe:17496_123456");
    
    // 清理
    cleanup_memory_test_surreal().await?;
    
    Ok(())
}
```

## 注意事项

1. **全局数据库实例**: 仍然使用全局的 `SUL_DB` 实例，但在每个测试前重新连接到内存数据库
2. **函数定义**: 保留 `define_common_functions()` 调用以支持必要的数据类型和函数
3. **权限设置**: 使用默认的 root/root 凭据，适合测试环境
4. **清理策略**: 采用表级别的清理而非数据库重建，更加高效

## 验证

可以通过以下命令验证内存数据库测试：
```bash
cargo test test_memory_database_init
cargo test test_memory_database_pe_operations  
cargo test test_memory_database_refno_enum_direct
```

## 向后兼容性

- 保留了原有的 `crate::init_test_surreal()` 函数
- 业务代码中的数据库初始化逻辑不变
- 仅影响测试环境的数据库连接方式

这次迁移确保了 SurrealDB RefnoEnum 测试的独立性、可靠性和性能，同时保持了与现有代码库的兼容性。
