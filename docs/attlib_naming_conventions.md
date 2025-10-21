# AVEVA PDMS/E3D attlib.dat 命名约定指南

## 概述

本文档总结了 AVEVA PDMS/E3D 系统 `attlib.dat` 文件中的命名约定和缩写规则，这些约定来自系统源代码的反编译分析。

## 核心数据结构命名

### ATGTIX - 属性全局表索引 (Attribute Global Table Index)

**完整名称**: `ATGTIX`  
**含义**: Attribute Global Table Index  
**功能**: 属性索引表，存储属性哈希值和定位信息

**命名构成**:
- `ATG` = Attribute Global (属性全局)
- `T` = Table (表)
- `IX` = Index (索引)

**数据结构**:
```rust
struct AttlibAttrIndex {
    attr_hash: u32,     // 属性哈希值 (531442-387951929)
    combined: u32,      // 组合定位字
}
```

**分解逻辑**:
```rust
record_num = combined / 512;    // 目标页号
slot_offset = combined % 512;   // 页内偏移
```

### ATGTDF - 属性全局表定义 (Attribute Global Table Definition)

**完整名称**: `ATGTDF`  
**含义**: Attribute Global Table Definition  
**功能**: 属性定义表，存储属性元数据和默认值

**命名构成**:
- `ATG` = Attribute Global (属性全局)
- `T` = Table (表)
- `DF` = Definition (定义)

**数据结构**:
```rust
struct AttlibAttrDefinition {
    attr_hash: u32,     // 属性哈希值
    data_type: u32,     // 数据类型代码
    default_flag: u32,  // 默认值标志
    default_value: any, // 默认值数据
}
```

**重要说明**: ATGTDF就是ATTCLD（Attribute Class Definition）的具体实现，两者是同一概念的不同名称。

### ATGTSX - 属性全局语法表 (Attribute Global Table Syntax)

**完整名称**: `ATGTSX`  
**含义**: Attribute Global Table Syntax  
**功能**: 属性/名词几何语法表，定义语法验证规则和参数要求

**命名构成**:
- `ATG` = Attribute Global (属性全局)
- `T` = Table (表)  
- `SX` = Syntax (语法)

**数据结构**:
```rust
struct AttlibAttrSyntax {
    attr_hash: u32,     // 属性哈希值
    param_count: u32,   // 参数数量
    param_types: Vec<u32>, // 参数类型列表
    validation_rules: u32, // 验证规则标志
    syntax_pattern: String, // 语法模式字符串
}
```

**表字段说明**:
- **recId (Record ID)**: 属性规则的唯一标识符，用于跨表引用
- **param (Parameters)**: 定义属性所需参数的类型、数量和顺序
- **flag (Flags)**: 控制属性行为（可见性、编辑权限、计算方式、继承性）

**功能特性**:
- **语法验证**: 确保属性/名词组合符合PDMS设计规范
- **参数管理**: 统一管理所有设计属性的参数定义
- **质量控制**: 通过标志位确保设计的一致性和完整性

## 数据类型映射

基于 `core.dll` 反编译分析，完整的数据类型映射如下：

### 基本类型 (1-4) - attlib.dat 支持存储默认值

| 类型代码 | 类型名称 | DB_Attribute 定义 | AttrVal 映射 | 存储支持 |
|---------|---------|------------------|-------------|----------|
| `1` | `LOG` | `BOOL` | `BoolType(bool)` | ✅ |
| `2` | `REAL` | `DOUBLE` | `DoubleType(f64)` | ✅ |
| `3` | `INT` | `INTEGER` | `IntegerType(i32)` | ✅ |
| `4` | `TEXT` | `STRING` | `StringType(String)` | ✅ |

### 扩展类型 (5-12) - 仅运行时支持

| 类型代码 | 类型名称 | DB_Attribute 定义 | 说明 |
|---------|---------|------------------|------|
| `5` | `REF` | `ELEMENT` | 元素引用 (DB_Element) |
| `6` | `NAME` | `WORD` | Noun名称 (DB_Noun*) |
| `7` | `ATTRIBUTE` | - | 属性引用 (DB_Attribute*) |
| `8` | `POINT` | `POSITION` | 3D点坐标 |
| `9` | `VECTOR` | `DIRECTION` | 3D向量 |
| `10` | `MATRIX` | `ORIENTATION` | 3D矩阵 |
| `11` | `TRANSFORM` | - | 3D变换 |
| `12` | `DATETIME` | `DATETIME` | 日期时间 |

## 文件结构和段组织

### attlib.dat 文件架构

```
attlib.dat
├── ATGTIX-1 (页1683): 属性索引表副本1 - 256条记录
├── ATGTDF-1 (页3):    属性定义表副本1 - 0条记录*
├── ATGTIX-2 (页2236): 属性索引表副本2 - 256条记录
└── ATGTDF-2 (页1741): 属性定义表副本2 - 0条记录*

* ATGTDF段在当前文件版本中为空，定义数据存储在其他位置
```

### 段指针表 (Segment Pointer Table)

位置: 文件偏移 0x0800 (2048字节)
格式: 8个32位大端序整数

| 索引 | 段名称 | 典型页号 | 说明 |
|------|--------|----------|------|
| 0 | ATGTDF-1 (ATTCLD) | 3 | 属性定义表副本1 |
| 1 | (保留) | 4 | 未使用 |
| 2 | ATGTIX-1 | 1683 | 属性索引表副本1 |
| 3 | (保留) | 1704 | 未使用 |
| 4 | ATGTDF-2 (ATTCLD) | 1741 | 属性定义表副本2 |
| 5 | (保留) | 1742 | 未使用 |
| 6 | ATGTIX-2 | 2236 | 属性索引表副本2 |
| 7 | ATGTDF-3 (ATGTSX) | 未知 | 语法表访问入口 |

**注意**: ATGTDF段就是ATTCLD的实际存储位置，两者是同一概念的两种名称。

## 哈希值范围约定

- **最小值**: 531442 (0x00081992)
- **最大值**: 387951929 (0x171F2939)
- **验证逻辑**: `MIN_HASH ≤ hash ≤ MAX_HASH`

## 特殊标记值

### 段控制标记
- `SEGMENT_END_MARK`: 0xFFFFFFFF - 段结束标记
- `PAGE_SWITCH_MARK`: 0x00000000 - 页切换标记

### 页面组织
- **页面大小**: 2048字节 (512个32位字)
- **数据格式**: 大端序 (Big Endian)
- **存储机制**: Fortran风格顺序文件

## 加载顺序约定

基于 IDA Pro 反编译分析的正确加载顺序：

```rust
// 1. 加载ATGTIX-1 (属性索引表副本1)
load_atgtix(segment_pointers[2], "ATGTIX-1");

// 2. 加载ATGTDF-1 (属性定义表副本1)
load_atgtdf(segment_pointers[0], "ATGTDF-1");

// 3. 加载ATGTIX-2 (属性索引表副本2)
load_atgtix(segment_pointers[6], "ATGTIX-2");

// 4. 加载ATGTDF-2 (属性定义表副本2)
load_atgtdf(segment_pointers[4], "ATGTDF-2");
```

## 调试日志约定

系统使用 `MTRENT` 宏输出调试信息：

```cpp
MTRENT("ATGTIX", 6u, (int)"\n");  // 开始加载ATGTIX
MTRENT("ATGTDF", 6u, (int)"\n");  // 开始加载ATGTDF
```

## 版本差异说明

### 当前文件版本特征
- **ATGTIX**: ✅ 正常 (512条唯一记录)
- **ATGTDF**: ❌ 为空 (设计特性)
- **属性定义**: 通过core.dll动态提供

### 数据源层次说明
1. **权威来源**: `core.dll` (IDA Pro反编译分析)
2. **运行时结构**: `attlib.dat` (分页存储格式)
3. **验证工具**: `JSON文件` (从core.dll提取的中间产物)

## 开发注意事项

### 解析器实现要点
1. 使用大端序读取32位整数
2. 正确处理页面边界和切换标记
3. 验证哈希值范围以过滤无效数据
4. 支持连续SEGMENT_END_MARK的结束检测

### 错误处理
- ATGTDF为空是正常现象，不应视为错误
- 索引查询失败时应检查哈希值有效性
- 页面切换时需要重置相关状态

### 扩展开发
- 新增数据类型需同步更新映射表
- 哈希算法变更需重新计算范围边界
- 跨版本兼容性需考虑段结构变化

---

**文档版本**: 1.0  
**分析基础**: IDA Pro 反编译 + 实文件解析  
**最后更新**: 2025-01-22
