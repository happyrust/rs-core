# query_noun_hierarchy 测试案例使用指南

## 概述

`query_noun_hierarchy` 函数是 rs-core 项目中用于查询名词层级关系的核心函数。本测试案例演示了该函数的所有使用方式，包括基础查询、指定父节点查询、组合查询和多父节点查询。

## 测试案例文件

- **文件位置**: `examples/test_noun_hierarchy.rs`
- **运行方式**: `cargo run --example test_noun_hierarchy`

## 功能测试

### 1. 基础查询

- **功能**: 根据名词类型和名称过滤查询
- **示例**: 查询名称包含 'B1' 的 PIPE 类型记录
- **参数**: `query_noun_hierarchy("PIPE", Some("B1"), None)`
- **用途**: 获取特定类型的所有组件，支持名称模糊匹配

### 2. 指定父节点查询

- **功能**: 查询特定父节点下的子节点
- **示例**: 查询父节点 `21900/1040` 下的所有 PIPE 类型记录
- **参数**: `query_noun_hierarchy("PIPE", None, Some(vec![parent_refno]))`
- **用途**: 获取特定设备或区域下的直接子组件

### 3. 组合查询

- **功能**: 指定父节点 + 名称过滤
- **示例**: 查询父节点 `21900/1040` 下名称包含 'B1' 的记录
- **参数**: `query_noun_hierarchy("PIPE", Some("B1"), Some(vec![parent_refno]))`
- **用途**: 精确定位某个父节点下的特定子节点

### 4. 多父节点查询

- **功能**: 同时查询多个父节点下的子节点
- **示例**: 查询多个父节点下的所有 EQUIPMENT 记录
- **参数**: `query_noun_hierarchy("EQUIPMENT", None, Some(vec![parent1, parent2]))`
- **用途**: 批量查询多个设备或区域下的组件，提高查询效率

## 使用建议

### 获取真实的父节点ID

在实际使用中，可以通过以下方式获取真实的父节点ID：

1. 先运行基础查询获取一些节点ID
2. 使用已知的设备或区域ID作为父节点
3. 从其他查询结果中获取父节点ID
4. 查看数据库中的现有层级关系

### RefnoEnum 的创建方式

```rust
// 从字符串创建
let parent_refno = RefnoEnum::from("21900/1040");

// 从 RefU64 创建
let refno = RefU64::from_two_nums(21900, 1040);
let parent_refno = RefnoEnum::Refno(refno);

// 使用 pe_key! 宏
let parent_refno = pe_key!("21900/1040");
```

### 常见使用场景

1. **设备管理**: 查询特定设备下的所有管道和阀门
2. **区域分析**: 获取某个区域内的所有设备类型
3. **批量查询**: 一次性查询多个系统的组件
4. **精确定位**: 在复杂层级中查找特定组件

## 调试提示

### 查询返回空结果

- 检查父节点ID是否真实存在
- 验证数据库连接和权限
- 查看控制台输出的SQL语句

### 性能优化

- 多父节点查询比多次单独查询更高效
- 合理使用名称过滤减少结果集大小
- 避免过于宽泛的查询条件

### 错误处理

- 函数返回 `anyhow::Result<Vec<NounHierarchyItem>>`
- 检查错误信息了解具体失败原因
- 使用 `?` 操作符或 `match` 语句处理结果

## 函数签名

```rust
pub async fn query_noun_hierarchy(
    noun: &str,                           // 名词类型（如 "PIPE", "EQUIPMENT"）
    name_filter: Option<&str>,            // 名称过滤器（支持模糊匹配）
    parent_refnos: Option<Vec<RefnoEnum>>, // 父节点参考号列表
) -> anyhow::Result<Vec<NounHierarchyItem>>
```

## 返回数据结构

```rust
pub struct NounHierarchyItem {
    pub name: String,                     // 组件名称
    pub id: RefnoEnum,                    // 组件ID
    pub noun: String,                     // 名词类型
    pub owner_name: Option<String>,       // 所有者名称
    pub owner: Option<RefnoEnum>,         // 所有者ID
    pub last_modified_date: Option<Datetime>, // 最后修改时间
}
```

## 相关文档

- [数据库架构文档](../database/)
- [查询函数API文档](../api/)
- [RefnoEnum 类型说明](../types/refno.rs)

## 更新日志

- **2025-11-13**: 添加指定父节点查询功能的完整测试案例
- **2025-11-13**: 完善文档和使用说明
- **2025-11-13**: 添加多父节点查询和组合查询示例
