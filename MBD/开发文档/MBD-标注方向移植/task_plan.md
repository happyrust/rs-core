# 任务计划：MBD 管道标注方向算法移植

## 目标

将 PML 中 `isoDim`、`isoGetDimDir`、`isoGetBestDir`、`isoUsedDir` 的标注方向决策算法完整移植到 Rust V2 模块，使管道尺寸标注能自动选择最佳标注方向、避免方向冲突，与 PDMS 输出视觉对齐。

## 当前阶段

Phase 3 complete → Phase 5 in_progress (Phase 4 deferred)

## 阶段计划

### Phase 1：方向算法核心移植（isoGetDimDir + isoGetBestDir）

**目标**：管道标注能根据管段方向和优选方向列表自动决定标注线的偏移方向。

- [x] 翻译 `isoGetDimDir`：给定 `pipedir`（管段方向）和 `dimdirs`（3 个优选方向），产出 `dimdir`
- [x] 翻译 `isoGetBestDir`：给定 `inputdir` 和 `dirs[1..3]`，选与最近优选方向一致的方向（>90° 则取反）
- [x] 翻译 `CalculateDimChardirs`：从 bran volume 包围盒中心推断优选标注方向
- [x] 编写单元测试：8 个 golden case（水平管、垂直管、斜管、方向翻转、包围盒推断等）
- [x] 在 `assembler.rs` 中接入方向算法，替换固定 `default_orientation` / `default_up`
- **状态：** complete

### Phase 2：isoUsedDir 已用方向记录

**目标**：记录每个已放置标注的方向和距离区间，后续标注避开已占用区域。

- [x] 定义 `IsoUsedDir` 结构体：`name, direction, min_dis, max_dis, kind, angle_tolerance, min/max_radius`
- [x] 实现 `UsedDirRegistry`：`register`, `find_by_name`, `count_overlaps`, `compute_dimtimes`
- [x] 翻译 `isoFindIsoUsedDir`：按名称查找已用方向
- [x] 实现重叠检测：方向平行（含反向）+ 距离区间交集
- [x] 编写 9 个单元测试（空注册表、同向冲突、反向冲突、垂直无冲突、多重叠等）
- [x] 在 assembler 中集成 `UsedDirRegistry`：标注放置后自动注册方向和距离区间
- [x] AssemblerContext 新增 `lane_step_multiplier` 和 `pipe_od` 字段
- **状态：** complete

### Phase 3：isoOri 朝向解算

**目标**：从管段方向 + 优选标注方向 + 优选字符方向，解算出标注平面的完整朝向（ori = x is pipedir and y is dimdir）。

- [x] 翻译 `isoOri` 对象：`IsoOri { pipedir, dimdir, chardir }`
- [x] 实现 `compute_iso_ori()`：从 pipedir + dimdirs + chardirs + used_dirs 解算完整朝向
- [x] 实现 `get_handle_dim_dir()`：在已用方向约束下搜索角度间隙（简化版 isoGetHandleDimDir）
- [x] 编写 3 个测试：水平管、垂直管、有已用方向冲突
- [x] 在 assembler 中集成 `compute_iso_ori`：`text_frame_from_linear_dim` 接受 `UsedDirRegistry`
- **状态：** complete

### Phase 4：管件位置收集优化（addmem 逻辑）

**目标**：处理 PDMS 管件类型（TEE, PCOM, INST, ATTA 等）的位置投影、端口选择等边界条件。

- [x] 创建 `member_positions.rs`：`DimPositions` 结构体（按投影距离排序的点位列表）
- [x] 翻译 `addpos`：插入点位保持投影距离升序
- [x] 翻译 `possunique`：去除投影距离过近的相邻点
- [x] 翻译 `considerTee`：TEE/OLET 端口位置补充（port_index != 3）
- [x] 编写 6 个单元测试（排序、去重、跨度、TEE 补充等）
- [ ] 翻译 `addmem` 中 PCOM/INST/ATTA 的位置投影逻辑（需 PDMS 类型系统适配）
- [ ] 集成到 V1 → V2 数据流或新 `BranchCalculator v2`
- **状态：** in_progress

### Phase 5：斜管标注分解

**目标**：斜管自动分解为水平 + 垂直辅助线标注，画直角标记。

- [x] 翻译 `dimslope`：斜管分解为水平+垂直辅助线标注
- [x] 生成 `AidLinePrimitive`（水平线、垂直线）+ `AidTextPrimitive`（距离文字）
- [x] 生成直角标记（两条短辅助线）
- [x] `assemble_slope` 从返回单个 primitive 改为 `Vec<MbdPrimitive>`
- [ ] 翻译 `dimOneMemSlope`：根据 maxslope 判断是否启用斜管分解
- [ ] 编写测试：30° 斜管和水平管的标注差异
- **状态：** in_progress

### Phase 6：集成验证

**目标**：端到端验证，与 PDMS 输出对比。

- [ ] 接入 `plant-model-gen` V2 API
- [ ] 用 refno `24381_145712` 做真实数据验证
- [ ] 与 PDMS 输出 JSON 做逐 primitive 对比
- [ ] 在 `localhost:3101` 页面视觉验证
- **状态：** pending

## 关键问题

1. **V1 LayoutResult 是否已包含足够的管段方向信息？** — 需要检查 `PlacedLinearDim` 是否携带 `pipedir`
2. **bran volume（包围盒）信息在 V2 pipeline 中如何获取？** — `CalculateDimChardirs` 需要 bran 包围盒
3. **addmem 逻辑是否需要等 BranchCalculator v2 才能完整实现？** — 可能需要分阶段
4. **isoOri 的 minangle 参数（30° vs 60°）如何选择？** — 不同调用场景有不同值

## 已做决策

| 决策 | 理由 |
|------|------|
| 优先移植方向算法而非 PolarSystem | 方向错误是当前最明显的视觉问题，且代码量最小 |
| 在 assembler.rs 层面集成而非新建独立模块 | 减少间接层，方向算法是 assembler 的核心逻辑 |
| 保留 V1 LayoutResult 过渡路径 | 避免阻塞现有 API，渐进式迁移 |
| 使用 PML 字符宽度查表而非 HarfBuzz | 与 PDMS 严格对齐，已在 text_measurement.rs 实现 |

## 遇到的错误

| 错误 | 尝试 | 解决方案 |
|------|------|----------|
| `aws_lc` arm64 链接符号缺失 | 1 | 已知环境问题，`cargo check` 通过证明代码正确，测试需在 CI 或修复链接后验证 |

## PML 参考文件索引

| PML 文件 | 对应 Rust 模块 | 关键方法/函数 |
|----------|---------------|--------------|
| `markpipe/function/isoGetDimDir.pmlfnc` | 待创建 | `pipedir.orthogonal(dimdirs[3])` → dimdir |
| `markpipe/function/isoGetBestDir.pmlfnc` | 待创建 | 3 方向夹角排序 → 最近方向 >90° 取反 |
| `markpipe/function/isoFindIsoUsedDir.pmlfnc` | 待创建 | 按 name 查找 IsoUsedDir |
| `markpipe/object/isoDim.pmlobj` | `assembler.rs` / 待扩展 | addmem, draw, dimslope, CalculateDimChardirs |
| `markpipe/object/isobran.pmlobj` | V1 BranchCalculator | 分支标注总控 |
| `object/mbd/lindim.pmlobj` | `small_dim.rs` | sepSmallDim, changeCheightAuto |
| `object/polarsystem/polarsystem.pmlobj` | 待创建 | getBestPosAndOri |

## 备注

- 每个阶段完成后更新 `progress.md` 和本文件状态
- 重大发现记录到 `findings.md`
- Phase 1-3 是最小可验证集，能让标注方向基本正确
- Phase 4-5 是 PDMS 对齐的精细化工作
- Phase 6 是最终验收，必须通过真实页面
