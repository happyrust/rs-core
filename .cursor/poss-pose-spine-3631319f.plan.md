<!-- 3631319f-a344-4fa2-967a-4bf255bc143d 2d6c5901-3771-4473-92d2-e69e6ef5dd87 -->
# 重构目标

将 `profile.rs` 中第 389-463 行的 POSS/POSE 处理逻辑重构为复用 `normalize_spine_segments` 函数，消除代码重复。

## 当前问题

在 `create_profile_geos` 函数中，当 `spine_paths.len() == 0` 且存在 POSS/POSE 时（第 389-463 行），代码手动实现了：

- 创建归一化的 Line3D 路径（从原点沿 Z 轴 10.0 单位）
- 计算 Frenet 标架旋转（`build_frenet_rotation`）
- 计算 bangle 旋转
- 组合旋转并创建 Transform

这与 `normalize_spine_segments` 函数中处理 `SpineCurveType::LINE` 的逻辑（第 105-132 行）完全相同，造成代码重复。

## 重构方案

### 1. 将 POSS/POSE 转换为 Spine3D::LINE 段

在 `spine_paths.len() == 0` 的分支中，当检测到 POSS/POSE 时：

- 创建一个 `Spine3D` 结构，类型为 `SpineCurveType::LINE`
- `pt0 = poss`，`pt1 = pose`
- 设置 `preferred_dir`（可以使用 plax 或默认值）

### 2. 统一调用 normalize_spine_segments

将创建的单个 `Spine3D::LINE` 段放入 `Vec<Spine3D>`，然后调用 `normalize_spine_segments` 函数处理，与多段 SPINE 的处理路径统一。

### 3. 复用返回结果

使用 `normalize_spine_segments` 返回的：

- `normalized_paths`: 归一化的路径段列表
- `segment_transforms`: 每段的完整变换（包含 position、rotation、scale）

### 4. 代码位置

- **修改文件**: `/Volumes/DPC/work/plant-code/rs-core/src/prim_geo/profile.rs`
- **主要修改区域**: 第 389-463 行的 `if spine_paths.len() == 0` 分支
- **需要调整**: 
- 将 POSS/POSE 转换为 `Spine3D::LINE` 段
- 调用 `normalize_spine_segments` 替代手动计算
- 复用返回的路径和变换数据

## 实施步骤

1. 在 POSS/POSE 分支中，创建 `Spine3D::LINE` 段并添加到 `spine_paths`
2. 将处理逻辑合并到统一的 `normalize_spine_segments` 调用路径
3. 移除重复的路径创建、Frenet 标架计算、bangle 旋转等手动实现代码
4. 确保 `SweepSolid` 的构建使用 `normalize_spine_segments` 返回的数据

## 预期效果

- 消除代码重复，POSS/POSE 和 SPINE 使用相同的归一化和变换逻辑
- 提高代码可维护性，路径处理逻辑集中在一处
- 保持功能一致性，确保两种路径的处理方式完全一致

### To-dos

- [ ] 将 POSS/POSE 转换为 Spine3D::LINE 段，创建包含单个 LINE 段的 spine_paths
- [ ] 将 POSS/POSE 的处理合并到统一的 normalize_spine_segments 调用路径，移除重复的手动计算代码
- [ ] 复用 normalize_spine_segments 返回的 normalized_paths 和 segment_transforms，更新 SweepSolid 构建逻辑
- [ ] 验证重构后功能保持一致，确保 POSS/POSE 和 SPINE 路径处理结果相同