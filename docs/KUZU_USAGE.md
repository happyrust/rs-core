# Kuzu 数据库使用指南

## 1. 查询 Kuzu 数据库

### 1.1 基本查询示例

```rust
// 创建连接
let conn = create_kuzu_connection()?;

// 查询所有表
let mut result = conn.query("CALL table_info() RETURN *;")?;

// 查询 ELBO 表数据
let mut result = conn.query("MATCH (e:Attr_ELBO) RETURN e LIMIT 10;")?;

// 查询记录数
let mut result = conn.query("MATCH (e:Attr_ELBO) RETURN COUNT(*) as count;")?;
```

### 1.2 运行查询示例

```bash
# 运行查询演示
cargo run --example kuzu_query_demo --features kuzu
```

## 2. Kuzu 可视化工具

### 2.1 Kuzu Explorer (官方 Web UI)

Kuzu Explorer 是官方提供的 Web 界面，可以可视化查看和查询数据库。

#### 安装和启动

```bash
# 1. 安装 Kuzu Explorer
npm install -g kuzu-explorer

# 2. 启动 Explorer（指向你的数据库路径）
kuzu-explorer --database ./data/kuzu_db

# 3. 在浏览器打开
# 默认地址: http://localhost:8000
```

#### 使用 Docker 运行

```bash
# 使用 Docker 运行 Kuzu Explorer
docker run -p 8000:8000 \
  -v $(pwd)/data/kuzu_db:/database \
  kuzudb/explorer:latest
```

### 2.2 Kuzu CLI (命令行客户端)

```bash
# 安装 Kuzu CLI
pip install kuzu

# 连接到数据库
kuzu ./data/kuzu_db

# 在 CLI 中执行查询
kuzu> CALL table_info() RETURN *;
kuzu> MATCH (e:Attr_ELBO) RETURN e LIMIT 5;
kuzu> :quit
```

### 2.3 Python 客户端

```python
import kuzu

# 创建数据库连接
db = kuzu.Database('./data/kuzu_db')
conn = kuzu.Connection(db)

# 执行查询
result = conn.execute("CALL table_info() RETURN *;")
while result.has_next():
    print(result.get_next())

# 查询 ELBO 表
result = conn.execute("MATCH (e:Attr_ELBO) RETURN e.refno, e.NAME LIMIT 10;")
while result.has_next():
    row = result.get_next()
    print(f"RefNo: {row[0]}, Name: {row[1]}")
```

## 3. 常用查询命令

### 3.1 元数据查询

```cypher
-- 查看所有表
CALL table_info() RETURN *;

-- 查看节点表
CALL table_info() WHERE type = 'NODE' RETURN name;

-- 查看关系表
CALL table_info() WHERE type = 'REL' RETURN name;
```

### 3.2 数据查询

```cypher
-- 查询 ELBO 表的前10条记录
MATCH (e:Attr_ELBO)
RETURN e
LIMIT 10;

-- 查询特定字段
MATCH (e:Attr_ELBO)
RETURN e.refno, e.NAME, e.STATUS_CODE
LIMIT 10;

-- 条件查询
MATCH (e:Attr_ELBO)
WHERE e.STATUS_CODE = 'ACTIVE'
RETURN e.refno, e.NAME;

-- 统计查询
MATCH (e:Attr_ELBO)
RETURN COUNT(*) as total_count;
```

### 3.3 关系查询

```cypher
-- 查询 PE 到 ELBO 的关系
MATCH (p:PE)-[:TO_ELBO]->(e:Attr_ELBO)
RETURN p.refno, p.name, e.STATUS_CODE
LIMIT 10;

-- 查询层次关系
MATCH (parent:PE)-[:OWNS]->(child:PE)
WHERE parent.noun = 'SITE'
RETURN parent.name, child.name, child.noun;

-- 多跳查询
MATCH path = (p:PE)-[:OWNS*1..3]->(c:PE)
WHERE p.noun = 'ZONE'
RETURN path;
```

## 4. 数据库文件结构

Kuzu 数据库文件存储在指定目录下：

```
data/kuzu_db/
├── catalog/          # 元数据目录
│   ├── metadata.db   # 表结构定义
│   └── ...
├── data/            # 数据文件
│   ├── nodes/       # 节点数据
│   └── rels/        # 关系数据
└── wal/             # Write-Ahead Log
```

## 5. 性能优化建议

1. **批量导入**: 使用 COPY 命令批量导入数据
2. **索引**: Kuzu 自动为主键创建索引
3. **查询优化**: 使用 EXPLAIN 查看查询计划

```cypher
-- 查看查询计划
EXPLAIN MATCH (e:Attr_ELBO) WHERE e.NAME = 'ELBO-001' RETURN e;
```

## 6. 故障排查

### 常见问题

1. **表不存在错误**
   - 确保已运行 `init_kuzu_schema()` 初始化 schema
   - 检查数据库路径是否正确

2. **连接失败**
   - 检查数据库文件权限
   - 确保路径存在且可写

3. **查询超时**
   - 检查查询复杂度
   - 考虑添加 LIMIT 限制返回结果数

### 调试命令

```rust
// 列出所有表
let tables = list_tables().await?;

// 验证 schema
validate_schema().await?;

// 重新初始化（慎用）
reinit_schema().await?;
```

## 7. 集成示例

### 在项目中使用

```rust
use aios_core::rs_kuzu::*;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化 Kuzu
    init_kuzu_database("./data/kuzu_db").await?;

    // 初始化 schema
    init_kuzu_schema().await?;

    // 查询数据
    let conn = create_kuzu_connection()?;
    let mut result = conn.query("MATCH (e:Attr_ELBO) RETURN COUNT(*)")?;

    if let Some(row) = result.next() {
        println!("ELBO 记录数: {}", row.get(0).unwrap().to_string());
    }

    Ok(())
}
```

## 8. 参考资源

- [Kuzu 官方文档](https://kuzudb.com/docs/)
- [Kuzu GitHub](https://github.com/kuzudb/kuzu)
- [Kuzu Explorer](https://github.com/kuzudb/kuzu-explorer)
- [Cypher 查询语言](https://neo4j.com/docs/cypher-manual/)