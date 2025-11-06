# 属性中文名 Hover 提示功能实现说明

## 功能概述

在属性面板中，当鼠标悬停（hover）到属性名称的标签上时，显示对应的中文名称作为提示。

## 实现方案

采用**全局缓存预加载**方案，在应用启动时从数据库一次性加载所有属性的中文名到内存缓存中，UI 层直接从缓存查询，无需每次访问数据库。

## 技术实现

### 1. 全局缓存模块

**文件：** `rs-core/src/rs_surreal/attr_cache.rs`

创建了全局属性中文名缓存：

```rust
pub static ATTR_CN_NAME_CACHE: Lazy<RwLock<HashMap<String, String>>> = ...;
```

核心函数：

- `load_attr_cn_names()` - 从数据库加载所有属性中文名到缓存
- `get_attr_cn_name(attr_name: &str)` - 从缓存中快速查询属性中文名
- `is_cache_loaded()` - 检查缓存是否已加载
- `cache_size()` - 获取缓存中的属性数量

### 2. 数据库初始化时加载

**文件：** `rs-core/src/lib.rs`

在以下函数中添加了缓存加载调用：

- `init_test_surreal()` - 测试环境初始化
- `init_surreal()` - 生产环境初始化

```rust
// 加载属性中文名缓存
rs_surreal::load_attr_cn_names().await?;
```

### 3. UI 层使用缓存

**文件：** `rs-plant3-d/src/utils/field.rs`

在 `field_pair_grid()` 函数中，渲染属性标签时添加 hover 提示：

```rust
let label_response = ui.label(label);

// 如果能从全局缓存中获取到中文名，添加 hover 提示
if let Some(cn_name) = aios_core::get_attr_cn_name(&label_text) {
    label_response.on_hover_text(cn_name);
}
```

## 数据来源

属性中文名数据存储在 `att_meta` 表中，定义在：

**文件：** `rs-core/resource/surreal/attr_metadata.surql`

表结构：
- `id` - 属性名称（如 "NAME", "REFNO", "OWNER"）
- `meta_cn_name` - 中文名称（如 "名称", "参考号", "所有者"）
- `hash` - 属性名的哈希值

## 性能优势

1. **启动时一次性加载** - 所有属性中文名在应用启动时加载到内存
2. **O(1) 查询速度** - 使用 HashMap 实现，查询效率极高
3. **无网络开销** - UI 层无需访问数据库
4. **线程安全** - 使用 `RwLock` 保证并发访问安全

## 使用示例

### 在其他 UI 组件中使用

如果需要在其他地方获取属性的中文名：

```rust
use aios_core::get_attr_cn_name;

// 获取属性中文名
if let Some(cn_name) = get_attr_cn_name("NAME") {
    println!("NAME 的中文名是: {}", cn_name);
}
```

### 检查缓存状态

```rust
use aios_core::{is_cache_loaded, cache_size};

if is_cache_loaded() {
    println!("缓存已加载，共有 {} 个属性", cache_size());
}
```

## 扩展性

如果需要添加更多元数据（如属性描述、属性类型等），可以：

1. 在 `attr_metadata.surql` 中添加字段
2. 在 `attr_cache.rs` 中修改缓存结构和加载逻辑
3. 提供相应的查询函数

## 注意事项

1. **初始化顺序** - 必须在数据库连接成功后才能加载缓存
2. **错误处理** - 如果加载失败，应用启动会报错
3. **内存占用** - 当前约有 700+ 个属性，内存占用可忽略不计
4. **热更新** - 如果数据库中的中文名更新，需要重启应用才能生效

## 测试建议

1. 启动应用，检查日志中是否有 "已加载 XXX 个属性中文名称到缓存" 信息
2. 在属性面板中，鼠标悬停到属性名上，验证是否显示中文提示
3. 测试常用属性：NAME、REFNO、OWNER、TYPE、POS、ORI 等
4. 测试 UDA 属性（以 `:` 开头的属性）

## 相关文件

- `rs-core/src/rs_surreal/attr_cache.rs` - 缓存模块
- `rs-core/src/rs_surreal/mod.rs` - 模块导出
- `rs-core/src/lib.rs` - 初始化调用
- `rs-core/resource/surreal/attr_metadata.surql` - 数据定义
- `rs-plant3-d/src/utils/field.rs` - UI 使用

## 更新日志

- 2025-11-05: 初版实现，支持属性名 hover 显示中文名


