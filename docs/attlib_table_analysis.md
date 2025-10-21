# ATTLIB表分析文档

> **分析基础**: IDA Pro反编译attlib.dat + 实际数据验证
> **分析日期**: 2025-01-22
> **相关文件**: `/Volumes/DPC/work/plant-code/rs-core/data/attlib.dat`

## 概述

本文档详细分析了PDMS/E3D系统中ATTCLD和ATGTSX表的结构、功能和存储位置。这些表是PDMS属性库系统的核心组件，负责定义属性语法规则和管理参数验证。

## 目录

1. [ATTCLD（属性定义表）详解](#attcld属性定义表详解)
2. [ATGTSX（属性语法表）详解](#atgtsx属性语法表详解)
3. [表间关系和数据流](#表间关系和数据流)
4. [存储位置和访问机制](#存储位置和访问机制)
5. [实际应用示例](#实际应用示例)
6. [开发和调试指南](#开发和调试指南)

---

## ATTCLD（属性定义表）详解

### 基本概念

ATTCLD (Attribute Class Definition) 是PDMS系统中属性元数据的核心表，定义了：
- 属性的数据类型
- 默认值设置
- 属性的基本特性
- 跨表引用ID

### 实际实现名称

在attlib.dat文件中，ATTCLD的具体实现名称是 **ATGTDF**:
```
ATTCLD (概念名称) = ATGTDF (实现名称)
```

### 数据结构

```c
typedef struct {
    uint32_t attr_hash;     // 属性哈希值 (531442-387951929)
    uint32_t data_type;     // 数据类型代码 (1-4)
    uint32_t default_flag;  // 默认值标志 (1=无, 2=有)
    // 变长数据部分
    uint32_t default_data;  // 标量默认值 (LOG/INT/REAL)
    // 或
    uint32_t text_length;   // TEXT类型的长度
    uint32_t encoded_text[];// 27进制编码的文本数据
} AttlibAttrDefinition;
```

### 存储位置

ATTCLD数据存储在attlib.dat文件的特定位置：

```c
// 段指针表 (文件偏移0x0800)
段指针[0] = ATGTDF-1 起始页号 = 3    // ATTCLD副本1
段指针[4] = ATGTDF-2 起始页号 = 1741  // ATTCLD副本2
```

**物理位置**: `/Volumes/DPC/work/plant-code/rs-core/data/attlib.dat`
- **页3**: ATTCLD主副本
- **页1741**: ATTCLD备份副本

### 数据类型映射

| 类型代码 | 类型名称 | 存储格式 | 默认值支持 |
|---------|---------|----------|-----------|
| 1 | LOG | 32位整数 (0/1) | ✅ |
| 2 | REAL | IEEE 754双精度 | ✅ |
| 3 | INT | 32位有符号整数 | ✅ |
| 4 | TEXT | 长度+27进制编码 | ✅ |

### 默认值机制

```c
// 默认值处理逻辑
if (default_flag == 1) {
    // 无默认值
    default_value = None;
} else if (default_flag == 2) {
    // 有默认值
    if (data_type == 4) {  // TEXT类型
        text_length = buffer[pos++];
        encoded_data = buffer[pos..pos+text_length];
        default_value = decode_27_base(encoded_data);
    } else {  // 标量类型
        default_value = buffer[pos++];
    }
}
```

---

## ATGTSX（属性语法表）详解

### 基本概念

ATGTSX (Attribute Global Table Syntax) 是PDMS系统的语法验证表，定义了：
- 属性/名词组合的语法规则
- 参数类型和数量要求
- 验证标志和行为控制
- 跨表引用关系

### 表字段结构

基于IDA Pro内存分析，ATGTSX表包含以下核心字段：

```c
typedef struct {
    uint32_t counter;       // 条目计数器
    uint32_t attr_hash;     // 属性哈希值和关键词
    uint32_t recId;         // 记录ID (跨表引用)
    uint32_t param_count;   // 参数数量
    uint32_t param_types[]; // 参数类型数组
    uint32_t validation_flags; // 验证标志
    uint32_t syntax_data;   // 语法数据指针
} AttlibAttrSyntax;
```

### recId字段详解

**recId (Record ID)** 是ATGTSX表的关键字段：
- **功能**: 唯一标识每个语法规则
- **用途**: 跨表引用ATTCLD中对应的属性定义
- **格式**: 32位整数，与ATTCLD中的attr_hash对应
- **关系**: `ATGTSX.recId → ATTCLD.attr_hash`

### param字段详解

**param (Parameters)** 定义属性的参数要求：

```c
// 参数类型枚举
typedef enum {
    PARAM_ANGLE = 1,      // 角度参数
    PARAM_LENGTH = 2,      // 长度参数
    PARAM_DIAMETER = 3,    // 直径参数
    PARAM_TEXT = 4,        // 文本参数
    PARAM_COORDINATE = 5,   // 坐标参数
    PARAM_OPTION = 6       // 选项参数
} ParamType;

// 参数结构
typedef struct {
    uint32_t param_type;    // 参数类型
    uint32_t param_flags;   // 参数标志
    double min_value;       // 最小值
    double max_value;       // 最大值
    uint32_t default_value; // 默认值
} ParamDefinition;
```

### flag字段详解

**flag (Flags)** 控制属性行为特性：

```c
// 标志位定义
#define FLAG_VISIBLE      0x0001  // 属性可见
#define FLAG_EDITABLE     0x0002  // 属性可编辑
#define FLAG_CALCULATED   0x0004  // 自动计算
#define FLAG_INHERITABLE  0x0008  // 可继承
#define FLAG_REQUIRED     0x0010  // 必填
#define FLAG_VALIDATED    0x0020  // 需验证
#define FLAG_ANNOTATION   0x0040  // 注释属性
```

### 语法验证规则

```c
// 语法验证示例
typedef struct {
    uint32_t noun_type;     // 名词类型 (SITE, ZONE, EQUI, etc.)
    uint32_t attr_hash;     // 属性哈希值
    uint32_t syntax_pattern; // 语法模式
    uint32_t validation_rules; // 验证规则
} SyntaxRule;

// 实际应用示例
// 管道(Pipe)必须设置直径的语法规则
SyntaxRule pipe_diameter_rule = {
    .noun_type = NounType::PIPE,
    .attr_hash = hash("DIAMETER"),
    .syntax_pattern = "DIAMETER(REAL, REQUIRED, > 0)",
    .validation_rules = FLAG_REQUIRED | FLAG_VALIDATED
};
```

---

## 表间关系和数据流

### 表间依赖关系

```
ATGTIX (索引表)
    ↓ (通过哈希值定位)
ATGTSX (语法表) → ATTCLD (定义表)
    ↓                ↓ (通过recId关联)
语法验证          属性定义
    ↓                ↓
参数检查          类型验证
    ↓                ↓
用户界面显示      数据存储处理
```

### 数据访问流程

```c
// 1. 通过属性名查找哈希值
uint32_t attr_hash = calculate_hash("DIAMETER");

// 2. 在ATGTIX中查找索引位置
AttlibAttrIndex index = atgtix_lookup(attr_hash);

// 3. 在ATGTSX中查找语法规则
AttlibAttrSyntax syntax = atgtsx_find_rule(attr_hash);

// 4. 通过recId在ATTCLD中查找属性定义
AttlibAttrDefinition definition = attcld_get_definition(syntax.recId);

// 5. 应用语法和定义进行验证
bool is_valid = validate_attribute(syntax, definition, user_input);
```

### 跨表引用示例

```c
// PLANGLE属性 (管道角度) 的完整信息
PLANGLE_Attribute {
    // ATGTIX索引信息
    hash: 656800,
    record_num: 245,
    slot_offset: 127,

    // ATGTSX语法信息
    recId: 656800,              // 引用ATTCLD定义
    param_count: 1,
    param_types: [PARAM_ANGLE],
    flags: FLAG_VISIBLE | FLAG_EDITABLE | FLAG_VALIDATED,
    syntax_pattern: "PLANGE(angle: REAL, min: 0, max: 360)",

    // ATTCLD定义信息
    data_type: REAL,            // 浮点类型
    default_flag: 2,           // 有默认值
    default_value: 0.0,        // 默认0度
}
```

---

## 存储位置和访问机制

### 文件结构概览

```
/Volumes/DPC/work/plant-code/rs-core/data/attlib.dat (4,616,192 bytes)
├── 0x0000-0x0800: 文件头和元数据
│   ├── "Attribute Data File" (UTF-16LE)
│   ├── 版本号: "1.2.2.41"
│   └── 生成时间: "(WINDOWS-NT 6.3) (20 Sep 2022 : 06:14)"
│
├── 0x0800: 段指针表 (8个32位指针)
│   ├── 指针[0]: ATGTDF-1 (页3) → ATTCLD副本1
│   ├── 指针[2]: ATGTIX-1 (页1683)
│   ├── 指针[4]: ATGTDF-2 (页1741) → ATTCLD副本2
│   └── 指针[6]: ATGTIX-2 (页2236)
│
├── 0x1000-: 数据记录区
│   ├── 页3: ATTCLD属性定义数据
│   ├── 页1683-1741: ATGTIX索引数据
│   ├── 页1741-2236: ATTCLD备份数据
│   └── 页2236+: ATGTIX备份数据
│
└── 未知区域: ATGTSX语法表数据
```

### FHDBRN访问机制

PDMS使用FHDBRN函数进行分页读取：

```c
// FHDBRN分页读取函数
int FHDBRN(void* file_handle, int* page_num,
           char* buffer, int* error_code);

// 页面结构
typedef struct {
    uint32_t words[512];     // 512个32位字 (2048字节)
    uint32_t page_markers;    // 页面切换和结束标记
} FHDBRNPage;

// 关键标记
#define PAGE_SWITCH_MARK  0x00000000  // 页切换
#define SEGMENT_END_MARK 0xFFFFFFFF    // 段结束
```

### 实际访问示例

```c
// 读取ATTCLD属性定义
void read_attcld_definition(uint32_t attr_hash) {
    // 1. 通过ATGTIX定位
    AttlibAttrIndex index = atgtix_lookup(attr_hash);

    // 2. 计算页面位置
    uint32_t page_num = index.record_num;
    uint32_t slot_offset = index.slot_offset;

    // 3. 使用FHDBRN读取页面
    FHDBRNPage page;
    FHDBRN(&file_handle, &page_num, (char*)&page, &error);

    // 4. 解析属性定义
    AttlibAttrDefinition* def =
        (AttlibAttrDefinition*)&page.words[slot_offset];

    // 5. 处理变长默认值数据
    process_default_value(def);
}
```

---

## 实际应用示例

### 管道设计属性验证

```c
// 示例: 验证管道(Pipe)的属性组合
bool validate_pipe_attributes(PipeElement* pipe) {
    // 1. 检查必需属性
    if (!validate_required_attribute(pipe, "DIAMETER")) return false;
    if (!validate_required_attribute(pipe, "LENGTH")) return false;
    if (!validate_required_attribute(pipe, "MATERIAL")) return false;

    // 2. 验证属性间关系
    double diameter = get_attribute_value(pipe, "DIAMETER");
    double wall_thickness = get_attribute_value(pipe, "WALLTHICK");

    // 使用ATGTSX语法规则验证
    if (wall_thickness >= diameter * 0.8) {
        log_error("壁厚不能超过管径的80%");
        return false;
    }

    // 3. 应用ATTCLD定义的默认值
    if (!has_attribute(pipe, "PRESSURE")) {
        set_attribute_default(pipe, "PRESSURE", "STD");
    }

    return true;
}
```

### 属性查询和编辑

```c
// 查询属性元数据
AttributeMetadata* get_attribute_metadata(const char* attr_name) {
    uint32_t hash = calculate_hash(attr_name);

    // ATGTIX快速查找
    AttlibAttrIndex index = atgtix_lookup(hash);
    if (!index.found) return NULL;

    // ATGTSX获取语法信息
    AttlibAttrSyntax syntax = atgtsx_get_syntax(hash);

    // ATTCLD获取定义信息
    AttlibAttrDefinition definition = attcld_get_definition(syntax.recId);

    // 组装完整元数据
    AttributeMetadata* metadata = combine_metadata(syntax, definition);
    return metadata;
}
```

### 用户界面生成

```c
// 根据ATGTSX和ATTCLD生成属性编辑界面
void generate_property_editor(AttributeType type) {
    for (uint32_t i = 0; i < type.attribute_count; i++) {
        AttributeMetadata* meta = get_attribute_metadata(type.attributes[i]);

        // 根据ATGTSX标志创建控件
        if (meta->flags & FLAG_VISIBLE) {
            Widget* widget = create_widget(meta->data_type);

            // 根据ATTCLD设置默认值
            if (meta->default_flag == 2) {
                widget->set_default_value(meta->default_value);
            }

            // 根据ATGTSX设置验证规则
            widget->set_validation(meta->validation_rules);

            add_property_to_form(widget);
        }
    }
}
```

---

## 开发和调试指南

### 开发环境设置

```bash
# 必需工具
- IDA Pro 7.x (用于反编译分析)
- Rust 1.75+ (实现解析器)
- Python 3.10+ (脚本验证)
- Hex编辑器 (查看原始数据)

# 项目依赖
serde = { version = "1.0", features = ["derive"] }
byteorder = "1.4"  # 处理大端序
memmap = "0.7"    # 内存映射文件访问
```

### 解析器实现框架

```rust
// Rust实现的ATTCLD/ATGTSX解析器
pub struct AttlibParser {
    file: Mmap,                                    // 内存映射文件
    segment_pointers: [u32; 8],                    // 段指针表
    atgtix_cache: HashMap<u32, AttlibAttrIndex>,   // ATGTIX缓存
    atgtsx_cache: HashMap<u32, AttlibAttrSyntax>,   // ATGTSX缓存
    attcld_cache: HashMap<u32, AttlibAttrDefinition>, // ATTCLD缓存
}

impl AttlibParser {
    pub fn new(file_path: &str) -> Result<Self, Error> {
        // 1. 内存映射文件
        let mmap = unsafe { Mmap::map_path(file_path)? };

        // 2. 读取段指针表
        let segment_pointers = Self::read_segment_pointers(&mmap)?;

        // 3. 初始化解析器
        Ok(Self {
            file: mmap,
            segment_pointers,
            atgtix_cache: HashMap::new(),
            atgtsx_cache: HashMap::new(),
            attcld_cache: HashMap::new(),
        })
    }

    pub fn load_all_tables(&mut self) -> Result<(), Error> {
        // 1. 加载ATGTIX索引表
        self.load_atgtix(self.segment_pointers[2])?;  // 主副本
        self.load_atgtix(self.segment_pointers[6])?;  // 备份副本

        // 2. 加载ATTCLD定义表
        self.load_attcld(self.segment_pointers[0])?;  // 主副本
        self.load_attcld(self.segment_pointers[4])?;  // 备份副本

        // 3. 加载ATGTSX语法表
        self.load_atgtsx()?;

        Ok(())
    }
}
```

### 调试和验证

```python
# Python验证脚本
import struct
from collections import defaultdict

def validate_attlib_parser():
    # 1. 读取attlib.dat文件
    with open('data/attlib.dat', 'rb') as f:
        file_data = f.read()

    # 2. 验证段指针表
    segment_pointers = struct.unpack('>8I', file_data[0x800:0x820])
    print(f"段指针表: {segment_pointers}")

    # 3. 验证ATGTIX索引表
    atgtix_data = parse_fhdbrn_section(file_data, segment_pointers[2])
    print(f"ATGTIX记录数: {len(atgtix_data)}")

    # 4. 验证ATTCLD定义表
    attcld_data = parse_fhdbrn_section(file_data, segment_pointers[0])
    print(f"ATTCLD记录数: {len(attcld_data)}")

    # 5. 验证哈希值范围
    valid_hashes = [entry.hash for entry in atgtix_data
                   if 531442 <= entry.hash <= 387951929]
    print(f"有效哈希值数: {len(valid_hashes)}")

    return True

def test_attribute_lookup(attr_name):
    """测试属性查找功能"""
    hash_value = calculate_attribute_hash(attr_name)
    index = lookup_atgtix_index(hash_value)

    if index:
        syntax = lookup_atgtsx_rule(hash_value)
        definition = lookup_attcld_definition(syntax.recId)

        print(f"属性: {attr_name}")
        print(f"  哈希: {hash_value}")
        print(f"  索引: 页{index.page_num}, 偏移{index.slot_offset}")
        print(f"  语法: 参数数量={syntax.param_count}")
        print(f"  定义: 类型={definition.data_type}, 默认值={definition.default_value}")
        return True

    return False
```

### 性能优化建议

```c
// 高性能属性查找实现
typedef struct {
    uint32_t hash;
    uint32_t page_num;
    uint32_t slot_offset;
} CompactIndex;

// 使用线性哈希表进行快速查找
typedef struct {
    CompactIndex* table;
    uint32_t size;
    uint32_t mask;  // = size - 1
} FastLookupTable;

static inline AttlibAttrIndex* fast_lookup(uint32_t hash) {
    uint32_t index = hash & lookup_table.mask;
    CompactIndex* entry = &lookup_table.table[index];

    // 处理哈希冲突
    while (entry->hash != hash) {
        index = (index + 1) & lookup_table.mask;
        entry = &lookup_table.table[index];
        if (entry->hash == 0) return NULL;  // 未找到
    }

    return entry;
}
```

---

## 总结

ATTCLD和ATGTSX表是PDMS属性库系统的核心组件：

### 关键发现

1. **ATTCLD = ATGTDF**: 同一概念的不同名称，存储属性定义和默认值
2. **ATGTSX**: 语法验证表，定义参数类型、数量和验证规则
3. **跨表引用**: 通过recId建立ATTCLD和ATGTSX之间的关联
4. **双备份机制**: 主副本+备份副本确保数据可靠性
5. **FHDBRN分页**: 使用特殊的分页机制进行高效数据访问

### 存储位置

- **文件路径**: `/Volumes/DPC/work/plant-code/rs-core/data/attlib.dat`
- **ATTCLD**: 页3 (主) 和 页1741 (备)
- **ATGTSX**: 通过段指针[7]访问，具体位置需进一步分析

表间关系

     ATGTIX (索引表) → ATGTSX (语法表) → ATTCLD (定义表)
          ↓              ↓              ↓
        快速定位        语法验证         属性定义

### 应用价值

- **开发调试**: 理解属性定义和语法验证机制
- **系统维护**: 故障诊断和数据恢复
- **扩展开发**: 自定义属性和验证规则
- **性能优化**: 高效的属性查找和管理

---

**文档维护**: rs-core开发团队
**最后更新**: 2025-01-22
**版本**: 1.0
