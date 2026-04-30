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
