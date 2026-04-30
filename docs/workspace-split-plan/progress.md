# Workspace 拆分规划进度

## 2026-04-30

### 已完成

- 审核当前 `rs-core-ws-split` worktree 的 workspace 拆分方案。
- 确认 `aios-mbd` 抽 crate 后无反向依赖 `aios_core`。
- 运行并记录验证命令：
  - `cargo check --workspace`
  - `cargo test -p aios-mbd`
  - `cargo test -p aios_core --test mbd_reexport`
  - `cargo test --workspace`
- 创建中文规划文件：
  - `task_plan.md`
  - `findings.md`
  - `progress.md`
- 创建并验证 workspace 架构图：
  - `workspace_split_architecture.svg`
  - `workspace_split_architecture.png`

### 验证结果

- `cargo check --workspace`：通过，有 `angle_deg` unused warning。
- `cargo test -p aios-mbd`：通过，24 passed。
- `cargo test -p aios_core --test mbd_reexport`：通过，1 passed。
- `cargo test --workspace`：失败，包含根 crate examples 编译错误和 `aws_lc` arm64 链接符号缺失。

### 下一步

- 确认所有新 crate 文件和兼容性测试被纳入提交。
- 将稳定验证命令写入 PR 描述或开发文档。

### 过程中遇到的问题

- 首次 SVG 校验失败，原因是部分中文文本写入后出现非法 XML 控制字符；已将图内说明文字调整为 ASCII，并通过 `rsvg-convert` 验证与 PNG 导出。

## 2026-04-30 Phase 1 执行

### 已完成

- 修复 `crates/aios-mbd/src/lib.rs` 中迁移后失效的 MBD rustdoc 相对链接。
- 修复 `crates/aios-mbd/src/iso_branch.rs` 中迁移后失效的 MBD rustdoc 相对链接。
- 删除 `iso_branch` 顶层未使用的 `angle_deg` 导入；测试中的 sanity 调用改为 `crate::iso_dim::angle_deg(...)`。

### 验证结果

- `cargo test -p aios-mbd`：通过，24 passed，未出现 `aios-mbd` warning。
- `cargo test -p aios_core --test mbd_reexport`：通过，1 passed。
- `cargo check --workspace`：通过。

### 剩余事项

- `crates/aios-mbd/*` 与 `tests/mbd_reexport.rs` 仍需在提交前确认纳入版本控制。
- `cargo test --workspace` 的历史 examples/lib-test/linker 问题仍未处理，继续按计划作为单独后续项。

## 2026-04-30 Phase 2 执行

### 已完成

- 新增 `validation.md`，固化当前 workspace 拆分 PR 的推荐阻塞命令。
- 在 `validation.md` 中记录 `cargo test --workspace` 当前不适合作为阻塞项的原因。
- 更新 `task_plan.md` 与 `findings.md`，把验收命令策略同步到规划文档。

### 验收命令策略

- 阻塞命令：
  - `cargo check --workspace`
  - `cargo test -p aios-mbd`
  - `cargo test -p aios_core --test mbd_reexport`
- 非阻塞后续项：
  - `cargo test --workspace`

### 下一步

- 进入 Phase 3：评估 `members = ["crates/*"]` 是否应收紧为显式成员，并评估 `[workspace.dependencies]`。

## 2026-04-30 Phase 3 执行

### 已完成

- 将 workspace 成员从 `members = ["crates/*"]` 收紧为 `members = ["crates/aios-mbd"]`。
- 新增 `[workspace.dependencies]`，统一声明 `glam` 和 `serde`。
- 根 crate 的 `glam` 改为 `workspace = true` 并追加 `rkyv` feature；`serde` 改为 `workspace = true`。
- `aios-mbd` 的 `serde`、`glam` 改为继承 workspace 依赖。
- 新增 `crates/aios-mbd/README.md`，记录 crate 边界、依赖方向、兼容入口和验收命令。

### 验证结果

- `cargo check --workspace`：通过。
- `cargo test -p aios-mbd`：通过，24 passed。
- `cargo test -p aios_core --test mbd_reexport`：通过，1 passed。
- `ReadLints`：相关 manifest 与 README 无 lint 错误。

### 下一步

- 进入 Phase 4：整理后续拆分候选与不建议拆分边界，形成下一轮拆分清单。

## 2026-04-30 下一阶段规划

### 已完成

- 使用 `planning-with-files` 继续沉淀下一阶段拆分方案。
- 基于代码依赖快速核对，确认 `pdms_hash`、`float_util` 是下一批更安全的纯工具候选。
- 新增 `next_split_plan.md`，记录下一阶段拆分原则、推荐顺序、任务拆分、验收标准与风险。
- 新增 `next_split_roadmap.svg`，并通过 `rsvg-convert` 校验。
- 导出 `next_split_roadmap.png`。

### 规划结论

- 下一步优先建立 `aios-pdms-core` 或 `aios-pdms-util`，只接收纯 PDMS hash 与浮点工具。
- `types/hash.rs` 暂不整体拆，只先稳定纯输入 API，根 crate 继续保留 `PlantAabb` / `Transform` 适配层。
- `rs_surreal`、`runtime`、`db_pool`、`tree_query`、`prim_geo`、`shape`、`tool/db_tool` 暂缓拆分。

### 产物

- `docs/workspace-split-plan/next_split_plan.md`
- `docs/workspace-split-plan/next_split_roadmap.svg`
- `docs/workspace-split-plan/next_split_roadmap.png`

## 2026-04-30 PDMS 基础工具拆分执行

### TDD 过程

- 先新增 `tests/pdms_core_reexport.rs`。
- 首次运行 `cargo test -p aios_core --test pdms_core_reexport` 失败，原因是 `aios_pdms_core` crate 尚不存在，符合预期 RED。
- 新增 `aios-pdms-core` 后再次运行测试并转绿。

### 已完成

- 新增 `crates/aios-pdms-core/Cargo.toml`。
- 新增 `crates/aios-pdms-core/src/lib.rs`。
- 新增 `crates/aios-pdms-core/src/pdms_hash.rs`，迁入 PDMS hash 纯函数并补单元测试。
- 新增 `crates/aios-pdms-core/src/float_util.rs`，迁入浮点规整/hash 纯函数并补单元测试。
- 新增 `crates/aios-pdms-core/README.md`。
- 根 crate 的 `src/types/pdms_hash.rs` 与 `src/types/float_util.rs` 改为 re-export。
- 根 `Cargo.toml` 增加 `crates/aios-pdms-core` workspace 成员和 path dependency。
- `ordered-float` 提升到 `[workspace.dependencies]`。
- `validation.md` 增加新 crate 与兼容性测试命令。

### 验证结果

- `cargo test -p aios-pdms-core`：通过，4 passed。
- `cargo test -p aios_core --test pdms_core_reexport`：通过，2 passed。
- `cargo check --workspace`：通过。

### 注意事项

- `cargo check --workspace` 本次耗时较长，但最终通过。
- `cargo test --workspace` 的历史 examples/lib-test/linker 问题仍按既定策略作为后续清理项。

## 2026-04-30 推荐验收命令复跑

### 已执行

- `cargo test -p aios-mbd`：通过，24 passed。
- `cargo test -p aios_core --test mbd_reexport`：通过，1 passed。
- `cargo test -p aios-pdms-core`：通过，4 passed。
- `cargo test -p aios_core --test pdms_core_reexport`：通过，2 passed。
- `cargo check --workspace`：通过。

### 结论

- MBD 拆分与 PDMS 基础工具拆分的推荐验收面均已通过。
- `cargo test --workspace` 仍不作为本轮阻塞项，原因见 `validation.md`。

## 2026-04-30 提交前清单

### 已完成

- 新增 `commit_checklist.md`，整理本次拆分应纳入提交的文件。
- 标记 `.cursor/rules/mcp-messenger.mdc` 与 `.cursor/rules/my-mcp.mdc` 为需人工确认的未跟踪项，避免混入 workspace 拆分提交。

### 当前注意事项

- `crates/aios-pdms-core/*` 与 `tests/pdms_core_reexport.rs` 仍是 untracked，需要提交时显式纳入。
- `.cursor/rules/*` 与本次拆分无直接关系，建议单独处理。

## 2026-04-30 PR 描述草稿

### 已完成

- 新增 `pr_summary.md`，整理本次 workspace 拆分的 PR 背景、变更概览、设计要点、验证结果和提交注意事项。

### 用途

- 后续创建 PR 时可直接取 `pr_summary.md` 内容作为描述基础。
- 可辅助 review 方快速理解为什么 `cargo test --workspace` 暂不作为本轮阻塞项。
