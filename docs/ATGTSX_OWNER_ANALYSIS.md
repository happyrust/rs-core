# ATGTSX 表与 OWNER 属性层级关系分析

**分析日期**: 2025-11-10  
**目标**: 从 attlib.dat 的 ATGTSX 表中提取 NOUN 类型层级关系  
**核心思路**: 通过解析 OWNER 属性的语法规则，确定哪些 NOUN 类型可以作为哪些类型的父节点

---

## 1. 问题背景

当前项目中 `noun_graph.json` 包含了 NOUN 类型的层级关系图，但这个文件的来源不明确。根据 PDMS 系统的设计，NOUN 类型的父子关系应该是通过 **OWNER 属性**的语法规则定义的。

### OWNER 属性的含义

在 PDMS 中，每个元素（PE）都有一个 OWNER 属性，指向其父节点。例如：
- SITE 的 OWNER 是 WORL
- ZONE 的 OWNER 是 SITE
- EQUI 的 OWNER 是 ZONE
- PIPE 的 OWNER 是 EQUI

ATGTSX 表定义了对于每个 NOUN 类型，OWNER 属性允许接受哪些类型的值，从而确定了层级关系。

---

## 2. ATGTSX 表结构分析

### 2.1 基于 IDA Pro 的反编译分析

从 `ATTLIB_Load_Syntax_ATGTSX` (0x108533B4) 函数的反编译代码可以看出：

```c
ATTLIB_Load_Syntax_ATGTSX(
    &v27,                  // 起始页号
    &unk_11BDA050,         // a3: pack_code 数组 (属性hash)
    &unk_11BE2050,         // a4: index_ptr 数组
    &unk_11BEA050,         // a5: 第三个数组
    &unk_10AB2354,         // a6: 最大条目数
    &dword_11BFA05C        // a7: 实际加载的条目数
);
```

### 2.2 加载逻辑

```c
for ( i = *a2; ; ++i ) {
    v11 = FHDBRN(&dword_11E1E860, &i, var80C, &unk_10AB2454);
    
    for ( j = 0; ; ) {
        v13 = ++j > 512 ? 0 : var80C[j - 1];
        
        if ( v13 == -1 || !v13 )  // 段结束或页切换
            break;
            
        *(_DWORD *)(a3 + 4 * *a7 - 4) = v13;           // pack_code
        ++j;
        *(_DWORD *)(a4 + 4 * *a7 - 4) = var80C[j++ - 1]; // index_ptr
        *(_DWORD *)(a5 + 4 * *a7 - 4) = var80C[j - 1];   // 第三个值
    }
}
```

### 2.3 数据结构推断

每条 ATGTSX 记录包含 **3 个 32 位字**：
1. **pack_code**: 属性hash值（可能是压缩编码）
2. **index_ptr**: 索引指针（指向详细定义）
3. **第三个值**: 未知用途（可能是标志或参数计数）

---

## 3. OWNER 属性的识别

### 3.1 OWNER 属性的 hash 值

```python
def db1_hash(s):
    s = s.upper()
    h = 0
    for c in s:
        h = h * 27 + (ord(c) - ord('A') + 1)
    return h + 0x81BF1

owner_hash = db1_hash('OWNER')  # = 8966124 (0x88CFEC)
```

### 3.2 查找策略

在 ATGTSX 表中查找所有 `pack_code == 0x88CFEC` 的记录，这些记录定义了不同 NOUN 类型的 OWNER 属性规则。

---

## 4. 层级关系提取算法

### 4.1 理论基础

ATGTSX 表的 `pack_code` 字段可能包含两部分信息：
- **属性hash**: OWNER 的 hash (0x88CFEC)
- **NOUN类型hash**: 该规则适用于哪个 NOUN 类型

或者，`index_ptr` 指向另一个表，其中包含：
- 允许的父节点类型列表
- 参数验证规则

### 4.2 解析步骤

1. **加载 ATGTSX 表**
   - 从 attlib.dat 读取段指针表
   - 使用 FHDBRN 机制读取 ATGTSX 段
   - 解析每条记录的三个字段

2. **过滤 OWNER 属性记录**
   - 查找 pack_code 中包含 OWNER hash 的记录
   - 或通过 index_ptr 查找相关定义

3. **提取 NOUN 类型关系**
   - 对于每条 OWNER 规则，确定：
     - 子节点类型（该规则适用的 NOUN）
     - 父节点类型（OWNER 允许的值）

4. **构建层级图**
   - 使用 petgraph 构建有向图
   - 节点：NOUN 类型 hash
   - 边：父子关系（child -> parent）

---

## 5. 实现计划

### 5.1 Rust 解析器

创建 `src/bin/parse_atgtsx_hierarchy.rs`：

```rust
use aios_core::attlib_parser::AttlibParser;
use petgraph::graph::DiGraph;
use std::collections::HashMap;

fn main() -> anyhow::Result<()> {
    // 1. 加载 attlib.dat
    let mut parser = AttlibParser::new("data/attlib.dat")?;
    
    // 2. 加载 ATGTSX 表
    let atgtsx_records = parser.load_atgtsx()?;
    
    // 3. 过滤 OWNER 属性记录
    let owner_hash = 0x88CFEC;
    let owner_rules: Vec<_> = atgtsx_records
        .iter()
        .filter(|r| is_owner_rule(r, owner_hash))
        .collect();
    
    // 4. 提取层级关系
    let mut graph = DiGraph::<u32, u32>::new();
    let mut noun_to_node = HashMap::new();
    
    for rule in owner_rules {
        let child_noun = extract_child_noun(rule);
        let parent_nouns = extract_parent_nouns(rule);
        
        for parent_noun in parent_nouns {
            add_edge(&mut graph, &mut noun_to_node, child_noun, parent_noun);
        }
    }
    
    // 5. 导出为 JSON
    let json = serde_json::to_string_pretty(&graph)?;
    std::fs::write("noun_graph_generated.json", json)?;
    
    // 6. 生成 hierarchy_report.json
    let report = generate_hierarchy_report(&graph)?;
    std::fs::write("hierarchy_report_generated.json", report)?;
    
    Ok(())
}
```

### 5.2 需要实现的函数

1. `AttlibParser::load_atgtsx()` - 加载 ATGTSX 表
2. `is_owner_rule()` - 判断是否为 OWNER 属性规则
3. `extract_child_noun()` - 提取子节点类型
4. `extract_parent_nouns()` - 提取允许的父节点类型列表
5. `generate_hierarchy_report()` - 生成层级报告

---

## 6. 下一步行动

### 6.1 立即执行

1. **扩展 AttlibParser**
   - 添加 `load_atgtsx()` 方法
   - 实现 ATGTSX 段的分页读取

2. **分析 pack_code 编码**
   - 通过 IDA Pro 查找 pack_code 的解码函数
   - 理解如何从 pack_code 提取 NOUN 类型信息

3. **分析 index_ptr 指向的数据**
   - 确定 index_ptr 的含义
   - 查找它指向的数据结构

### 6.2 验证方法

1. 对比生成的 `noun_graph_generated.json` 与现有的 `noun_graph.json`
2. 检查已知的层级关系（如 SITE -> WORL, ZONE -> SITE）
3. 使用 `is_owner_type()` 函数验证结果

---

## 7. 关键问题

### 7.1 待解决

1. **pack_code 的编码格式**
   - 是否包含 NOUN 类型信息？
   - 如何解码？

2. **index_ptr 的含义**
   - 指向哪个表？
   - 如何解析指向的数据？

3. **参数类型的表示**
   - OWNER 属性的值类型如何编码？
   - 如何提取允许的 NOUN 类型列表？

### 7.2 需要 IDA Pro 分析

1. 查找 `sub_10853FC8` (Attlib_DecodePackCode) 函数
2. 分析 pack_code 的解码逻辑
3. 查找使用 ATGTSX 数据进行验证的代码

---

## 8. 重要发现（2025-11-10）

### 8.1 ATGTSX 表结构

通过 Rust 解析器分析，确认了 ATGTSX 表的实际结构：

```rust
struct AtgtSxRecord {
    pack_code: u32,      // 27进制编码的属性名称（如 'OWNER'）
    index_ptr: u32,      // 索引指针（用途待确定）
    third_value: u32,    // 第三个值（可能是参数计数）
}
```

**关键发现**：
- `pack_code` 存储的是**属性名称的 27 进制编码**，不是 hash 值
- OWNER 属性的 pack_code = 0x0080B3FB，解码后为 "OWNER"
- ATGTSX 表共有 6973 条记录
- **ATGTSX 表只存储属性语法规则，不包含 NOUN 类型信息**

### 8.2 层级关系的真实来源

通过 IDA Pro 分析发现：

1. **层级关系不在 attlib.dat 中**
   - attlib.dat 只存储属性定义和语法规则
   - 不包含 NOUN 类型之间的父子关系

2. **层级关系在运行时生成**
   - 函数：`DB_Noun::eleTypesHierarchy(DB_Attribute*, vector<DB_Noun*>&)`
   - 地址：0x10a67214
   - 这个函数基于 ATGTSX 规则动态计算允许的父类型

3. **现有的 `noun_graph.json` 来源**
   - 可能是通过运行 PDMS/E3D 程序，调用 `eleTypesHierarchy()` 生成的
   - 或者是从 PDMS 文档/配置文件中提取的

### 8.3 新的解决方案

由于层级关系不在 attlib.dat 中，有以下几种方案：

#### 方案 A：使用现有的 `noun_graph.json`
- **优点**：数据已经存在，可以直接使用
- **缺点**：来源不明确，可能不完整

#### 方案 B：通过 IDA Pro 分析 `eleTypesHierarchy()` 函数
- **优点**：可以理解层级关系的生成逻辑
- **缺点**：需要深入分析复杂的 C++ 代码

#### 方案 C：从 PDMS 数据库中提取
- **优点**：数据最准确
- **缺点**：需要访问实际的 PDMS 数据库

#### 方案 D：从 PDMS 文档中提取
- **优点**：官方文档可能包含完整的层级定义
- **缺点**：需要找到相关文档

### 8.4 推荐方案

**立即执行**：
1. 验证现有的 `noun_graph.json` 是否完整
2. 基于 `noun_graph.json` 生成 `hierarchy_report.json`
3. 创建初始化脚本填充 SurrealDB 的三张表

**长期改进**：
1. 通过 IDA Pro 分析 `eleTypesHierarchy()` 函数的实现
2. 理解层级关系的生成规则
3. 实现一个独立的层级关系生成器

## 9. 参考资料

- `docs/attlib_parsing_logic.md` - ATTLIB 文件格式
- `docs/attlib_table_analysis.md` - ATGTSX 表结构
- `src/attlib_parser.rs` - 现有解析器实现
- `src/noun_graph.rs` - 层级验证逻辑
- `examples/parse_atgtsx.rs` - ATGTSX 表解析器

