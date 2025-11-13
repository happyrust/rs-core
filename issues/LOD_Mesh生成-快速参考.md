# LOD Mesh 生成优化 - 快速参考卡片

## 🎯 一句话总结
在 `mesh_generate.rs` 第 528 行后添加 `else` 块，为每个 LOD 级别在独立目录下生成 mesh 文件。

---

## 📍 修改位置

**文件**：`src/fast_model/mesh_generate.rs`  
**行号**：第 528 行之后  
**函数**：`gen_inst_meshes`

---

## 💻 需要添加的代码

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
                // 文件名与基础 mesh 相同，但保存在不同目录
                let lod_mesh_path = lod_dir.join(format!("{}.mesh", mesh_id));
                if let Err(e) = lod_mesh.mesh.ser_to_file(&lod_mesh_path) {
                    debug_model_warn!("   ⚠️  保存 LOD {:?} mesh 失败: {} - {}", lod_level, mesh_id, e);
                } else {
                    debug_model_debug!("   ✅ 生成 LOD {:?} mesh: {}", lod_level, lod_mesh_path.display());
                }
            }
            None => {
                debug_model_warn!("   ⚠️  生成 LOD {:?} mesh 失败: {}", lod_level, mesh_id);
            }
        }
    }
}
```

---

## 🧪 测试命令

### 1. 编译验证
```bash
cargo build
```

### 2. 单个 Refno 测试
```bash
cargo run --bin aios-database -- \
  --config DbOption \
  --debug-model-refnos="21485_13393" \
  --gen-mesh
```

**预期结果**：在 3 个目录下各生成 1 个文件
- `assets/meshes/lod_L1/4892393770286273807.mesh`
- `assets/meshes/lod_L2/4892393770286273807.mesh`
- `assets/meshes/lod_L3/4892393770286273807.mesh`

### 3. 导出测试
```bash
cargo run --bin aios-database -- \
  --config DbOption \
  --export-all-relates \
  --verbose
```

**预期结果**：成功生成
- `geometry_L1.glb`
- `geometry_L2.glb`
- `geometry_L3.glb`

---

## 📊 关键指标

| 指标 | 修改前 | 修改后 | 变化 |
|------|--------|--------|------|
| 生成文件数 | 1 个/geo_hash | 3 个/geo_hash | +200% |
| 生成目录数 | 1 个 | 3 个 | +200% |
| 生成时间 | T | ~3T | +200% |
| 磁盘空间 | S | ~3S | +200% |
| 导出成功率 | ❌ 失败 | ✅ 成功 | 修复 |

---

## 🔍 验证清单

- [ ] 代码编译通过（`cargo build`）
- [ ] 单个 geo_hash 生成 4 个文件
- [ ] 日志包含 "✅ 生成 LOD L1/L2/L3 mesh" 信息
- [ ] Prepack LOD 导出成功
- [ ] 无 "⚠️ LOD mesh file not found" 警告
- [ ] 生成的 GLB 文件可以在 Viewer 中加载

---

## 🐛 常见问题

### Q1: 编译失败 - 找不到 `LodLevel`
**A**: 确保导入语句正确：`use aios_core::mesh_precision::LodLevel;`

### Q2: 生成时间过长
**A**: 正常现象，生成时间会增加约 3 倍。可以考虑后期优化（并行生成）。

### Q3: 某些 LOD 生成失败
**A**: 不影响整体流程，会记录警告日志。导出时会使用降级策略。

### Q4: 磁盘空间不足
**A**: 定期清理旧的 mesh 文件，或使用压缩存储。

---

## 📁 相关文件

- **核心文件**：`src/fast_model/mesh_generate.rs`
- **导出模块**：`src/fast_model/export_model/export_prepack_lod.rs`
- **配置文件**：`DbOption.toml`
- **测试代码**：`src/test/test_gen_model/lod_precision.rs`

---

## 🔗 相关文档

- [详细开发计划](./LOD_Mesh生成优化开发计划.md)
- [Prepack LOD 格式规范](../docs/PREPACK_FORMAT_SPECIFICATION.md)

---

**创建时间**：2025-01-12  
**预计完成时间**：3-4 小时  
**状态**：待实施

