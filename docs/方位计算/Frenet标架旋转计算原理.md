# Frenet 标架旋转计算原理

## 概述

Frenet 标架（Frenet Frame）是沿曲线定义的局部坐标系，用于确定截面在扫描路径上的方向。`frenet_rotation` 是将标准坐标系（XYZ）旋转到 Frenet 标架（right/up/tangent）的旋转四元数。

## 数学原理

### Frenet 标架的三个轴

1. **Tangent（切线方向）**: 沿路径方向的单位向量
2. **Normal（法向量）**: 指向曲线曲率中心的单位向量
3. **Binormal（副法向量）**: `tangent × normal`，垂直于切线和法向量

但在实际应用中，我们使用：
- **Tangent**: 路径方向
- **Right**: 参考方向与切线的叉积
- **Up**: 切线与右向量的叉积

### 坐标系变换

Frenet 标架旋转将标准坐标系 `(X, Y, Z)` 映射到 Frenet 标架 `(right, up, tangent)`：

```
标准坐标系 → Frenet 标架
X → right
Y → up  
Z → tangent
```

## LINE 段的 Frenet 标架计算

### 代码位置
`profile.rs` 第 96-107 行

### 计算步骤

```rust
// 1. 参考上方向（plax 的归一化）
let ref_up = plax.normalize_or_zero();

// 2. 计算右向量：ref_up × direction
let right = ref_up.cross(direction).normalize_or_zero();

// 3. 计算正交化的上向量：direction × right
let up = direction.cross(right).normalize_or_zero();

// 4. 构建 Frenet 标架旋转（从标准坐标系 XYZ 到 right/up/direction）
let frenet_rotation = Quat::from_mat3(&Mat3::from_cols(right, up, direction));
```

### 详细说明

#### 步骤 1: 参考方向
```rust
let ref_up = plax.normalize_or_zero();
```
- `plax`: 截面参考方向（通常来自 profile 的 PLAX 属性）
- 用作构建 Frenet 标架的参考方向

#### 步骤 2: 计算右向量
```rust
let right = ref_up.cross(direction).normalize_or_zero();
```
- **叉积**: `ref_up × direction`
- **几何意义**: 垂直于参考方向和路径方向的向量
- **归一化**: 确保单位长度

**数学原理**:
- 如果 `ref_up` 和 `direction` 不平行，叉积结果垂直于两者
- 如果平行（或接近平行），`normalize_or_zero()` 会返回零向量

#### 步骤 3: 计算上向量
```rust
let up = direction.cross(right).normalize_or_zero();
```
- **叉积**: `direction × right`
- **几何意义**: 垂直于路径方向和右向量的向量
- **结果**: 形成右手坐标系 `(right, up, direction)`

**数学原理**:
- `direction × right` 确保 `up` 垂直于 `direction` 和 `right`
- 形成正交基：`right ⊥ up ⊥ direction`

#### 步骤 4: 构建旋转矩阵
```rust
let frenet_rotation = Quat::from_mat3(&Mat3::from_cols(right, up, direction));
```

**矩阵结构**:
```
Mat3::from_cols(right, up, direction)
= [right.x  up.x  direction.x]
  [right.y  up.y  direction.y]
  [right.z  up.z  direction.z]
```

**旋转含义**:
- 将标准坐标系 `(X, Y, Z)` 旋转到 `(right, up, direction)`
- `X` 轴 → `right`
- `Y` 轴 → `up`
- `Z` 轴 → `direction`

### 可视化示例

假设：
- `direction = Vec3::Z`（沿 Z 轴）
- `plax = Vec3::Y`（参考方向为 Y 轴）

计算过程：
1. `ref_up = Vec3::Y`
2. `right = Vec3::Y × Vec3::Z = Vec3::X`
3. `up = Vec3::Z × Vec3::X = Vec3::Y`
4. Frenet 标架：`(X, Y, Z)` → `(X, Y, Z)`（无旋转）

如果 `plax` 不是标准方向：
- `plax = Vec3(1, 1, 0).normalize()`（45度方向）
- `direction = Vec3::Z`
- `right = plax × direction`（指向某个方向）
- `up = direction × right`（形成正交基）

## ARC 段的 Frenet 标架计算

### 代码位置
`profile.rs` 第 143-164 行（THRU）和第 200-221 行（CENT）

### 计算步骤

```rust
// 1. 径向量：从圆心指向起点
let radial = (spine.pt0 - center).normalize_or_zero();

// 2. 切线方向：axis × radial（顺时针则取反）
let tangent = if clock_wise {
    -axis.cross(radial).normalize_or_zero()
} else {
    axis.cross(radial).normalize_or_zero()
};

// 3. 参考方向：优先使用 spine.preferred_dir，否则使用 plax
let ref_dir = if spine.preferred_dir.length_squared() > 1e-6 {
    spine.preferred_dir.normalize_or_zero()
} else {
    plax.normalize_or_zero()
};

// 4. 计算 Frenet 标架（与 LINE 类似）
let right = ref_dir.cross(tangent).normalize_or_zero();
let up = tangent.cross(right).normalize_or_zero();
let frenet_rotation = Quat::from_mat3(&Mat3::from_cols(right, up, tangent));
```

### 详细说明

#### 步骤 1: 计算径向量
```rust
let radial = (spine.pt0 - center).normalize_or_zero();
```
- **几何意义**: 从圆心指向圆弧起点的单位向量
- **用途**: 用于计算切线方向

#### 步骤 2: 计算切线方向
```rust
let tangent = if clock_wise {
    -axis.cross(radial).normalize_or_zero()
} else {
    axis.cross(radial).normalize_or_zero()
};
```

**数学原理**:
- **叉积**: `axis × radial`
- **几何意义**: 
  - `axis` 是圆弧的法向量（垂直于圆弧平面）
  - `radial` 是从圆心指向起点的向量
  - `axis × radial` 得到切线方向
- **顺时针处理**: 如果圆弧是顺时针，取反

**示例**:
- 如果圆弧在 XY 平面，`axis = Vec3::Z`
- `radial = Vec3::X`（起点在 X 轴）
- `tangent = Vec3::Z × Vec3::X = Vec3::Y`（切线沿 Y 轴）

#### 步骤 3: 参考方向
```rust
let ref_dir = if spine.preferred_dir.length_squared() > 1e-6 {
    spine.preferred_dir.normalize_or_zero()
} else {
    plax.normalize_or_zero()
};
```
- **优先级**: `spine.preferred_dir` > `plax`
- **用途**: 用于构建 Frenet 标架的右向量

#### 步骤 4: 构建 Frenet 标架（与 LINE 相同）
```rust
let right = ref_dir.cross(tangent).normalize_or_zero();
let up = tangent.cross(right).normalize_or_zero();
let frenet_rotation = Quat::from_mat3(&Mat3::from_cols(right, up, tangent));
```

**关键区别**:
- LINE 使用 `direction`（直线方向）
- ARC 使用 `tangent`（圆弧切线方向）

## 旋转矩阵到四元数转换

### `Quat::from_mat3` 的作用

```rust
Quat::from_mat3(&Mat3::from_cols(right, up, tangent))
```

**转换原理**:
- 将 3×3 旋转矩阵转换为四元数
- 保持旋转的等效性
- 四元数表示更紧凑，计算更高效

**矩阵结构**:
```
[right.x  up.x  tangent.x]
[right.y  up.y  tangent.y]
[right.z  up.z  tangent.z]
```

**旋转效果**:
- 将标准坐标系 `(X, Y, Z)` 旋转到 `(right, up, tangent)`
- 保持右手坐标系

## 完整旋转链

### 最终旋转计算

```rust
// Frenet 标架旋转
let frenet_rotation = Quat::from_mat3(&Mat3::from_cols(right, up, tangent));

// bangle 旋转（绕路径方向）
let bangle_rotation = Quat::from_axis_angle(tangent, bangle.to_radians());

// 组合旋转
let final_rotation = frenet_rotation * bangle_rotation;
```

**旋转顺序**:
1. 先应用 `frenet_rotation`：将坐标系旋转到 Frenet 标架
2. 再应用 `bangle_rotation`：绕路径方向（tangent）旋转 `bangle` 度

**数学表示**:
```
final_rotation = frenet_rotation × bangle_rotation
```

**几何意义**:
- `frenet_rotation`: 确定截面在路径上的基本方向
- `bangle_rotation`: 绕路径方向旋转截面（用于调整截面角度）

## 关键点总结

1. **Frenet 标架的作用**: 为每个路径点建立局部坐标系，确定截面的方向
2. **参考方向的重要性**: `plax` 或 `preferred_dir` 用于确定截面的初始朝向
3. **正交基构建**: 通过叉积确保 `(right, up, tangent)` 形成右手正交坐标系
4. **旋转组合**: Frenet 标架旋转和 bangle 旋转组合，形成最终的截面方向

## 相关代码位置

- **LINE 段**: `profile.rs` 第 96-107 行
- **ARC 段 (THRU)**: `profile.rs` 第 143-164 行
- **ARC 段 (CENT)**: `profile.rs` 第 200-221 行
- **旋转组合**: `profile.rs` 第 112-113 行（LINE）、第 169-170 行（ARC）

---

**创建时间**: 2025-01-XX  
**相关文档**: 
- `Transform计算流程分析.md`
- `SweepLoft3d方位问题分析.md`










