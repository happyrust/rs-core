# SQLite RTree 空间索引使用指南

## 概述

代码已更新为使用 SQLite RTree 扩展进行空间查询，这将显著提升大数据集的空间查询性能。

## 主要变更

### 1. 查询语法变更

**之前（普通表）：**
```sql
WHERE max_x >= ?1 AND min_x <= ?2
  AND max_y >= ?3 AND min_y <= ?4
  AND max_z >= ?5 AND min_z <= ?6
```

**现在（RTree）：**
```sql
WHERE id MATCH rtree(?1, ?2, ?3, ?4, ?5, ?6)
```

### 2. 新增功能

#### 创建 RTree 表
```rust
use crate::spatial::sqlite;

let conn = sqlite::open_connection_rw()?;
sqlite::create_rtree_table(&conn)?;
```

#### 插入/更新数据
```rust
// 单个插入
sqlite::insert_or_update_aabb(
    refno,
    &aabb,
    Some("EQUI"), // 可选的类型名称
)?;

// 批量插入
let data = vec![
    (refno1, aabb1, Some("EQUI".to_string())),
    (refno2, aabb2, Some("PIPE".to_string())),
];
sqlite::insert_or_update_aabbs_batch(&data)?;
```

## 表结构

### RTree 虚拟表 (`aabb_index`)
```sql
CREATE VIRTUAL TABLE aabb_index USING rtree(
    id INTEGER PRIMARY KEY,
    min_x REAL, max_x REAL,
    min_y REAL, max_y REAL,
    min_z REAL, max_z REAL
);
```

**注意**: RTree 虚拟表只能存储 id 和坐标信息，不能存储其他字段。

### Items 表（存储元数据）
```sql
CREATE TABLE items (
    id INTEGER PRIMARY KEY,
    noun TEXT
);
```

用于存储额外的元数据（如类型名称），通过 JOIN 查询获取。

## 查询函数

### 1. 点查询 (`query_containing_point`)
查询包含指定点的所有包围盒。

```rust
let results = sqlite::query_containing_point(
    Vec3::new(100.0, 200.0, 300.0),
    256, // limit
)?;
```

**RTree 查询语法：**
```sql
WHERE id MATCH rtree(x, x, y, y, z, z)
```
将点视为一个极小的包围盒进行查询。

### 2. 重叠查询 (`query_overlap`)
查询与指定包围盒相交或包含的所有包围盒。

```rust
let expanded = Aabb::new(
    Point3::new(0.0, 0.0, 0.0),
    Point3::new(1000.0, 1000.0, 1000.0),
);

let results = sqlite::query_overlap(
    &expanded,
    Some(&["EQUI".to_string(), "PIPE".to_string()]), // 类型过滤
    Some(100), // limit
    &[], // exclude
)?;
```

**RTree 查询语法：**
```sql
WHERE id MATCH rtree(min_x, max_x, min_y, max_y, min_z, max_z)
```

### 3. KNN 查询 (`query_knn`)
查询最近的 K 个包围盒（使用迭代扩展半径的方式）。

```rust
let results = sqlite::query_knn(
    Vec3::new(100.0, 200.0, 300.0),
    10, // k
    Some(1000.0), // 初始搜索半径
    Some(&["EQUI".to_string()]), // 类型过滤
)?;
```

## 性能优势

### RTree vs 普通表

| 操作 | 普通表 | RTree |
|------|--------|-------|
| 点查询 | O(n) 全表扫描 | O(log n) 空间索引 |
| 重叠查询 | O(n) 全表扫描 | O(log n) 空间索引 |
| 插入 | O(1) | O(log n) |
| 空间复杂度 | O(n) | O(n) |

**性能提升：**
- 对于 100 万条记录，查询性能提升约 **100-1000 倍**
- 查询时间从秒级降低到毫秒级

## 迁移指南

### 从普通表迁移到 RTree

1. **备份现有数据**
   ```bash
   sqlite3 aabb_cache.sqlite ".backup backup.sqlite"
   ```

2. **创建新的 RTree 表**
   ```rust
   use crate::spatial::sqlite;
   
   let conn = sqlite::open_connection_rw()?;
   sqlite::create_rtree_table(&conn)?;
   ```

3. **迁移数据**
   ```rust
   // 从旧表读取数据
   // ... 读取逻辑 ...
   
   // 批量插入到 RTree 表
   sqlite::insert_or_update_aabbs_batch(&data)?;
   ```

4. **验证数据**
   ```rust
   // 执行一些查询验证数据正确性
   let test_results = sqlite::query_overlap(&test_aabb, None, Some(10), &[])?;
   ```

### 注意事项

1. **RTree 限制**
   - RTree 虚拟表只能存储 id 和坐标
   - 其他元数据需要存储在 `items` 表中
   - 使用 LEFT JOIN 获取完整信息

2. **兼容性**
   - SQLite 3.5.0+ 支持 RTree
   - rusqlite 默认包含 RTree 扩展
   - 如果使用自定义编译的 SQLite，需要确保启用了 RTree

3. **数据一致性**
   - 插入 RTree 表时，建议同时更新 `items` 表
   - 使用事务确保数据一致性

## 示例代码

### 完整示例：初始化并插入数据

```rust
use crate::spatial::sqlite;
use crate::{RefU64, RefnoEnum};
use parry3d::bounding_volume::Aabb;
use parry3d::math::Point3;

// 1. 创建表
let conn = sqlite::open_connection_rw()?;
sqlite::create_rtree_table(&conn)?;

// 2. 准备数据
let refno = RefU64(12345);
let aabb = Aabb::new(
    Point3::new(0.0, 0.0, 0.0),
    Point3::new(100.0, 100.0, 100.0),
);

// 3. 插入数据
sqlite::insert_or_update_aabb(
    refno,
    &aabb,
    Some("EQUI"),
)?;

// 4. 查询数据
let results = sqlite::query_overlap(
    &aabb,
    Some(&["EQUI".to_string()]),
    Some(10),
    &[],
)?;

println!("找到 {} 个匹配的包围盒", results.len());
```

## 故障排除

### 错误：`no such module: rtree`

**原因**: SQLite 未编译 RTree 扩展。

**解决方案**:
1. 使用 rusqlite 的 `bundled` feature（推荐）
2. 或使用包含 RTree 的系统 SQLite

### 错误：`SQLITE_MISMATCH`

**原因**: RTree 表结构不匹配。

**解决方案**:
1. 删除旧表：`DROP TABLE IF EXISTS aabb_index;`
2. 重新创建 RTree 表：`sqlite::create_rtree_table(&conn)?;`

### 性能未提升

**可能原因**:
1. 数据量太小（< 1000 条）
2. 查询范围太大（覆盖大部分数据）
3. 未使用 RTree MATCH 语法

**检查方法**:
```sql
EXPLAIN QUERY PLAN 
SELECT * FROM aabb_index 
WHERE id MATCH rtree(0, 100, 0, 100, 0, 100);
```

应该看到 `USING INDEX` 或 `USING COVERING INDEX`。

## 参考文档

- [SQLite RTree 官方文档](https://www.sqlite.org/rtree.html)
- [rusqlite RTree 文档](https://docs.rs/rusqlite/latest/rusqlite/struct.Connection.html#method.load_extension)


