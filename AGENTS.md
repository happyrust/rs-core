# Repository Guidelines

## Project Structure & Module Organization
项目代码集中在 `src/`，其中 `aios_db_mgr` 与 `query_provider` 实现核心数据库桥接，`rs_surreal` 聚合 SurrealDB 适配，而 `geometry`、`material`、`version_control` 等模块支撑三维语义；命令入口存放于 `src/bin/` 与 `examples/`，演示如 `test_unified_query` 可直连双引擎；架构与同步方案记录在 `docs/`，性能数据、夹具与输出保存在 `benches/`、`test-files/`、`test_output/`，可复用现有 `.cypher`、`.json`、`.log` 文件；资源及二进制字典集中在 `resource/` 与 `data/`。

## Build, Test, and Development Commands
仓库使用 `rust-toolchain.toml` 固定 nightly，请先运行 `cargo check` 以验证依赖；常规构建采用 `cargo build --release` 生成高性能产物，调试阶段可执行 `cargo build`；单元与特性测试通过 `cargo test` 覆盖主要模块，针对查询管线可运行 `cargo test test_query_provider -- --nocapture`；集成路径借助 `cargo run --example test_unified_query` 验证 Surreal 流程；性能基准位于 `cargo bench --bench query_provider_bench`；生成文档使用 `cargo doc --open`。

## Coding Style & Naming Conventions
核心库启用了 `#![feature(let_chains, trivial_bounds, result_flattening)]`，提交前须确保新增代码在这些 feature 下可编译；执行 `cargo fmt --all` 保持官方 `rustfmt` 风格并遵循 4 空格缩进；模块与文件命名使用 `snake_case`（例如 `data_center.rs`），类型使用 `CamelCase`（如 `PdmsDatabaseInfo`），常量以 `SCREAMING_SNAKE_CASE` 命名；特性相关逻辑需通过 `#[cfg(feature = "live")]` 等条件编译明确包裹。

**外部系统命名约定**：AVEVA PDMS/E3D 系统的内部缩写和数据结构命名请参考 `docs/attlib_naming_conventions.md`。

## SurrealDB Types & Query Patterns
项目使用 SurrealDB 作为主要数据库，类型系统与查询模式遵循以下规范：

**类型别名与导入**：
- `surrealdb_types` 仅为 `surrealdb::types` 的模块别名
- 应始终使用完整路径 `use surrealdb::types::SurrealValue;` 导入核心值类型
- 查询接口通过 `SurrealQueryExt` trait 扩展 `Surreal<Any>`

**查询方法规范**：
- **禁止**直接使用 `.query().await?.take()` 并 unwrap，必须使用项目提供的扩展方法
- **推荐**使用 `query_take::<T>(sql, index)` 执行查询并反序列化第 `index` 个结果
- 使用 `query_response(sql)` 获取完整的 `Response` 对象以便多结果处理
- 所有查询方法已集成 `#[track_caller]` 实现精确错误定位

**类型约束与转换**：
- 查询目标类型 `T` 必须满足 `T: SurrealValue` 和 `usize: SurrealQueryResult<T>`
- 反序列化失败会通过 `anyhow::Error` 传播，并附带 SQL 语句和调用位置信息
- 优先使用具体类型（如 `Vec<RefNo>`）而非手动解析 `SurrealValue` 枚举

**使用示例**：
```rust
use crate::rs_surreal::query_ext::SurrealQueryExt;

// 单结果查询
let result: Vec<RefNo> = db.query_take("SELECT REFNO FROM pe WHERE noun = 'SITE'", 0).await?;

// 多结果查询
let response = db.query_response("SELECT * FROM pe LIMIT 10; SELECT count() FROM pe;").await?;
let data: Vec<PeData> = response.take(0)?;
let count: i64 = response.take(1)?;
```

## Testing Guidelines
测试框架依赖 Cargo 内建机制，建议在本地同时执行 `cargo test` 与 `cargo test --lib` 对比输出；针对数据库差异，可运行 `cargo run --example test_unified_query` 并比对 `surreal_perf.log`；独立模块可使用 `cargo test test_memory_database_init`、`cargo test test_gensec_spine -- --nocapture` 等现有命令作为模板；新增集成夹具放入 `test-files/`，输出日志放入 `test_output/`，文件命名遵循 `test_模块_场景.log` 以便归档。

## Commit & Pull Request Guidelines
历史记录采用类 Conventional Commit 规范，如 `feat: 重构 surreal 查询缓存`、`test: add simplified RefnoEnum tests`，建议继续使用 `feat|fix|refactor|test|docs` 前缀描述范围；提交信息需概括影响模块与动机，并在需要时引用 issue 或阶段性文档；创建 PR 前请附测试命令列表、关键日志或截图（可引用 `docs/`、`test_output/` 中的材料），确认夜间与 release 构建均通过；若改动影响 `examples/` 或外部脚本，请在说明中标注并更新相应使用文档。

## Configuration & Safety Notes
连接配置位于 `DbOption.toml` 与 `DbOption_*.toml`，请使用本地副本并避免提交真实凭据；示例依赖的资产与中间结果存放在 `resource/`、`data/`、`all_attr_info.*`，拉取前确认体积较大的二进制已同步；日志与性能对比文件建议留在忽略目录，避免污染仓库，同时注意清理 `target/` 以减少版本库噪音。
