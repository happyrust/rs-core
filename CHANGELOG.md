# Changelog - rs-core

All notable changes to the rs-core library will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- **修复 SurrealDB 查询中 `fn::ses_date()` 和 `in.id != none` 导致的 "Expected any, got record" 错误**
  
  #### 问题描述
  - 在 SurrealDB 查询中使用 `fn::ses_date()` 函数和 `in.id != none` 条件会触发记录存在性检查
  - 这些检查在只读事务中执行嵌套查询，导致类型不匹配错误
  - 错误信息：`Internal error: Expected any, got record`
  
  #### 修复内容
  
  **1. 跳过历史版本查询（临时方案）**
  - `src/rs_surreal/query.rs` (第 748-765 行)
    - 修改 `get_children_refnos` 函数
    - 当 `!refno.is_latest()` 时直接返回空数组，避免调用 `fn::ses_date()`
  
  - `src/rs_surreal/queries/hierarchy.rs` (第 141-177 行)
    - 修改 `HierarchyQueryService::get_children_refnos` 函数
    - 跳过历史版本查询，仅处理最新版本
  
  **2. 使用 `dt` 字段替代 `fn::ses_date(in.id)`**
  - `src/rs_surreal/inst.rs` (第 55-67, 83-93, 271-301 行)
    - `query_tubi_insts` 函数：使用 `in.dt` 替代 `fn::ses_date(in.id)`
    - `query_tubi_insts_by_flow` 函数：使用 `in.dt` 替代 `fn::ses_date(in.id)`
    - `query_insts_by_zone` 函数：使用 `in.dt` 替代 `fn::ses_date(in.id)`
  
  #### 技术细节
  - **根本原因**：`fn::ses_date()` 函数内部使用 `record::exists()` 和嵌套 `SELECT` 查询
  - **为什么会失败**：SurrealDB 在只读事务中无法执行某些元数据查询操作
  - **解决方案**：使用已有的 `dt` 字段，避免函数调用和嵌套查询
  - **优点**：简单、高效、无事务问题
  - **限制**：临时方案不支持历史版本查询，需要后续改进
  
  #### 相关文档
  - 详细分析见：`docs/QUERY_INSTS_TRANSACTION_ERROR_ANALYSIS.md`

### Changed
- **优化 SurrealDB 查询性能**
  - 使用直接字段访问替代函数调用，减少数据库负载
  - 简化查询逻辑，提升查询效率

### TODO
- [ ] 实现支持历史版本的查询方案（使用 `dt` 字段）
- [ ] 确保所有 `inst_relate` 记录都正确设置了 `dt` 字段
- [ ] 考虑在数据库层面添加 `dt` 字段的索引

## [Previous Changes]
See git history for previous changes.

