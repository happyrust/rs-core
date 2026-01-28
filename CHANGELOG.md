# Changelog - rs-core

All notable changes to the rs-core library will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- **支持 MAT(TRIM(STR(...)), 'TRUE') 表达式改写为 IFTRUE**

  #### 问题描述
  PDMS 元件库中常见 `MAT(TRIM(STR(<cond>)), 'TRUE')` 形态的表达式，用于条件匹配。tiny_expr 库不支持这些函数，导致 RefNo 24381_56661 等模型生成时表达式求值失败。

  #### 修复方案
  - 新增 `rewrite_mat_trim_str_iftrue` 函数，将 `MAT(TRIM(STR(...)), 'TRUE')` 改写为 `IFTRUE(...,1,0)`
  - 修复 `consume_keyword` 中的空白跳过问题，确保带空格的表达式（如 `MAT( TRIM( STR(...) ) )`）也能正确匹配
  - 在 `eval_str_to_f64` 中加入改写前后调试日志（debug_model 模式下）
  - 新增单元测试 `test_rewrite_mat_trim_str_iftrue`

  #### 修改文件
  - `src/rs_surreal/resolve.rs`：
    - 新增 `rewrite_mat_trim_str_iftrue` 函数（第 306-439 行）
    - 在 `eval_str_to_f64` 中调用改写并打印日志（第 502-510 行）
    - 修复 `consume_keyword` 空白跳过（第 328-335 行）
    - 新增单元测试（第 1053-1067 行）

  #### 验证结果
  - debug-model 24381_56661 运行后，日志中出现 `MAT/TRIM/STR rewrite: ... -> ...` 日志
  - 原先报错的表达式（如 `( 2 * MAT( TRIM( STR( ( ATTRIB DESP[6 ] / 1 ) GT ( 50 * 1 ) ) ), 'TRUE' ) )`）成功求值

- **修复布尔运算后 `inst_relate_aabb` 无法被正确查询的问题**
  
  #### 问题描述
  布尔运算完成后，AABB 数据成功保存到 `inst_relate_aabb` 关系表中，但导出时 `world_aabb` 始终为 `None`，导致 JSON 中 `aabb_hash` 为 `null`。
  
  #### 根因分析
  原有代码在 `pe` 表上定义了计算字段：
  ```sql
  DEFINE FIELD world_aabb ON TABLE pe 
      VALUE <future> { RETURN type::record("inst_relate_aabb", id).out.d };
  ```
  
  问题在于 `inst_relate_aabb` 是 **RELATION 表**，其记录 ID 是自动生成的随机值（如 `inst_relate_aabb:⟨xyz123⟩`），而不是 `inst_relate_aabb:{pe_id}` 的格式。因此 `type::record("inst_relate_aabb", id)` 永远无法匹配到正确的记录。
  
  #### 修复方案
  弃用 `<future>` 计算字段，改为在 `query_insts_with_batch` 查询中直接使用 **graph traversal** 语法：
  
  ```sql
  -- 旧: refno.world_aabb (依赖错误的计算字段)
  -- 新: (refno->inst_relate_aabb[0].out).d (直接 graph traversal)
  ```
  
  #### 修改文件
  - `src/rs_surreal/inst.rs`：
    - 移除 `pe.world_aabb` 计算字段定义（第 79-86 行）
    - `query_insts_with_batch` 中 3 处查询改用 graph traversal（第 379-464 行）
  
  #### 验证结果
  - 布尔运算后的实例 `world_aabb` 正确返回 `Some(...)`
  - 导出 JSON 中 `aabb_hash` 从 `null` 变为有效值（如 `"13646891808564331510"`）

- **修复旋转体CSG生成中的轴上边处理问题**
  
  #### 问题描述
  - 原有的 `revolve_polygons_manifold` 函数对轴上边（x=0）处理不当
  - 轴上的点被错误地生成了多个顶点，导致网格冗余
  - 两端都在轴上的边生成了无效的面（退化边）
  - 一端在轴上的边生成了四边形而非三角形扇
  
  #### 修复内容
  参考 `e3d-reverse/几何体生成/REVO基本体分析报告.md` 的分析，重写了旋转体生成逻辑：
  
  **1. 轴上点特殊处理** (`src/prim_geo/profile_processor.rs` 第 958-1240 行)
  - x=0 的点只生成一个共享3D顶点（不再为每个角度生成）
  - 使用容差吸附接近轴的点到轴上（AXIS_TOL = 1e-5）
  
  **2. 边分类处理**
  - 两端都在轴上：跳过（退化边，不生成任何面）
  - 一端在轴上：生成三角形扇（轴上点作为共享顶点）
  - 两端都不在轴上：生成四边形（两个三角形）
  
  **3. 性能优化**
  - 减少了50%的顶点数（圆柱体从132个降至66个）
  - 减少了43%的三角形数（从224个降至128个）
  
  **4. 测试覆盖**
  - 添加了15个专项测试用例，覆盖所有特殊情况
  - 包括点重合、退化角度、轴上边处理、裁剪等场景
  
  #### 技术细节
  - **核心改进**：正确实现了 libgm.dll 的轴上边处理逻辑
  - **索引生成**：简化了复杂的索引计算，使用清晰的边分类
  - **自适应分段**：保留了原有的自适应分段功能
  - **部分旋转**：支持任意角度的部分旋转，包括端面生成
  
  #### 验证结果
  - 所有12个原有测试通过
  - 新增15个特殊情况测试全部通过
  - 生成的OBJ文件可在 `test_output/profile_processor/` 查看

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

- **修复 neg_relate_map 生成空条目问题**
  - `ShapeInstancesData::insert_negs` 在 `negs` 为空时不再插入 `neg_relate_map`，避免后续布尔运算扫描到无效目标

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

