# AVEVA E3D `attlib.dat` 文件格式解析

> 最近一次校验：2025-06-20  
> 关联二进制：`core.dll`（IDA Pro 反编译分析）
> 基于 FHDBRN Fortran I/O 机制

## 1. 文件结构概览

`attlib.dat` 是 AVEVA E3D/PDMS 的属性库文件，采用 Fortran 风格的二进制存储格式。文件通过 `core.dll` 中的 `FHDBRN` 函数进行记录式读取。

### 1.1 核心特征
- **存储类型**：Fortran 顺序文件，使用 `FHDBRN` 函数读取
- **记录单位**：每条记录包含 512 个 32 位字（2048 字节）
- **字节序**：大端序（Big Endian）
- **内存映射**：支持分页加载，按需读取特定记录

### 1.2 主要加载流程（core.dll）
- `sub_10851210`：外层入口，文件管理与调度
- `Attlib_LoadAttrIndex` (`0x10852A64`)：加载 ATGTIX 属性索引段
- `Attlib_LoadAttrDefinitions` (`0x10852E20`)：加载属性定义与默认值
- `Attlib_LoadSyntaxTable` (`0x108533B4`)：加载语法/别名表
- `Attlib_DecodePackCode` (`0x10853FC8`)：27 进制压缩解码
- `sub_1044FC20`：FHDBRN 读取函数的具体实现

---

## 2. 物理文件布局

### 2.1 文件头结构

```
偏移 0x0000: UTF-16LE 编码字符串
├── 0x0000: "Attribute Data File"
├── 0x0014: 版本号 "1.2.2.41"  
├── 0x0024: 生成时间 "(WINDOWS-NT 6.3) (20 Sep 2022 : 06:14)"
└── 0x0100: 全零填充区域（记录对齐）
```

### 2.2 段指针表

```
偏移 0x0800: 8 个 32 位段指针（大端序）
├── +0x00: 段1偏移
├── +0x04: 段2偏移
├── ...
└── +0x1C: 段8偏移
```

### 2.3 数据记录区

```
偏移 0x1000+: 实际数据段
└── 多个 2048 字节记录块
    ├── 每块含 512 个 32 位字
    ├── 以 0 作为页切换标记
    └── 以 -1(0xFFFFFFFF) 作为段结束标记
```

---

## 3. FHDBRN 读取机制详解

基于对 `sub_1044FC20` 函数的反编译分析：

```c
// FHDBRN 核心调用
v10 = FHDBRN(&dword_11E1E860, &page_num, 
             buffer_ptr, &error_code);
```

### 3.1 工作原理
1. **记录定位**：通过页号 `page_num` 定位到具体记录
2. **缓冲区读取**：每次读取 2048 字节到缓冲区
3. **页切换机制**：遇到 0 值表示当前页结束，切换下一页
4. **段结束检测**：遇到 -1 表示当前数据段结束

### 3.2 分页遍历要点
- **固定页长**：每页 2048 字节 = 512 个大端 `u32`，`FHDBRN` 始终整页读取。
- **页号含义**：段指针表提供的值即记录号（`offset / 2048`），直接作为 `FHDBRN` 的 `page_num`。
- **关键标记**：
  - `0x00000000` → 当前页数据结束，下一个读取周期需将 `page_num += 1`。
  - `0xFFFFFFFF` → 段结束，立即停止遍历并返回。
- **遍历骨架**：

```text
page = section_start
while True:
    words = read_page(page)                  // 512 个 u32
    for word in words:
        if word == 0x00000000:               // 页切换
            page += 1
            break                            // 重新读取下一页
        if word == 0xFFFFFFFF:               // 段终止
            return collected
        consume(word)                        // 由具体段解析逻辑处理
```

> `consume(word)` 需要维护当前游标（例如 ATGTIX 每 3 个词形成一条记录），否则跨页拼接会错位。

### 3.3 记录解析逻辑
```c
// 从 Attlib_LoadAttrIndex 函数提取
while (1) {
    word = buffer[++index];           // 读取 32 位字（attr_hash）

    if (word < RANGE_MIN || word > RANGE_MAX)  // 范围检查
        break;

    attr_hash = word;                 // 属性哈希值
    uint32_t combined = buffer[++index];  // 读取组合字（包含 record_num 和 slot_offset）
    record_num = combined / 512;      // 记录号 = 商
    slot_offset = combined % 512;     // 页内偏移 = 余数

    if (word == 0) {                  // 页切换
        page_num++;
        continue;
    }

    if (word == -1) {                 // 段结束
        break;
    }
}
```

---

## 4. ATGTIX 属性索引段结构

基于 `Attlib_LoadAttrIndex` 函数分析：

### 4.1 AttlibAttrIndex 结构
```c
typedef struct {
    uint32_t attr_hash;     // 属性哈希值 (531442 - 387951929)
    uint32_t combined;      // 组合字（包含 record_num 和 slot_offset）
} AttlibAttrIndex;
```

#### 二元组字段说明
| 顺序 | 字段        | 含义                                   | 备注 |
|------|-------------|----------------------------------------|------|
| 0    | `attr_hash` | 属性 / Noun 哈希值                     | 531 442 – 387 951 929，低于此值直接丢弃 |
| 1    | `combined`  | 组合字（包含 record_num 和 slot_offset）| 需要分解：`record_num = combined / 512`，`slot_offset = combined % 512` |

#### 分解方式
```c
uint32_t combined = index_entry[1];
uint32_t record_num = combined / 512;    // 商 = 页号
uint32_t slot_offset = combined % 512;   // 余数 = 页内偏移
```

根据二元组可以立即定位 ATGTDF 段中的属性定义：

```
byte_offset = record_num * 2048 + slot_offset * 4
// 或等价的简化形式
byte_offset = combined * 4
```

解析器通常在读取到 `combined` 后直接加载对应页的 512 个词，然后从 `slot_offset` 开始解析 `[attr_hash, data_type, default_flag, ...]`，确保跨页数据不会错位。

### 4.2 哈希值范围说明

属性哈希值范围 **531442 - 387951929** 是从 AVEVA E3D `core.dll` 的 IDA Pro 反编译分析中提取的实际边界值：

```c
// 从 Attlib_LoadAttrIndex 函数 (0x10852A64) 反编译得到
v13 = 531442;                           // 最小哈希值
v14 = 387951929;                        // 最大哈希值

if (v15 < v10 || v15 > v11)            // 范围检查: 531442 - 387951929
    break;
```

#### 边界值含义：
- **531442 (0x81952)**: 最小有效属性哈希值，低于此值的为系统保留或无效值
- **387951929 (0x171FAD39)**: 最大有效属性哈希值，超过此值进入 UDA 属性范围

#### UDA 属性边界：
```python
if hash_val > 0x171FAD39:  # UDA属性标识
    k = ((hash_val - 0x171FAD39) % 0x1000000)
```

#### 设计目的：
1. **系统稳定性**: 防止属性索引表访问越界
2. **内存安全**: 确保哈希查找不会访问无效内存区域  
3. **类型区分**: 区分标准属性和用户定义属性(UDA)
4. **性能优化**: 限制哈希表大小以提高查找效率

### 4.3 读取流程
```c
// 关键代码片段 (0x10852A91)
MTRENT("ATGTIX", 6u, (int)"\n");        // 调试日志开始

v13 = FHDBRN(&dword_11E1E860, &v12, var80C, &unk_10AB23FC);

// 处理 512 个 32 位字
v15 = ++v14 > 512 ? 0 : var80C[v14 - 1];
if (v15 < v10 || v15 > v11)            // 范围检查: 531442 - 387951929
    break;

// 存储索引信息
*(_DWORD *)(a3 + 4 * *a7 - 4) = v15;      // 哈希值
*(_DWORD *)(a4 + 4 * *a7 - 4) = v16 / 512; // 记录号  
*(_DWORD *)(a5 + 4 * *a7 - 4) = v16 % 512; // 页内偏移
```

---

## 5. ATGTDF 属性定义段结构

基于对 `Attlib_LoadAttrDefinitions` 函数 (`0x10852E20`) 的反编译分析：

### 5.1 属性定义记录结构

```c
typedef struct {
    uint32_t attr_hash;     // 属性哈希值
    uint32_t data_type;     // 数据类型代码
    uint32_t default_flag;  // 默认值标志
    // 后续根据 data_type 和 default_flag 跟着额外的数据
} AttlibAttrDefinitionHeader;
```

### 5.2 类型代码映射

#### 5.2.1 基本类型（attlib.dat 中定义）

**通过 IDA Pro 反编译 `sub_10852E20` (Attlib_LoadAttrDefinitions) 确认**：attlib.dat 中**只存储 4 种基本类型**，没有扩展类型（Element、Point 等）。

**关键汇编代码证据**（地址 0x10853073）：
```assembly
0x10853004: mov eax, [ebp+var_824]      // eax = data_type
0x1085300a: cmp eax, 1                  // 检查是否为 1 (LOG)
0x1085300d: jnz loc_10853028            // 不是 1，跳转
0x1085301d: mov dword ptr [eax], 0      // 存储 0（无默认值）
0x10853023: jmp loc_10852EEF            // 继续下一条记录

0x10853028: cmp eax, 2                  // 检查是否为 2 (REAL)
0x1085302e: jnz loc_10853311            // 不是 2，跳转
...

0x10853073: cmp eax, 4                  // 检查是否为 4 (TEXT)
0x10853076: jnz loc_10853167            // 不是 4，跳转到标量处理
```

**完整的类型处理逻辑**：

```c
// 从 IDA Pro 反编译代码 (0x10852f92-0x1085319c)
v19 = var80C[v17++];  // 读取 data_type
*(_DWORD *)(a4 + 4 * *a8 - 4) = v19;  // 存储 data_type

v20 = var80C[v17++];  // 读取 default_flag

if ( v20 == 1 )
{
    // 无默认值
    *(_DWORD *)(a5 + 4 * *a8 - 4) = 0;
}
else if ( v20 == 2 )
{
    // 有默认值
    *(_DWORD *)(a5 + 4 * *a8 - 4) = ++*a10;

    if ( v19 == 4 )  // TEXT 类型
    {
        // 读取长度
        v21 = var80C[v17++];
        *(_DWORD *)(a6 + 4 * *a10 - 4) = v21;

        // 检查缓冲区溢出
        if ( *a10 + v21 >= *a9 )
            goto ERROR;

        // 读取文本数据
        for (i = 1; i <= v21; i++)
        {
            ++v17;
            ++*a10;
            *(_DWORD *)(a6 + 4 * *a10 - 4) = var80C[v17 - 1];
        }
    }
    else  // 标量类型 (LOG, REAL, INT)
    {
        // 直接读取一个 32 位字
        *(_DWORD *)(a6 + 4 * *a10 - 4) = var80C[v17++];
    }
}
else
{
    // 无效的 default_flag
    MOLMES(&unk_10AB2430);  // 错误消息
    goto ERROR;
}
```

**类型代码映射**：

| 代码 | 类型 | 存储方式 | 说明 |
|------|------|---------|------|
| 1 | LOG | 标量 (u32) | 逻辑/布尔类型，存储为 0 或 1 |
| 2 | REAL | 标量 (u32) | 浮点类型，存储为 IEEE 754 双精度的位表示 |
| 3 | INT | 标量 (u32) | 整数类型，存储为有符号 32 位整数 |
| 4 | TEXT | 长度 + 数据 | 文本类型，先存长度，再存 27 进制编码的字符串 |

#### 5.2.2 完整类型系统（运行时扩展）

**重要发现**：通过 IDA Pro 对 `DB_Element::getAtt` 函数的重载分析（地址 0x10001860-0x10001b10），发现 AVEVA E3D 运行时支持的**完整属性类型系统**远超 attlib.dat 中定义的 4 种基本类型。

完整的数据类型枚举（基于 C++ 函数签名推导）：

```rust
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttlibDataType {
    // === attlib.dat 存储的基本类型 (1-4) ===
    Log = 1,        // 逻辑/布尔类型 (bool, C++: _N)
    Real = 2,       // 浮点类型 (double, C++: N)  
    Int = 3,        // 整数类型 (int, C++: H)
    Text = 4,       // 文本类型 (string, 27进制编码)
    
    // === 运行时扩展类型 (5-11) ===
    Ref = 5,        // 元素引用类型 (DB_Element)
    Name = 6,       // Noun名称类型 (DB_Noun*)
    Attribute = 7,  // 属性引用类型 (DB_Attribute*)
    Point = 8,      // 3D点类型 (D3_Point)
    Vector = 9,     // 3D向量类型 (D3_Vector)
    Matrix = 10,    // 3D矩阵类型 (D3_Matrix)
    Transform = 11, // 3D变换类型 (D3_Transform)
    DateTime = 12,  // 日期时间类型 (DB_DateTime)
}
```

#### 5.2.3 类型系统证据（IDA Pro 函数签名）

从 `core.dll` 导出的 `DB_Element::getAtt` 重载函数清单：

| 地址 | C++ 签名片段 | 数据类型 | Rust 映射 |
|------|-------------|---------|----------|
| 0x10001860 | `AAV?$basic_string@D...` | `std::string&` | `Text` |
| 0x10001880 | `AAH@Z` | `int&` | `Int` |
| 0x100018b0 | `AAV?$vector@H...` | `vector<int>&` | `Int` 数组 |
| 0x100018f0 | `AAN@Z` | `double&` | `Real` |
| 0x10001910 | `AAV?$vector@N...` | `vector<double>&` | `Real` 数组 |
| 0x10001950 | `AA_N@Z` | `bool&` | `Log` |
| 0x10001970 | `AAV?$vector@_N...` | `vector<bool>&` | `Log` 数组 |
| 0x10001990 | `AAV1@@Z` | `DB_Element&` | `Ref` |
| 0x100019c0 | `AAV?$vector@VDB_Element...` | `vector<DB_Element>&` | `Ref` 数组 |
| 0x100019f0 | `AAPBVDB_Noun@@@Z` | `DB_Noun**` | `Name` |
| 0x10001a10 | `AAV?$vector@PBVDB_Noun...` | `vector<DB_Noun*>&` | `Name` 数组 |
| 0x10001a30 | `AAPBV2@@Z` | `DB_Attribute**` | `Attribute` |
| 0x10001a50 | `AAV?$vector@PBVDB_Attribute...` | `vector<DB_Attribute*>&` | `Attribute` 数组 |
| 0x10001a70 | `AAVD3_Point@@@Z` | `D3_Point&` | `Point` |
| 0x10001a90 | `AAVD3_Vector@@@Z` | `D3_Vector&` | `Vector` |
| 0x10001ab0 | `AAVD3_Matrix@@@Z` | `D3_Matrix&` | `Matrix` |
| 0x10001ad0 | `AAVD3_Transform@@@Z` | `D3_Transform&` | `Transform` |
| 0x10001af0 | `AAVDB_DateTime@@@Z` | `DB_DateTime&` | `DateTime` |

#### 5.2.4 数组类型支持

E3D 运行时支持以下数组类型（通过 `vector<T>` 重载）：

```rust
pub enum AttlibArrayType {
    IntArray,           // vector<int>, 0x100018b0
    RealArray,          // vector<double>, 0x10001910
    LogArray,           // vector<bool>, 0x10001970
    RefArray,           // vector<DB_Element>, 0x100019c0
    NameArray,          // vector<DB_Noun*>, 0x10001a10
    AttributeArray,     // vector<DB_Attribute*>, 0x10001a50
}
```

#### 5.2.5 类型存储策略

- **attlib.dat 文件**：仅存储基本类型 (1-4: LOG, REAL, INT, TEXT)
- **运行时内存**：支持完整类型系统 (1-12)，包括几何和引用类型
- **类型转换**：基本类型 → 扩展类型的映射在 `core.dll` 运行时完成
- **默认值**：只有基本类型 (1-4) 可以在 attlib.dat 中存储默认值

### 5.3 默认值处理机制

基于反编译代码第59-101行的逻辑：

```c
if (default_flag == 1) {
    // 无默认值
    *(_DWORD *)(a5 + 4 * *a8 - 4) = 0;
} else if (default_flag == 2) {
    // 有默认值，需要读取额外数据
    *(_DWORD *)(a5 + 4 * *a8 - 4) = ++*a10;  // 指向默认值缓冲区的索引
    
    if (data_type == 4) {  // TEXT 类型
        uint32_t text_length = var80C[v17++];  // 文本长度
        *(_DWORD *)(a6 + 4 * *a10 - 4) = text_length;
        
        // 读取文本内容 (text_length 个 32 位字)
        for (i = 1; i <= text_length; i++) {
            ++v17;
            ++*a10;
            *(_DWORD *)(a6 + 4 * *a10 - 4) = var80C[v17 - 1];  // 27 进制编码的字符
        }
    } else {
        // 标量默认值 (LOG/INT/REAL)
        *(_DWORD *)(a6 + 4 * *a10 - 4) = var80C[v17++];
    }
}
```

### 5.4 ATGTDF 段读取流程

```c
// 关键代码片段 (0x10852E20)
MTRENT("ATGTDF", 6u, (int)"\n");        // 调试日志开始
v13 = 531442;                           // 最小哈希值
v14 = 387951929;                        // 最大哈希值

v16 = FHDBRN(&dword_11E1E860, &v15, var80C, &unk_10AB2424);

// 解每条属性定义记录
v18 = ++v17 > 512 ? 0 : var80C[v17 - 1];  // attr_hash
if (v18 < v13 || v18 > v14) break;       // 范围检查

*(_DWORD *)(a3 + 4 * *a8 - 4) = v18;     // 存储 attr_hash
v19 = var80C[v17++];                     // 读取 data_type
*(_DWORD *)(a4 + 4 * *a8 - 4) = v19;     // 存储 data_type
v20 = var80C[v17++];                     // 读取 default_flag

// 处理默认值逻辑 (见上节)
```

### 5.5 调试输出格式

```
ATTLIB Attribute/Noun Definitions Table
Counter    Hash code and word    Data type    Default pointer
1          656603                 LOG(        3           1
2          656604                 REAL(       2           2
3          656605                 INT(        3           3
4          656606                 TEXT(       4           4

ATTLIB Definition Defaults
[27进制编码的默认值数据]
```

### 5.6 完整属性定义结构

```rust
#[repr(C)]
pub struct AttlibAttrDefinition {
    pub header: AttlibAttrDefinitionHeader,
    pub default_value: AttlibDefaultValue,
}

pub enum AttlibDefaultValue {
    None,                               // default_flag = 1
    Scalar(u32),                         // LOG/INT/REAL 类型
    Text(Vec<u32>),                      // TEXT 类型，27 进制编码
}
```

### 5.7 ATGTDF 解析业务步骤

结合 ATGTIX 索引与 FHDBRN 分页机制，属性定义段的执行流程如下：

1. **读取段指针**：从文件头 0x0800 处取出 ATGTDF 起始页号 `start_page`。  
2. **准备索引映射**：使用 ATGTIX 的三元组快速获取 `attr_hash → (record_num, slot_offset)`，以便直接定位。  
3. **分页遍历**：从 `start_page` 起调用 `FHDBRN`，按 §3.2 所述处理 `0x00000000`（翻页）与 `0xFFFFFFFF`（段结束）标记。  
4. **解析定义头**：对每条记录读取连续三个词 `[attr_hash, data_type, default_flag]` 并校验范围；若 `attr_hash` 不在索引中，可视为 UDA 或跳过。  
5. **解析默认值**：  
   - `default_flag == 1` → 无默认值，直接继续下一条；  
   - `default_flag == 2` 且 `data_type ∈ {LOG, INT, REAL}` → 读取一个 32 位字并按类型解释；  
   - `default_flag == 2 && data_type == TEXT` → 先读取长度 `length`，再取后续 `length` 个词，通过 `Attlib_DecodePackCode` (Base27) 解码字符串。  
6. **组合结果**：将定义对象挂载到对应的 ATGTIX 索引项，形成 `hash → {page, offset, data_type, default}` 的完整结构；如需调试，可记录 `record_num` 与 `slot_offset` 以核对 IDA 输出。  
7. **终止条件**：遇到段结束标记或索引已全部覆盖即停止，以免越界解析后续语法/别名段。

---

## 6. 完整文件结构图

```
attlib.dat (4,616,192 字节)
├── 文件头 (0x0000-0x0800)
│   ├── 0x0000: "Attribute Data File" (UTF-16LE)
│   ├── 0x0008: "1.2.2.41" (版本号)
│   ├── 0x0010: "(WINDOWS-NT 6.3) (20 Sep 2022 : 06:14)"
│   └── 0x0800: 8个段指针 (大端序32位)
│
└── 数据记录区 (0x1000+)
    ├── ATGTIX 段 (属性索引表)
    │   └── [属性哈希, 记录号, 页内偏移] 数组
    │       ├── 哈希范围: 531442 - 387951929
    │       └── 连续存储的三元组
    │
    ├── ATGTDF 段 (属性定义表)
    │   └── [属性哈希, 数据类型, 默认值标志, 默认值数据] 数组
    │       ├── 类型代码: 1=LOG, 2=REAL, 3=INT, 4=TEXT
    │       ├── 默认值标志: 1=无默认值, 2=有默认值
    │       └── TEXT类型默认值: 长度+27进制编码数据
    │
    └── 其他数据段
        ├── 语法/别名表
        └── 验证规则等
```

---

## 7. 数据记录分页机制

### 7.1 页结构
```
每页 = 2048 字节 = 512 × 32 位字
├── 字 0-511: 实际数据
├── 标记 0: 页切换标记
└── 标记 -1: 段结束标记
```

### 7.2 定位算法
```c
// 通过哈希值查找属性位置
record_num = attr_index.record_num;
slot_offset = attr_index.slot_offset; 
byte_offset = record_num * 2048 + slot_offset * 4;
```

---

## 8. 与现有解析脚本的对比

### 8.1 Python 解析器的问题
当前 `parse_attlib*.py` 脚本假设连续存储，忽略了：
1. **分页机制**：没有处理 FHDBRN 的多页读取
2. **索引映射**：直接扫描而未使用 ATGTIX 索引表
3. **终结标记**：未正确处理页切换(0)和段结束(-1)标记
4. **属性定义结构**：未正确解析 ATGTDF 段的复杂结构

### 8.2 正确的解析方法
```python
class AttlibCorrectParser:
    def __init__(self):
        self.fhdbrn = FHDBRNReader()
        self.attr_index = self.load_atgtix()  # 先加载索引
        self.attr_definitions = self.load_atgtdf()  # 加载定义
        
    def load_attribute(self, attr_hash):
        # 通过索引定位
        index = self.attr_index[attr_hash]
        page_data = self.fhdbrn.read_page(index.record_num)
        return self.parse_attribute_record(page_data, index.slot_offset)
        
    def parse_attribute_defaults(self, data_type, default_flag, buffer):
        # 根据类型处理默认值
        if default_flag == 1:
            return None
        elif data_type == 4:  # TEXT 类型
            length = buffer.pop(0)
            text_data = buffer[:length]
            return self.decode_27_base(text_data)
        else:
            return buffer.pop(0)  # 标量值
```

---

## 9. IDA Pro 关键函数地址

| 函数名 | 地址 | 功能 |
|--------|------|------|
| `sub_1044FC20` | 0x1044FC20 | FHDBRN 读取实现 |
| `Attlib_LoadAttrIndex` | 0x10852A64 | 属性索引加载 |
| `Attlib_LoadAttrDefinitions` | 0x10852E20 | 属性定义加载 |
| `Attlib_LoadSyntaxTable` | 0x108533B4 | 语法表加载 |
| `Attlib_LogAttrDefinitions` | 0x10853988 | 属性定义调试输出 |
| `Attlib_DecodePackCode` | 0x10853FC8 | 27 进制解码 |

---

## 10. 实现建议

### 10.1 Rust 解析器结构
```rust
#[repr(C)]
pub struct AttlibAttrIndex {
    pub attr_hash: u32,
    pub combined: u32,
}

impl AttlibAttrIndex {
    pub fn record_num(&self) -> u32 {
        self.combined / 512
    }

    pub fn slot_offset(&self) -> u32 {
        self.combined % 512
    }
}

#[repr(C)]
pub struct AttlibAttrDefinitionHeader {
    pub attr_hash: u32,
    pub data_type: u32,
    pub default_flag: u32,
}

pub enum AttlibDefaultValue {
    None,
    Scalar(u32),
    Text(Vec<u32>), // 27进制编码
}

pub struct AttlibParser {
    file: std::fs::File,
    attr_index: HashMap<u32, AttlibAttrIndex>,
    attr_definitions: HashMap<u32, AttlibAttrDefinition>,
}

impl AttlibParser {
    fn load_atgtix(&mut self) -> Result<(), Error> {
        // 使用 FHDBRN 风格读取 ATGTIX 段
    }
    
    fn load_atgtdf(&mut self) -> Result<(), Error> {
        // 使用 FHDBRN 风格读取 ATGTDF 段
    }
    
    fn read_page(&self, page_num: u32) -> [u8; 2048] {
        // 定位到指定页并读取 2048 字节
    }
    
    fn get_attribute(&self, hash: u32) -> Option<AttlibAttrDefinition> {
        // 通过索引定位并解析属性
    }
    
    fn decode_27_base(&self, encoded: &[u32]) -> String {
        // 27进制解码器实现
    }
}
```

### 10.2 验证方法
```bash
# 对比原始文件和重新解析的结果
python3 scripts/verify_attlib_parsing.py data/attlib.dat

# 使用 IDA Pro 对比内存结构
cargo test test_attlib_parsing -- --nocapture
```

---

## 11. 后续工作

1. **完整实现 FHDBRN 读取器**：支持分页和标记检测
2. **重构现有解析脚本**：基于正确的文件结构重写
3. **性能优化**：利用索引机制实现属性快速查找
4. **版本兼容性**：支持不同版本的 attlib.dat 格式
5. **调试工具开发**：实现类似 IDA 的内存布局查看器

---

## 12. 完整解析流程示例

基于 IDA Pro 反编译结果，以下图首先概括 `data/attlib.dat` 的读取与解析步骤，随后给出关键伪代码，便于脚本或 Rust 实现时对照。

### 12.1 读取与解析流程图

```mermaid
flowchart TD
    A[打开 attlib.dat<br/>校验文件头与版本] --> B[读取 0x0800 处段指针]
    B --> C[初始化 FHDBRN 仿真器<br/>page_size = 2048]
    C --> D[定位 ATGTIX 起始页号]
    D --> E[循环读取 512×u32 页缓冲]
    E --> F{取出下一个 word}
    F -->|0x00000000| D
    F -->|0xFFFFFFFF| G[完成 ATGTIX]
    F -->|合法哈希| H[记录 attr_index[attr_hash] = {record_num, slot_offset}]
    G --> I[定位 ATGTDF 起始页号]
    I --> J[循环读取属性定义 word 序列]
    J --> K{word 判定}
    K -->|0x00000000| I
    K -->|0xFFFFFFFF| L[完成属性定义]
    K -->|默认标志解析| M[生成 default_value]
    M --> N[TEXT 类型 → 读取 length + 数据数组 → 27 进制解码]
    M --> O[LOG/INT/REAL → 读取单个 u32 默认值]
    M --> P[default_flag == 1 → 无默认值]
    N --> Q[构造 AttlibAttribute]
    O --> Q
    P --> Q
    Q --> R[合并索引 + 定义<br/>填充属性字典]
    R --> S[结合 Noun/别名段<br/>生成完整元数据]
```

**流程图说明**
- ATGTIX 与 ATGTDF 都使用 0x00000000 作为翻页标记、0xFFFFFFFF 作为段结束标记；处理逻辑需严格一致。
- “默认标志解析”对应 IDA 中 `Attlib_LoadAttrDefinitions` 对 `default_flag` 的分支；TEXT 类型必须先读取长度再进行 27 进制解码。
- 最终的元数据层建议同时保留 `record_num`、`slot_offset` 等调试信息，便于与 IDA 内存结构互相验证。

### 12.2 属性解析伪代码

```rust
fn parse_attribute_example(&self, attr_hash: u32) -> Result<AttlibAttribute> {
    // 1. 通过 ATGTIX 索引定位
    let index = self.attr_index[&attr_hash].clone();

    // 2. 分解 combined 字
    let record_num = index.record_num();
    let slot_offset = index.slot_offset();

    // 3. 读取对应页
    let mut page = [0u8; 2048];
    self.read_fhdbrn_page(record_num, &mut page)?;

    // 4. 解析页内数据
    let words = Self::decode_big_endian_words(&page);
    let cursor = slot_offset as usize;

    // 5. 读取属性定义头
    let def_hash = words[cursor];
    let data_type = words[cursor + 1];
    let default_flag = words[cursor + 2];
    
    // 6. 解析默认值
    let default_value = match default_flag {
        1 => AttlibDefaultValue::None,
        2 => {
            match data_type {
                4 => {  // TEXT 类型
                    let length = words[cursor + 3];
                    let encoded_text = &words[cursor + 4..cursor + 4 + length as usize];
                    AttlibDefaultValue::Text(encoded_text.to_vec())
                }
                _ => {  // LOG/INT/REAL
                    AttlibDefaultValue::Scalar(words[cursor + 3])
                }
            }
        }
        _ => return Err("Invalid default flag"),
    };

    // 7. 构建完整属性对象
    Ok(AttlibAttribute {
        hash: def_hash,
        data_type: self.map_data_type(data_type)?,
        default_value,
        // ... 其他字段
    })
}
```

---

## 13. 快速参考

### 13.1 关键常量
- **记录大小**: 2048 字节
- **每页字数**: 512 个 32 位字
- **哈希范围**: 531442 - 387951929
- **页切换标记**: 0x00000000
- **段结束标记**: 0xFFFFFFFF

### 13.2 数据类型映射

#### 基本类型（attlib.dat 存储）
| 代码 | 类型 | C++ 类型 | 说明 |
|------|------|----------|------|
| 1 | LOG | `bool` | 逻辑/布尔类型 |
| 2 | REAL | `double` | 双精度浮点类型 |
| 3 | INT | `int` | 32位整数类型 |
| 4 | TEXT | `std::string` | 文本类型 (27进制编码) |

#### 扩展类型（运行时支持）
| 代码 | 类型 | C++ 类型 | 说明 |
|------|------|----------|------|
| 5 | REF | `DB_Element` | 元素引用类型 |
| 6 | NAME | `DB_Noun*` | Noun名称类型 |
| 7 | ATTR | `DB_Attribute*` | 属性引用类型 |
| 8 | POINT | `D3_Point` | 3D点坐标 |
| 9 | VECTOR | `D3_Vector` | 3D向量 |
| 10 | MATRIX | `D3_Matrix` | 3D矩阵 |
| 11 | TRANSFORM | `D3_Transform` | 3D变换矩阵 |
| 12 | DATETIME | `DB_DateTime` | 日期时间类型 |

#### 数组类型支持
所有基本类型和部分扩展类型都支持数组形式（`vector<T>`）

### 13.3 默认值标志
| 标志 | 含义 | 处理方式 |
|------|------|----------|
| 1 | 无默认值 | 直接跳过 |
| 2 | 有默认值 | 根据类型解析后续数据 |

### 13.4 关键函数签名
```c
// FHDBRN 读取函数
int FHDBRN(void* file_handle, int* page_num, 
           char* buffer, int* error_code);

// 属性索引加载
int Attlib_LoadAttrIndex(int handle, int* page_num, 
                        int* hashes, int* pages, 
                        int* offsets, int* count);

// 属性定义加载
int Attlib_LoadAttrDefinitions(int handle, int* page_num,
                              int* hashes, int* data_types,
                              int* default_flags, int* default_data,
                              int* count, int* data_count);
```

---

## 14. 调试输出解析示例

当 E3D 系统启用详细调试时，会输出类似以下信息：

```
ATTLIB Attribute/Noun Definitions Table
Counter    Hash code and word    Data type    Default pointer
1          656603                 LOG(        1
2          656604                 REAL(       2
3          656605                 INT(        3  
4          656606                 TEXT(       4

ATTLIB Definition Defaults
27进制编码的文本默认值数据...
```

这种调试输出对于验证解析器正确性非常有用。

---

## 15. 解析流程总结

基于 IDA Pro 反编译的完整分析，attlib.dat 的正确解析流程是：

1. **文件头验证**：确认 "Attribute Data File" 标识和版本信息
2. **段指针加载**：从 0x0800 读取 8 个段的偏移地址
3. **ATGTIX 索引加载**：使用 FHDBRN 逐记录读取属性索引表
4. **ATGTDF 定义加载**：基于索引，读取具体的属性定义数据
5. **默认值解析**：根据 data_type 和 default_flag 处理变长数据
6. **27 进制解码**：将编码的文本转换为可读字符串
7. **内存结构映射**：填充到对应的 E3D 运行时数据结构

这种设计实现了：
- **高效内存管理**：按需加载属性数据
- **快速索引查找**：O(1) 哈希定位
- **灵活数据存储**：支持多种类型和变长字段
- **版本兼容性**：稳定的分页格式

---

## 16. 关键发现总结

通过 IDA Pro 反编译，我们发现了：

1. **真正的文件结构**：基于 FHDBRN 的分页式存储，不是连续二进制
2. **属性定义格式**：ATGTDF 段的复杂结构，包含变长默认值处理
3. **类型映射**：
   - attlib.dat 存储 4 种基本类型 (1=LOG, 2=REAL, 3=INT, 4=TEXT)
   - 运行时支持 12 种完整类型系统（包括 REF, NAME, POINT, VECTOR, MATRIX, TRANSFORM, DATETIME 等）
   - 通过分析 `DB_Element::getAtt` 的 23 个重载函数确认
4. **默认值机制**：TEXT 类型使用长度前缀的 27 进制编码
5. **调试输出格式**：可通过 `Attlib_LogAttrDefinitions` 验证解析结果
6. **数组支持**：运行时支持 INT, REAL, LOG, REF, NAME, ATTR 的数组类型

---

## 17. 实用脚本
- `scripts/attlib_parse_from_doc.py`：重新解析 `attlib.dat`，输出全量属性索引并校验 ELBO 覆盖情况。
- `scripts/attlib_query_noun.py`：结合解析结果与 `all_attr_info.json` 的 noun 映射，查询指定 Noun 的属性元数据（例如 `ELBO`）。

两者均复用了 `AttlibFullParser` 和文中总结的结构，可作为进一步自动化工具的起点。

---

> 文档维护：rs-core 团队  
> 基于 IDA Pro 反编译分析，确保与 core.dll 二进制行为一致  
> 如发现新的数据段或字段变化，请及时更新本文档

---

*最后更新：2025-06-20 (基于 IDA Pro 0x10852E20 函数反编译)*
