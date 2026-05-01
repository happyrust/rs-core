# MBD V2 Phase 6 验收计划

> 生成时间: 2026-05-02 · 基于深度代码分析

## 总体状态

| 阶段 | 状态 | 说明 |
|---|---|---|
| Phase 1-3: 基础框架 | ✅ 完成 | 11 primitives, assembler, pipeline, avoidance, text_measurement |
| Phase 4: PolarSystem | ✅ 完成 | polar_system, branch_calculator, 13 单元测试 |
| Phase 5: 精细化 | ✅ 完成 | overall, port, angle, weld, aid 图元 |
| Phase 6: 验收 | 🔴 进行中 | 主样本通过，批量验收待执行 |
| Phase 4.5: Bridge 解耦 | ⏳ 待规划 | 验收通过后执行 |
| Phase 6.2-6.3: V1 下线 | ⏳ 待规划 | 最后执行 |

## 模块架构 (13 子模块, ~6000+ 行)

```
rs-core/src/mbd/v2/
├── pipeline.rs          789行  顶层入口, build_mbd_v2_pipe_data
├── assembler.rs        ~1800行  V1→V2 图元转换, chain stacking
├── avoidance.rs        ~1150行  2D 避让引擎 (label+leader)
├── primitive.rs          693行  11 种图元类型定义
├── polar_system.rs      ~500行  柱坐标放置优化
├── small_dim.rs         ~380行  小尺寸错层/字高
├── dim_direction.rs     ~350行  标注方向决策
├── text_measurement.rs   350行  PDMS 字符宽度表精确复刻
├── branch_calculator.rs ~300行  V2 增量集成, dimtimes 偏移
├── iso_ori.rs           ~280行  标注坐标系
├── used_dir.rs          ~225行  已用方向注册/冲突计数
├── member_positions.rs  ~210行  管件位置收集/排序
└── leader_router.rs     ~130行  引线路由
```

## 关键发现

### 1. cheight 40x 差异 (P0)

- **生产**: `default_cheight = 100.0` mm (pipeline.rs `production_defaults()`)
- **测试**: `default_cheight = 2.5` mm (AssemblerContext::default())
- **结论**: 避让算法使用绝对坐标，比例关系正确。但所有测试覆盖仅在 2.5mm scale
- **已处理**: 添加 6 个 cheight=100mm 测试到 avoidance.rs

### 2. dimtimes offset 公式差异 (P1)

**V1 (PDMS 原文 `isoDim.drawDim`):**
```
offset = od + cheight × 1.2 × (dimtimes - 1)
```

**V2 (`branch_calculator.rs`):**
```
offset = od/2 + cheight + cheight × 1.2 × (dimtimes - 1)
```

| 场景 | OD | cheight | dimtimes | V1 | V2 | 差值 |
|---|---|---|---|---|---|---|
| DN200 一层 | 229 | 100 | 1 | 229.0 | 214.5 | -14.5 |
| DN200 二层 | 229 | 100 | 2 | 349.0 | 334.5 | -14.5 |
| 小管径 | 50 | 100 | 1 | 50.0 | 125.0 | +75.0 |

**已处理**: 添加 `use_pdms_offset_formula` 配置项，默认使用 V2 公式，可切回 PDMS 兼容

### 3. overall dim 路径语义 (已确认)

V2 使用 `compute_path_total_length()` 累加 segment 几何长度 (路径总长)，非首尾直线距离。对折线 BRAN 语义正确。

### 4. V2 API 数据流

```
get_mbd_pipe_v2() → mbd_pipe_api.rs:568
  ↓ generate_mbd_data(V1)              ← Phase 4.5 解耦目标
  ↓ data.layout_result (V1 LayoutResult)
  ↓ build_mbd_v2_pipe_data(V2)         ← rs-core pipeline.rs
  → MbdV2PipeData (primitives)         → 前端 1:1 映射
```

前端 `plant3d-web/src/api/mbdPipeApi.ts` 已完全接入 V2 API (`/api/mbd/v2/pipe`)。

## 执行计划

### P0: 验收 (当前)

| # | 任务 | 验收标准 | 预估 | 状态 |
|---|---|---|---|---|
| 1 | 修复构建环境 | `cargo test` 链接通过 | 0.5天 | 阻塞 |
| 2 | 运行 cheight=100mm 测试 | 6 个测试全部 pass | 即时 | 待构建 |
| 3 | 10 条 BRAN/HANG 典型验收 | primitives 与 PDMS 参考一致 | 2-3天 | 下一步 |
| 4 | 100 条批量 JSON 无 error | 零 panic/error | 2-3天 | 下一步 |

### P1: 精调

| # | 任务 | 验收标准 | 预估 | 状态 |
|---|---|---|---|---|
| 5 | dimtimes 真实样本对齐 | 3+ 个 dimtimes≥3 样本偏差 <1mm | 1-2天 | 代码完成 |
| 6 | overall dim 折线样本确认 | 5 个 BRAN 折线路径总长一致 | 1天 | 已确认 |

### P2: 解耦与下线

| # | 任务 | 验收标准 | 预估 | 状态 |
|---|---|---|---|---|
| 7 | Phase 4.5 Bridge 解耦 | BranchCalcV2 直连 SurrealDB | 3-5天 | 待规划 |
| 8 | Phase 6.2 plant3d-web V1 移除 | V1 layout 代码删除 | 1-2天 | 待规划 |
| 9 | Phase 6.3 V1 API 废弃 | deprecated + 文档归档 | 1天 | 待规划 |

## 风险矩阵

| 风险 | 级别 | 缓解措施 |
|---|---|---|
| cheight 100mm 避让行为 | 高 | 6 个测试已写入，待运行验证 |
| dimtimes offset 与 PDMS 差异 | 高 | `use_pdms_offset_formula` 配置已添加 |
| isoline 5° 方向阈值误判 | 中 | 批量验收中统计误判率 |
| overall dim 路径歧义 | 中 | 已确认使用路径总长 |
| 2D 投影不考虑 3D 遮挡 | 低 | 设计决策，已接受 |
| V1 Bridge 技术债 | 低 | Phase 4.5 解耦计划 |

## 代码变更记录

| 文件 | 变更 | 行数 |
|---|---|---|
| `rs-core/src/mbd/v2/avoidance.rs` | +6 个 cheight=100mm 测试 | +110 |
| `rs-core/src/mbd/v2/branch_calculator.rs` | +use_pdms_offset_formula 配置 + 3 测试 | +60 |

## 相关文件

- `rs-core/src/mbd/v2/` — V2 核心模块 (13 个)
- `plant-model-gen/src/web_api/mbd_pipe_api.rs` — V2 API handler
- `plant3d-web/src/api/mbdPipeApi.ts` — 前端 V2 类型 + 适配器
- `test-1/mbd-v2-dev-plan.html` — 可视化开发方案
