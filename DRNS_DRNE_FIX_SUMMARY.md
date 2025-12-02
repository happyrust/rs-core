# SweepSolid DRNS/DRNE 方向处理问题修复

## 问题概述

在 `src/prim_geo/sweep_solid.rs` 中，对 DRNS（起始端面方向）和 DRNE（结束端面方向）的处理存在多个导致模型方位错误的问题。

## 具体问题

### 1. 类型不一致 (原第 578 行)

```rust
// 错误代码
let x_angle = (-self.drne).angle_between(Vec3::X).abs();   // 使用 Vec3::X
let y_angle = (-self.drne).angle_between(DVec3::Y).abs();  // ❌ 使用 DVec3::Y
```

**问题**：X 轴计算使用 `Vec3::X`，Y 轴计算却使用 `DVec3::Y`，类型不一致导致方向计算错误。

### 2. 对 Option 类型直接操作

```rust
// 错误代码
if self.drne.is_normalized() && self.is_drne_sloped() {
    let x_angle = (-self.drne).angle_between(Vec3::X).abs();  // ❌ drne 是 Option<DVec3>
    // ...
    transform_top = Mat4::from_quat(glam::Quat::from_rotation_arc(Vec3::Z, -self.drne))
}
```

**问题**：`self.drne` 是 `Option<DVec3>` 类型，直接对 Option 取负号和调用方法是不正确的。

### 3. DRNS 和 DRNE 处理不一致

- DRNS 直接使用 `self.drns.angle_between(...)`
- DRNE 使用 `(-self.drne).angle_between(...)`

虽然语法上可能通过（如果有某些扩展 trait），但逻辑不清晰，容易出错。

## 修复方案

### 修复后的代码

```rust
// DRNS 处理（起始端面）
if self.drns.is_normalized() && self.is_drns_sloped() {
    // 解包 DRNS（起始端面方向）
    let drns_vec = self.drns.unwrap().as_vec3();
    let x_angle = drns_vec.angle_between(Vec3::X).abs();
    let scale_x = if x_angle < ANGLE_RAD_F64_TOL {
        1.0
    } else {
        1.0 / (x_angle.sin())
    };
    let y_angle = drns_vec.angle_between(Vec3::Y).abs();  // ✅ 统一使用 Vec3::Y
    let scale_y = if y_angle < ANGLE_RAD_F64_TOL {
        1.0
    } else {
        1.0 / (y_angle.sin())
    };
    transform_btm =
        Mat4::from_quat(glam::Quat::from_rotation_arc(Vec3::Z, drns_vec))
            * Mat4::from_scale(Vec3::new(scale_x, scale_y, 1.0));
}

// DRNE 处理（结束端面）
if self.drne.is_normalized() && self.is_drne_sloped() {
    // 先解包 DRNE，然后取负值（因为 DRNE 是结束端面的方向）
    let drne_vec = -self.drne.unwrap().as_vec3();
    let x_angle = drne_vec.angle_between(Vec3::X).abs();
    let scale_x = if x_angle < ANGLE_RAD_F64_TOL {
        1.0
    } else {
        1.0 / (x_angle.sin())
    };
    let y_angle = drne_vec.angle_between(Vec3::Y).abs();  // ✅ 统一使用 Vec3::Y
    let scale_y = if y_angle < ANGLE_RAD_F64_TOL {
        1.0
    } else {
        1.0 / (y_angle.sin())
    };
    transform_top =
        Mat4::from_quat(glam::Quat::from_rotation_arc(Vec3::Z, drne_vec))
            * Mat4::from_scale(Vec3::new(scale_x, scale_y, 1.0));
}
```

## 修复要点

1. **先解包 Option**：使用 `self.drns.unwrap()` 和 `self.drne.unwrap()` 明确解包
2. **转换类型**：使用 `.as_vec3()` 将 `DVec3` 转换为 `Vec3`
3. **统一坐标系**：X 和 Y 轴计算都使用 `Vec3::X` 和 `Vec3::Y`
4. **明确方向**：DRNE 取负值是因为它是结束端面的外法向

## 影响范围

此修复影响：
- **SweepSolid**（PrimLoft）：土建模型中的扫掠实体
- **GENSEC**：通用截面
- **WALL**：墙体
- 所有使用 DRNS/DRNE 进行端面斜切的几何体

## 测试建议

1. 测试有斜切端面的 H 型钢：
   ```bash
   cargo test test_h_beam_drns_drne
   ```

2. 生成具有斜切端面的模型并检查：
   - 端面是否在正确的位置
   - 端面方向是否正确
   - 整体模型方位是否正确

## 相关文件

- `src/prim_geo/sweep_solid.rs`: 主要修复位置（第 554-589 行）
- `src/prim_geo/profile.rs`: DRNS/DRNE 从数据库读取和转换
- `src/geometry/sweep_mesh.rs`: CSG 网格生成时使用 DRNS/DRNE
- `src/test/test_h_beam_drns_drne.rs`: 端面斜切测试用例

## 编译验证

```bash
cd /Volumes/DPC/work/plant-code/rs-core
cargo build --lib
# ✅ 编译通过，无错误无警告
```
