# 移除 Tubing 的 inst_relate 创建逻辑

Tubing 已有专门的 `tubi_relate` 表，不应该再创建 `inst_relate`，否则会覆盖管件的正确数据。

## 问题分析

### 当前流程

```
save_instance_data_optimize
├── 处理 inst_info_map → 为管件创建 inst_relate（正确）
└── 处理 inst_tubi_map → 为 tubing 创建 inst_relate（错误，覆盖管件）
```

### 根因

`pdms_inst.rs` 的 `save_instance_data_optimize` 函数为 `inst_tubi_map` 中的每个条目创建 `inst_relate`，但：
1. `inst_tubi_map` 使用管件的 refno 作为 key
2. 这导致管件的正确 inst_relate 被覆盖
3. 覆盖后的 out 字段变成 `inst_info:⟨2⟩`（tubing 的 geo_hash）

### 数据表职责

| 表 | 用途 | 是否需要 |
|---|------|---------|
| `inst_relate` | 管件 → inst_info | ✓ 需要 |
| `tubi_relate` | tubing → inst_geo | ✓ 需要 |
| `inst_relate`（tubing） | tubing → inst_info | ✗ 不需要 |

## 修复方案

**移除 `inst_tubi_map` 创建 `inst_relate` 的逻辑**

修改位置：`pdms_inst.rs` 第 440-533 行

1. 移除为 `inst_tubi_map` 创建 `inst_relate` 的代码块
2. 保留 `inst_tubi_map` 处理 aabb 和 transform 的逻辑（如果需要）
3. 从 delete 列表中排除 `inst_tubi_map` 的 refno

## 验证

```bash
cargo run --bin aios-database -- --debug-model 17496_171606 --regen-model
# 查询: SELECT * FROM inst_relate WHERE in = pe:17496_171626
# 期望: out = inst_info:⟨6947897769041577988⟩
```
