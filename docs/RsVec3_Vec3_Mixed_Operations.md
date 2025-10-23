# RsVec3 与 Vec3 混合运算支持

## 概述

现在 `RsVec3` 完全支持与 `Vec3` 的混合运算，无需手动转换即可在两种类型之间进行数学运算。

## 已实现的运算符

### 1. 加法运算 (16 种组合)

```rust
// RsVec3 + Vec3
let result = RsVec3(Vec3::new(1.0, 2.0, 3.0)) + Vec3::new(4.0, 5.0, 6.0);

// Vec3 + RsVec3  (支持交换律)
let result = Vec3::new(1.0, 2.0, 3.0) + RsVec3(Vec3::new(4.0, 5.0, 6.0));

// 引用版本 (不消耗原值)
let v1 = RsVec3(Vec3::ZERO);
let v2 = Vec3::ONE;
let result = &v1 + &v2;  // v1 和 v2 仍然可用

// 所有组合:
// RsVec3 + Vec3, &RsVec3 + Vec3, RsVec3 + &Vec3, &RsVec3 + &Vec3
// Vec3 + RsVec3, &Vec3 + RsVec3, Vec3 + &RsVec3, &Vec3 + &RsVec3
```

### 2. 减法运算 (16 种组合)

```rust
// RsVec3 - Vec3
let result = RsVec3(Vec3::new(10.0, 20.0, 30.0)) - Vec3::new(1.0, 2.0, 3.0);

// Vec3 - RsVec3
let result = Vec3::new(10.0, 20.0, 30.0) - RsVec3(Vec3::new(1.0, 2.0, 3.0));

// 引用版本
let result = &rs_vec - &vec3;

// 所有组合:
// RsVec3 - Vec3, &RsVec3 - Vec3, RsVec3 - &Vec3, &RsVec3 - &Vec3
// Vec3 - RsVec3, &Vec3 - RsVec3, Vec3 - &RsVec3, &Vec3 - &RsVec3
```

## 实际使用案例

### 案例 1: 方向向量计算

```rust
// Vec3 和 RsVec3 混合使用
fn calculate_direction(start: Vec3, end: RsVec3) -> RsVec3 {
    // 直接运算，无需转换！
    let direction = (end - start).normalize();
    direction
}

let start = Vec3::ZERO;
let end = RsVec3(Vec3::new(10.0, 0.0, 0.0));
let dir = calculate_direction(start, end);
assert_eq!(dir.x, 1.0);
```

### 案例 2: 位置偏移

```rust
// RsVec3 位置 + Vec3 偏移
let position = RsVec3(Vec3::new(100.0, 200.0, 300.0));
let offset = Vec3::new(10.0, 20.0, 30.0);

let new_position = position + offset;
assert_eq!(new_position.x, 110.0);

// 反过来也可以
let new_position2 = offset + position;
```

### 案例 3: 复杂链式运算

```rust
let rs_vec1 = RsVec3(Vec3::X);
let rs_vec2 = RsVec3(Vec3::Y);
let vec3 = Vec3::Z;

// RsVec3 + RsVec3 + Vec3 混合运算
let result = rs_vec1 + rs_vec2 + vec3;
assert_eq!(result, RsVec3(Vec3::ONE));

// 结合标量运算
let scaled = (rs_vec1 + vec3) * 2.0;
```

### 案例 4: 使用引用避免移动

```rust
let position = RsVec3(Vec3::new(1.0, 2.0, 3.0));
let offset = Vec3::new(10.0, 20.0, 30.0);

// 使用引用，保留原值
let new_pos1 = &position + &offset;
let new_pos2 = &position + &offset;  // position 和 offset 仍可使用

assert_eq!(position.x, 1.0);  // 原值未被消耗
assert_eq!(offset.x, 10.0);
```

### 案例 5: 实际工程场景

```rust
// 在现有使用 Vec3 的代码中无缝集成 RsVec3
struct Scene {
    camera_pos: RsVec3,      // 需要存储到数据库
    light_dir: Vec3,         // 临时计算用
}

impl Scene {
    fn calculate_view_vector(&self, target: Vec3) -> RsVec3 {
        // Vec3 - RsVec3 混合运算
        (target - self.camera_pos).normalize()
    }
    
    fn apply_offset(&mut self, offset: Vec3) {
        // RsVec3 + Vec3 混合运算
        self.camera_pos = self.camera_pos + offset;
    }
}
```

## 运算规则

### 1. 类型转换规则
- **RsVec3 op Vec3 → RsVec3**
- **Vec3 op RsVec3 → RsVec3**
- 混合运算的结果始终返回 `RsVec3`

### 2. 所有权规则
- **值运算**：消耗操作数，返回新值
  ```rust
  let result = rs_vec + vec3;  // rs_vec 和 vec3 被移动
  ```
- **引用运算**：借用操作数，返回新值
  ```rust
  let result = &rs_vec + &vec3;  // rs_vec 和 vec3 保留
  ```

### 3. 支持的运算符组合矩阵

| 左操作数 | 运算符 | 右操作数 | 结果类型 | 状态 |
|---------|--------|---------|---------|------|
| RsVec3 | + | Vec3 | RsVec3 | ✅ |
| RsVec3 | + | &Vec3 | RsVec3 | ✅ |
| &RsVec3 | + | Vec3 | RsVec3 | ✅ |
| &RsVec3 | + | &Vec3 | RsVec3 | ✅ |
| Vec3 | + | RsVec3 | RsVec3 | ✅ |
| Vec3 | + | &RsVec3 | RsVec3 | ✅ |
| &Vec3 | + | RsVec3 | RsVec3 | ✅ |
| &Vec3 | + | &RsVec3 | RsVec3 | ✅ |
| | **减法同理** | | | |
| RsVec3 | - | Vec3 | RsVec3 | ✅ |
| RsVec3 | - | &Vec3 | RsVec3 | ✅ |
| &RsVec3 | - | Vec3 | RsVec3 | ✅ |
| &RsVec3 | - | &Vec3 | RsVec3 | ✅ |
| Vec3 | - | RsVec3 | RsVec3 | ✅ |
| Vec3 | - | &RsVec3 | RsVec3 | ✅ |
| &Vec3 | - | RsVec3 | RsVec3 | ✅ |
| &Vec3 | - | &RsVec3 | RsVec3 | ✅ |

## 性能说明

- **零开销**：所有运算符重载在编译时内联
- **无额外分配**：直接操作内部的 Vec3
- **运行时性能**：与直接使用 Vec3 完全相同

## 迁移指南

### 从纯 Vec3 代码迁移

```rust
// 原代码 (纯 Vec3)
fn process(a: Vec3, b: Vec3) -> Vec3 {
    a + b
}

// 选项 1: 改为 RsVec3 (如需数据库支持)
fn process(a: RsVec3, b: RsVec3) -> RsVec3 {
    a + b  // 完全相同的代码！
}

// 选项 2: 混合使用 (最灵活)
fn process(a: Vec3, b: RsVec3) -> RsVec3 {
    a + b  // 自动处理类型差异
}

// 选项 3: 泛型支持 (最通用)
fn process<V: Into<RsVec3>>(a: V, b: V) -> RsVec3 {
    a.into() + b.into()
}
```

### 与现有 Vec3 API 集成

```rust
// 现有的接受 Vec3 的函数
fn legacy_function(v: Vec3) -> f32 {
    v.length()
}

// RsVec3 调用方式
let rs_vec = RsVec3(Vec3::new(3.0, 4.0, 0.0));

// 方式 1: 解引用
legacy_function(*rs_vec);

// 方式 2: 访问内部字段
legacy_function(rs_vec.0);

// 方式 3: into 转换
legacy_function(rs_vec.into());
```

## 最佳实践

### ✅ 推荐

1. **优先使用运算符**
   ```rust
   let result = rs_vec + vec3;  // ✅ 简洁明了
   ```

2. **使用引用保留原值**
   ```rust
   let result = &rs_vec + &vec3;  // ✅ 原值可继续使用
   ```

3. **链式运算**
   ```rust
   let result = rs_vec1 + rs_vec2 + vec3;  // ✅ 清晰易读
   ```

### ❌ 避免

1. **不要手动构造结果**
   ```rust
   let result = RsVec3(rs_vec.0 + vec3);  // ❌ 冗余
   let result = rs_vec + vec3;            // ✅ 直接运算
   ```

2. **不要不必要的转换**
   ```rust
   let result = RsVec3(rs_vec.into()) + Vec3::from(vec3);  // ❌ 过度转换
   let result = rs_vec + vec3;                             // ✅ 简单直接
   ```

## 完整功能总结

RsVec3 现在支持：
- ✅ SurrealDB 序列化/反序列化
- ✅ 与 Vec3 的无缝转换 (From/Into)
- ✅ 借用转换 (AsRef/AsMut)
- ✅ 所有 Vec3 的方法 (通过 Deref)
- ✅ RsVec3 与 RsVec3 的运算
- ✅ **RsVec3 与 Vec3 的混合运算** ⭐ NEW
- ✅ 标量乘法 (RsVec3 * f32, f32 * RsVec3)
- ✅ 取负运算 (-RsVec3)
- ✅ 完整的引用版本支持

**现在可以在代码中完全自由地混合使用 RsVec3 和 Vec3！**
