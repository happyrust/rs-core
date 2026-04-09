# Repository Guidelines

## 构建与调试

- 默认使用 debug 模式，不要编译 release
- 不要使用 `cargo clean`
- 默认不要运行 `cargo test`
- Rust 侧验证优先使用 CLI、样例程序、`curl`/HTTP JSON `GET`/`POST`、真实接口回路
- 如需确认可编译，优先使用最小范围的 `cargo check` 或由上游仓的 debug `web_server` 增量编译带动验证；只有用户明确要求时才允许运行 `cargo test`

## MBD / old PML 迁移约定

- MBD 迁移以 old PML 的 branch-level solver 语义为事实源，优先对齐 `BranchCalculator`
- 布局失败时应输出 suppress 原因，不要硬画错误标注
- ELBO/BEND 的尺寸、角度、标签应与所属直段共用偏移基线与方向系统
