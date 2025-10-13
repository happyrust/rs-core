# Kuzu PE + Owner API 实现指南 (external/rs-core)

## 概述

本文档说明在 `external/rs-core` 中需要实现的 API，用于支持 Kuzu PE + Owner 精简写入模式。

## 需要实现的 API

### 1. save_pe_nodes_batch

**文件位置**: `external/rs-core/src/rs_kuzu/operations.rs`

**功能**: 批量保存 PE 节点和 OWNS 关系到 Kuzu 数据库

**签名**:
```rust
#[cfg(feature = "kuzu-pe-owner")]
pub async fn save_pe_nodes_batch(
    pe_batch: &[(SPdmsElement, RefU64)]
) -> anyhow::Result<()>
```

**参数**:
- `pe_batch`: PE 元素和 owner refno 的列表
  - `SPdmsElement`: PE 节点数据（包含 refno, name, noun, dbnum, sesno 等）
  - `RefU64`: owner 的 refno

**返回值**:
- `Ok(())`: 保存成功
- `Err(...)`: 保存失败，包含错误信息

### 2. 辅助方法 (如果需要)

#### get_owner_refno (NamedAttrMap)

**文件位置**: `external/rs-core/src/types/named_attmap.rs`

**功能**: 从 NamedAttrMap 获取 owner 的 RefU64

**签名**:
```rust
impl NamedAttrMap {
    #[inline]
    pub fn get_owner_refno(&self) -> Option<RefU64> {
        self.get_owner().refno_option()
    }
}
```

**说明**:
- `get_owner()` 已存在（返回 `RefnoEnum`）
- 需要添加一个便捷方法转换为 `RefU64`
- 用于 `src/versioned_db/pe_kuzu.rs:219` 行

## 实现细节

### save_pe_nodes_batch 实现

```rust
#[cfg(feature = "kuzu-pe-owner")]
pub async fn save_pe_nodes_batch(
    pe_batch: &[(SPdmsElement, RefU64)]
) -> anyhow::Result<()> {
    use crate::rs_kuzu::create_kuzu_connection;

    // 1. 获取 Kuzu 连接
    let conn = create_kuzu_connection()?;

    // 2. 开启事务
    conn.execute("BEGIN TRANSACTION")?;

    for (pe, owner_refno) in pe_batch {
        // 3. 创建/更新 PE 节点
        let create_pe_sql = format!(
            "MERGE (n:PE {{refno: '{}'}})
             SET n.name = '{}',
                 n.noun = {},
                 n.dbnum = {},
                 n.sesno = {}",
            pe.refno.refno(),  // 使用 refno() 方法获取字符串
            escape_string(&pe.name),  // 转义特殊字符
            pe.noun,
            pe.dbnum.unwrap_or(0),
            pe.sesno
        );

        conn.execute(&create_pe_sql).map_err(|e| {
            anyhow::anyhow!("创建 PE 节点失败 {}: {}", pe.refno.refno(), e)
        })?;

        // 4. 创建 OWNS 关系（如果有 owner）
        if owner_refno.0 != 0 {
            let create_owns_sql = format!(
                "MATCH (owner:PE {{refno: '{}'}}), (child:PE {{refno: '{}'}})
                 MERGE (owner)-[r:OWNS]->(child)",
                owner_refno.refno(),
                pe.refno.refno()
            );

            conn.execute(&create_owns_sql).map_err(|e| {
                anyhow::anyhow!(
                    "创建 OWNS 关系失败 {} -> {}: {}",
                    owner_refno.refno(),
                    pe.refno.refno(),
                    e
                )
            })?;
        }
    }

    // 5. 提交事务
    conn.execute("COMMIT")?;

    Ok(())
}

/// 转义 Cypher 字符串中的特殊字符
fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\")
     .replace('\'', "\\'")
     .replace('"', "\\\"")
     .replace('\n', "\\n")
     .replace('\r', "\\r")
     .replace('\t', "\\t")
}
```

### 关键点说明

1. **事务控制**:
   - 每批数据使用一个事务
   - 确保原子性（要么全部成功，要么全部失败）
   - Kuzu 仅支持单写事务，自动串行化

2. **错误处理**:
   - 使用 `map_err` 转换错误，提供详细上下文
   - 事务失败时自动回滚
   - 返回 `anyhow::Result` 便于错误传播

3. **Cypher 语法**:
   - `MERGE`: 如果节点不存在则创建，存在则更新
   - `MATCH ... MERGE`: 先匹配两个节点，再创建关系
   - `SET`: 设置节点属性

4. **性能优化**:
   - 批量操作减少事务开销
   - 使用索引加速查询（refno 字段）
   - 避免不必要的属性写入

## 数据结构

### SPdmsElement 字段

保存到 Kuzu 的字段：

```rust
pub struct SPdmsElement {
    pub refno: RefnoEnum,      // 参考号（唯一标识）
    pub owner: RefnoEnum,       // 父节点参考号
    pub name: String,           // 名称
    pub noun: String,           // 类型
    pub dbnum: Option<i32>,     // 数据库编号
    pub sesno: i32,             // 会话编号
    // ... 其他字段不保存
}
```

### Kuzu 图结构

```cypher
// PE 节点
(:PE {
    refno: String,    // 唯一标识
    name: String,     // 名称
    noun: String,     // 类型
    dbnum: Int,       // 数据库编号
    sesno: Int        // 会话编号
})

// OWNS 关系
(owner:PE)-[OWNS]->(child:PE)
```

## 测试建议

### 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[cfg(feature = "kuzu-pe-owner")]
    async fn test_save_pe_nodes_batch() {
        // 初始化测试数据库
        let test_db = "./test_kuzu_pe_owner.db";
        init_kuzu(test_db, SystemConfig::default()).await.unwrap();
        init_kuzu_schema().await.unwrap();

        // 创建测试数据
        let pe1 = SPdmsElement {
            refno: RefU64(100_200).into(),
            name: "Test Site".to_string(),
            noun: "SITE".to_string(),
            dbnum: Some(1112),
            sesno: 1,
            ..Default::default()
        };

        let pe2 = SPdmsElement {
            refno: RefU64(100_300).into(),
            name: "Test Zone".to_string(),
            noun: "ZONE".to_string(),
            dbnum: Some(1112),
            sesno: 1,
            ..Default::default()
        };

        let batch = vec![
            (pe1, RefU64(0)),           // 无 owner
            (pe2, RefU64(100_200)),     // owner 是 pe1
        ];

        // 执行保存
        save_pe_nodes_batch(&batch).await.unwrap();

        // 验证数据
        let conn = create_kuzu_connection().unwrap();

        // 验证 PE 节点
        let count_query = "MATCH (n:PE) RETURN count(n) AS count";
        let result = conn.query(count_query).unwrap();
        assert_eq!(result.len(), 2);

        // 验证 OWNS 关系
        let owns_query = "MATCH ()-[r:OWNS]->() RETURN count(r) AS count";
        let result = conn.query(owns_query).unwrap();
        assert_eq!(result.len(), 1);

        // 清理
        std::fs::remove_dir_all(test_db).ok();
    }
}
```

### 集成测试

使用 `examples/test_kuzu_pe_owner_only.rs` 进行端到端测试。

## Feature 配置

### Cargo.toml (external/rs-core)

```toml
[features]
kuzu = []
kuzu-pe-owner = ["kuzu"]
```

### 条件编译

```rust
#[cfg(feature = "kuzu-pe-owner")]
pub async fn save_pe_nodes_batch(...) -> anyhow::Result<()> {
    // 实现
}

#[cfg(not(feature = "kuzu-pe-owner"))]
pub async fn save_pe_nodes_batch(...) -> anyhow::Result<()> {
    Err(anyhow::anyhow!("kuzu-pe-owner feature 未启用"))
}
```

## 性能指标

### 目标性能

- 批量大小: 1000 个 PE 节点/批
- 写入速度: > 1000 节点/秒
- 内存占用: < 100MB (1000 节点批)
- 事务时间: < 1 秒/批

### 优化建议

1. **批量大小调优**:
   - 小数据集: 500-1000
   - 大数据集: 1000-2000
   - 超大数据集: 考虑分批+并行

2. **索引策略**:
   - 导入时禁用索引创建
   - 导入完成后统一创建索引

3. **连接复用**:
   - 复用 Kuzu 连接
   - 避免频繁创建/销毁连接

## 调用流程

```
gen-model (src/versioned_db/pe_kuzu.rs)
    ↓
save_pe() 函数
    ↓
提取 (PE, owner_refno)
    ↓
批量累积到 pe_batch
    ↓
external/rs-core (src/rs_kuzu/operations.rs)
    ↓
save_pe_nodes_batch(&pe_batch)
    ↓
Kuzu Database
```

## 常见问题

### Q1: RefnoEnum vs RefU64?

**A**:
- `RefnoEnum` 是枚举类型，可能包含版本信息
- `RefU64` 是基础类型，直接存储 64 位整数
- 使用 `.refno()` 方法从 `RefnoEnum` 获取 `RefU64`

### Q2: 如何处理重复的 PE 节点？

**A**: 使用 `MERGE` 而不是 `CREATE`，Kuzu 会自动处理：
- 如果 refno 已存在，更新属性
- 如果 refno 不存在，创建新节点

### Q3: OWNS 关系方向？

**A**: `(owner)-[OWNS]->(child)`
- `owner`: 父节点
- `child`: 子节点
- 方向表示"拥有"关系

### Q4: 事务失败如何处理？

**A**:
- 事务自动回滚，不影响数据库状态
- 返回错误给调用方
- 调用方可以重试或记录错误

## 参考资料

- Kuzu 文档: https://kuzudb.com/docs
- Cypher 查询语言: https://neo4j.com/docs/cypher-manual
- 项目文档: `docs/kuzu-pe-owner-only-api-spec.md`

---

**文档版本**: v1.0
**更新日期**: 2025-01-13
**维护者**: gen-model 项目组
