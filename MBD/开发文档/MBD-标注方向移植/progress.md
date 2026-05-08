# MBD 标注方向移植 — 进度日志

## 2026-04-30：初始规划

### 已完成

- 分析 PML 源码：`isoGetDimDir.pmlfnc`、`isoGetBestDir.pmlfnc`、`isoFindIsoUsedDir.pmlfnc`
- 分析 PML 源码：`isoDim.pmlobj`（1538 行，核心标注类）
- 分析当前 V2 代码：`assembler.rs`、`avoidance.rs`、`pipeline.rs` 的方向处理
- 量化 aios_core 内部耦合：types(79 文件) / shape(50) / parsed_data(46)
- 创建开发计划三件套：task_plan.md / findings.md / progress.md
- 记录 PML 方向算法的伪代码和 Rust 等价实现思路

### 关键发现

1. PML 标注方向算法核心只有 ~50 行（isoGetDimDir + isoGetBestDir），Rust 等价 ~100 行
2. CalculateDimChardirs 依赖 bran volume（包围盒），需要确认 V2 pipeline 数据源
3. V1 PlacedLinearDim.direction 已包含标注方向，但未经 PML 优选逻辑
4. isoUsedDir 是避免多标注重叠的核心机制，与 avoidance.rs 的 lane 机制互补

### Phase 1 执行

- 创建 `src/mbd/v2/dim_direction.rs`（~240 行），实现：
  - `iso_get_best_dir()` — 方向一致性校正
  - `iso_get_dim_dir()` — 标注偏移方向决策
  - `calculate_dim_char_dirs()` — 从包围盒推断优选方向
  - `resolve_dim_direction()` — 集成入口
  - `PreferredDirs` / `DimDirectionResult` 结构体
- 编写 8 个单元测试：水平管、垂直管、斜管、方向翻转、包围盒推断等
- 注册到 `src/mbd/v2/mod.rs` 并导出公共 API
- `cargo check` 编译通过
- `cargo test` 因已知 `aws_lc` arm64 链接问题无法在本地运行（与本次代码无关）

### assembler.rs 集成完成

- `AssemblerContext` 新增 `bran_bbox_center: Option<Vec3V2>` 字段
- `text_frame_from_linear_dim` 优先使用 `resolve_dim_direction`（当 bran_bbox_center 存在时）
- 无 bran_bbox_center 时 fallback 到原有逻辑，保持向后兼容
- 新增 `mid_v3` 和 `dot_v3` 辅助函数
- `cargo check` 编译通过

### Phase 2-5 已完成

- Phase 2: `used_dir.rs` — IsoUsedDir + UsedDirRegistry
- Phase 3: `iso_ori.rs` — compute_iso_ori + get_handle_dim_dir
- Phase 4: `member_positions.rs` — DimPositions + consider_tee
- Phase 5: `assembler.rs` — 斜管标注分解（AidLine + AidText + 直角标）
- 全部已提交: `1936ab2` + `71c8bf6`
- 已推送到 origin/dev-3.1

### 已提交汇总

| Commit | 分支 | 说明 |
|--------|------|------|
| 7fec378 | dev-3.1 | 清理 .cursor 旧文件 (-7655行) |
| 1936ab2 | dev-3.1 | PML 标注方向算法 Rust 移植 (+1872行) |
| 71c8bf6 | dev-3.1 | MBD V2 改进 + 查询扩展 (+646行) |
| 60952c0 | feat/workspace-split | 抽离 aios-mbd crate |
| 1a1472b | feat/workspace-split | 下一步拆分计划 |
| 7ef8bb1 | feat/workspace-split | 抽离 aios-pdms-core crate |

### 下一步

- Phase 6：集成验证，需要 plant-model-gen + plant3d-web 环境
- Phase 4 剩余：PCOM/INST/ATTA 位置投影（需 PDMS 类型系统适配）
- feat/workspace-split 创建 PR

### 涉及的关键文件

| 文件 | 角色 |
|------|------|
| `rs-core/src/mbd/v2/assembler.rs` | V2 primitive 组装，需要集成方向算法 |
| `rs-core/src/mbd/v2/pipeline.rs` | V2 入口，需要传入方向上下文 |
| `rs-core/src/mbd/v2/mod.rs` | V2 模块声明，需要新增子模块 |
| `rs-core/MBD/markpipe/function/isoGetDimDir.pmlfnc` | PML 参考 |
| `rs-core/MBD/markpipe/function/isoGetBestDir.pmlfnc` | PML 参考 |
| `rs-core/MBD/markpipe/object/isoDim.pmlobj` | PML 参考（核心标注类） |
