# GENSEC 旋转问题修复方案

## 问题总结

GENSEC `pe:⟨17496_266210⟩` 的子元素旋转计算不正确。

**根本原因**：
在 `src/transform/mod.rs` 中计算 GENSEC 子元素的旋转时，只使用了 SPINE 的路径方向（从 pt0 到 pt1），但**没有考虑 SPINE 的 YDIR 属性**。

**当前代码**（`transform/mod.rs:114-120`）：
```rust
if parent_is_gensec {
    // Find spine and get its vertices
    if let Ok(pts) = get_spline_pts(parent_refno).await {
        if pts.len() == 2 {
            pos_extru_dir = Some((pts[1] - pts[0]).normalize());
        }
    }
}
```

然后在第 249 行：
```rust
quat = cal_spine_orientation_basis(z_axis, false);
```

这个函数使用固定的参考轴（垂直时用 Global Y，水平时用 Global Z），而不是 SPINE 的 YDIR。

## 修复方案

### 方案 1：修改 `cal_spine_orientation_basis` 函数（推荐）

**优点**：
- 保持接口一致性
- 支持 YDIR 参数
- 向后兼容

**实现**：

1. 在 `src/rs_surreal/spatial.rs` 中添加新函数：

```rust
/// 针对 SPINE 方向的专用方位计算（支持 YDIR）
pub fn cal_spine_orientation_basis_with_ydir(
    spine_dir: DVec3,
    ydir: Option<DVec3>,
    neg: bool
) -> DQuat {
    let z_axis = spine_dir.normalize();
    
    // 如果提供了 YDIR，使用它作为参考
    let y_ref = if let Some(y) = ydir {
        let y_norm = y.normalize();
        // 防止 YDIR 与 spine_dir 共线
        if y_norm.dot(z_axis).abs() > 0.99 {
            // 回退到默认逻辑
            if z_axis.dot(DVec3::Z).abs() > 0.999 {
                DVec3::Y
            } else {
                DVec3::Z
            }
        } else {
            y_norm
        }
    } else {
        // 回退到默认逻辑
        if z_axis.dot(DVec3::Z).abs() > 0.999 {
            DVec3::Y
        } else {
            DVec3::Z
        }
    };
    
    // 构造正交基：Z = spine_dir, Y ≈ y_ref, X = Y × Z
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

2. 在 `src/transform/mod.rs` 中修改代码：

```rust
// 在第 112 行后添加：
let mut spine_ydir: Option<DVec3> = None;

// 修改第 114-120 行：
if parent_is_gensec {
    // Find spine and get its path data (including YDIR)
    if let Ok(spine_paths) = get_spline_path(parent_refno).await {
        if let Some(first_spine) = spine_paths.first() {
            // 使用第一个 spine 段的方向
            let dir = (first_spine.pt1 - first_spine.pt0).normalize();
            pos_extru_dir = Some(dir.as_dvec3());
            
            // 获取 YDIR
            let ydir = first_spine.preferred_dir;
            if ydir.length_squared() > 0.01 {
                spine_ydir = Some(ydir.as_dvec3());
            }
        }
    }
}

// 修改第 249 行：
quat = cal_spine_orientation_basis_with_ydir(z_axis, spine_ydir, false);
```

### 方案 2：直接使用 `cal_ori_by_ydir` 函数

**优点**：
- 代码更简单
- 复用现有函数

**缺点**：
- 需要确保 YDIR 存在
- 可能需要回退逻辑

**实现**：

```rust
if parent_is_gensec {
    if !is_world_quat {
        if !z_axis.is_normalized() {
            return Ok(None);
        }
        
        // 如果有 YDIR，使用 cal_ori_by_ydir
        if let Some(ydir) = spine_ydir {
            quat = cal_ori_by_ydir(ydir, z_axis);
        } else {
            quat = cal_spine_orientation_basis(z_axis, false);
        }
    }
}
```

## 推荐实施步骤

1. **实施方案 1**（更稳健）
2. 添加测试用例验证修复
3. 检查其他可能受影响的代码路径

## 测试验证

运行以下测试验证修复：

```bash
cargo test test_gensec_17496_266210 -- --nocapture
cargo test test_spine_orientation_basis_analysis -- --nocapture
```

预期结果：
- GENSEC `17496/266210` 的旋转应该是 `Y is Z and Z is -X 0.1661 Y`
- 而不是当前的 `Y is Z and Z is -Y`

## 影响范围

- 所有 GENSEC 类型的元素
- 可能影响 WALL 类型（也使用 SPINE）
- 不影响其他类型（SCTN, PIPE 等）

