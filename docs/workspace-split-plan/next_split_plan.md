# 下一阶段 Workspace 拆分开发方案

## 规划目标

在 `aios-mbd` 作为第一块样板稳定后，继续选择低耦合、纯计算、可独立测试的小模块推进拆分，避免一次性移动数据库、运行时、几何生成等高耦合区域。

## 拆分原则

- 先拆纯函数与纯 DTO，后拆带 IO/全局状态/数据库连接的模块。
- 每次只新增一个 crate 或一个清晰边界，保持 PR 可审。
- 旧路径通过 `aios_core` re-export 兼容，下游迁移后置。
- 每个新 crate 必须有独立测试和兼容性测试。
- 每次拆分后必须更新 `validation.md` 与 crate README。

## 推荐顺序

### Step 1：抽出 PDMS 基础工具 crate

建议名称：`aios-pdms-core` 或 `aios-pdms-util`

首批候选：

- `src/types/pdms_hash.rs`
- `src/types/float_util.rs`

理由：

- 当前未发现对 `aios_core` 的反向依赖。
- 逻辑属于稳定基础算法，适合多个上层模块复用。
- 可用小规模单元测试保护行为。

不建议一次性纳入：

- `src/tool/db_tool.rs`，因为它包含全局 UDA map、文件读取和 `PdmsDatabaseInfo`。
- `src/types/hash.rs`，因为它仍引用 `PlantAabb` 和 `Transform`。

### Step 2：拆出稳定 hash/transform 输入接口

目标不是直接移动整个 `src/types/hash.rs`，而是先把底层无 `crate::` 依赖的函数形态稳定下来。

建议做法：

- 将 `gen_trs_hash(translation, rotation, scale)` 保留为纯输入 API。
- 根 crate 的 `gen_plant_transform_hash` 继续作为适配层，负责从 `Transform` 取值。
- 根 crate 的 `gen_plant_aabb_hash` 继续作为适配层，负责从 `PlantAabb` 取值。

这样可以避免新 crate 反向依赖 `aios_core::types` 或 `aios_core::plant_transform`。

### Step 3：建立拆分候选矩阵

对每个候选模块记录以下信息：

- 是否包含 `crate::` 反向引用。
- 是否包含数据库、配置、文件 IO、全局状态。
- 是否有可迁移的单元测试。
- 是否需要 re-export 兼容路径。
- 是否会影响 wasm 或 server 下游。

### Step 4：延后高耦合模块

暂缓拆分：

- `rs_surreal`
- `runtime`
- `db_pool`
- `tree_query`
- `prim_geo`
- `shape`
- `tool/db_tool`

这些模块涉及数据库连接、全局缓存、几何 trait、运行时初始化或大量内部类型，适合在纯工具 crate 稳定后再规划。

## 建议任务拆分

### Task A：建立 `aios-pdms-core`

- 新增 `crates/aios-pdms-core/Cargo.toml`。
- 移动或复制 `pdms_hash`、`float_util` 到新 crate。
- 根 crate 保留 `pub use aios_pdms_core::*` 或局部 re-export。
- 为 `db1_hash`、`db1_dehash`、浮点 round/hash 补单元测试。

### Task B：兼容层回归

- 确认旧路径仍可用：
  - `aios_core::types::pdms_hash::*`
  - `aios_core::types::float_util::*`
  - `aios_core::tool::db_tool::db1_hash`
- 新增兼容性测试，覆盖下游常用入口。

### Task C：验证与文档

- 更新 `validation.md` 增加：
  - `cargo test -p aios-pdms-core`
  - 对应兼容性测试命令
- 为新 crate 添加 README。
- 更新 workspace 架构图和进度记录。

## 验收标准

- 新 crate 无反向依赖 `aios_core`。
- 根 crate 旧路径继续可编译。
- `cargo check --workspace` 通过。
- 新 crate 独立测试通过。
- 兼容性测试通过。
- 本轮不要求修复历史 `cargo test --workspace` 全量失败。

## 主要风险

- `db_tool` 中已有 `pdms_hash` re-export，迁移时需要保留旧函数名和缓存语义。
- `types/hash.rs` 名称容易与 `pdms_hash.rs` 混淆，拆分时需要明确“PDMS 名称 hash”和“几何稳定 hash”的边界。
- 如果新 crate 过早接收 `PlantAabb`、`Transform` 等根 crate 类型，会造成循环依赖风险。
