# 数据库相关函数使用说明

本文档说明了新添加的数据库相关函数的使用方法。

## SurrealDB 函数

### fn::get_world($dbnum: number)

通过数据库编号获取对应的 WORLD 参考号。

**参数：**
- `$dbnum`: 数据库编号 (number)

**返回值：**
- 成功时返回 WORLD 的参考号 (record)
- 失败时返回 none

**示例：**
```surql
SELECT fn::get_world(1112) AS world_refno;
```

### fn::query_sites_of_db($world_refno: record)

查询指定 WORLD 下的所有 SITE 节点。

**参数：**
- `$world_refno`: WORLD 节点的参考号 (record)

**返回值：**
- 成功时返回 SITE 节点的参考号数组 (array<record>)
- 失败时返回空数组 []

**示例：**
```surql
SELECT fn::query_sites_of_db(pe:123_456) AS site_refnos;
```

## Rust API

### BasicQueryService::get_world_by_dbnum(dbnum: u32)

通过数据库编号获取对应的 WORLD 参考号。

**参数：**
- `dbnum`: 数据库编号 (u32)

**返回值：**
- `Result<Option<RefnoEnum>>`: 成功时返回 WORLD 的参考号，失败时返回错误

**示例：**
```rust
use aios_core::rs_surreal::queries::BasicQueryService;

let dbnum = 1112u32;
let world_refno = BasicQueryService::get_world_by_dbnum(dbnum).await?;
if let Some(world) = world_refno {
    println!("找到 WORLD: {:?}", world);
}
```

### BasicQueryService::query_sites_of_world(world_refno: RefnoEnum)

查询指定 WORLD 下的所有 SITE 节点。

**参数：**
- `world_refno`: WORLD 节点的参考号 (RefnoEnum)

**返回值：**
- `Result<Vec<RefnoEnum>>`: 成功时返回 SITE 节点的参考号列表，失败时返回错误

**示例：**
```rust
use aios_core::rs_surreal::queries::BasicQueryService;
use aios_core::types::RefnoEnum;

let world_refno = RefnoEnum::from("123_456");
let sites = BasicQueryService::query_sites_of_world(world_refno).await?;
println!("找到 {} 个 SITE 节点", sites.len());
```

## 组合使用示例

```rust
use aios_core::rs_surreal::queries::BasicQueryService;

async fn load_sites_by_dbnum(dbnum: u32) -> anyhow::Result<Vec<RefnoEnum>> {
    // 1. 通过数据库编号获取 WORLD
    let world_refno = BasicQueryService::get_world_by_dbnum(dbnum).await?;

    if let Some(world) = world_refno {
        // 2. 查询 WORLD 下的所有 SITE
        let sites = BasicQueryService::query_sites_of_world(world).await?;
        Ok(sites)
    } else {
        Ok(vec![])
    }
}
```

## 部署说明

1. 首先在 SurrealDB 中执行函数定义：
   ```bash
   surreal sql --conn http://localhost:8000 --user root --pass root --ns test --db test < db.surql
   ```

2. 然后在 Rust 代码中使用对应的 API 方法。

## 测试

使用 `test_db_functions.surql` 脚本来测试函数是否正常工作：

```bash
surreal sql --conn http://localhost:8000 --user root --pass root --ns test --db test < test_db_functions.surql
```
