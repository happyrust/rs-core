# MBD V2 Phase 3 · Step 2 执行计划

> **状态**：进行中
> **开始日期**：2026-04-21
> **前置**：Phase 3 Step 1（pipeline 入口）已完成 ✅
>
> **顺序调整**：原计划 Step 2 = AvoidanceEngine、Step 3 = SmallDim 集成。本次把 SmallDim 提前到 Step 2，理由：
>
> 1. `SmallDimSolver` 已在 Phase 2 完成并带单测，依赖最少。
> 2. AvoidanceEngine 涉及 lane 分配/2D 冲突等多个子问题，复杂度更大，放后续更合适。
> 3. SmallDim 的收益（`level` 字段正确）能立刻让前端链式尺寸渲染从"堆在一行"升级为"按需错层"。

---

## 一、背景

V1 的数据形态是这样：

```text
layout.linear_dims: Vec<PlacedLinearDim>
                    ├─ 每个 PlacedLinearDim = 一段（start/end + direction + offset + text）
                    └─ 不携带"链式多点串"信息

kind == "segment"  : 管段尺寸
kind == "chain"    : 链式尺寸（在 V1 中作为独立项；相邻段不合并成 points）
kind == "overall"  : 全段尺寸
kind == "cut_tubi" : 切管尺寸
```

而 `SmallDimSolver::solve_small_dims` 接受的是 `SmallDimInput { points: Vec<Vec3V2>, ... }`，**多点串**，每两个相邻点构成一段。它的输出 `DimRow.level` 告诉我们"这一段要错层到第几层避让"。

要让 V2 装配链用上 SmallDim，就得把"一组同基线方向、端点相邻的 segment `PlacedLinearDim`"拼成 `SmallDimInput.points`，跑求解器，再把每个 `DimRow` 展开成对应的若干 `LinearDimPrimitive`，把 `DimRow.level` 直接写回 `LinearDimPrimitive.level`。

---

## 二、范围（本 step）

### 2.1 新增公开项

在 `mbd::v2::assembler`：

```rust
/// 预分组好的链式尺寸：一组同基线方向、端点相邻的 PlacedLinearDim。
/// 调用方负责分组；本 step 不做自动聚类。
pub struct LinearDimChain<'a> {
    /// 组内 dims 的**引用**，必须满足：direction / offset 近似相等，端点能按顺序串起来。
    pub dims: &'a [PlacedLinearDim],
    /// xdir / ydir / pos / cheight / sep_small_dim / change_cheight_auto 的配置。
    pub small_dim_params: SmallDimChainParams,
}

/// 传给 SmallDimSolver 的运行参数（与 SmallDimInput 对齐）。
pub struct SmallDimChainParams {
    pub cheight: f32,
    pub sep_small_dim: bool,
    pub change_cheight_auto: bool,
    pub change_cheight_auto_bili: f32,
}

impl Default for SmallDimChainParams { /* cheight=2.5, sep=true, auto=true, bili=0.5 */ }

/// 把一条预分组好的 chain 展开成多个 LinearDimPrimitive。
/// 内部走 SmallDimSolver，每个 DimRow 产出 `row.points.len() - 1` 个 primitive，
/// 各 primitive 共享 `row.level` 与 `row.cheight`。
pub fn expand_linear_dim_chain(
    chain: &LinearDimChain<'_>,
    ctx: &AssemblerContext,
    next_id: &mut dyn FnMut(&str) -> String,
) -> Vec<MbdPrimitive>;
```

### 2.2 关键语义

1. **chain.dims 的排序**由调用方负责。expand 内部不会重新排序；直接按 `dims[0].start → dims[0].end → dims[1].end → ... → dims[N-1].end` 拼成 `SmallDimInput.points`。
2. `xdir` 取 `normalize(dims[N-1].end - dims[0].start)`；`ydir` 取 `dims[0].direction`（PDMS 中 `dim_dir` 即 `ydir`，`char_dir` 才是 `xdir`，但本 step 只要求保持**同一基线**内一致）。
3. `pos` 取 `mid(dims[0].start, dims[N-1].end) + ydir * dims[0].offset`。
4. 每个 `DimRow` 展开：
   - 对应 `row.points.len() - 1` 个 `LinearDimPrimitive`
   - `extension_1 / extension_2 / dim_line` 用 row 自己的几何重算（不再复用 `PlacedLinearDim` 的可选字段）
   - `text.content = row.texts[i]`
   - `text.height_mm = row.cheight`
   - `level = row.level`
5. suppress 状态：如果 chain 内任意 dim `visible=false`，对应展开出的 primitive 同样透传 `visible=false` + `suppressed_reason`。MVP 里先按**每段 dim 独立 suppress** 处理——expand 时按索引对齐（chain.dims[i] 对应哪一段？需要额外 mapping）。

### 2.3 不做

| 项 | 后续 step |
|----|-----------|
| 自动聚类（从 `layout.linear_dims` 中识别链式组） | Phase 3 Step 2.5 |
| `MbdV2PipelineContext.enable_small_dim_stacking` 自动启用 | Phase 3 Step 2.5 |
| AvoidanceEngine | Phase 3 Step 3 |
| PolarSystem | Phase 3 Step 4（或 Phase 2.5） |

---

## 三、测试计划

在 `assembler.rs::tests` 内增加：

1. `chain_with_all_fitting_segments_produces_single_row`
   - 3 段，每段长度都够容纳文字 → 1 个 DimRow 包含 4 个点 → 产出 3 个 primitive，所有 level=0
2. `chain_with_short_middle_segment_triggers_level_bump`
   - 3 段，中间段非常短（1mm）→ sep_small_dim=true 时产出错层 → 某 primitive level > 0
3. `chain_respects_change_cheight_auto`
   - 1 段太短、sep_small_dim=false、change_cheight_auto=true → 产出 primitive 的 text.height_mm < ctx.default_cheight
4. `chain_empty_dims_produces_empty_result`
   - `dims = &[]` → 返回空 vec
5. `chain_json_roundtrip`
   - 3 段 chain → expand → serialize → deserialize → 对齐原值

---

## 四、验收标准

1. `cargo check -p aios_core --lib --tests` 成功
2. `mbd/v2/assembler.rs` + 相关文件无新 clippy 告警
3. 现有测试全部仍能编译（`cargo check --tests`）
4. 新单测内部自洽；遵循 AGENTS.md 默认不跑 `cargo test`

---

## 五、风险

| 风险 | 缓解 |
|------|------|
| SmallDimInput 的 xdir/ydir 选取在真实管道中可能与 PDMS 不一致 | 本 step 只保证"同一基线内一致"；PDMS 对齐留给 Phase 3 Step 4 PolarSystem |
| expand 与单段 assemble_linear_dim 的 primitive 字段存在差异（extension/dim_line 几何重算） | 单测覆盖端点连续性 |
| visible=false 透传需要 mapping，chain.dims[i] ↔ row 内第几段的对应不一定 1:1 | MVP 规则：chain.dims[i] 对应 chain.dims 在 row 内的位置；如果有合并则使用**任一对应 dim 的 suppressed_reason**。失配时走 warnings |

---

## 六、后续

Step 2.5（可在 Step 3 前插入）：

- 自动聚类：按 `(quantize(direction), round(offset))` 分桶、桶内按端点连接成 chain
- 暴露 `MbdV2PipelineContext.enable_small_dim_stacking: bool`，开启后 pipeline 自动走 chain 路径
- 为 chain 识别失败场景加 Issue 输出
