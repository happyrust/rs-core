# DB_Noun 数据结构分析

## 概述

`DB_Noun` 是 PDMS (Plant Design Management System) core.dll 中的核心数据结构，用于表示元素类型（Element Types）。所有的 PDMS 元素（如 PIPE、EQUI、ZONE、SITE 等）都通过 `DB_Noun` 来定义其类型信息。

## 结构定义

### 基本信息
- **大小**: 184 字节
- **类型**: C++ 类（struct）
- **成员数量**: 6 个已识别成员
- **虚表地址**: 0x10a6719c

### 成员字段

```cpp
struct DB_Noun {
    char[116]   pad_00_74;        // 偏移: 0x00, 大小: 116 字节
    int         type_id;          // 偏移: 0x74, 大小: 4 字节 - 类型哈希值
    char[40]    pad_78_9C;        // 偏移: 0x78, 大小: 40 字节
    int         ps_next;          // 偏移: 0xA0, 大小: 4 字节
    char[16]    pad_A0_AC;        // 偏移: 0xA4, 大小: 16 字节
    const DB_Noun* hard_type;     // 偏移: 0xB4, 大小: 4 字节 - 指向硬类型的指针
};
```

### 扩展结构（从反编译代码推断）

通过分析 `DB_Noun::ReadData` 函数，可以推断出更多字段：

```cpp
struct DB_Noun_Extended {
    // 基础部分 (0x00 - 0xB7, 184 字节)
    char        vtable_ptr[4];              // 虚表指针
    char        pad_04_74[112];             // 填充区域
    int         type_id;                     // 0x74: 类型哈希值/ID
    char        valid_flag;                  // 0x78: 有效标志
    char        pad_79_9C[35];
    int         ps_next;                     // 0xA0: 可能是链表指针
    char        pad_A4_B4[16];
    const DB_Noun* hard_type;                // 0xB4: 硬类型指针

    // 扩展部分（从 ReadData 函数推断）
    char        pad_B8_D0[24];
    bool        flag_D0;                     // 0xD0: 某种布尔标志
    char        pad_D1_D8[7];
    char        description[84];             // 0xD8+: 元素类型描述字符串
};
```

## 关键方法

### 1. `DB_Noun::ReadData()` - 读取类型数据
- **地址**: 0x10457d00
- **功能**: 从数据库读取元素类型的完整信息
- **主要操作**:
  1. 检查 `type_id` 是否有效
  2. 从 Fortran 接口读取多个字段值
  3. 设置描述信息
  4. 处理 UDET（User Defined Element Type）

### 2. `DB_Noun::findNoun()` - 查找类型
- **地址**: 0x104575c0
- **签名**: `bool DB_Noun::findNoun(const std::string& name, DB_Noun** result)`
- **功能**: 根据名称查找对应的 DB_Noun 对象
- **处理逻辑**:
  - 如果名称以 `:` 开头，首先尝试查找 UDET
  - 如果不是 UDET，调用 `ATNSYN` (Fortran 函数)进行同义词查找

### 3. 其他重要方法
- `DB_Noun::Eletypes()` - 获取元素类型层次
- `DB_Noun::EletypesHierarchy()` - 获取类型层次结构
- `DB_Noun::validProperties()` - 获取有效属性列表
- `DB_Noun::addUdas()` - 添加用户定义属性
- `DB_Noun::findOldKey()` - 查找旧版本键值

## 全局字典和常量

### 类型字典
```cpp
// 主字典：type_id -> DB_Noun*
std::map<int, const DB_Noun*> dictionary_;              // 0x10f5ed4c

// UDET 字典：name -> DB_Noun*
std::map<std::string, const DB_Noun*> dictionaryUdet_;  // 0x10f5ebd8

// UDET 键字典：key -> DB_Noun*
std::map<int, const DB_Noun*> dictionaryUdetKey_;       // 0x10f5ebe8

// 旧键字典（兼容性）
std::map<int, const DB_Noun*> oldKeyDictionary_;        // 0x10f5ebc8
```

### 预定义类型常量（部分列表）

| 变量名 | 地址 | 描述 |
|--------|------|------|
| NOUN_SPLO | 0x10f5ec6c | - |
| NOUN_UDET | 0x10f5ec70 | 用户定义类型 |
| NOUN_CELL | 0x10f5ecb0 | 单元 |
| NOUN_TASK | 0x10f5ecb4 | 任务 |
| NOUN_PIPE | ? | 管道 |
| NOUN_EQUI | ? | 设备 |
| NOUN_ZONE | ? | 区域 |

### 静态变量
```cpp
static int  udetBuildNumber_;           // 0x10c87144
static bool useOldKeyDictionary_;       // 0x10c87148
```

## Fortran 接口

DB_Noun 与 Fortran 代码有紧密集成，通过以下函数交互：
- `sub_1084F0DC()` - 从 Fortran 读取字段值
- `CHTOA()` - 字符转换
- `ATNSYN()` - 属性名称同义词查找

## 与 SurrealDB 的映射关系

在 rs-core 项目中，PDMS 的元素类型需要映射到 SurrealDB 的 `noun` 字段：

```rust
// PDMS -> SurrealDB 映射
DB_Noun.type_id         -> pe.noun (String)
DB_Noun.description     -> 类型名称（如 "PIPE", "EQUI", "ZONE"）
DB_Noun.hard_type       -> 类型继承关系
```

### 典型元素类型层次

```
WORL (世界)
  └─ SITE (站点)
      └─ ZONE (区域)
          └─ EQUI (设备)
              └─ PIPE (管道)
                  └─ BRAN (分支)
```

## RTTI 信息

```cpp
// RTTI 类型描述符
??_R0?AVDB_Noun@@@8                    // 0x10c8714c

// RTTI 完整对象定位器
??_R4DB_Noun@@6B@                      // 0x10abd060
??_R3DB_Noun@@8                        // 0x10abd074
??_R2DB_Noun@@8                        // 0x10abd084
??_R1A@?0A@EA@DB_Noun@@8               // 0x10abd08c
??_R1A@?0A@EN@DB_Noun@@8               // 0x10abd184
```

## 使用示例（从反编译代码推断）

```cpp
// 创建和初始化
DB_Noun* noun = new DB_Noun(type_name);
noun->ReadData();

// 按名称查找
DB_Noun* result;
if (DB_Noun::findNoun("PIPE", &result)) {
    // 找到了 PIPE 类型
    int typeId = result->type_id;
}

// 获取类型层次
std::vector<DB_Noun*> hierarchy;
noun->EletypesHierarchy(hierarchy);
```

## 注意事项

1. **内存布局**: 大部分字段仍然是 padding，需要进一步的逆向工程来识别具体用途
2. **哈希算法**: `type_id` 的计算方法需要进一步研究
3. **Fortran 集成**: 许多核心逻辑在 Fortran 代码中，需要分析 PDMS 的 Fortran 层
4. **线程安全**: 静态字典的访问可能需要同步机制

## 相关文件

- IDA 分析文件: core.dll
- 虚表地址: 0x10a6719c
- 主要函数地址:
  - ReadData: 0x10457d00
  - findNoun: 0x104575c0

## 后续研究方向

1. 完整识别所有 184 字节的字段用途
2. 分析类型继承和属性系统
3. 研究 UDET（用户定义类型）的创建机制
4. 理解与 DB_Attribute、DB_Element 的关系
5. 映射所有预定义类型常量的实际值
