# aios-pdms-core

`aios-pdms-core` 承载可脱离 `aios_core` 的 PDMS 基础工具函数。

## 当前范围

本 crate 只包含纯计算工具：

- `pdms_hash`：PDMS 属性名 hash / dehash 与 UDA 判断。
- `float_util`：三位小数规整与基于规整值的 hash 辅助函数。

## 边界

本 crate 不负责：

- 读取属性配置文件。
- 维护全局 UDA 缓存。
- 数据库连接或 SurrealDB 查询。
- `PlantAabb`、`Transform` 等根 crate 类型适配。

这些职责继续留在 `aios_core`，通过适配层调用本 crate 的纯函数。

## 依赖方向

依赖方向必须保持单向：

```text
aios_core -> aios-pdms-core
```

`aios-pdms-core` 不应依赖 `aios_core`。

## 兼容入口

旧路径通过 `aios_core` 继续可用：

```rust
aios_core::types::pdms_hash::db1_hash("SCTN");
aios_core::types::float_util::f32_round_3(1.23456);
```

新 crate 也可直接使用：

```rust
aios_pdms_core::pdms_hash::db1_hash("SCTN");
aios_pdms_core::float_util::f32_round_3(1.23456);
```

## 验收命令

修改本 crate 后至少运行：

```bash
cargo test -p aios-pdms-core
cargo test -p aios_core --test pdms_core_reexport
cargo check --workspace
```
