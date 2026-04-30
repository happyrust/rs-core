# Workspace 拆分 Findings

## 已验证事实

- `cargo check --workspace` 可以通过；修复 `iso_branch` unused import 后已无 `aios-mbd` warning。
- `cargo test -p aios-mbd` 可以通过，结果为 24 passed。
- `cargo test -p aios_core --test mbd_reexport` 可以通过，验证 `aios_core::mbd::*` 兼容路径。
- `cargo test --workspace` 当前失败，失败点集中在根 crate 的 examples、lib test 链接阶段，与 `aios-mbd` 对外路径兼容性无直接关系；详细验收边界已记录在 `validation.md`。

## 设计结论

- MBD 是适合第一批拆分的模块：当前 `crates/aios-mbd` 只依赖 `serde` 和 `glam`，没有依赖 `aios_core`，依赖方向健康。
- `pub use aios_mbd as mbd` 是低风险兼容策略，可以让下游继续使用 `aios_core::mbd::*`。
- `tests/mbd_reexport.rs` 是必要的兼容性测试，应随拆分一起提交。

## 发现的问题

- `crates/aios-mbd/src/lib.rs` 和 `crates/aios-mbd/src/iso_branch.rs` 原有 `../../MBD/...` 文档链接已修正为 `../../../MBD/...`。
- `crates/aios-mbd/src/iso_branch.rs` 原有未使用的 `angle_deg` 导入已删除，测试内改为显式 `crate::iso_dim::angle_deg(...)`。
- `crates/aios-mbd/*` 与 `tests/mbd_reexport.rs` 当前是 untracked 文件，提交时必须确认纳入版本控制。
- `Cargo.lock` 被 `.gitignore` 忽略，workspace 依赖变化不会通过 lockfile 固化；这是仓库既有策略，但需要在 CI 里用稳定命令兜底。

## 验收策略

- 本次 workspace 拆分的阻塞命令应使用 `validation.md` 中列出的三条稳定命令。
- `cargo test --workspace` 应作为后续清理目标，而不是本次拆分 PR 的失败判定标准。

## Phase 3 结构决策

- `members = ["crates/*"]` 已收紧为 `members = ["crates/aios-mbd"]`，避免临时实验目录或未来未准备好的 crate 被 Cargo 自动纳入 workspace。
- `glam` 与 `serde` 已提升到 `[workspace.dependencies]`。根 crate 的 `glam` 通过 `workspace = true` 继承并追加 `rkyv` feature；`aios-mbd` 继承基础 `serde` feature，保持依赖面较窄。
- `crates/aios-mbd/README.md` 已补充 crate 边界、依赖方向、兼容入口和验收命令。

## 下一阶段候选调研

- `src/types/pdms_hash.rs` 未发现 `crate::`、`surrealdb`、`tokio`、`DashMap`、`config` 依赖，是纯 PDMS hash 算法候选。
- `src/types/float_util.rs` 未发现 `crate::`、`surrealdb`、`tokio`、`DashMap`、`config` 依赖，是纯浮点规整/hash 工具候选。
- `src/types/hash.rs` 仍引用 `crate::types::PlantAabb` 与 `crate::plant_transform::Transform`，不宜整体直接拆；可先抽出底层 `gen_trs_hash` / 数字格式化策略。
- `src/prim_geo/attmap_csg.rs` 依赖多个 `prim_geo::*` shape、`BrepShapeTrait`、`AttrMap`、`NamedAttrMap`，应等几何/属性 DTO 边界清楚后再拆。
- `src/tool/db_tool.rs` 依赖 `PdmsDatabaseInfo`、全局 UDA map、文件读取和 `types::pdms_hash` re-export，应保留在根 crate 或拆成更晚的 IO/配置层。

## PDMS 基础工具拆分结果

- 已新增 `crates/aios-pdms-core`，当前只包含 `pdms_hash` 与 `float_util` 两个纯工具模块。
- 根 crate 的 `src/types/pdms_hash.rs` 与 `src/types/float_util.rs` 已改为 re-export 新 crate，保留旧调用路径。
- 新增 `tests/pdms_core_reexport.rs`，先验证 RED，再通过新 crate 实现转绿。
- `ordered-float` 已提升到 `[workspace.dependencies]`，由根 crate 和 `aios-pdms-core` 共享。
- `tool/db_tool` 暂未迁移，只继续调用 `crate::types::pdms_hash` 兼容层，保留缓存和文件 IO 责任在根 crate。

## 不建议现在做的事

- 不建议在同一个 PR 中修复所有历史 examples。
- 不建议马上拆数据库、SurrealDB runtime、全局配置模块。
- 不建议移除 `aios_core::mbd` 兼容路径。
