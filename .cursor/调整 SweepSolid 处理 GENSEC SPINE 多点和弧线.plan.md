<!-- a6c4abeb-236a-4544-b2b9-f57cda2e8be7 42dfa0bb-7d84-4c2a-861a-0331d7b778fd -->
# 调整 SweepSolid 处理 GENSEC SPINE 多点和弧线 + CSG 生成 + 测试案例

## 问题分析

当前实现中，`profile.rs` 的 `create_profile_geos` 函数在处理 GENSEC 下的 SPINE 时：

1. 将 SPINE 的子元素（POINSP 和 CURVE）解析成多个 `Spine3D` 对象
2. 对每个 `Spine3D` 调用 `generate_paths()`，为每个段生成独立的 `SweepPath3D`
3. 为每个 `SweepPath3D` 创建一个独立的 `SweepSolid`

**问题**：当 SPINE 有多个点时，应该将这些点连接成一条连续路径，然后沿着这条完整路径进行 sweep，而不是为每个段创建独立的 SweepSolid。

**重要更新**：`gen_brep_shell()` 和 `gen_occ_shape()` 已被移除，现在需要使用 CSG 方式生成网格。

## 命名约定决策

**采用选项 A**：保持当前命名约定，通过模块路径 `prim_geo::Spine3D` 明确作用域。

- `Spine3D` - 保持原名
- `SweepPath3D` - 保持原名
- `Arc3D` - 保持原名
- `Line3D` - 保持原名
- `SpineCurveType` - 保持原名
- `SweepSolid` - 保持原名

## 解决方案

### 方案 1：扩展 SweepPath3D 支持多段路径（采用）

1. **扩展 `SweepPath3D` 枚举** (`src/prim_geo/spine.rs`)

- 添加 `MultiSegment(Vec<SweepPath3D>)` 变体，支持多段路径
- 实现 `length()` 方法，计算所有段的总长度
- 实现路径连接逻辑，确保相邻段正确连接

2. **修改 `profile.rs` 中的路径生成逻辑** (`src/prim_geo/profile.rs`)

- 在 `create_profile_geos` 函数中，当处理 GENSEC 下的 SPINE 时：
- 收集一个 SPINE 下的所有 `Spine3D` 段
- 将所有段的路径连接成一条连续路径
- 创建一个包含多段路径的 `SweepSolid`，而不是为每个段创建独立的 `SweepSolid`

3. **实现 `SweepSolid` 的 CSG 生成方法** (`src/prim_geo/sweep_solid.rs`)

- 实现 `gen_csg_shape()` 和 `gen_csg_mesh()` 方法（替代已移除的 `gen_brep_shell()` 和 `gen_occ_shape()`）
- 处理 `SweepPath3D::MultiSegment` 情况，实现多段路径的 CSG 网格生成

## 实现细节

### 1. THRU 类型弧线（通过三点确定圆弧）

- **中心点计算**：使用 `circum_center(pt0, pt1, thru_pt)` 计算外接圆心
- `pt0`: 起始点
- `pt1`: 结束点  
- `thru_pt`: 中间通过点（CURVE 的 POS 属性）
- **半径计算**：`center.distance(pt0)` 或 `center.distance(pt1)`
- **角度计算**：`angle = (PI - vec0.angle_between(vec1)) * 2.0`，其中 `vec0 = pt0 - thru_pt`，`vec1 = pt1 - thru_pt`
- **旋转轴**：`axis = vec1.cross(vec0).normalize()`
- **方向判断**：`clock_wise = axis.z < 0.0`

### 2. CENT 类型弧线（中心点已知）

- **中心点**：直接使用 `center_pt`（CURVE 的 POS 属性）
- **半径计算**：`center_pt.distance(pt0)` 或 `center_pt.distance(pt1)`
- **角度计算**：`angle = (PI - vec0.angle_between(vec1)) * 2.0`，其中 `vec0 = pt0 - center_pt`，`vec1 = pt1 - center_pt`
- **旋转轴**：`axis = vec1.cross(vec0).normalize()`

### 3. 截面采样和变换矩阵计算

#### 截面采样策略

- **直线段**：沿路径均匀采样多个截面位置
- 采样数量：根据路径长度和 LOD 设置确定
- 采样间隔：`segment_length / (num_segments - 1)`
- **弧线段**：沿弧线均匀采样多个截面位置
- 采样数量：根据弧长和角度确定
- 采样间隔：`arc_angle / (num_segments - 1)`

#### 截面变换矩阵计算（参考 `get_face_mat4` 实现）

**对于每个采样位置 `t`（0.0 到 1.0，沿路径归一化）：**

1. **路径方向计算**

- 直线段：`z_dir = normalize(end - start)`
- 弧线段：计算该点的切线方向

2. **drns/drne 插值**

- 如果 `drns` 和 `drne` 都存在且倾斜：
- 使用线性插值：`interpolated_dir = lerp(drns, drne, t)`
- 如果只有 `drns` 倾斜：使用 `drns`
- 如果只有 `drne` 倾斜：使用 `drne`
- 如果都不倾斜：使用路径方向 `z_dir`

3. **变换矩阵构建**

- 旋转：使用 `Quat::from_rotation_arc(Vec3::Z, interpolated_dir)` 或路径方向
- 缩放：当倾斜时计算缩放因子（参考 `get_face_mat4`）
- 平移：根据采样位置计算

4. **三角化连接相邻截面**

- 使用四边形三角化连接相邻截面的对应顶点
- 确保顶点顺序一致
- 计算正确的法向量

### 4. 端面处理

- 起始端面：应用 `drns` 变换（如果存在）
- 结束端面：应用 `drne` 变换（如果存在）
- 使用扇形三角化处理端面

## 测试案例设计

创建以下测试案例，覆盖不同情况，并生成 OBJ 模型：

1. **测试案例 1：多个 POINSP 点的直线路径**

- 6 个 POINSP 点形成连续直线路径
- 验证多个段正确连接

2. **测试案例 2：包含 THRU 类型弧线的混合路径**

- 多个 POINSP + 1 个 THRU 类型 CURVE
- 验证直线段和弧线段正确连接

3. **测试案例 3：包含 CENT 类型弧线的混合路径**

- 多个 POINSP + 1 个 CENT 类型 CURVE
- 验证 CENT 类型弧线处理正确

4. **测试案例 4：带 drns/drne 倾斜的路径**

- 多个点 + 倾斜的起始和结束方向
- 验证倾斜截面的变换正确

5. **测试案例 5：多个 SPINE 的情况**

- 一个 GENSEC 下有多个 SPINE
- 验证每个 SPINE 独立处理

每个测试案例应：

- 生成 CSG 网格
- 导出为 OBJ 格式文件
- 验证网格的连续性和正确性

## 文件修改清单

1. `src/prim_geo/spine.rs`

- 扩展 `SweepPath3D` 枚举，添加 `MultiSegment` 变体
- 实现多段路径的长度计算和连接逻辑

2. `src/prim_geo/profile.rs`

- 修改 `create_profile_geos` 函数中的 SPINE 处理逻辑
- 实现将多个 `Spine3D` 段连接成一条连续路径的函数

3. `src/prim_geo/sweep_solid.rs`

- 实现 `gen_csg_shape()` 和 `gen_csg_mesh()` 方法
- 实现单段和多段路径的 CSG 网格生成逻辑
- 参考 `src/geometry/csg.rs` 中其他几何体的实现方式

4. `src/rs_surreal/spatial.rs`（如需要）

- 检查 `cal_zdis_pkdi_in_section_by_spine` 函数是否需要调整

5. **测试文件**（新建）

- `src/test/test_sweep_solid_multi_segment.rs` - 测试多段路径
- `test_output/sweep_solid_*.obj` - 生成的 OBJ 模型文件

### To-dos

- [ ] 扩展 SweepPath3D 枚举，添加 MultiSegment 变体支持多段路径
- [ ] 实现将多个 Spine3D 段连接成连续路径的逻辑
- [ ] 修改 profile.rs 中的 create_profile_geos 函数，将多个段合并为一条路径
- [ ] 扩展 SweepSolid 的 gen_brep_shell 和 gen_occ_shape 方法，支持多段路径
- [ ] 检查并更新 spatial.rs 中的相关计算函数，确保多段路径处理正确