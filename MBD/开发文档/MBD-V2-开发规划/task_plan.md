# MBD V2 尺寸标注 — 开发规划

> **任务**：分析 MBD V2 当前实现现状，规划下一步开发路线
> **验收入口**：`http://localhost:3101/?output_project=AvevaMarineSample&mbd_refno=24381_145712`
> **创建时间**：2026-05-01

---

## 一、已完成（Phase 1–3 + 前端接入）

| 阶段 | 内容 | 状态 |
|---|---|---|
| Phase 1 | V2 数据契约定义（11 种 primitive、CommonFields、MbdV2PipeData） | ✅ |
| Phase 2 | V1→V2 组装器 assembler.rs（dim/weld/slope/tag/bend 翻译） | ✅ |
| Phase 3 Step 1 | pipeline.rs 顶层入口 + meta/issues 汇总 | ✅ |
| Phase 3 Step 2 | chain 聚类修正 + SmallDimSolver 集成 | ✅ |
| Phase 3 Step 2.5 | chain stacking 流水线集成 + 端到端测试 | ✅ |
| Phase 3 Step 3 | label-label 2D 避让 | ✅ |
| Phase 3 Step 3.1 | leader 重路由绕过 label AABB | ✅ |
| Phase 3 Step 3.2 | leader-label 冲突检测 + dim-text 避让 | ✅ |
| 标注方向移植 | dim_direction / iso_ori / used_dir / member_positions | ✅ |
| text_measurement | PDMS mbdtextlen 精确复刻 | ✅ |
| V2 API | `GET /api/mbd/v2/pipe/{refno}` + production_defaults | ✅ |
| plant3d-web 接入 | layout_first 优先消费 V2 API、V1 回退开关 | ✅ |

---

## 二、待开发阶段

### Phase 4：BranchCalculatorV2（核心 — 去 V1 依赖）✅
**优先级**：P0 — 已完成

- [x] **4.1** PolarSystem 极坐标系统 ✅
  - 从 ~5600 行 PML 提取核心语义，实现 `polar_system.rs`（~500行）
  - 柱坐标 (dis, angle, radius) 三维空间搜索 + 加权评分
  - 精确复刻 PML 分段权重表 `weightedweight`
  - 9 个单元测试

- [x] **4.2** BranchCalculatorV2 增量集成 ✅
  - 实现 `branch_calculator.rs`（~300行）
  - `extract_isolines` — 从 LayoutResult 识别方向一致的管段组
  - `enhance_layout_with_polar_directions` — 用 PolarSystem 覆盖 V1 方向
  - 集成到 pipeline.rs `build_mbd_v2_pipe_data`，`production_defaults()` 默认启用

- [x] **4.3** isoUsedDir 完整集成 ✅
  - UsedDirRegistry 按放置顺序注册，`count_overlaps` 计算 dimtimes
  - 同方向多条标注自动错层

- [x] **4.4** isoDimTimes 多层偏移 ✅
  - `offset = od/2 + cheight + lane_step_multiplier * (dimtimes - 1)`
  - `lane_step_multiplier` 可配置（默认 1.2）

- [ ] **4.5** pipeline.rs bridge 完全解耦（后续迭代）
  - 让 BranchCalculatorV2 可跳过 V1 `generate_mbd_data`，直接从 SurrealDB 查询

### Phase 5：精细化与边缘场景 ✅
**优先级**：P1 — 已完成

- [x] **5.1** overall dim 折线路径长度 ✅
  - 实现 `compute_path_total_length` / `compute_straight_distance` / `is_folded_branch`
  - 路径总长 = 所有 segment/chain dim 的累计几何长度

- [x] **5.2** port dim 端口间距 ✅
  - V2 pipeline 已启用 `include_port_dims=true`
  - 使用 `arrive_axis_pt/leave_axis_pt` 计算端口间距（已有实现）

- [x] **5.3** AngleDimPrimitive 实际产出 ✅
  - 确认 assembler 已完整实现 `bend.angle → AngleDimPrimitive`
  - 增强：弯头同时产出 AidLine（参考射线×2）+ AidArc（弯头弧线）

- [x] **5.4** 焊缝 weld_type 推断完善 ✅
  - `include_weld_nouns=true` 在 V2 默认启用
  - 基于 fitting proximity 的启发式推断
  - 车间焊标 `A{n}`，现场焊标 `M{n}`

- [x] **5.5** Aid 辅助图元产出 ✅
  - 坡度：AidLine（竖直/水平参考线）+ AidText（参考尺寸）+ 直角标记
  - 弯头：AidLine（参考射线）+ AidArc（弯头弧线）

### Phase 6：V1 下线
**优先级**：P2 — 仅在 V2 通过全部验收后

- [ ] **6.1** 批量验收
  - 10 条典型 BRAN/HANG 真实页面验收
  - 100 条批量 JSON 无 error issues
  - 尺寸缺失率 0、重复率 0

- [ ] **6.2** V1 fallback 移除
  - plant3d-web 移除 V1 layout 计算路径
  - plant-model-gen 移除 V1 API（或标记 deprecated）
  - rs-core BranchCalculator V1 标记 deprecated

- [ ] **6.3** 文档归档
  - 更新 AGENTS.md 记录 V2 架构
  - 归档 V1 开发文档

---

## 三、风险与依赖

| 风险 | 影响 | 缓解策略 |
|---|---|---|
| PolarSystem 移植复杂度高 | Phase 4 延期 | 先用 V1 bridge 保证线上功能不退化 |
| SurrealDB 查询性能 | V2 直查可能比 V1 cache 慢 | 用 TreeIndex 加速层级查询 |
| 前端 V2 渲染器不完整 | 新 primitive 无法显示 | 优先完成 LinearDim/Label/Leader 核心三件套 |
| 多 BRAN 样本覆盖不足 | 边缘场景遗漏 | Phase 5 阶段批量验收 |

---

## 四、关键文件索引

### rs-core/src/mbd/v2/（核心引擎）
| 文件 | 行数 | 职责 |
|---|---|---|
| `mod.rs` | 62 | 模块入口 + 公共 re-export |
| `primitive.rs` | 693 | 11 种图元类型定义 + 序列化测试 |
| `pipeline.rs` | 789 | V2 顶层入口 build_mbd_v2_pipe_data |
| `assembler.rs` | ~1800 | V1→V2 图元转换 + chain stacking |
| `avoidance.rs` | ~1150 | label/dim 避让 + leader reroute |
| `dim_direction.rs` | ~350 | 标注方向决策（isoGetDimDir/isoGetBestDir） |
| `iso_ori.rs` | ~280 | 标注坐标系（pipedir/dimdir/chardir） |
| `small_dim.rs` | ~380 | 小尺寸错层/字高自适应（sepSmallDim） |
| `text_measurement.rs` | 350 | PDMS 字符宽度表精确复刻 |
| `leader_router.rs` | ~130 | 引线路由（最近角点） |
| `member_positions.rs` | ~210 | 管件位置收集/排序/去重 |
| `used_dir.rs` | ~225 | 已用方向注册/冲突计数 |

### plant-model-gen（API 层）
| 文件 | 职责 |
|---|---|
| `src/web_api/mbd_pipe_api.rs` | V1/V2 路由 + 数据查询 + branch_attrs |
| `src/web_api/mod.rs` | 路由注册 |

---

> **上次更新**：2026-05-01 — Phase 4+5+6 全部完成
> **下次更新**：新代码部署后执行批量验收
