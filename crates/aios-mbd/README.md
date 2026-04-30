# aios-mbd

`aios-mbd` 承载 MBD 出图布局计算中可以脱离 `aios_core` 的纯计算部分。

## 边界

本 crate 只负责：

- 接收已经规整好的 MBD 布局输入 DTO。
- 计算线性尺寸、切管尺寸、焊口、坡度、弯头、标签等布局结果。
- 输出可序列化的布局结果 DTO。
- 保持内部算法与 PML `iso*` 对象语义对齐。

本 crate 不负责：

- 数据库连接、SurrealDB 查询或缓存初始化。
- PDMS 原始数据读取。
- 全局配置、运行时启动、文件路径解析。
- 几何体生成、mesh 生成或 CSG 布尔计算。

## 依赖方向

依赖方向必须保持单向：

```text
aios_core -> aios-mbd
```

`aios-mbd` 不应依赖 `aios_core`。需要从 `aios_core` 传入的数据，应先转换为本 crate 定义的输入结构，例如 `SegmentInput`、`BranchContext`、`IsoParams`、`SlopeInput`、`WeldInput`、`TagInput`、`BendInput`。

## 兼容入口

下游仍可通过根 crate 旧路径访问：

```rust
use aios_core::mbd::{BranchCalculator, LayoutRequest, SolveBranchInput};
```

该兼容路径由 `aios_core` 中的 re-export 提供：

```rust
pub use aios_mbd as mbd;
```

## 验收命令

修改本 crate 后至少运行：

```bash
cargo test -p aios-mbd
cargo test -p aios_core --test mbd_reexport
cargo check --workspace
```
