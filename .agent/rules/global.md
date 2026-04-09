---
trigger: always_on
---

**核心开发约定**

> **调试模式优先**：默认使用 debug 模式，不要编译 release。
> **不要清理缓存**：不要使用 `cargo clean`。
> **禁止默认跑 cargo test**：Rust 侧验证优先使用 CLI、样例程序、`curl`/HTTP JSON `GET`/`POST`、真实接口回路；只有用户明确要求时才允许运行 `cargo test`。
> **最小化编译范围**：如需确认可编译，优先使用最小范围的 `cargo check`，或通过上游仓的 debug `web_server` 增量编译带动验证。

**MBD / old PML 迁移约定**

> **事实源**：MBD 布局迁移优先对齐 old PML 的 `BranchCalculator` / `isobran` 语义。
> **失败要 suppress**：布局或几何不成立时输出 suppress reason，不要硬画错误标注。
> **弯头一致性**：ELBO/BEND 的尺寸、角度、标签应与所属直段共用偏移基线与方向系统。
