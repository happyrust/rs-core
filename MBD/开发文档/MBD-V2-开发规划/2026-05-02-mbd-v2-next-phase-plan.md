# MBD V2 下一阶段开发计划

> **日期**：2026-05-02（更新）
> **范围**：Phase 7–9，从"V1 bridge 过渡"走向"V2 独立运行"
> **前置**：Phase 1–6 已完成（类型定义 → assembler → 小尺寸 → 避让 → PolarSystem → 验收框架）

---

## 一、当前状态总结

### 已完成（Phase 1–8）

| 阶段 | 交付物 | 状态 |
|---|---|---|
| Phase 1 | V2 primitive 类型定义（11 种图元 + CommonFields） | ✅ |
| Phase 2 | assembler.rs — V1 LayoutResult → V2 MbdPrimitive 翻译 | ✅ |
| Phase 3 | SmallDimSolver 链式小尺寸错层 + Label/Leader 避让引擎 | ✅ |
| Phase 4 | PolarSystem + BranchCalculatorV2 增量集成 + UsedDir | ✅ |
| Phase 5 | 弯头 AidLine/AidArc + 坡度辅助图元 | ✅ |
| Phase 6 | 主样本验收通过（24381_145712，15→24 primitives） | ✅（部分） |
| Phase 7.1 | 样本注册脚本 + 基准快照 | ✅ |
| Phase 7.2 | 扩展批量验收脚本（NaN/零方向/CSV 报告） | ✅ |
| Phase 7.4 | 生产字高 100mm 测试（6 个测试全部通过） | ✅ |
| Phase 8.1 | V2 数据源 trait `data_source.rs`（5 个测试通过） | ✅ |
| Phase 8.2 | `build_mbd_v2_pipe_data_direct` 直算入口（3 个测试通过） | ✅ |
| Phase 8.3 | API 双路径 `?v2_direct=true`（plant-model-gen 编译通过） | ✅ |
| Phase 9a | V2 渲染器骨架 `v2_renderer.rs`（~280行 Bevy 渲染器） | ✅ |
| 测试修复 | 6 个预存测试失败修复（pipeline 3 + assembler 1 + avoidance 2） | ✅ |
| 全量回归 | 154/154 pass, 0 fail | ✅ |

### 未完成 / 已知缺口

| 缺口 | 影响 | 优先级 |
|---|---|---|
| 10 条 BRAN 真实页面验收 | 验收覆盖率不足（需运行中 web_server） | P0 |
| 100 条批量 JSON 验收 | 回归基线缺失（脚本已就绪） | P0 |
| rs-plant3-d Bevy fork 修复 | 渲染器编译受阻 | P1 |
| Phase 9b-c 渲染实现（WeldMark/AngleDim/AidLine 等真实 mesh 渲染） | 辅助图元未渲染 | P1 |
| V1 API 退役 | 维护成本 | P2 |
| Overall dim 折线 BRAN 支持 | 功能缺失 | P2 |

---

## 二、Phase 7：验收补全与回归基线（预计 3–5 天）

### 目标
建立可重复执行的回归基线，确保后续 Phase 8 直算重构不引入回归。

### 7.1 批量 BRAN 样本库建设

**任务**：
1. 从 SurrealDB 查询项目内所有 BRAN，按类型分类（直管 / 折线 / T 接 / 多段）
2. 筛选出 ≥30 条覆盖不同拓扑的代表样本
3. 为每条样本记录基准数据：
   - `primitives_count`（各 kind 数量）
   - `issues_count`（按 severity 分类）
   - `dims_by_kind`（segment/chain/port/cut_tubi 分布）

**交付物**：
- `scripts/build-sample-registry.sh` — 自动化样本注册
- `test_data/mbd_v2_baseline.json` — 基准快照

### 7.2 批量 JSON 验收自动化

**任务**：
1. 扩展 `scripts/batch-validate-v2.sh` 支持 ≥100 条样本
2. 增加断言项：
   - 零 error issues
   - `primitives` 非空
   - `linear_dim` ≥1
   - 无 `NaN`/`Infinity` 坐标
   - `direction` 不全为零
3. 输出 CSV 报告 + pass/fail 汇总

**验收标准**：
- 100 条样本 pass rate ≥ 95%
- error issues = 0
- 失败样本有明确 root cause 分类

### 7.3 真实页面 10 条 BRAN 验收

**任务**：
1. 选取 10 条代表性 BRAN（含直管、折线、T 接）
2. 在 `http://localhost:3101/?output_project=...&mbd_refno=...` 下逐条截图
3. 对照验收清单：
   - 尺寸无缺失 / 无重复
   - 方向无明显偏转
   - 文字无不可读堆叠
   - 小尺寸正确错层或缩字高
   - 焊缝/坡度标记显示

**交付物**：
- `MBD-V2-开发规划/phase7-visual-acceptance-report.md`
- 10 张截图（作为 git LFS 或外部存储引用）

### 7.4 生产字高测试覆盖

**任务**：
1. 在 `pipeline.rs` 和 `avoidance.rs` 测试中增加 `cheight=100.0` 的测试用例
2. 验证 `max_lanes=6` 在 100mm 字高下是否足够
3. 验证 chain stacking 阈值在生产字高下的行为

---

## 三、Phase 8：V2 直算引擎（预计 7–10 天）

### 目标
让 `build_mbd_v2_pipe_data` 不再依赖 V1 `generate_mbd_data`，直接从 SurrealDB 查询管段数据并产出 V2 primitive。

### 8.1 数据源直查层

**任务**：
1. 新建 `rs-core/src/mbd/v2/data_source.rs`
2. 定义 V2 数据查询接口（trait）：

```rust
#[async_trait]
pub trait MbdV2DataSource {
    async fn query_branch_members(&self, branch_refno: &str) -> Result<Vec<BranchMember>>;
    async fn query_welds(&self, branch_refno: &str) -> Result<Vec<WeldData>>;
    async fn query_fittings(&self, branch_refno: &str) -> Result<Vec<FittingData>>;
    async fn query_branch_attrs(&self, branch_refno: &str) -> Result<BranchAttrs>;
}
```

3. 实现 `SurrealDbDataSource`，查询规范遵循 AGENTS.md 中的 SurrealDB 规范：
   - `tubi_relate` 用复合 ID Range
   - 强类型 `#[derive(SurrealValue)]`
   - 不做全表扫描

**约束**：
- trait 可测试（可 mock）
- 查询粒度与 V1 `generate_mbd_data` 一致，方便结果对比

### 8.2 BranchCalculatorV2 完整实现

**任务**：
1. 重构 `branch_calculator.rs`，从"增强 V1 LayoutResult"变为"直接从 BranchMember 产出 primitive"
2. 核心流程：

```
BranchMember[] → extract_isolines → PolarSystem per isoline
    → compute_dim_placements（方向 + offset + dimtimes）
    → assemble LinearDimPrimitive / WeldMark / SlopeMark / Label / Leader
    → SmallDimSolver 链式错层
    → avoidance
    → MbdV2PipeData
```

3. 关键差异点：
   - 不经过 V1 `PlacedLinearDim`，直接产出 `LinearDimPrimitive`
   - 不经过 `assembler.rs` 翻译层
   - PolarSystem 从 isoline 建立，不从 LayoutResult 推导

**验收标准**：
- 对 Phase 7 基准样本，V2 直算的 primitive 结构与 V1 bridge 结果可比较
- 允许方向/偏移有合理差异（PolarSystem 的优化结果可能不同），但 primitive 种类和数量应一致

### 8.3 Pipeline 双路径并行

**任务**：
1. `MbdV2PipelineContext` 新增 `use_v2_direct: bool`（默认 false）
2. `build_mbd_v2_pipe_data` 根据开关选择：
   - `false` → 现有 V1 bridge 路径（保持兼容）
   - `true` → V2 直算路径
3. plant-model-gen API 层通过 query 参数 `v2_direct=true` 控制

**交付物**：
- `GET /api/mbd/v2/pipe/{refno}?v2_direct=true` 走直算
- 默认仍走 V1 bridge，避免回归

---

## 四、Phase 9：前端原生渲染 + V1 退役（预计 5–7 天）

### 9.1 前端 V2 Primitive 原生渲染

**任务**：
1. plant3d-web 的 `useMbdPipeAnnotationThree.ts` 增加 `renderV2Primitive(prim: MbdPrimitive)` 分发器
2. 按 `primitive.kind` 分发到对应的 three.js 渲染器：
   - `linear_dim` → DimensionLine3D
   - `label` → TextSprite3D
   - `leader_line` → PolyLine3D
   - `weld_mark` → WeldCrossSymbol3D
   - `slope_mark` → SlopeIndicator3D
   - `angle_dim` → ArcDimension3D
   - `aid_line` / `aid_arc` / `aid_circle` → HelperGeometry3D
3. 不再做前端二次排版，所有坐标直接使用 primitive 中的世界坐标

### 9.2 V1 API 降级与退役

**步骤**：
1. 将 `/api/mbd/pipe/{refno}` 标记为 `@deprecated`
2. 前端默认请求 V2 API
3. 保留 `mbd_api=v1` 查询参数作为应急回滚
4. 观察 2 周无问题后，删除 V1 API 和 `assembler.rs`

### 9.3 退役清单

| 待删除 | 条件 |
|---|---|
| `assembler.rs`（1800 行） | V2 直算稳定 + 前端切 V2 |
| V1 `generate_mbd_data` 中的 MBD 分支 | 无其他调用方 |
| V1 `BranchCalculator::solve_branch` | V2 直算覆盖所有场景 |
| 前端 `computeDimensionOffsetDir` 等双算逻辑 | 前端原生渲染稳定 |

---

## 五、风险与缓解

| 风险 | 影响 | 缓解措施 |
|---|---|---|
| V2 直算与 V1 bridge 结果差异大 | 用户感知变化 | 双路径并行 + 基准对比 |
| SurrealDB 查询性能 | 响应时间增加 | 复合 ID Range + 预热缓存 |
| PolarSystem 极端拓扑（闭环/超长分支） | 方向计算异常 | 降级到 V1 方向 + Issue 告警 |
| 前端原生渲染适配工作量 | 进度延迟 | 分阶段渲染（先 linear_dim，再其他） |
| cheight 差异导致避让在生产环境失效 | 文字堆叠 | Phase 7 先补生产字高测试 |

---

## 六、里程碑时间线

| 周次 | 阶段 | 关键交付 |
|---|---|---|
| W1 | Phase 7.1–7.2 | 样本库 + 批量验收脚本 |
| W1–W2 | Phase 7.3–7.4 | 真实页面验收 + 生产字高测试 |
| W2–W3 | Phase 8.1 | V2 数据源 trait + SurrealDB 实现 |
| W3–W4 | Phase 8.2 | BranchCalculatorV2 完整实现 |
| W4 | Phase 8.3 | 双路径并行 + 基准对比验证 |
| W5 | Phase 9.1 | 前端 V2 原生渲染（linear_dim 优先） |
| W5–W6 | Phase 9.2–9.3 | V1 退役 + 稳定观察 |

---

## 七、决策待确认

1. **Overall dim 策略**：折线 BRAN 的 overall dim 是否需要支持？当前已关闭。如果需要，表达方式是路径总长还是首尾直连？
2. **多项目兼容**：V2 直算的 SurrealDB 查询是否需要支持多项目（ns/db 切换）？
3. **前端渲染优先级**：是否接受分阶段渲染（先 linear_dim / label / leader，后 angle_dim / aid_*）？
4. **V1 退役时间窗口**：是否接受 2 周观察期？或者需要更长？
5. **批量验收样本来源**：除 AvevaMarineSample 外，是否有其他项目需要覆盖？

---

## 附录 A：文件变更预览

### Phase 7
```
新增  scripts/build-sample-registry.sh
新增  scripts/batch-validate-v2-extended.sh
新增  test_data/mbd_v2_baseline.json
新增  MBD-V2-开发规划/phase7-visual-acceptance-report.md
修改  rs-core/src/mbd/v2/pipeline.rs          (测试补充)
修改  rs-core/src/mbd/v2/avoidance.rs          (测试补充)
```

### Phase 8
```
新增  rs-core/src/mbd/v2/data_source.rs        (~400 行)
重构  rs-core/src/mbd/v2/branch_calculator.rs   (~600 行, 从 ~300 扩展)
修改  rs-core/src/mbd/v2/pipeline.rs            (双路径)
修改  rs-core/src/mbd/v2/mod.rs                 (注册 data_source)
修改  plant-model-gen/src/web_api/mbd_pipe_api.rs  (v2_direct 参数)
```

### Phase 9
```
修改  rs-plant3-d/src/...                       (前端 V2 渲染)
删除  rs-core/src/mbd/v2/assembler.rs           (V1 退役后)
修改  plant-model-gen/src/web_api/mbd_pipe_api.rs  (V1 API 删除)
```
