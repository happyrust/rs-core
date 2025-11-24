# SweepLoft 截面局部变换重构方案

## 📋 重构目标

将 SweepLoft 的截面局部变换逻辑从直接修改几何体改为使用统一的 `get_local_transform()` 变换系统，实现关注点分离：

- **截面保持不变**: SPRO/SANN/SREC 截面数据保持原始坐标
- **路径变换**: 将 SweepPath 的每个段（POINSP/CURVE）变换到局部坐标系
- **方位继承**: SweepSolid 的整体方位继承路径起始点的局部变换

## 🎯 核心设计原则

1. **几何体与变换分离**: 几何体只负责形状描述，空间变换由矩阵系统处理
2. **性能优化**: 变换几何体参数而非采样点，减少计算量
3. **架构统一**: 与现有 `get_local_transform()` 系统完全集成
4. **逻辑清晰**: 截面定义形状，路径定义空间轨迹

## 🔄 实施方案

### 1. 修改 SweepSolid 结构

**文件**: `src/prim_geo/sweep_solid.rs`

```rust
pub struct SweepSolid {
    pub profile: CateProfileParam,
    pub drns: Option<DVec3>,
    pub drne: Option<DVec3>,
    pub bangle: f32,
    pub plax: Vec3,
    pub extrude_dir: DVec3,
    pub height: f32,
    pub path: SweepPath3D,
    pub lmirror: bool,
    pub first_segment_refno: Option<RefnoEnum>,  // 新增：第一段路径的 refno
}
```

**变更点**:
- 添加 `first_segment_refno` 字段存储路径起始点的实体引用

### 2. 修改 get_trans() 方法

**文件**: `src/prim_geo/sweep_solid.rs`

```rust
#[inline]
async fn get_trans(&self) -> bevy_transform::prelude::Transform {
    // 使用路径起始点的局部变换方位
    if let Some(first_refno) = self.first_segment_refno {
        if let Ok(Some(local_transform)) = get_local_transform(first_refno).await {
            let scale = self.get_scaled_vec3();
            return Transform {
                rotation: local_transform.rotation,
                scale,
                translation: Vec3::ZERO,  // 位置由路径本身处理
            };
        }
    }
    
    // 回退方案
    Transform {
        rotation: Quat::IDENTITY,
        scale: self.get_scaled_vec3(),
        translation: Vec3::ZERO,
    }
}
```

**变更点**:
- `get_trans()` 改为 async 函数
- 使用路径起始点的局部变换方位
- 移除硬编码的 IDENTITY 返回值

### 3. 变换路径几何体

**文件**: `src/geometry/sweep_mesh.rs`

#### 3.1 变换 Line3D

```rust
fn transform_line(line: &Line3D, transform: &Transform) -> Line3D {
    Line3D {
        start: transform.transform_point(line.start),
        end: transform.transform_point(line.end),
        is_spine: line.is_spine,
    }
}
```

#### 3.2 变换 Arc3D

```rust
fn transform_arc(arc: &Arc3D, transform: &Transform) -> SegmentPath {
    // 检查缩放类型
    let scale = transform.scale;
    let is_uniform_scale = (scale.x - scale.y).abs() < 1e-6 
                        && (scale.y - scale.z).abs() < 1e-6;
    
    if is_uniform_scale {
        // 均匀缩放：直接变换参数
        SegmentPath::Arc(Arc3D {
            center: transform.transform_point(arc.center),
            start_pt: transform.transform_point(arc.start_pt),
            radius: arc.radius * scale.x,
            axis: (transform.rotation * arc.axis).normalize(),
            angle: arc.angle,
            clock_wise: arc.clock_wise,
            pref_axis: (transform.rotation * arc.pref_axis).normalize(),
        })
    } else {
        // 非均匀缩放：转换为多段线近似
        convert_arc_to_polyline(arc, transform)
    }
}
```

#### 3.3 修改 sample_path_frames()

```rust
async fn sample_path_frames(
    segments: &[SegmentPath],
    arc_segments_per_segment: usize,
    plax: Vec3,
    spine_segments: &[Spine3D],  // 新增：用于获取实体 refno
) -> Option<Vec<PathSample>> {
    let mut transformed_segments = Vec::new();
    
    // 1. 变换每个段
    for (i, segment) in segments.iter().enumerate() {
        let entity_refno = spine_segments[i].refno;
        let transform = get_local_transform(entity_refno)
            .await
            .ok()
            .flatten()
            .unwrap_or(Transform::IDENTITY);
        
        let transformed_segment = match segment {
            SegmentPath::Line(line) => {
                SegmentPath::Line(transform_line(line, &transform))
            }
            SegmentPath::Arc(arc) => {
                transform_arc(arc, &transform)
            }
        };
        transformed_segments.push(transformed_segment);
    }
    
    // 2. 从变换后的段采样（使用现有采样逻辑）
    sample_from_transformed_segments(&transformed_segments, arc_segments_per_segment, plax)
}
```

### 4. 修改生成函数

**文件**: `src/geometry/sweep_mesh.rs`

```rust
pub async fn generate_sweep_solid_mesh(
    sweep: &SweepSolid,
    settings: &LodMeshSettings,
    refno: Option<RefU64>,
) -> Option<PlantMesh> {
    // 生成原始截面数据（不变换）
    let profile = get_profile_data(&sweep.profile, refno)?;
    
    // 需要获取 Spine3D 段信息
    let spine_segments = extract_spine_segments_from_path(&sweep.path)?;
    
    let frames = sample_path_frames(
        &sweep.path.segments, 
        arc_segments, 
        Vec3::Z, 
        &spine_segments
    ).await?;
    
    // 生成网格（不再需要后处理变换）
    let mesh = generate_mesh_from_frames(&profile, &frames, sweep.drns, sweep.drne);
    
    Some(mesh)
}
```

**文件**: `src/prim_geo/profile.rs`

```rust
// 创建 SweepSolid 时设置 first_segment_refno
let mut solid = SweepSolid {
    profile: profile.clone(),
    drns,
    drne,
    bangle,
    plax,
    extrude_dir,
    height,
    path: sweep_path,
    lmirror: att.get_bool("LMIRR").unwrap_or_default(),
    first_segment_refno: spine_paths.first().map(|s| s.refno),
};
```

## 📁 修改文件清单

1. **src/prim_geo/sweep_solid.rs**
   - 添加 `first_segment_refno` 字段
   - 修改 `get_trans()` 为 async 并使用局部变换
   - 更新相关构造函数

2. **src/geometry/sweep_mesh.rs**
   - 添加 `transform_line()` 和 `transform_arc()` 函数
   - 修改 `sample_path_frames()` 支持段变换
   - 修改 `generate_sweep_solid_mesh()` 为 async
   - 简化 `get_profile_data()`（移除变换逻辑）

3. **src/prim_geo/profile.rs**
   - 修改 `create_profile_geos()` 设置 `first_segment_refno`
   - 更新 SweepSolid 构造调用

4. **src/transform/mod.rs**
   - `get_local_transform()` 已修改为单参数（已完成）

## ✅ 验证方法

1. **单元测试**: 运行现有的 SweepSolid 相关测试
2. **集成测试**: 验证 GENSEC/WALL 的扫掠几何体生成
3. **变换测试**: 确认局部变换正确应用到路径
4. **性能测试**: 对比重构前后的生成性能

## 🎯 预期收益

1. **架构改善**: 实现几何体与变换的清晰分离
2. **性能提升**: 减少采样点变换的计算开销
3. **维护性**: 统一使用变换系统，易于扩展
4. **一致性**: 与其他几何体处理方式保持统一

## ⚠️ 注意事项

1. **异步传播**: `get_trans()` 改为 async 会影响调用链
2. **缩放处理**: 非均匀缩放时圆弧需要特殊处理
3. **兼容性**: 确保现有测试用例继续通过
4. **缓存考虑**: `get_local_transform()` 已有缓存，性能影响最小

---

**创建时间**: 2024-11-24  
**状态**: 🟡 待实施  
**优先级**: 高（架构重构）
