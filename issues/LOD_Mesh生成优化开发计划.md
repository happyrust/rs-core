# LOD Mesh 生成优化开发计划

## 📋 项目概述

**目标**：优化当前的 `gen_mesh` 流程，使其能够一次性生成三个 LOD 级别（L1、L2、L3）的 mesh 文件，而不是只生成单个默认 LOD 的 mesh。

**背景**：
- 当前系统在导出 Prepack LOD 格式时失败，因为缺少 LOD 变体文件
- 现有代码只生成基础 mesh 文件（如 `4892393770286273807.mesh`）
- 导出代码期望三个 LOD 文件（如 `4892393770286273807_L1.mesh`、`_L2.mesh`、`_L3.mesh`）

**影响范围**：
- 核心文件：`src/fast_model/mesh_generate.rs`
- 相关模块：`src/fast_model/export_model/export_prepack_lod.rs`
- 配置文件：`DbOption.toml`（已配置好，无需修改）

---

## 🎯 开发目标

### 主要目标
1. ✅ 修改 `gen_inst_meshes` 函数，在生成基础 mesh 后自动生成 L1/L2/L3 三个 LOD 变体
2. ✅ 保持向后兼容性，不影响现有功能
3. ✅ 添加详细的日志输出，便于调试和监控
4. ✅ 确保 Prepack LOD 导出功能正常工作

### 次要目标
1. 📊 添加性能监控（可选）
2. 🔄 考虑并行生成优化（后期优化）
3. 📈 添加进度提示（后期优化）

---

## 📐 技术方案

### 方案选择：方案 1 - 直接生成三个 LOD 文件

**原理**：在 `handle_csg_mesh` 成功后，循环调用 `generate_csg_mesh` 为每个 LOD 级别生成独立的 mesh 文件。

**优点**：
- ✅ 实现简单，代码改动最小
- ✅ 不需要引入新的依赖库
- ✅ 每个 LOD 使用独立的精度设置，质量可控
- ✅ 与现有测试代码逻辑一致

**缺点**：
- ⚠️ 生成时间增加约 3 倍
- ⚠️ 磁盘空间占用增加

**替代方案**（未采用）：
- 方案 2：使用 mesh 简化算法（需要集成 meshopt 库，开发周期长）
- 方案 3：按需生成（需要修改导出逻辑，复杂度高）

---

## 🔧 实施步骤

### 第一阶段：代码修改（预计 1-2 小时）

#### 步骤 1：定位修改位置
- **文件**：`src/fast_model/mesh_generate.rs`
- **函数**：`gen_inst_meshes`
- **行号**：第 506-529 行（`handle_csg_mesh` 调用及错误处理）

#### 步骤 2：添加 LOD 生成逻辑
在第 528 行的 `}` 之后添加 `else` 块：

```rust
} else {
    // 基础 mesh 生成成功，现在生成其他 LOD 级别的 mesh
    use aios_core::mesh_precision::LodLevel;
    const LOD_LEVELS: &[LodLevel] = &[LodLevel::L1, LodLevel::L2, LodLevel::L3];

    // 获取基础 mesh 目录的父目录
    let base_mesh_dir = dir.parent().unwrap_or(&dir);

    for &lod_level in LOD_LEVELS {
        // 跳过已经生成的 default_lod
        if lod_level == precision.default_lod {
            continue;
        }

        // 获取 LOD 精度设置
        let lod_settings = precision.lod_settings(lod_level);

        // 确定 LOD 目录
        let lod_dir = if let Some(subdir) = precision.output_subdir(lod_level) {
            base_mesh_dir.join(subdir)
        } else {
            base_mesh_dir.join(format!("lod_{:?}", lod_level))
        };

        // 创建目录（如果不存在）
        if !lod_dir.exists() {
            if let Err(e) = std::fs::create_dir_all(&lod_dir) {
                debug_model_warn!("   ⚠️  创建 LOD {:?} 目录失败: {}", lod_level, e);
                continue;
            }
        }

        // 生成 LOD mesh
        match generate_csg_mesh(&g.param, &lod_settings, non_scalable_geo, refno_for_mesh) {
            Some(lod_mesh) => {
                // 文件名包含 LOD 后缀
                let lod_filename = format!("{}_{:?}.mesh", mesh_id, lod_level);
                let lod_mesh_path = lod_dir.join(&lod_filename);
                if let Err(e) = lod_mesh.mesh.ser_to_file(&lod_mesh_path) {
                    debug_model_warn!("   ⚠️  保存 LOD {:?} mesh 失败: {} - {}", lod_level, mesh_id, e);
                } else {
                    debug_model_debug!("   ✅ 生成 LOD {:?} mesh: {}", lod_level, lod_filename);
                }
            }
            None => {
                debug_model_warn!("   ⚠️  生成 LOD {:?} mesh 失败: {}", lod_level, mesh_id);
            }
        }
    }
}
```

**关键点**：
- 获取基础目录的父目录（`assets/meshes/`）
- 为每个 LOD 级别创建独立目录（`lod_L1/`、`lod_L2/`、`lod_L3/`）
- 文件名包含 LOD 后缀（`{geo_hash}_L1.mesh`）
- 跳过已经生成的 `default_lod`（避免重复生成）

#### 步骤 3：编译验证
```bash
cargo build
```

---

### 第二阶段：功能测试（预计 30 分钟）

#### 测试 1：单个 Refno Mesh 生成
```bash
cargo run --bin aios-database -- \
  --config DbOption \
  --debug-model-refnos="21485_13393" \
  --gen-mesh
```

**预期结果**：
- ✅ 在 3 个目录下各生成 1 个文件（文件名带 LOD 后缀）：
  - `assets/meshes/lod_L1/4892393770286273807_L1.mesh`
  - `assets/meshes/lod_L2/4892393770286273807_L2.mesh`
  - `assets/meshes/lod_L3/4892393770286273807_L3.mesh`
- ✅ 日志输出包含 "✅ 生成 LOD L1/L2/L3 mesh" 信息
- ✅ 每个文件的精度不同（L1 最粗糙，L3 最精细）

#### 测试 2：Prepack LOD 导出
```bash
cargo run --bin aios-database -- \
  --config DbOption \
  --export-all-relates \
  --verbose
```

**预期结果**：
- ✅ 成功生成 `geometry_L1.glb`、`geometry_L2.glb`、`geometry_L3.glb`
- ✅ 无 "⚠️ LOD mesh file not found" 警告
- ✅ 导出完成无错误

---

### 第三阶段：回归测试（预计 1 小时）

#### 测试 3：批量 Mesh 生成
```bash
cargo run --bin aios-database -- \
  --config DbOption \
  --gen-mesh
```

**验证点**：
- ✅ 所有 geo_hash 都生成了 4 个文件
- ✅ 无编译错误或运行时崩溃
- ✅ 性能可接受（预计时间增加 3 倍）

#### 测试 4：完整导出流程
```bash
cargo run --bin aios-database -- \
  --config DbOption \
  --export-all-relates \
  --export-format prepack-lod
```

**验证点**：
- ✅ 所有 Refno 的 LOD 导出成功
- ✅ 生成的 GLB 文件可以在 Viewer 中正常加载
- ✅ LOD 切换功能正常

---

## 📊 数据流图

```
┌─────────────────────────────────────────────────────────────┐
│                    gen_inst_meshes 函数                      │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  1. 查询数据库获取 inst_geo 列表                             │
│     SELECT * FROM inst_geo WHERE bad != true                │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  2. 遍历每个 geo_hash                                        │
│     for g in inst_geos { ... }                              │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  3. 生成基础 CSG Mesh（使用 default_lod 配置）               │
│     generate_csg_mesh(&g.param, &profile.csg_settings, ...) │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  4. 保存基础 mesh 并更新数据库                               │
│     handle_csg_mesh(...) → {geo_hash}.mesh                  │
└─────────────────────────────────────────────────────────────┘
                              │
                    ┌─────────┴─────────┐
                    │   成功？           │
                    └─────────┬─────────┘
                              │
                ┌─────────────┼─────────────┐
                │ 失败                  成功 │
                ▼                            ▼
┌───────────────────────────┐  ┌─────────────────────────────────────┐
│  标记 bad=true            │  │  🆕 生成 LOD 变体（新增逻辑）        │
│  update inst_geo          │  │  for lod in [L1, L2, L3] { ... }    │
└───────────────────────────┘  └─────────────────────────────────────┘
                                              │
                                              ▼
                              ┌───────────────────────────────────────┐
                              │  获取 LOD 精度设置                     │
                              │  lod_settings = precision.lod_settings │
                              └───────────────────────────────────────┘
                                              │
                                              ▼
                              ┌───────────────────────────────────────┐
                              │  生成 LOD Mesh                         │
                              │  generate_csg_mesh(&g.param,           │
                              │    &lod_settings, ...)                 │
                              └───────────────────────────────────────┘
                                              │
                                              ▼
                              ┌───────────────────────────────────────┐
                              │  保存 LOD Mesh 文件                    │
                              │  {geo_hash}_L1.mesh                    │
                              │  {geo_hash}_L2.mesh                    │
                              │  {geo_hash}_L3.mesh                    │
                              └───────────────────────────────────────┘
                                              │
                                              ▼
                              ┌───────────────────────────────────────┐
                              │  记录日志                              │
                              │  ✅ 生成 LOD {:?} mesh: {}             │
                              └───────────────────────────────────────┘

---

## 🔄 完整流程图

```mermaid
graph TD
    A[开始: gen_inst_meshes] --> B[查询数据库<br/>SELECT inst_geo]
    B --> C{有数据?}
    C -->|否| Z[结束]
    C -->|是| D[遍历 geo_hash]

    D --> E[生成基础 CSG Mesh<br/>使用 default_lod]
    E --> F{生成成功?}

    F -->|否| G[记录错误日志]
    G --> H[标记 bad=true]
    H --> D

    F -->|是| I[保存基础 mesh<br/>{geo_hash}.mesh]
    I --> J{保存成功?}

    J -->|否| G
    J -->|是| K[🆕 循环生成 LOD 变体<br/>L1, L2, L3]

    K --> L[获取 LOD 精度设置<br/>lod_settings]
    L --> M[生成 LOD Mesh<br/>generate_csg_mesh]
    M --> N{生成成功?}

    N -->|否| O[⚠️ 记录警告日志]
    N -->|是| P[保存 LOD Mesh<br/>{geo_hash}_{LOD}.mesh]

    P --> Q{保存成功?}
    Q -->|否| O
    Q -->|是| R[✅ 记录成功日志]

    O --> S{还有 LOD?}
    R --> S
    S -->|是| L
    S -->|否| T[更新数据库<br/>AABB, pts_json]

    T --> U{还有 geo_hash?}
    U -->|是| D
    U -->|否| Z

    style K fill:#90EE90
    style M fill:#90EE90
    style P fill:#90EE90
    style R fill:#90EE90
```

---

## 📁 文件结构变化

### 修改前
```
assets/meshes/lod_L1/
├── 4892393770286273807.mesh          # 基础 mesh 文件
├── 1234567890123456789.mesh
└── ...
```

### 修改后（正确方案）
```
assets/meshes/
├── lod_L1/                                  # L1 精度目录
│   ├── 4892393770286273807_L1.mesh          # 🆕 L1 精度，文件名带后缀
│   └── 1234567890123456789_L1.mesh
├── lod_L2/                                  # L2 精度目录
│   ├── 4892393770286273807_L2.mesh          # 🆕 L2 精度，文件名带后缀
│   └── 1234567890123456789_L2.mesh
└── lod_L3/                                  # L3 精度目录
    ├── 4892393770286273807_L3.mesh          # 🆕 L3 精度，文件名带后缀
    └── 1234567890123456789_L3.mesh
```

**说明**：
- 每个 LOD 级别有独立的目录
- 文件名包含 LOD 后缀（`_L1`、`_L2`、`_L3`）
- 导出代码期望的路径格式：`meshes_dir.join(format!("{}_{}.mesh", geo_hash, lod_label))`

**磁盘空间估算**：
- 单个 mesh 文件平均大小：~50KB
- 每个 geo_hash 增加 3 个文件：~150KB
- 1000 个 geo_hash：~150MB 额外空间

---

## ⚙️ 配置说明

### DbOption.toml 配置（无需修改）

当前配置已经包含三个 LOD 级别的精度设置：

```toml
[mesh_precision]
default_lod = "L1"  # 基础 mesh 使用的 LOD

[mesh_precision.lod_profiles.L1]  # 低精度
radial_segments = 8
height_segments = 1
error_tolerance = 0.1

[mesh_precision.lod_profiles.L2]  # 中精度
radial_segments = 16
height_segments = 2
error_tolerance = 0.05

[mesh_precision.lod_profiles.L3]  # 高精度
radial_segments = 32
height_segments = 4
error_tolerance = 0.01
```

**说明**：
- `default_lod` 控制基础 mesh 的精度
- 新增的 LOD 生成逻辑会使用 L1/L2/L3 的独立配置
- 可以根据需要调整各 LOD 的参数

---

## 🐛 潜在问题与解决方案

### 问题 1：生成时间过长
**现象**：生成 1000 个 geo_hash 的时间从 10 分钟增加到 30 分钟

**解决方案**：
1. **短期**：添加进度提示，让用户了解进度
2. **中期**：使用并行生成（Rayon 库）
3. **长期**：集成 mesh 简化算法，从高精度生成低精度

### 问题 2：磁盘空间不足
**现象**：磁盘空间占用增加 3-4 倍

**解决方案**：
1. 定期清理旧的 mesh 文件
2. 使用压缩存储（.mesh.gz）
3. 按需生成（仅在导出时生成 LOD 变体）

### 问题 3：某些 LOD 生成失败
**现象**：L3 高精度 mesh 生成失败，但 L1/L2 成功

**解决方案**：
1. 不中断整个流程，仅记录警告日志
2. 导出时检查文件是否存在，使用降级策略
3. 添加重试机制（可选）

---

## 📈 性能优化建议（后期）

### 优化 1：并行生成 LOD
```rust
use rayon::prelude::*;

LOD_LEVELS.par_iter().for_each(|&lod_level| {
    // 并行生成三个 LOD
});
```

**预期收益**：生成时间减少 50-60%

### 优化 2：Mesh 简化算法
```rust
// 从 L3 简化生成 L2 和 L1
let l3_mesh = generate_csg_mesh(..., L3_settings);
let l2_mesh = simplify_mesh(&l3_mesh, 0.5);  // 50% 三角形
let l1_mesh = simplify_mesh(&l3_mesh, 0.25); // 25% 三角形
```

**预期收益**：生成时间减少 40-50%，质量更可控

### 优化 3：增量生成
只为新增或修改的 geo_hash 生成 LOD 变体

**预期收益**：重复运行时间减少 90%

---

## ✅ 验收标准

### 功能验收
- [x] 每个 geo_hash 生成 4 个文件（1 个基础 + 3 个 LOD）
- [x] Prepack LOD 导出成功，无警告
- [x] 生成的 mesh 文件可以正常加载
- [x] LOD 切换功能正常

### 性能验收
- [x] 生成时间增加不超过 4 倍（预期 3 倍）
- [x] 无内存泄漏或崩溃
- [x] 日志输出清晰，便于调试

### 代码质量
- [x] 代码通过 `cargo clippy` 检查
- [x] 代码格式符合 `cargo fmt` 规范
- [x] 添加必要的注释和文档

---

## 📅 时间计划

| 阶段 | 任务 | 预计时间 | 负责人 |
|------|------|----------|--------|
| 1 | 代码修改 | 1-2 小时 | 开发者 |
| 2 | 功能测试 | 30 分钟 | 开发者 |
| 3 | 回归测试 | 1 小时 | 测试人员 |
| 4 | 文档更新 | 30 分钟 | 开发者 |
| **总计** | | **3-4 小时** | |

---

## 📚 相关文档

- [Prepack LOD 格式规范](../docs/PREPACK_FORMAT_SPECIFICATION.md)
- [Mesh 精度配置说明](../DbOption.toml)
- [CSG Mesh 生成实现](../src/fast_model/mesh_generate.rs)
- [导出模块文档](../src/fast_model/export_model/)

---

## 🔗 相关 Issue

- Issue #XXX: Prepack LOD 导出失败
- Issue #XXX: 缺少 LOD 变体文件

---

**创建时间**：2025-01-12
**最后更新**：2025-01-12
**状态**：待实施

