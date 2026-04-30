# Workspace 拆分提交清单

## 本次拆分应纳入提交

### Workspace 与依赖

- `Cargo.toml`

### MBD 拆分收敛

- `crates/aios-mbd/Cargo.toml`
- `crates/aios-mbd/README.md`
- `crates/aios-mbd/src/lib.rs`
- `crates/aios-mbd/src/iso_branch.rs`
- `crates/aios-mbd/src/iso_dim.rs`
- `crates/aios-mbd/src/iso_extras.rs`
- `crates/aios-mbd/src/iso_params.rs`
- `tests/mbd_reexport.rs`

### PDMS 基础工具拆分

- `crates/aios-pdms-core/Cargo.toml`
- `crates/aios-pdms-core/README.md`
- `crates/aios-pdms-core/src/lib.rs`
- `crates/aios-pdms-core/src/pdms_hash.rs`
- `crates/aios-pdms-core/src/float_util.rs`
- `src/types/pdms_hash.rs`
- `src/types/float_util.rs`
- `tests/pdms_core_reexport.rs`

### 规划与验证文档

- `docs/workspace-split-plan/task_plan.md`
- `docs/workspace-split-plan/findings.md`
- `docs/workspace-split-plan/progress.md`
- `docs/workspace-split-plan/validation.md`
- `docs/workspace-split-plan/next_split_plan.md`
- `docs/workspace-split-plan/workspace_split_architecture.svg`
- `docs/workspace-split-plan/workspace_split_architecture.png`
- `docs/workspace-split-plan/next_split_roadmap.svg`
- `docs/workspace-split-plan/next_split_roadmap.png`
- `docs/workspace-split-plan/commit_checklist.md`

## 本次提交前需要人工确认

- `.cursor/rules/mcp-messenger.mdc`
- `.cursor/rules/my-mcp.mdc`

这两个文件当前是 untracked，且不属于 workspace 拆分代码路径。除非明确希望把本地 Cursor 规则纳入仓库，否则建议不要混入本次拆分提交。

## 推荐最终验证命令

```bash
cargo test -p aios-mbd
cargo test -p aios_core --test mbd_reexport
cargo test -p aios-pdms-core
cargo test -p aios_core --test pdms_core_reexport
cargo check --workspace
```

## 已知非阻塞项

- `cargo test --workspace` 当前仍会触发根 crate 历史 examples 与 lib-test/linker 问题，详见 `validation.md`。
