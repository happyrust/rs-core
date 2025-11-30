# attlib.dat 属性解析器

## 概述

`attlib_parser` 模块实现了对 AVEVA PDMS/E3D 属性库文件 `attlib.dat` 的解析。该解析器基于 IDA Pro 对 `core.dll` 的逆向分析开发，完整实现了 FHDBRN 分页读取机制和 27 进制属性名称编解码。

## 文件结构

```text
attlib.dat 文件布局
├── 0x0000 - 0x07FF  文件头区域
├── 0x0800 - 0x0FFF  段指针表 (8 个 u32 指针)
└── 0x1000 - EOF     数据记录区域 (按 2048 字节分页)
```

### 段指针表

| 索引 | 含义 | 说明 |
|------|------|------|
| 0 | ATGTDF-1 | 属性定义段 1 |
| 1 | 保留 | - |
| 2 | ATGTIX-1 | 属性索引段 1 |
| 3 | ATGTSX | 语法表段 |
| 4 | ATGTDF-2 | 属性定义段 2 |
| 5 | 保留 | - |
| 6 | ATGTIX-2 | 属性索引段 2 |
| 7 | 保留 | - |

## 数据类型

| 类型值 | 名称 | Rust 类型 | 说明 |
|--------|------|-----------|------|
| 1 | LOG | bool | 逻辑/布尔值 |
| 2 | REAL | f32 | 单精度浮点数 |
| 3 | INT | i32 | 32 位有符号整数 |
| 4 | TEXT | String | 27 进制编码文本 |

## 核心常量

```rust
const PAGE_SIZE: usize = 2048;           // 页面大小（字节）
const WORDS_PER_PAGE: usize = 512;       // 每页 32 位字数
const MIN_HASH: u32 = 531442;            // 最小有效哈希 (0x81BF2)
const MAX_HASH: u32 = 387951929;         // 最大有效哈希 (0x171FAD39)
const PAGE_SWITCH_MARK: u32 = 0x00000000;// 页切换标记
const SEGMENT_END_MARK: u32 = 0xFFFFFFFF;// 段结束标记
const DATA_REGION_START: u64 = 0x1000;   // 数据区起始偏移
const SEGMENT_POINTERS_OFFSET: u64 = 0x0800; // 段指针表偏移
```

## 快速开始

### 基本使用

```rust
use aios_core::attlib_parser::{AttlibParser, AttlibDataType, encode_base27};

fn main() -> std::io::Result<()> {
    // 创建解析器
    let mut parser = AttlibParser::new("data/attlib.dat")?;
    
    // 加载所有数据
    parser.load_all()?;
    
    // 查询属性
    if let Some(attr) = parser.get_attribute_by_name("AEMAN") {
        println!("属性: {:?}", attr);
    }
    
    // 列出所有属性
    for attr in parser.list_all_attributes() {
        println!("{}: {:?}", attr.name, attr.data_type);
    }
    
    Ok(())
}
```

### 运行示例

```bash
# 基本解析
cargo run --example parse_attlib

# 导出 JSON
cargo run --example parse_attlib -- --json
```

## API 参考

### AttlibParser

| 方法 | 说明 |
|------|------|
| `new(path)` | 创建解析器实例 |
| `load_all()` | 加载所有数据 |
| `read_header()` | 读取文件头 |
| `get_attribute(hash)` | 按哈希查询属性 |
| `get_attribute_by_name(name)` | 按名称查询属性 |
| `get_full_attribute(hash)` | 获取完整属性信息 |
| `list_all_attributes()` | 列出所有属性 |
| `export_json()` | 导出为 JSON |

### 工具函数

| 函数 | 说明 |
|------|------|
| `encode_base27(name)` | 属性名称 → 哈希值 |
| `decode_hash_to_name(hash)` | 哈希值 → 属性名称 |
| `decode_base27(words)` | 27 进制编码 → 字符串 |

## 数据结构

### AttlibAttrIndex（属性索引）

```rust
pub struct AttlibAttrIndex {
    pub attr_hash: u32,  // 属性哈希值
    pub combined: u32,   // 组合位置信息
}

impl AttlibAttrIndex {
    pub fn record_num(&self) -> u32;   // 记录号 = combined / 512
    pub fn slot_offset(&self) -> u32;  // 槽偏移 = combined % 512
}
```

### AttlibAttrDefinition（属性定义）

```rust
pub struct AttlibAttrDefinition {
    pub attr_hash: u32,              // 属性哈希值
    pub data_type: u32,              // 数据类型 (1-4)
    pub default_flag: u32,           // 默认值标志 (1=无, 2=有)
    pub default_value: AttlibDefaultValue,
}
```

### AttlibDefaultValue（默认值）

```rust
pub enum AttlibDefaultValue {
    None,              // 无默认值
    Scalar(u32),       // 标量值 (LOG/INT/REAL)
    Text(Vec<u32>),    // 文本值 (27 进制编码)
}
```

### AttlibAttribute（完整属性）

```rust
pub struct AttlibAttribute {
    pub hash: u32,
    pub name: String,
    pub data_type: AttlibDataType,
    pub default_value: AttlibDefaultValue,
    pub default_text: Option<String>,
}
```

## PDMS 哈希编码

基于 `db_tool.rs` 中的 `db1_dehash_uncached` 函数：

### 哈希范围

| 范围 | 类型 | 编码方式 |
|------|------|----------|
| ≤ 0x81BF1 | 无效 | - |
| 0x81BF2 ~ 0x171FAD39 | 普通属性 | 27 进制 |
| > 0x171FAD39 | UDA 属性 | 64 进制 |

### 27 进制字符映射

```text
k % 27 + 64 → ASCII
余数 0 → '@', 1 → 'A', 2 → 'B', ... 26 → 'Z'
```

### 编码示例

```rust
// 解码 (基于 db1_dehash_uncached)
let name = decode_hash_to_name(0x0009C18E);  // → "NAME"
let size = decode_hash_to_name(0x0009E770);  // → "SIZE"

// UDA 属性
let uda = decode_hash_to_name(0x20000000);   // → ":..."
```

## 相关文档

- [attlib_parsing_logic.md](../attlib_parsing_logic.md) - 详细解析逻辑分析
- [attlib_naming_conventions.md](../attlib_naming_conventions.md) - 命名约定
- [attlib_table_analysis.md](../attlib_table_analysis.md) - 表结构分析

## IDA Pro 关键函数

| 函数名 | 地址 | 说明 |
|--------|------|------|
| FHDBRN | 0x10766040 | 分页读取函数 |
| ATTLIB_Load_Index_ATGTIX | 0x10852A64 | 属性索引加载 |
| ATTLIB_Load_Def_ATGTDF | 0x10852E20 | 属性定义加载 |
| Attlib_DecodePackCode | 0x10853FC8 | 27 进制解码 |
