# RsVec3 无缝使用指南

## 概述

`RsVec3` 是对 `glam::Vec3` 的封装，提供了以下额外功能：
- ✅ 实现了 `SurrealValue` trait，可以无缝存储到 SurrealDB
- ✅ 完全兼容 `Vec3` 的所有方法和字段访问
- ✅ 支持所有常用的数学运算符
- ✅ 实现了序列化/反序列化（rkyv, serde）

## 核心特性

### 1. 自动解引用 (Deref)

由于 `RsVec3` 实现了 `Deref` 和 `DerefMut`，你可以直接调用 `Vec3` 的所有方法：

```rust
let v = RsVec3(Vec3::new(3.0, 4.0, 0.0));

// 直接调用 Vec3 的方法
let length = v.length();           // 5.0
let normalized = v.normalize();    // 归一化
let dot = v.dot(other_vec);        // 点积
let cross = v.cross(other_vec);    // 叉积

// 直接访问字段
println!("x={}, y={}, z={}", v.x, v.y, v.z);
```

### 2. 类型转换

#### From/Into 转换

```rust
// Vec3 -> RsVec3
let vec3 = Vec3::new(1.0, 2.0, 3.0);
let rs_vec: RsVec3 = vec3.into();

// RsVec3 -> Vec3
let back: Vec3 = rs_vec.into();

// 或直接访问内部字段
let inner_vec3 = rs_vec.0;
```

#### AsRef/AsMut 转换

```rust
let rs_vec = RsVec3(Vec3::new(1.0, 2.0, 3.0));

// 借用转换
let vec_ref: &Vec3 = rs_vec.as_ref();

// 可变借用
let mut rs_vec = RsVec3(Vec3::ZERO);
let vec_mut: &mut Vec3 = rs_vec.as_mut();
vec_mut.x = 10.0;
```

### 3. 数学运算符

#### 标量乘法

```rust
let v = RsVec3(Vec3::new(1.0, 2.0, 3.0));

// RsVec3 * f32
let result1 = v * 2.0;        // (2.0, 4.0, 6.0)

// f32 * RsVec3 (支持交换律)
let result2 = 2.0 * v;        // (2.0, 4.0, 6.0)

// 引用也可以
let result3 = &v * 2.0;       // 不消耗原值
```

#### 向量加减法

```rust
let v1 = RsVec3(Vec3::new(1.0, 2.0, 3.0));
let v2 = RsVec3(Vec3::new(4.0, 5.0, 6.0));

// 加法
let sum = v1 + v2;            // (5.0, 7.0, 9.0)

// 减法
let diff = v2 - v1;           // (3.0, 3.0, 3.0)

// 引用也可以
let sum_ref = &v1 + &v2;      // 不消耗原值
```

#### 取负

```rust
let v = RsVec3(Vec3::new(1.0, -2.0, 3.0));
let neg = -v;                 // (-1.0, 2.0, -3.0)

// 引用也可以
let neg_ref = -&v;
```

## 实际使用示例

### 示例 1：方向向量计算

```rust
fn calculate_direction(start: RsVec3, end: RsVec3) -> RsVec3 {
    // 完全像使用 Vec3 一样！
    let direction = (end - start).normalize();
    direction
}

let start = RsVec3(Vec3::new(0.0, 0.0, 0.0));
let end = RsVec3(Vec3::new(10.0, 0.0, 0.0));
let dir = calculate_direction(start, end);
```

### 示例 2：位置变换

```rust
fn transform_position(pos: RsVec3, offset: RsVec3, scale: f32) -> RsVec3 {
    (pos + offset) * scale
}

let pos = RsVec3(Vec3::new(1.0, 2.0, 3.0));
let offset = RsVec3(Vec3::new(10.0, 20.0, 30.0));
let result = transform_position(pos, offset, 0.5);
```

### 示例 3：与现有 Vec3 代码混用

```rust
// 现有的 Vec3 函数
fn calculate_with_vec3(v: Vec3) -> Vec3 {
    v.normalize() * 10.0
}

// RsVec3 可以无缝调用
let rs_vec = RsVec3(Vec3::new(3.0, 4.0, 0.0));

// 方式 1: 通过 Deref 自动解引用
let result = calculate_with_vec3(*rs_vec);

// 方式 2: 显式转换
let result = calculate_with_vec3(rs_vec.into());

// 方式 3: 直接访问内部字段
let result = calculate_with_vec3(rs_vec.0);
```

### 示例 4：存储到 SurrealDB

```rust
use surrealdb::types::SurrealValue;

#[derive(Serialize, Deserialize)]
struct Position {
    point: RsVec3,
}

let pos = Position {
    point: RsVec3(Vec3::new(10.0, 20.0, 30.0))
};

// 自动序列化为 SurrealDB 数组: [10.0, 20.0, 30.0]
db.create("positions").content(pos).await?;

// 从数据库读取时自动反序列化
let loaded: Position = db.select(("positions", id)).await?;
```

## 在原有 Vec3 代码中替换

### 场景 1: 函数参数

```rust
// 原代码
fn process(v: Vec3) -> Vec3 {
    v * 2.0
}

// 改为 RsVec3（两种方式）

// 方式 1: 直接替换
fn process(v: RsVec3) -> RsVec3 {
    v * 2.0  // 运算符重载，无需修改！
}

// 方式 2: 泛型（同时支持两种类型）
fn process<V: AsRef<Vec3>>(v: V) -> Vec3 {
    v.as_ref() * 2.0
}
```

### 场景 2: 结构体字段

```rust
// 原代码
struct Transform {
    position: Vec3,
    direction: Vec3,
}

// 改为 RsVec3
struct Transform {
    position: RsVec3,
    direction: RsVec3,
}

// 使用时几乎不需要改代码
impl Transform {
    fn apply_offset(&mut self, offset: RsVec3) {
        self.position = self.position + offset;  // 运算符重载
    }
    
    fn get_scaled_direction(&self, scale: f32) -> RsVec3 {
        self.direction * scale  // Deref + 运算符重载
    }
}
```

### 场景 3: 返回值转换

```rust
// 如果需要返回 Vec3
fn get_vec3() -> Vec3 {
    let rs_vec = RsVec3(Vec3::new(1.0, 2.0, 3.0));
    
    // 方式 1: into()
    rs_vec.into()
    
    // 方式 2: 直接访问
    rs_vec.0
    
    // 方式 3: 解引用
    *rs_vec
}
```

## 最佳实践

### ✅ 推荐做法

1. **优先使用运算符**：`v1 + v2` 而不是 `RsVec3(v1.0 + v2.0)`
2. **利用 Deref**：直接调用 Vec3 方法，如 `v.normalize()`
3. **使用引用避免移动**：`&v1 + &v2` 而不是 `v1 + v2`（如果后续还需要使用）
4. **混合使用**：需要 Vec3 时用 `.into()` 或 `.0`

### ❌ 避免做法

1. **不要手动构造**：避免 `RsVec3(Vec3::new(x + y, ...))`，使用 `v1 + v2`
2. **不要重复转换**：如果已经是 RsVec3，不需要再转 Vec3 再转回来

## 性能说明

- **零开销抽象**：`RsVec3` 只是 `Vec3` 的简单封装（newtype pattern）
- **运算符重载**：编译时内联，运行时性能与直接使用 `Vec3` 相同
- **Deref 自动解引用**：编译期处理，无运行时开销

## 总结

`RsVec3` 提供了：
- ✅ 完全兼容 Vec3 的 API
- ✅ 额外的 SurrealDB 支持
- ✅ 零学习成本：像使用 Vec3 一样使用
- ✅ 零性能开销：编译期优化

**记住：你可以把 RsVec3 当作 Vec3 来用，它只是多了数据库序列化能力！**
