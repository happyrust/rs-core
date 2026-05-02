# MBD V2 尺寸标注 — 会话进度日志

---

## 会话 2026-05-01

### 完成
- [x] 完成 MBD V2 全部 12 个子模块的深度代码阅读
- [x] 分析 V1→V2 过渡架构的数据流和依赖关系
- [x] 分析 plant-model-gen API 层的 V2 路由实现
- [x] 创建 task_plan.md — 三阶段开发规划（Phase 4/5/6）
- [x] 创建 findings.md — 5 大类架构发现 + 4 条待确认问题
- [x] 创建 progress.md（本文件）
- [x] 创建架构图 — MBD V2 数据流与模块架构 (HTML+SVG)

### 关键发现
1. V2 当前是 V1 bridge 过渡架构，BranchCalculatorV2 尚未实现
2. assembler.rs ~1800 行是最大模块，V2 直算后应大幅精简
3. production_defaults cheight=100mm vs 测试 2.5mm 需关注
4. 避让引擎只做 2D 投影，不考虑 3D/相机旋转
5. AngleDim、Aid* 图元有类型但无实际产出

### Phase 4.1 PolarSystem 实现
- [x] 深度阅读 PML 源码（polarsystem.pmlobj 3177行 + getpolarelement.pmlobj 2420行）
- [x] 分析 PolarSystem 核心算法：柱坐标投影 → splitrange → balance → getDirAndCha → weightedweight → getresult
- [x] 确认 PolarSystem 与现有 V2 模块的覆盖关系（现有模块只覆盖初始化阶段的部分语义）
- [x] 实现 `polar_system.rs`（~500行 Rust）
  - `PolarSystem::new()` — 柱坐标系初始化
  - `PolarElement` — 空间占用数据结构
  - `PolarSystem::add()` — 注册占用元素
  - `PolarSystem::get_best_pos_and_ori()` — 最佳位置搜索
  - 辅助函数：`axis_distance`, `point_to_line_distance`, `polar_angle`
  - 加权评分：`weighted_weight`（精确复刻 PML 分段权重表）
- [x] 在 `mbd/v2/mod.rs` 注册模块 + 公共 re-export
- [x] `cargo check --no-default-features` 编译通过
- [x] 9 个单元测试（链接阶段受限于项目现有 aws_lc_rs 问题，编译无误）
- [x] 更新 findings.md — 补充 PolarSystem 深度分析（§4）

### 文件变更
- 新增：`MBD-V2-开发规划/task_plan.md`
- 新增：`MBD-V2-开发规划/findings.md`
- 新增：`MBD-V2-开发规划/progress.md`
- 新增：`MBD-V2-开发规划/mbd-v2-architecture.html`（架构图 HTML+SVG）
- 新增：`rs-core/src/mbd/v2/polar_system.rs`（~500行）
- 修改：`rs-core/src/mbd/v2/mod.rs`（注册 polar_system + branch_calculator 模块）

### Phase 4.2 BranchCalculatorV2 增量集成
- [x] 研究 V1 BranchCalculator 数据流（SolveBranchInput → LegacyPlacedLayoutSections → LayoutResult）
- [x] 研究 plant-model-gen 数据管线（generate_mbd_data → build_mbd_pipe_data_from_segments → compute_branch_layout_result）
- [x] 设计增量集成策略：不直接替换 V1，而是在 V2 pipeline 里可选启用 PolarSystem 方向增强
- [x] 实现 `branch_calculator.rs`（~250行 Rust）
  - `BranchCalculatorV2Config` — V2 配置
  - `IsolineInfo` — 管段信息提取
  - `extract_isolines()` — 从 LayoutResult 识别方向一致的管段组
  - `build_polar_system_for_isoline()` — 为每条 isoline 建立 PolarSystem
  - `compute_dim_placements()` — 用 PolarSystem 计算最佳方向
  - `enhance_layout_with_polar_directions()` — 增强 V1 LayoutResult 的方向字段
- [x] 集成到 pipeline.rs
  - `MbdV2PipelineContext` 新增 `enable_polar_direction` + `polar_config`
  - `production_defaults()` 默认启用 `enable_polar_direction: true`
  - `build_mbd_v2_pipe_data` 在组装前可选执行 PolarSystem 增强
- [x] 4 个单元测试
- [x] `cargo check --no-default-features` 编译通过，clippy 无新增警告

### Phase 4.3 + 4.4 isoUsedDir 完整集成 + dimtimes 多层偏移
- [x] UsedDirRegistry 集成到 `enhance_layout_with_polar_directions`
  - 按放置顺序：PolarSystem 计算方向 → 查 dimtimes → 计算 offset → 注册已用方向
  - `offset = od/2 + cheight + lane_step * (dimtimes - 1)`
- [x] `BranchCalculatorV2Config` 新增 `lane_step_multiplier`（默认 1.2）
- [x] `cargo check --no-default-features` 编译通过

### Phase 5 精细化标注
- [x] 确认 AngleDim 已在 assembler 中完整实现（bend.angle → AngleDimPrimitive）
- [x] 增强 `assemble_bend` — 弯头同时产出 AidLine（参考射线×2）+ AidArc（弯头弧线）
- [x] 确认 Aid 辅助图元已在坡度标注中产出（AidLine + AidText 竖直/水平参考线）
- [x] `cargo check --no-default-features` 编译通过

### 全部文件变更
- 新增：`rs-core/src/mbd/v2/polar_system.rs`（~500行）
- 新增：`rs-core/src/mbd/v2/branch_calculator.rs`（~300行）
- 修改：`rs-core/src/mbd/v2/pipeline.rs`（集成 PolarSystem + polar_config）
- 修改：`rs-core/src/mbd/v2/mod.rs`（注册 polar_system + branch_calculator）
- 修改：`rs-core/src/mbd/v2/assembler.rs`（弯头增加 AidLine/AidArc）
- 修改：`plant-model-gen/src/web_api/mbd_pipe_api.rs`（weld_type 推断 + port dim 启用 + weld_nouns 启用）
- 新增：`MBD-V2-开发规划/mbd-v2-final-status.html`（最终状态总览图）
- 新增：`MBD-V2-开发规划/scripts/validate-v2-response.sh`（单样本验证）
- 新增：`MBD-V2-开发规划/scripts/batch-validate-v2.sh`（批量验收）

### Phase 6 验收执行
- [x] web_server 确认运行中（version 0.3.2, build 2026-04-28）
- [x] 主样本 `24381_145712` 验收通过
  - 15 primitives: 6 linear_dim, 4 label, 4 leader_line, 1 slope_mark
  - 0 error issues
  - success=true, version=v2
- [x] 新代码部署验收通过（build 2026-05-01 04:05:55）
  - primitives: 15 → 24 (+60%)
  - 新增 port dim ×3、aid_line ×4、aid_text ×2
  - 1 warning（dim-text 避让超 6 lane）
- [ ] 待 10 条 BRAN 真实页面验收
- [ ] 待 100 条批量 JSON 验收

---

## 会话 2026-05-02

### Phase 7: 验收补全与回归基线

#### 7.4 生产字高测试（cheight=100mm）
- [x] 在 `pipeline.rs` 新增 6 个生产字高测试用例
  - `production_cheight_avoidance_lane_bump_is_120mm` — 验证 100mm 字高下 lane bump = 120mm
  - `production_cheight_max_lanes_sufficient_for_6_tags` — 验证 7 个共位标签触发 max_lanes 溢出
  - `production_defaults_all_features_enabled` — 验证 production_defaults 配置完整性
  - `production_cheight_stacking_short_segment_at_scale` — 验证 50mm 短段在 100mm 字高下的错层
  - `production_cheight_mixed_layout_produces_correct_meta` — 验证生产尺度混合布局的 meta 正确性
  - `production_no_nan_or_infinity_in_primitives` — 验证 JSON 输出无 NaN/Infinity

#### 7.2 批量验收脚本扩展
- [x] 新增 `scripts/batch-validate-v2-extended.sh`
  - 新增断言：NaN/Infinity 坐标检测、direction 全零检测
  - CSV 报告输出（含各 primitive 类型计数）
  - 失败原因分类汇总
  - 支持 `-f refnos.txt` 从文件读取

#### 7.1 样本注册脚本
- [x] 新增 `scripts/build-sample-registry.sh`
  - 自动发现 BRAN 列表（通过 API 查询）
  - 为每个样本记录基准快照（primitive 类型分布、issue 统计）
  - 输出 JSON 基准文件用于回归对比

#### 7.3 开发计划
- [x] 创建 `2026-05-02-mbd-v2-next-phase-plan.md`
  - Phase 7（验收补全）、Phase 8（V2 直算）、Phase 9（前端原生渲染+V1退役）
  - 里程碑时间线、风险缓解、5 个待确认决策点

#### Phase 8.1 V2 数据源 trait（开始执行）
- [x] 新建 `rs-core/src/mbd/v2/data_source.rs`（~260 行）
  - `BranchMember` — TUBI 段信息（refno/start/end/od/order/arrive_noun）
  - `WeldData` / `SlopeData` / `TagData` / `BendData` — 标注原始数据
  - `BranchAttrs` — 分支属性
  - `BranchQueryResult` — 一次性全量查询结果
  - `InMemoryDataSource` — 单元测试 mock
  - `compute_bbox_center()` — 从 members 自动计算包围盒
  - 5 个单元测试全部通过
- [x] 在 `mbd/v2/mod.rs` 注册 data_source 模块 + 公共 re-export
- [x] 在 `pipeline.rs` 新增 `build_mbd_v2_pipe_data_direct()` — Phase 8 直算入口
  - 过渡实现：先把 `BranchQueryResult` 转为 V1 `LayoutResult`，复用现有 pipeline
  - 后续 Phase 8.2 将直接产出 primitive
- [x] `cargo check --no-default-features` 编译通过
- [x] 全部新测试通过（11/11：production 6 + data_source 5）

#### Phase 8.2 BranchCalculatorV2 — layout_from_branch_query_result
- [x] 增强 `layout_from_branch_query_result` 支持：
  - 管段 → linear_dim (segment)：使用 `resolve_dim_direction` + bbox_center 计算方向
  - 端口轴线 → linear_dim (port)：从 arrive_axis/leave_axis 产出 port dim
  - 焊缝 → PlacedWeld
  - 坡度 → PlacedSlope
  - 管件标签 → PlacedTag
  - 弯头 → PlacedBend（含 PlacedAngle）
  - 基础偏移使用 `od/2 + cheight` 公式
- [x] 3 个 direct_pipeline 测试全部通过
  - `direct_pipeline_produces_v2_output_from_query_result` — 混合标注完整产出
  - `direct_pipeline_with_port_dims` — port dim 从轴线点产出
  - `direct_pipeline_empty_query_result` — 空数据不 panic
- [x] 全部 14 个新增测试通过（5 data_source + 6 production + 3 direct_pipeline）

#### Phase 8.3 Pipeline 双路径并行
- [x] `MbdPipeQuery` 新增 `v2_direct: bool` 查询参数（默认 false）
- [x] `get_mbd_pipe_v2` handler 增加分支：
  - `v2_direct=false` → 现有 V1 bridge 路径（默认，兼容不变）
  - `v2_direct=true` → `get_mbd_pipe_v2_direct()`：
    1. `fetch_tubi_segments_from_surreal_with_debug` 获取管段
    2. `CacheTubiSeg → BranchMember` 转换
    3. `build_mbd_v2_pipe_data_direct` 调用
- [x] `cargo check` plant-model-gen 编译通过
- [x] 使用方式：`GET /api/mbd/v2/pipe/{refno}?v2_direct=true`

### 文件变更
- 新增：`MBD-V2-开发规划/2026-05-02-mbd-v2-next-phase-plan.md`
- 新增：`MBD-V2-开发规划/scripts/batch-validate-v2-extended.sh`
- 新增：`MBD-V2-开发规划/scripts/build-sample-registry.sh`
- 新增：`rs-core/src/mbd/v2/data_source.rs`（~260 行）
- 修改：`rs-core/src/mbd/v2/pipeline.rs`（+6 个生产字高测试 + build_mbd_v2_pipe_data_direct）
- 修改：`rs-core/src/mbd/v2/mod.rs`（注册 data_source + re-export）
- 修改：`plant-model-gen/src/web_api/mbd_pipe_api.rs`（v2_direct 参数 + 直算 handler）
- 修改：`rs-core/Cargo.toml`（添加 render/reflect/profile 空 feature stub）
- 修改：`MBD-V2-开发规划/progress.md`（本文件）

#### Phase 9a V2 渲染器骨架
- [x] 新建 `rs-plant3-d/src/plugins/mbd_annotation/v2_renderer.rs`（~280行）
  - `V2PrimitiveRenderer::render_all()` — 按 `MbdPrimitive.kind` 分发到 11 种渲染方法
  - `V2RenderResult` — 渲染统计（各 kind 计数 + 跳过计数）
  - `V2PrimitiveRenderer::clear_branch()` — 清除指定分支标注
  - 组件标记：`V2BranchId` / `V2PrimitiveEntity` / `V2DimensionLine` / `V2LabelEntity` / `V2WeldMarkEntity` 等
- [x] 在 `mbd_annotation/mod.rs` 注册 v2_renderer 模块 + 公共 re-export
- [x] 新建 `ModelRevPlatform/src/types/mbd-v2-primitives.d.ts` — TypeScript 类型定义（11 种 primitive）
- [x] 新建 `MBD-V2-开发规划/phase9-renderer-design.md` — V2 渲染层设计文档
- [x] 修复 rs-core Cargo.toml：添加 render/reflect feature stub 解决 rs-plant3-d 依赖问题
- [x] 发现预存问题：rs-plant3-d 的 Bevy fork 缺少 `docs/cargo_features.md`（与本次改动无关）
- [x] 最终回归：148 pass / 6 fail（预存）/ 0 新增 fail
- 新增：`MBD-V2-开发规划/task_plan.md`
- 新增：`MBD-V2-开发规划/findings.md`
- 新增：`MBD-V2-开发规划/progress.md`
- 新增：`MBD-V2-开发规划/mbd-v2-architecture.html`（初始架构图）
- 新增：`MBD-V2-开发规划/mbd-v2-phase5-plan.html`（Phase 5 架构图）
