# GENSEC 旋转问题分析

## 问题描述

GENSEC `pe:⟨17496_266210⟩` 的旋转计算不正确。

### 当前计算结果
```
rotation: [-0.70710677, 0, 0, 0.7071068]  // Quaternion (x, y, z, w)
scale: [1, 1, 80.392334]
```

这个四元数 `(-0.707, 0, 0, 0.707)` 表示：
- 绕 X 轴旋转 -90 度
- 转换为旋转矩阵：
  ```
  X-axis: (1,  0,  0)
  Y-axis: (0,  0,  1)   // Y → Z
  Z-axis: (0, -1,  0)   // Z → -Y
  ```
- PDMS 表示：`Y is Z and Z is -Y`

### 正确的结果应该是
```
Y is Z and Z is -X 0.1661 Y
```

这意味着：
- Local Y-axis = Global Z = `(0, 0, 1)`
- Local Z-axis = `(-X, 0.1661Y, 0)` 归一化 = `(-0.9863, 0.1648, 0)`
- Local X-axis = Y × Z = `(0, 0, 1) × (-0.9863, 0.1648, 0)` = `(-0.1648, 0.9863, 0)`

正确的旋转矩阵应该是：
```
X-axis: (-0.1648,  0.9863, 0)
Y-axis: ( 0,       0,      1)
Z-axis: (-0.9863, -0.1648, 0)
```

## 问题根源分析

### 1. `cal_spine_orientation_basis` 函数的逻辑

当前实现（`src/rs_surreal/spatial.rs:55-82`）：

```rust
pub fn cal_spine_orientation_basis(v: DVec3, neg: bool) -> DQuat {
    let is_vertical = v.normalize().dot(DVec3::Z).abs() > 0.999;
    
    let (x_dir, y_dir) = if is_vertical {
        // 垂直构件：优先让 Y 轴指北 (Global Y)
        let y_target = DVec3::Y;
        let x_res = y_target.cross(v).normalize();
        let y_res = v.cross(x_res).normalize();
        (x_res, y_res)
    } else {
        // 非垂直构件（包括水平）：优先让 Y 轴朝上 (Global Z)
        let y_target = DVec3::Z;
        let x_res = y_target.cross(v).normalize();
        let y_res = v.cross(x_res).normalize();
        (x_res, y_res)
    };
    
    DQuat::from_mat3(&DMat3::from_cols(final_x, final_y, v))
}
```

**问题**：
- 这个函数假设 `v` 是 **Local Z 轴**（挤出方向）
- 但对于 GENSEC，SPINE 的方向应该是什么？

### 2. GENSEC 的 SPINE 方向

从 `src/prim_geo/profile.rs:150-220` 可以看到：

```rust
let mut spine_paths = if type_name == "GENSEC" || type_name == "WALL" {
    let children_refnos = collect_descendant_filter_ids(&[refno], &["SPINE"], None).await?;
    let mut paths = vec![];
    for &spine_refno in children_refnos.iter() {
        let spine_att = get_named_attmap(spine_refno).await?;
        // ...
        paths.push(Spine3D {
            pt0: att1.get_position()?,
            pt1: att2.get_position()?,
            preferred_dir: spine_att.get_vec3("YDIR").unwrap_or(Vec3::Z),
            // ...
        });
    }
    paths
}
```

**关键点**：
- SPINE 有 `YDIR` 属性，表示期望的 Y 方向
- SPINE 的路径方向（从 pt0 到 pt1）应该是 Local Z 轴（挤出方向）

### 3. 在 `transform/mod.rs` 中的调用

```rust
if parent_is_gensec {
    if !is_world_quat {
        if !z_axis.is_normalized() {
            return Ok(None);
        }
        quat = cal_spine_orientation_basis(z_axis, false);
    }
}
```

这里的 `z_axis` 是什么？需要查看 `pos_extru_dir` 的来源。

## 问题诊断

### 可能的原因 1：SPINE 方向计算错误

如果 SPINE 的路径方向计算为垂直方向（Z 轴），但实际上应该是水平方向（带有 Y 分量），那么 `cal_spine_orientation_basis` 会选择错误的分支。

### 可能的原因 2：没有考虑 YDIR

`cal_spine_orientation_basis` 函数没有接受 `YDIR` 参数，而是使用固定的参考轴：
- 垂直时：参考 Global Y
- 水平时：参考 Global Z

但 PDMS 中，SPINE 有 `YDIR` 属性来指定期望的 Y 方向。

### 可能的原因 3：坐标系混淆

从你的数据来看：
- 当前计算：`Y is Z and Z is -Y`（绕 X 轴旋转 -90°）
- 正确结果：`Y is Z and Z is -X 0.1661 Y`

两者的 Y 轴都是 Z，说明 Y 轴的计算是对的。
问题在于 Z 轴：
- 当前：Z = -Y（全局 Y 的负方向）
- 正确：Z = (-X, 0.1661Y, 0)（主要是 -X 方向，带一点 Y 分量）

这说明 **SPINE 的路径方向计算错误**。

## 解决方案

### 方案 1：修复 SPINE 路径方向计算

检查 `pos_extru_dir` 的来源，确保它正确反映了 SPINE 的实际方向（从起点到终点）。

### 方案 2：使用 YDIR 参数

修改 `cal_spine_orientation_basis` 函数，接受 `YDIR` 参数：

```rust
pub fn cal_spine_orientation_basis_with_ydir(
    spine_dir: DVec3,  // SPINE 路径方向（Local Z）
    ydir: Option<DVec3>,  // 期望的 Y 方向
    neg: bool
) -> DQuat {
    let z_axis = spine_dir.normalize();
    
    // 如果提供了 YDIR，使用它作为参考
    let y_ref = if let Some(y) = ydir {
        y.normalize()
    } else {
        // 回退到默认逻辑
        if z_axis.dot(DVec3::Z).abs() > 0.999 {
            DVec3::Y
        } else {
            DVec3::Z
        }
    };
    
    // 构造正交基
    let x_dir = y_ref.cross(z_axis).normalize();
    let y_dir = z_axis.cross(x_dir).normalize();
    
    let (final_x, final_y) = if neg {
        (-x_dir, -y_dir)
    } else {
        (x_dir, y_dir)
    };
    
    DQuat::from_mat3(&DMat3::from_cols(final_x, final_y, z_axis))
}
```

### 方案 3：使用 `cal_ori_by_ydir`

如果 SPINE 有 YDIR，应该使用 `cal_ori_by_ydir` 函数：

```rust
if let Some(ydir) = spine_att.get_dvec3("YDIR") {
    quat = cal_ori_by_ydir(ydir.normalize(), z_axis);
} else {
    quat = cal_spine_orientation_basis(z_axis, false);
}
```

## 下一步

1. 运行测试 `test_gensec_17496_266210` 查看实际的 SPINE 数据
2. 检查 SPINE 的 YDIR 属性
3. 验证 SPINE 路径方向的计算
4. 根据实际情况选择合适的修复方案

