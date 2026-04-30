# Workspace 拆分验收命令

## 推荐阻塞命令

当前 `rs-core` workspace 拆分 PR 建议先使用以下命令作为阻塞验收面：

```bash
cargo check --workspace
cargo test -p aios-mbd
cargo test -p aios_core --test mbd_reexport
cargo test -p aios-pdms-core
cargo test -p aios_core --test pdms_core_reexport
```

这些命令分别覆盖：

- workspace 级编译检查，确保根 crate 与子 crate 能一起解析、编译。
- `aios-mbd` 独立 crate 的单元测试。
- `aios_core::mbd::*` 旧路径兼容性，避免下游调用路径被拆分破坏。
- `aios-pdms-core` 独立 crate 的单元测试。
- `aios_core::types::pdms_hash::*` 与 `aios_core::types::float_util::*` 旧路径兼容性。

## 当前不建议作为阻塞项

暂不建议把下面命令作为本次 workspace 拆分 PR 的阻塞项：

```bash
cargo test --workspace
```

原因是它会编译并运行根 crate 更大的测试面，包括历史 examples 和 lib test 链接阶段。目前已观察到以下既有问题：

- 部分 `examples/*.rs` 缺少 `main` 函数。
- 部分 `examples/*.rs` 在非 async 函数中使用 `.await`。
- Rust 2024 下部分 example 直接调用 `std::env::set_var`，缺少 `unsafe` 包裹。
- 部分 example 引用已不存在或未导入的函数。
- macOS arm64 上 lib test 链接阶段出现 `aws_lc` 符号缺失。

这些问题不属于 `aios-mbd` 抽 crate 的 API 兼容性问题，建议单独立项清理。

## PR 描述建议

PR 的测试说明可以写为：

```text
已执行：
- cargo check --workspace
- cargo test -p aios-mbd
- cargo test -p aios_core --test mbd_reexport
- cargo test -p aios-pdms-core
- cargo test -p aios_core --test pdms_core_reexport

未作为阻塞项：
- cargo test --workspace

原因：
当前根 crate 历史 examples/lib-test 存在独立编译与链接问题，已记录为后续清理项。
```

## CI 收敛建议

短期 CI：

- 使用推荐阻塞命令保护本次拆分。
- 对 `cargo test --workspace` 设置为非阻塞观察项，保留日志。

中期 CI：

- 清理或隔离历史 examples。
- 将可长期维护的 examples 移到 `tests/` 或显式 feature 下。
- 修复 `aws_lc` arm64 链接配置后，再提升 `cargo test --workspace` 为阻塞项。
