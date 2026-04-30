# rs-core Workspace 拆分开发方案

## 目标

将 `rs-core` 从单 crate 逐步演进为 workspace，先把 MBD 出图布局逻辑拆成独立 `aios-mbd` crate，同时保持 `aios_core::mbd::*` 兼容路径，降低后续模块解耦、测试和复用成本。

## 当前状态

- 状态：已完成 MBD 与 PDMS 基础工具首轮拆分
- 当前 worktree：`/Volumes/DPC/work/plant-code/rs-core-ws-split`
- 已有拆分：`crates/aios-mbd`
- 根 crate：`aios_core`
- 兼容策略：`src/lib.rs` 中 `pub use aios_mbd as mbd`

## 阶段计划

### Phase 1：收敛当前 MBD 拆分

- [x] 修复 `aios-mbd` 内 rustdoc 相对链接，把残留 `../../MBD/...` 调整为 `../../../MBD/...`。
- [x] 删除 `crates/aios-mbd/src/iso_branch.rs` 中未使用的 `angle_deg` 导入，保证新 crate 零 warning。
- [ ] 确认 `crates/aios-mbd` 所有新文件纳入版本控制，避免只提交删除 `src/mbd` 而漏提交新 crate。
- [ ] 保留 `tests/mbd_reexport.rs` 作为下游兼容性防线。

### Phase 2：明确 workspace 验收命令

- [x] 把当前可通过命令写入 PR 描述或开发文档：
  - `cargo check --workspace`
  - `cargo test -p aios-mbd`
  - `cargo test -p aios_core --test mbd_reexport`
- [x] 暂不把 `cargo test --workspace` 作为阻塞项，先记录根 crate 既有 examples/lib-test/linker 问题。
- [ ] 后续单独清理 examples 或配置 CI 只跑稳定目标。

### Phase 3：完善 workspace 结构

- [x] 评估并把 `members = ["crates/*"]` 收紧为显式成员 `["crates/aios-mbd"]`，避免未来临时目录误入 workspace。
- [x] 引入 `[workspace.dependencies]` 统一 `glam`、`serde` 等共享依赖版本。
- [x] 为 `aios-mbd` 补齐 crate 边界说明，包括输入 DTO、输出 DTO、与 `aios_core` 的依赖方向。

### Phase 4：后续拆分候选

- [x] 从依赖最少、业务边界清晰的模块继续规划拆分候选。
- [x] 明确每次拆分必须满足：无反向依赖、保留旧 API re-export、补兼容性测试、最小可验证命令通过。
- [x] 明确暂不拆数据库、运行时、全局配置相关模块，避免过早引入跨 crate 初始化和 feature 复杂度。

### Phase 5：执行 PDMS 基础工具拆分

- [x] 先添加 `pdms_core_reexport` 兼容性测试并确认 RED。
- [x] 新增 `crates/aios-pdms-core`。
- [x] 将 `pdms_hash` 与 `float_util` 纯函数迁移到新 crate。
- [x] 根 crate 保留 `aios_core::types::pdms_hash::*` 与 `aios_core::types::float_util::*` 兼容路径。
- [x] 更新 workspace 成员和共享依赖。
- [x] 运行新 crate、兼容性测试和 workspace check。

## 验收标准

- `aios-mbd` 可独立 `cargo test -p aios-mbd`。
- `aios_core::mbd::*` 下游路径保持可用。
- 根 crate 可 `cargo check --workspace`。
- 新增 workspace 结构不引入循环依赖。
- CI/PR 文档明确哪些失败是既有根 crate 问题，哪些是本次拆分必须修复的问题。

## 风险控制

- 不一次性移动大批模块，先以 MBD 作为样板。
- 不在拆分 PR 中混入业务行为重写。
- 对外 API 通过 re-export 兼容，下游迁移可以后置。
- 对历史 examples 的清理单独立项，不阻塞 workspace 骨架落地。
