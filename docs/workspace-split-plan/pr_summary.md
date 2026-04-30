# PR 描述草稿

## 背景

`rs-core` 目前仍以单 crate 承载大量领域逻辑。为了降低后续模块拆分、独立测试和跨项目复用成本，本次先建立 Cargo workspace 骨架，并选择两个边界较清晰的方向作为样板：

- MBD 出图布局纯计算逻辑。
- PDMS 基础工具函数。

## 变更概览

- 将根 `Cargo.toml` 改为 workspace，并显式纳入：
  - `crates/aios-mbd`
  - `crates/aios-pdms-core`
- 新增 `aios-mbd` crate，承载 MBD layout solver 相关 DTO 与纯计算逻辑。
- 根 crate 通过 `pub use aios_mbd as mbd` 保留 `aios_core::mbd::*` 旧路径。
- 新增 `aios-pdms-core` crate，承载 `pdms_hash` 与 `float_util` 纯工具函数。
- 根 crate 的 `src/types/pdms_hash.rs` 与 `src/types/float_util.rs` 改为 re-export，保留旧路径。
- 新增兼容性测试：
  - `tests/mbd_reexport.rs`
  - `tests/pdms_core_reexport.rs`
- 新增 workspace 拆分规划与验证文档：
  - `docs/workspace-split-plan/`

## 设计要点

- 保持依赖方向单向：`aios_core -> aios-mbd`、`aios_core -> aios-pdms-core`。
- 新 crate 不依赖 `aios_core`，避免循环依赖。
- 旧 API 路径继续可用，下游迁移可以后置。
- workspace 成员使用显式列表，避免临时目录被 `crates/*` 自动纳入。
- `glam`、`serde`、`ordered-float` 通过 `[workspace.dependencies]` 统一版本。

## 已执行验证

```bash
cargo test -p aios-mbd
cargo test -p aios_core --test mbd_reexport
cargo test -p aios-pdms-core
cargo test -p aios_core --test pdms_core_reexport
cargo check --workspace
```

验证结果：

- `aios-mbd`：24 passed。
- `mbd_reexport`：1 passed。
- `aios-pdms-core`：4 passed。
- `pdms_core_reexport`：2 passed。
- `cargo check --workspace`：通过。

## 未作为阻塞项

```bash
cargo test --workspace
```

原因：

当前根 crate 历史 examples 与 lib-test/linker 存在独立问题，包括缺少 `main`、非 async 函数中 `.await`、Rust 2024 下 `std::env::set_var` unsafe 调用、缺失函数引用，以及 macOS arm64 上 `aws_lc` 链接符号缺失。这些问题与本次两个新 crate 的兼容路径无直接关系，已记录在 `validation.md`，建议后续单独清理。

## 提交注意

需要纳入本次 PR 的新增文件包括：

- `crates/aios-mbd/*`
- `crates/aios-pdms-core/*`
- `tests/mbd_reexport.rs`
- `tests/pdms_core_reexport.rs`
- `docs/workspace-split-plan/*`

不建议混入本次 PR：

- `.cursor/rules/mcp-messenger.mdc`
- `.cursor/rules/my-mcp.mdc`
