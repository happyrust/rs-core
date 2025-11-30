# ProfileProcessor 测试 OBJ 文件

本目录包含 `ProfileProcessor` 测试用例生成的 OBJ 模型文件，用于可视化验证。

## 📁 文件列表

### Extrusion（拉伸）测试

| 文件名 | 描述 | 尺寸 |
|--------|------|------|
| `extrusion_rectangle_200x100x300.obj` | 矩形截面拉伸 | 200×100×300mm |
| `extrusion_rounded_rectangle_150x150x250.obj` | 带圆角矩形拉伸 | 150×150×250mm，圆角20mm |
| `extrusion_l_shape_150x150x150.obj` | L形截面拉伸 | L形轮廓，高度150mm |
| `extrusion_square_with_circular_hole_200x200x300.obj` | 方形外轮廓+圆形内孔 | 200×200×300mm，内孔直径80mm |
| `extrusion_h_beam_200x200x1000.obj` | H型钢截面拉伸 | H200×200，高度1000mm |
| `extrusion_multiple_holes_300x300x400.obj` | 多孔洞拉伸 | 300×300×400mm，3个内孔 |

### Revolution（旋转）测试

| 文件名 | 描述 | 参数 |
|--------|------|------|
| `revolution_cylinder_r50_h200_360deg.obj` | 圆柱体 | 半径50mm，高度200mm，360° |
| `revolution_cone_r60_r20_h150_360deg.obj` | 圆锥体 | 底部60mm，顶部20mm，高度150mm，360° |
| `revolution_frustum_r80_r40_h200_360deg.obj` | 圆台（带圆角） | 底部80mm，顶部40mm，高度200mm，带圆角，360° |
| `revolution_half_cylinder_r50_h200_180deg.obj` | 半圆柱 | 半径50mm，高度200mm，180° |

## 🔍 查看方式

可以使用以下工具查看 OBJ 文件：

- **Blender**（推荐）：免费开源，功能强大
- **MeshLab**：免费开源，轻量级
- **3D Viewer**（Windows 10/11）：系统自带
- **在线查看器**：如 https://3dviewer.net/

## 📊 测试统计

- **总测试数**: 17 个
- **生成 OBJ 文件**: 10 个
- **测试状态**: ✅ 全部通过

## 🎯 测试覆盖

- ✅ 基础形状（矩形、L形）
- ✅ 圆角处理（FRADIUS）
- ✅ 孔洞处理（单孔、多孔）
- ✅ 工程场景（H型钢）
- ✅ 旋转体（圆柱、圆锥、圆台）
- ✅ 部分旋转（180度）
- ✅ 自动检测外轮廓
- ✅ 边界情况处理

## 📝 注意事项

1. 所有 OBJ 文件包含顶点、法线和面信息
2. UV 坐标已生成，可用于纹理映射
3. 文件使用毫米（mm）作为单位
4. 法线已正确计算，适合渲染

## 🔄 重新生成

运行以下命令重新生成所有 OBJ 文件：

```bash
cargo test --lib profile_processor -- --nocapture
```

文件将自动导出到 `test_output/profile_processor/` 目录。



