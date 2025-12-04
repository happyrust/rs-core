# LPYRAMID 偏移计算修复

> 基于 AVEVA E3D core.dll 逆向分析

## 概述

LPYRA (Linear Pyramid) 是 PDMS/E3D 中的线性棱锥原语，通过定义底面和顶面的尺寸、位置以及偏移量来生成截锥或棱锥形状。本文档记录了 `rs-core` 中 LPyramid 模型生成的偏移计算修复过程。

## 1. 属性定义

| 属性 | 说明 |
|------|------|
| **PAAX_PT/DIR** | A 轴原点和方向（高度轴） |
| **PBAX_PT/DIR** | B 轴原点和方向（宽度轴） |
| **PCAX_PT/DIR** | C 轴原点和方向（深度轴） |
| **PBTP/PCTP** | 顶面 B/C 方向尺寸 |
| **PBBT/PCBT** | 底面 B/C 方向尺寸 |
| **PTDI/PBDI** | 到顶面/底面的距离 |
| **PBOF** | B 方向偏移（仅应用于顶面） |
| **PCOF** | C 方向偏移（仅应用于顶面） |

## 2. 问题分析

### 2.1 原始问题

在修复前，`rs-core` 的 LPyramid 实现存在以下问题：

1. **csg.rs (网格生成)**：偏移 `offset_3d` 使用原始的 `pbax_dir`/`pcax_dir` 计算，但顶点位置使用正交化后的方向向量，导致偏移方向不一致。

2. **lpyramid.rs (OCC/Truck 实现)**：偏移使用 `DVec3` 向量形式计算，但在局部坐标系中应该直接使用标量偏移。

### 2.2 core.dll 分析

通过 IDA Pro 逆向分析 core.dll 发现：

```cpp
// core.dll 中 LPyramid 的偏移计算逻辑
// 偏移始终在正交化后的局部坐标系中应用
void LPyramid::computeTopFace() {
    // 正交化轴方向
    Vec3 b_dir = orthogonalize(pbax_dir, paax_dir);
    Vec3 c_dir = orthogonalize(pcax_dir, paax_dir, b_dir);
    
    // 顶面顶点 = 顶面中心 + 局部偏移（在正交化坐标系中）
    for (corner : corners) {
        top_vertex = top_center 
                   + b_dir * (corner.x * pbtp/2 + pbof)  // B方向带偏移
                   + c_dir * (corner.y * pctp/2 + pcof); // C方向带偏移
    }
    
    // 底面顶点无偏移
    for (corner : corners) {
        bot_vertex = bot_center 
                   + b_dir * (corner.x * pbbt/2)
                   + c_dir * (corner.y * pcbt/2);
    }
}
```

关键发现：

- **偏移仅应用于顶面**，底面无偏移
- **偏移在正交化后的坐标系中计算**
- 局部实现（OCC/Truck）中偏移直接作为 X/Y 标量使用

## 3. 修复方案

### 3.1 csg.rs 修复

**文件**: `src/geometry/csg.rs`
**函数**: `generate_lpyramid_mesh()`

```rust
// 修复前：使用原始方向计算偏移
let offset_3d = lpyr.pbax_dir * lpyr.pbof + lpyr.pcax_dir * lpyr.pcof;

// 修复后：使用正交化后的方向计算偏移
let axis_dir = safe_normalize(lpyr.paax_dir)?;
let (fallback_u, fallback_v) = orthonormal_basis(axis_dir);

// 正交化 B 轴方向
let mut pb_dir = safe_normalize(lpyr.pbax_dir).unwrap_or(fallback_u);
pb_dir = (pb_dir - axis_dir * pb_dir.dot(axis_dir)).normalize_or_zero();
if pb_dir.length_squared() <= MIN_LEN * MIN_LEN { pb_dir = fallback_u; }

// 正交化 C 轴方向
let mut pc_dir = safe_normalize(lpyr.pcax_dir).unwrap_or(fallback_v);
pc_dir = (pc_dir - axis_dir * pc_dir.dot(axis_dir) 
        - pb_dir * pc_dir.dot(pb_dir)).normalize_or_zero();
if pc_dir.length_squared() <= MIN_LEN * MIN_LEN { pc_dir = fallback_v; }

// 使用正交化方向计算偏移
let offset_3d = pb_dir * lpyr.pbof + pc_dir * lpyr.pcof;
```

顶点计算修正：

```rust
// 顶面：带偏移（offset_3d 为世界坐标偏移）
let top = if tx > MIN_LEN && ty > MIN_LEN {
    let mut idxs = [0u32; 4];
    for (i, (ox, oy)) in offsets.iter().enumerate() {
        let pos = center + pb_dir * (ox * tx) + pc_dir * (oy * ty) 
                + axis_dir * height + offset_3d;  // 加上偏移
        idxs[i] = add_vert(pos, &mut vertices, &mut normals, &mut aabb);
    }
    Some(idxs)
} else { None };

// 底面：无偏移
let bot = if bx > MIN_LEN && by > MIN_LEN {
    let mut idxs = [0u32; 4];
    for (i, (ox, oy)) in offsets.iter().enumerate() {
        let pos = center + pb_dir * (ox * bx) + pc_dir * (oy * by);  // 无偏移
        idxs[i] = add_vert(pos, &mut vertices, &mut normals, &mut aabb);
    }
    Some(idxs)
} else { None };
```

### 3.2 lpyramid.rs 修复 (OCC)

**文件**: `src/prim_geo/lpyramid.rs`
**函数**: `gen_occ_shape()`

```rust
// 修复后：局部坐标系中直接使用标量偏移
let offset_x = self.pbof as f64;  // PBOF -> X方向
let offset_y = self.pcof as f64;  // PCOF -> Y方向

// 顶面：带偏移
let pts = vec![
    DVec3::new(-tx + offset_x, -ty + offset_y, h2),
    DVec3::new( tx + offset_x, -ty + offset_y, h2),
    DVec3::new( tx + offset_x,  ty + offset_y, h2),
    DVec3::new(-tx + offset_x,  ty + offset_y, h2),
];

// 底面：无偏移
let pts = vec![
    DVec3::new(-bx, -by, -h2),
    DVec3::new( bx, -by, -h2),
    DVec3::new( bx,  by, -h2),
    DVec3::new(-bx,  by, -h2),
];
```

### 3.3 lpyramid.rs 修复 (Truck)

**函数**: `gen_brep_shell()`

```rust
// 修复后：与 OCC 实现保持一致
let offset_x = self.pbof as f64;
let offset_y = self.pcof as f64;

// 顶面顶点（带偏移）
let pts = vec![
    builder::vertex(Point3::new(-tx + offset_x, -ty + offset_y, h2)),
    builder::vertex(Point3::new( tx + offset_x, -ty + offset_y, h2)),
    builder::vertex(Point3::new( tx + offset_x,  ty + offset_y, h2)),
    builder::vertex(Point3::new(-tx + offset_x,  ty + offset_y, h2)),
];

// 底面顶点（无偏移）
let pts = vec![
    builder::vertex(Point3::new(-bx, -by, -h2)),
    builder::vertex(Point3::new( bx, -by, -h2)),
    builder::vertex(Point3::new( bx,  by, -h2)),
    builder::vertex(Point3::new(-bx,  by, -h2)),
];
```

## 4. 坐标系约定

### 4.1 世界坐标系 (csg.rs)

```text
              axis_dir (A轴/高度)
                  ↑
                  |
                  |    pc_dir (C轴)
                  |   /
                  |  /
                  | /
                  +--------→ pb_dir (B轴)
              (center)
```

- `center`: 底面中心点 = `paax_pt + axis_dir * pbdi`
- `height`: 总高度 = `ptdi - pbdi`
- 偏移 `offset_3d` 为世界坐标向量

### 4.2 局部坐标系 (OCC/Truck)

```text
              Z (高度)
              ↑
              |
              |    Y (C轴/PCOF)
              |   /
              |  /
              | /
              +--------→ X (B轴/PBOF)
            原点
```

- 原点在几何中心
- X 轴对应 B 轴方向，Y 轴对应 C 轴方向
- 外部 `Transform` 负责坐标系旋转和定位

## 5. 测试验证

### 5.1 测试用例

```rust
// 测试带偏移的 LPyramid
let pyramid = LPyramid {
    pbax_pt: Vec3::ZERO,
    pbax_dir: Vec3::X,
    pcax_pt: Vec3::ZERO,
    pcax_dir: Vec3::Y,
    paax_pt: Vec3::ZERO,
    paax_dir: Vec3::Z,
    pbtp: 40.0,    // 顶面宽度
    pctp: 40.0,    // 顶面深度
    pbbt: 80.0,    // 底面宽度
    pcbt: 80.0,    // 底面深度
    ptdi: 50.0,    // 顶面距离
    pbdi: -50.0,   // 底面距离
    pbof: 20.0,    // B方向偏移
    pcof: 10.0,    // C方向偏移
};
```

### 5.2 测试结果

```text
running 5 tests
✅ 无偏移 LPyramid 验证通过
✅ 带偏移 LPyramid (PBOF=20, PCOF=10) 验证通过
✅ 顶点锥体 (顶面退化为点，带偏移) 验证通过
✅ 旋转坐标系 LPyramid (45° 旋转) 验证通过
✅ 导出 OBJ - 顶点数: 8, 面数: 12
test result: ok. 5 passed; 0 failed
```

### 5.3 生成的 OBJ 验证

```obj
# 顶面顶点 (带偏移, 中心约在 20,10,50)
v 0.000 -10.000 50.000
v 40.000 -10.000 50.000
v 40.000 30.000 50.000
v 0.000 30.000 50.000

# 底面顶点 (无偏移, 中心在 0,0,-50)
v -40.000 -40.000 -50.000
v 40.000 -40.000 -50.000
v 40.000 40.000 -50.000
v -40.000 40.000 -50.000
```

## 6. 修改文件清单

| 文件 | 修改内容 |
|------|----------|
| `src/geometry/csg.rs` | 修复 `generate_lpyramid_mesh()`，使用正交化方向计算偏移 |
| `src/prim_geo/lpyramid.rs` | 修复 `gen_occ_shape()` 和 `gen_brep_shell()`，使用局部标量偏移 |
| `src/test/test_lpyramid_fix.rs` | 新增测试文件 |
| `src/test/mod.rs` | 添加测试模块引用 |

## 7. 相关文档

- [DRNS_DRNE_截面计算分析.md](./DRNS_DRNE_截面计算分析.md) - 端面斜切角度修复
- [src/prim_geo/category.rs](../../src/prim_geo/category.rs) - LPyramid 坐标系构造

---

文档更新时间: 2024-12-04
