# aios_core Workspace 拆分计划

## 背景

- 单体 crate，70+ pub mod，60+ 外部依赖，target 15GB
- 改任何一行重编整个 lib ~15s（不含锁等待）
- 多 cargo 并行时互抢文件锁，编译时间翻倍

## 依赖耦合扫描结果

| 模块 | 内部依赖 | 外部 crate 依赖 | 可提取性 |
|------|---------|----------------|---------|
| `mbd` (2051行) | 无 (纯自引用) | serde, glam | **极易** |
| `math` (135行) | 无 | anyhow | 太小，不值得 |
| `geometry`+`prim_geo` | 双向依赖，需一起提 | glam, nalgebra, parry3d | 中等 |
| `rs_surreal`+`query_provider` | SUL_DB, types | surrealdb | 较难 |
| `types` | consts, helper, orm | serde, smol_str | 最难 (核心) |

## 下游使用 mbd 的地方

- `plant-model-gen/src/web_api/mbd_pipe_api.rs` — 通过 `aios_core::mbd::*` 引用
- rs-core 内部无其他模块引用 mbd

## 执行计划

### Phase 0: worktree 准备
```bash
git worktree add ../rs-core-ws-split feat/workspace-split
```

### Phase 1: 提取 aios-mbd（预计 1h）

1. 根 Cargo.toml 加 `[workspace]`
2. 建 `crates/aios-mbd/Cargo.toml`，依赖 serde + glam
3. `src/mbd/` 全部移动到 `crates/aios-mbd/src/`
4. 根 crate 加 `aios-mbd` 为 dependency
5. `lib.rs` 中 `pub mod mbd;` → `pub use aios_mbd as mbd;`
6. 下游零修改（路径 `aios_core::mbd::*` 不变）
7. 运行 `cargo test -p aios-mbd` + `cargo test -p aios_core -- mbd`

### Phase 2: 评估后续（按需）

根据 Phase 1 效果决定是否继续拆 geometry/prim_geo 等。

## 预期收益

- 改 mbd 代码：编译 ~2s（vs 当前 ~15s）
- mbd 独立 CI 测试
- 为后续拆分建立模式
