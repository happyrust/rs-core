# normalize_spine_segments 重构方案

## 1. 背景与问题

### 1.1 当前问题

`normalize_spine_segments` 函数（profile.rs:51-445，约 395 行）过于复杂，混合了两种完全不同的场景：

1. **SPINE 路径**：从 SPINE 子元素（POINSP/CURVE）构建的多段路径
2. **POSS/POSE 路径**：从元素的 POSS/POSE 属性构建的单段直线

函数内部有多个条件分支：
- `is_from_poss_pose` 分支（第86-131行）：路径沿 Z 轴单位化，用 transforms 还原
- `is_single_line` 分支（第134-164行）：使用实际路径，但仍计算 transforms
- `has_curve` 分支（第166-277行）：使用实际几何，transforms 为空
- 多段直线分支（第279-428行）：路径单位化到 100.0，用 transforms 还原

这种"单位化路径 + transforms 还原"的设计增加了不必要的复杂度。

### 1.2 重构目标

1. **拆分职责**：将 POSS/POSE 和 SPINE 的处理逻辑完全分离
2. **简化逻辑**：所有路径都使用实际几何坐标，不做单位化处理
3. **删除冗余**：移除 `segment_transforms` 字段及相关复杂逻辑

## 2. 重构方案

### 2.1 拆分为两个独立函数

#### 函数 A：`convert_spine_to_segments` - 处理 SPINE 路径

```rust
/// 将 SPINE 路径（POINSP/CURVE）转换为 SegmentPath 列表
/// 所有段使用实际几何坐标，不做单位化
fn convert_spine_to_segments(segments: &[Spine3D]) -> anyhow::Result<Vec<SegmentPath>> {
    const EPSILON: f32 = 1e-3;
    let mut result = Vec::new();

    if segments.is_empty() {
        return Ok(result);
    }

    // 连续性检查（仅 warning）
    for i in 1..segments.len() {
        let prev_end = segments[i - 1].pt1;
        let curr_start = segments[i].pt0;
        if prev_end.distance(curr_start) > EPSILON {
            tracing::warn!("Spine 段不连续: 段 {} 到段 {}", i - 1, i);
        }
    }

    // 遍历所有段，按实际几何生成 SegmentPath
    for spine in segments.iter() {
        match spine.curve_type {
            SpineCurveType::LINE => {
                result.push(SegmentPath::Line(Line3D {
                    start: spine.pt0,
                    end: spine.pt1,
                    is_spine: true,
                }));
            }
            SpineCurveType::THRU => {
                // 三点圆弧：计算圆心、半径、角度、轴
                let center = circum_center(spine.pt0, spine.pt1, spine.thru_pt);
                let radius = center.distance(spine.pt0);
                // ... 计算 angle, axis
                result.push(SegmentPath::Arc(Arc3D { center, radius, ... }));
            }
            SpineCurveType::CENT => {
                // 中心点已知的圆弧
                let center = spine.center_pt;
                // ... 类似处理
                result.push(SegmentPath::Arc(Arc3D { ... }));
            }
            SpineCurveType::UNKNOWN => {
                return Err(anyhow!("未知的曲线类型"));
            }
        }
    }

    Ok(result)
}
```

#### 函数 B：`convert_poss_pose_to_segment` - 处理 POSS/POSE 路径

```rust
/// 将 POSS/POSE 两点转换为 SegmentPath
/// 直接使用实际坐标，不做单位化
fn convert_poss_pose_to_segment(poss: Vec3, pose: Vec3) -> SegmentPath {
    SegmentPath::Line(Line3D {
        start: poss,
        end: pose,
        is_spine: true,
    })
}
```

### 2.2 修改调用方 `create_profile_geos`

当前逻辑（profile.rs:604-693）将 POSS/POSE 转换为 `Spine3D` 后统一调用 `normalize_spine_segments`。

重构后：

```rust
// 在 create_profile_geos 中区分两种情况

let segments: Vec<SegmentPath> = if has_poss_pose {
    // POSS/POSE 场景：直接转换
    if let (Some(poss), Some(pose)) = (att.get_poss(), att.get_pose()) {
        vec![convert_poss_pose_to_segment(poss, pose)]
    } else {
        return Ok(false);
    }
} else if !spine_paths.is_empty() {
    // SPINE 场景：转换所有段
    convert_spine_to_segments(&spine_paths)?
} else {
    return Ok(false);
};

// 构建 SweepPath3D 和 SweepSolid
let sweep_path = SweepPath3D::from_segments(segments);
let loft = SweepSolid {
    profile: profile.clone(),
    path: sweep_path,
    // ... 其他字段（不再需要 segment_transforms）
};
```

### 2.3 删除 `segment_transforms` 字段

从 `SweepSolid` 结构体中删除 `segment_transforms` 字段，需修改以下位置：

| 文件 | 行号 | 修改内容 |
|------|------|----------|
| `sweep_solid.rs` | 55 | 删除 `pub segment_transforms: Vec<Transform>` 字段 |
| `sweep_solid.rs` | 138 | 删除 `Default::default()` 中的 `segment_transforms` |
| `sweep_solid.rs` | 179, 187-191, 204, 216 | 删除 `hash_unit_mesh_params()` 中相关逻辑 |
| `sweep_solid.rs` | 254 | 删除 `gen_unit_shape()` 中的 `unit.segment_transforms = ...` |
| `sweep_solid.rs` | 269-272 | 简化 `get_scaled_vec3()` 返回 `Vec3::ONE` |
| `sweep_solid.rs` | 281-282 | 简化 `get_trans()` 返回 `Transform::IDENTITY` |
| `profile.rs` | 735 | 删除构建 `SweepSolid` 时的 `segment_transforms` 字段 |

### 2.4 调整实例化 transform 策略

**核心原则**：除了"单线段 + 无倾斜面"场景使用 `geo_transform` 实现复用，其他情况统一使用 `Transform::IDENTITY`。

```rust
// profile.rs 中构建 CateCsgShape 时
let is_simple_line = loft.path.as_single_line().is_some() && !loft.is_sloped();

let geo_transform = if is_simple_line {
    // 单线段+无倾斜：geo_transform 包含旋转和缩放（用于复用）
    Transform {
        translation: poss,           // 起点位置
        rotation: final_rotation,    // 方位
        scale: Vec3::new(1.0, 1.0, length / 100.0),  // Z 方向缩放
    }
} else {
    // 其他情况：实际几何已在正确坐标系，不需要额外变换
    Transform::IDENTITY
};

csg_shapes_map.push(CateCsgShape {
    csg_shape: Box::new(loft),
    transform: geo_transform,  // 复用信息在这里
    // ...
});
```

**说明**：这与当前代码逻辑一致，只是将 `segment_transforms` 的职责转移到 `geo_transform`。

## 2.5 当前代码状态分析

### 现有策略已符合目标

**结论：当前代码已经实现了"除了单线段+无倾斜面，其他情况 geo_transform 不需要额外变换"的策略。**

| 场景 | segment_transforms | 实例 transform | 符合要求 |
|------|-------------------|----------------|----------|
| 包含圆弧路径 | **空** | `IDENTITY` | ✅ |
| 多段直线路径 | 每段有 | `IDENTITY`（GENSEC/WALL） | ✅ |
| 单线段 + 有倾斜 | 1个（rotation） | 只用旋转，scale=ONE | ✅ |
| 单线段 + 无倾斜 | 1个（完整） | first_transform（可复用） | ✅ |

### 关键代码证据

1. **圆弧路径返回空 transforms**：`profile.rs:276`
   ```rust
   // has_curve 分支结束时
   return Ok((normalized_segments, transforms)); // transforms 为空
   ```

2. **实例 transform 策略**：`profile.rs:745-805`
   ```rust
   let is_simple_line = loft.path.as_single_line().is_some() && !loft.is_sloped();
   // GENSEC/WALL:
   //   is_simple_line → first_transform（可复用）
   //   is_sloped_line → rotation only
   //   其他 → Transform::IDENTITY
   ```

3. **复用判断**：`sweep_solid.rs:153-158`
   ```rust
   fn is_reuse_unit(&self) -> bool {
       self.path.as_single_line().is_some() && !self.is_sloped()
   }
   ```

### 重构方案与现有策略的兼容性

| 场景 | 当前实现 | 重构后 | 兼容性 |
|------|----------|--------|--------|
| 圆弧/多段 | transforms 为空 | 删除字段 | ✅ 无影响 |
| 单线段+倾斜 | 只用 rotation | 使用实际几何 | ✅ 可简化 |
| 单线段+无倾斜 | 完整 transform | 通过 geo_transform 保留 | ✅ 兼容 |

## 3. 修改文件清单

| 文件 | 修改类型 | 说明 |
|------|----------|------|
| `src/prim_geo/profile.rs` | 重写 | 删除 `normalize_spine_segments`，新增两个简单函数，修改调用逻辑 |
| `src/prim_geo/sweep_solid.rs` | 修改 | 删除 `segment_transforms` 字段及相关方法 |
| `src/geometry/sweep_mesh.rs` | 修改 | 修改 `sample_path_frames_sync()` 和圆弧精度计算逻辑 |
| `src/geometry/mod.rs` | 检查 | 确认 CSG shape 构建不依赖 `segment_transforms` |

## 3.1 sweep_mesh.rs 详细修改方案（补充）

### 3.1.1 segment_transforms 使用位置分析

`sweep_mesh.rs` 中共有 **4 个核心使用位置**：

| 位置 | 行号 | 功能 | 严重程度 |
|------|------|------|----------|
| 位置1 | 420 | 单段圆弧变换 | 中 |
| 位置2 | 436 | 多段路径变换 | **高（核心）** |
| 位置3 | 561 | 参考方向推导 | 中 |
| 位置4 | 1268 | 圆弧精度计算 | 中 |

### 3.1.2 各位置详细修改方案

#### 位置1：单段圆弧变换（行 420）

**当前代码：**
```rust
if segments.len() == 1 {
    if let SegmentPath::Arc(arc) = &segments[0] {
        let transform = segment_transforms.first().unwrap_or(&Transform::IDENTITY);
        let transformed_arc = transform_arc(arc, transform);
        // ...
    }
}
```

**修改方案：** 删除变换逻辑，直接使用原始圆弧
```rust
if segments.len() == 1 {
    if let SegmentPath::Arc(arc) = &segments[0] {
        // 前提：调用方已确保 arc 在正确坐标系下
        return sample_arc_frames(arc, arc_segments_per_segment, plax);
    }
}
```

---

#### 位置2：多段路径变换（行 436）— 核心修改

**当前代码：**
```rust
let mut transformed_segments = Vec::new();
for (i, segment) in segments.iter().enumerate() {
    let transform = segment_transforms.get(i).unwrap_or(&Transform::IDENTITY);
    let transformed_segment = match segment {
        SegmentPath::Line(line) => SegmentPath::Line(transform_line(line, transform)),
        SegmentPath::Arc(arc) => transform_arc(arc, transform),
    };
    transformed_segments.push(transformed_segment);
}
```

**修改方案：** 删除变换逻辑，直接使用原始段
```rust
// 前提：调用方已确保 segments 在实际坐标系下
let transformed_segments = segments.to_vec();
```

**关键前提：** 重构 `convert_spine_to_segments` 时，必须确保输出的 `Vec<SegmentPath>` 已经是实际几何坐标。

---

#### 位置3：参考方向推导（行 561）

**影响分析：** 此处从 `transformed_segments` 推导 `ref_up`，如果位置2正确处理，此处无需修改。

---

#### 位置4：圆弧精度计算（行 1268）

**当前代码：**
```rust
for (i, seg) in sweep.path.segments.iter().enumerate() {
    let SegmentPath::Arc(arc) = seg else { continue };
    let tf = sweep.segment_transforms.get(i).unwrap_or(&Transform::IDENTITY);
    let plane_scale = arc_plane_max_scale(arc, tf);
    let radius = arc.radius.abs() * plane_scale;
    // ...
}
```

**修改方案：** 由于圆弧已使用实际几何，plane_scale = 1.0
```rust
for (i, seg) in sweep.path.segments.iter().enumerate() {
    let SegmentPath::Arc(arc) = seg else { continue };
    // 不再需要缩放校正，圆弧已是实际尺寸
    let radius = arc.radius.abs();
    let arc_len = arc.angle.abs() * radius;
    let segs = compute_arc_segments(settings, arc_len, radius);
    max_segs = max_segs.max(segs);
}
```

### 3.1.3 函数签名修改

**sample_path_frames_sync 函数：**

```rust
// 修改前
fn sample_path_frames_sync(
    segments: &[SegmentPath],
    arc_segments_per_segment: usize,
    plax: Vec3,
    segment_transforms: &[Transform],  // 删除此参数
) -> Option<Vec<PathSample>>

// 修改后
fn sample_path_frames_sync(
    segments: &[SegmentPath],
    arc_segments_per_segment: usize,
    plax: Vec3,
) -> Option<Vec<PathSample>>
```

### 3.1.4 核心约束（必须满足）

**重构成功的关键前提：**

1. **预变换策略**：`convert_spine_to_segments` 必须输出已变换到实际坐标系的路径段
2. **坐标系一致性**：所有 `SegmentPath` 在构造时就是"世界坐标系"下的实际几何
3. **精度信息保留**：圆弧的 `radius` 字段必须是实际半径，不是归一化的 1.0

## 4. 代码量估算

| 项目 | 变化 |
|------|------|
| 删除 `normalize_spine_segments` | -395 行 |
| 新增 `convert_spine_to_segments` | +60 行 |
| 新增 `convert_poss_pose_to_segment` | +8 行 |
| 修改 `create_profile_geos` 调用 | ±50 行 |
| 修改 `SweepSolid` | -30 行 |
| **净减少** | **~310 行** |

## 5. 实施步骤

### 步骤 1：新增两个转换函数
在 `profile.rs` 中添加 `convert_spine_to_segments` 和 `convert_poss_pose_to_segment` 函数。

### 步骤 2：修改 `create_profile_geos` 调用逻辑
区分 POSS/POSE 和 SPINE 两种情况，分别调用对应的转换函数。

### 步骤 3：删除 `normalize_spine_segments` 函数
确认新逻辑正确后，删除旧的复杂函数。

### 步骤 4：删除 `SweepSolid.segment_transforms` 字段
从结构体定义和所有使用处删除该字段。

### 步骤 5：调整实例化 transform 策略
统一使用 `Transform::IDENTITY`。

### 步骤 6：编译测试
确保所有代码编译通过，运行相关测试验证功能正确。

## 6. 风险与注意事项

1. **兼容性**：删除 `segment_transforms` 会影响序列化格式，如有持久化数据需注意兼容
2. **测试覆盖**：需确保 GENSEC、WALL、STWALL、SCTN 等类型的几何生成仍正确
3. **端面倾斜**：DRNS/DRNE 的处理逻辑需保留在 `SweepSolid` 的 mesh 生成中
4. **STWALL 遗漏**：当前 `profile.rs` 第515行的 SPINE 收集条件未包含 STWALL，需补充

### 6.1 补充 STWALL 处理

```rust
// profile.rs 第515行修改建议
let mut spine_paths = if type_name == "GENSEC"
    || type_name == "WALL"
    || type_name == "STWALL" {  // 新增
    // ...
}
```

## 7. 实施前提条件

### 7.1 硬性前提

| 条件 | 状态 | 验证方法 |
|------|------|----------|
| 完整的测试覆盖 | 需确认 | 运行 `cargo test` |
| 基准几何输出 | 需准备 | 导出 GENSEC/WALL/STWALL 样例的 OBJ |
| 数据迁移方案 | 需设计 | SurrealDB 版本兼容层 |
| 缓存失效策略 | 已有 | mesh_sig.json 签名机制 |

### 7.2 数据迁移

| 数据源 | 操作 |
|--------|------|
| SurrealDB `inst_geo` 表 | 需要版本兼容层或重新生成 |
| Foyer 缓存目录 | 建议清空重建 |
| `mesh_sig.json` 签名文件 | 签名版本号需升级触发重建 |

### 7.3 验证清单

- [ ] GENSEC 单段直线生成正确
- [ ] GENSEC 多段直线生成正确
- [ ] GENSEC 含圆弧路径生成正确
- [ ] WALL 弧形墙生成正确
- [ ] STWALL 结构墙生成正确
- [ ] DRNS/DRNE 端面倾斜生成正确
- [ ] 布尔运算结果正确
- [ ] LOD 多级精度正确
- [ ] 缓存复用率统计无下降

## 8. 总体评价

### 8.1 风险等级

| 风险类型 | 等级 | 说明 |
|----------|------|------|
| 功能回归 | 中 | 需完整回归测试 |
| 数据兼容 | 高 | 需迁移方案 |
| 性能影响 | 低 | 预计无负面影响 |

### 8.2 建议结论

**✅ 建议采纳**

- 当前代码已符合"除了单线段+无倾斜面，其他情况 geo_transform 不需要额外变换"的目标策略
- sweep_mesh.rs 修改方案已补充完整
- 复用信息通过 `geo_transform` 承载，可完全删除 `segment_transforms` 字段

实施前仍需：
1. 设计 rkyv 序列化兼容层
2. 完善测试覆盖后再进行重构
3. 采用渐进式实施策略（准备 → 核心重构 → 清理）

