# Noun → Attributes 查询机制

基于 IDA Pro 对 `core.dll` 的逆向结果，总结 PDMS noun 到 attribute 的真实映射流程，并给出在工具侧获取“属性描述信息”的实现思路。

## 核心结论概览

- **属性定义来源**：`attlib.dat` 提供属性元数据，分布在 **ATGTIX**（索引）、**ATGTDF**（数据段）、**ATGTSX**（语法）三段；`DB_Attribute::ReadData()` 按需从这三段读取类型、默认值、单位、语法约束等信息。
- **属性列表来源**：每个 noun 的属性列表存放在数据库 **DAB** 的 `PRDISP` 字段（hash **240391897 = 0x0E5416D9**）。这是数据库字段，不在 `attlib.dat` 的字段定义中。
- **运行时链路**：`DB_Noun::validProperties()` → `internalGetField(PRDISP)` → 通过 `ATNAIN`/`DB_Noun::internalGetField` 读取 DAB 中的属性 ID 列表 → `DB_Attribute_findAttribute_by_id()` 将 ID 转成 `DB_Attribute*` → `DB_Attribute::ReadData()` 再去 `attlib.dat` 补全描述。
- **全局属性对象**：常用属性（如 `ATT_NAME`、`ATT_POS`）在 `core.dll` 里静态创建并注册到 `DB_Attribute::dictionary_`，与动态加载的属性共用同一查找表。
- **Hash 编码**：字符串 hash 使用 **base27 + 常数偏移 0x81BF1 (531441)** 的算法（亦即 ENHAS2），同时用于 noun 名称、字段名、属性名。

### PRDISP 最终读取点（core.dll 内部）

- 入口：`DB_Noun::internalGetField(vec)` @ `0x10457BE0`
- 路径：`internalGetField(PRDISP)` → `sub_1084F7C0("ATNAIN")` → `ATTLIB_Read_Page_Header_AndCache` → `FHDBRN`
- 数据源：`FHDBRN` 的文件句柄指向 **数据库 DAB 文件**（不是 attlib.dat），在 `ATTLIB_OpenAndInit` 之后初始化。
- 页面定位：`ATNAIN` 先用 `ATGTIX-2` 查 noun hash → `(page, offset)`，再用 `ATGTDF-2` 查字段 hash（PRDISP）得到字段偏移，再计算页内位置 `offset + field_idx`。
- 读取格式：`page_cache[page_slot][offset+field_idx]` 首个 DWORD 为属性数量 N，随后 N 个 DWORD 即属性 hash 列表。
- 返回：`internalGetField` 将这组 hash 列表填入调用者提供的 `vector<int>`，`validProperties()` 继续用 `DB_Attribute_findAttribute_by_id()` 转换。

#### ATNAIN 函数详解

**定义**：ATNAIN 是 **"Attribute Table Noun Attribute Index"** 的缩写，是 PDMS 数据库访问中的关键操作标识符。

**技术实现**：

```cpp
// 在 sub_1084F7C0 函数中的实现
void __cdecl sub_1085001C(int a1, int a2, _DWORD *a3, int a4)
{
  _BYTE v4[6]; // [esp+Ch] [ebp-8Ch] BYREF
  // ...
  memcpy(v4, "ATNAIN", sizeof(v4));  // 操作标识符
  v9 = 3;
  *a3 = 0;
  JUMPOUT(0x1085013D);
}
```

**核心功能**：

- **操作标识**：告诉数据库系统这是 noun-attribute 索引查询
- **索引触发**：激活 ATGTIX-2 和 ATGTDF-2 的双重索引机制
- **页面定位**：指导系统在 DAB 文件中定位正确的页面和偏移

**在查询链路中的作用**：

```text
DB_Noun::internalGetField(PRDISP) 
→ sub_1084F7C0("ATNAIN")           // 使用 ATNAIN 标识符
→ ATTLIB_Read_Page_Header_AndCache // 页面缓存
→ FHDBRN                            // 底层文件读取
```

**与其他标识符的关系**：

| 标识符 | 含义 | 作用 |
|--------|------|------|
| **ATNAIN** | Noun Attribute Index | noun 属性索引查询 |
| ATGTIX | Attribute Global Index | 属性全局索引 |
| ATGTDF | Attribute Global Definition | 属性全局定义 |
| ATGTSX | Attribute Global Syntax | 属性全局语法 |

**实际意义**：ATNAIN 是 PDMS noun-attribute 查询的"钥匙"，它不是数据本身，而是访问数据的操作指令，启动了从 DAB 文件中读取 PRDISP 字段的完整流程。

### 完整文件访问路径（初始化到落盘位置）

1. **数据库启动**  
   `DB_Open` / `ATTLIB_OpenAndInit` 会同时打开 attlib.dat 以及当前库的 DAB 文件，分别得到两个文件句柄。`FHDBRN` 接受句柄指针，调用时由上层传入“当前数据库句柄”，因而在 `ATNAIN` 流程中实际读取的是 DAB（类型元数据表）。

2. **字段索引定位**  
   - `ATGTIX-2`：noun hash → `(page, offset)`（定位到该 noun 的记录页和起始槽位）  
   - `ATGTDF-2`：字段 hash（PRDISP=0x0E5416D9）→ `field_idx`（该字段在记录中的槽序）  

3. **页面缓存 + FHDBRN 读取**  
   `ATTLIB_Read_Page_Header_AndCache` 先查 LRU 缓存命中，否则调用 `FHDBRN(&dab_handle, &page_num, buffer, &err)` 读入 2048B 页面，缓存到 `page_cache`。

4. **页内解析**  
   页内偏移 = `offset + field_idx`；`[offset+field_idx]` 的值是属性数量 N，随后 N 个 32-bit 值即属性 hash 列表（正是 PRDISP 字段内容）。

5. **属性对象补全**  
   `validProperties()` 将 hash 数组传给 `DB_Attribute_findAttribute_by_id()`，若未缓存会新建 `DB_Attribute` 并在首次访问时调用 `DB_Attribute::ReadData()`，这一步才去 **attlib.dat** 读取属性定义、默认值、语法等元数据。

## Hash 算法（base27 + 0x81BF1）

```python
def pdms_hash(name: str) -> int:
    h = 0x81BF1  # 531441
    mul = 1
    for ch in name[:6].upper():
        v = 0 if ch == ' ' else (ord(ch) - 64)  # A=1 … Z=26
        h += mul * v
        mul *= 27
    return h & 0xFFFFFFFF

# 校验
pdms_hash("PRDISP") == 240391897     # 0x0E5416D9
pdms_hash("NAME")   == 639374        # 0x0009C18E
pdms_hash("ELBO")   == 0x000CA439
```

## 运行时调用链（精简图）

```text
DB_Noun::validProperties()                         // 0x10459240
    └─ internalGetField(PRDISP=240391897, ids)     // 0x10457BE0
         └─ ATNAIN / sub_1084F7C0("ATNAIN")        // 0x1084F7C0
              ├─ ATGTIX 查字段 hash → (page, off)  // 0x10852A64
              ├─ ATGTIX 查 noun hash → (page, off)
              ├─ ATTLIB_Read_Page_Header_AndCache  // 0x1044FC20
              │    └─ FHDBRN 从 DAB 读页           // 0x10766040
              └─ 从缓存页 offset 处取出 attr_id[]
    └─ 对每个 id 调用 DB_Attribute_findAttribute_by_id() // 0x1045E5F0
         ├─ 命中 dictionary_ 直接返回
         └─ 未命中则 new DB_Attribute(id) 并调用 ReadData()
               └─ ReadData() 访问 attlib.dat 的 ATGTIX/ATGTDF/ATGTSX
```

### ATGTIX-2 实测解析（MCP/IDA）

- 数据区起点 0x1000，页大小 2048B（512×u32），页切换标记 0，段结束标记 0xFFFFFFFF，hash 范围 0x81BF2..0x171FAD39，与 IDA 中 ATTLIB_Load_Index_ATGTIX 常量一致。
- 0x800 处的 8 个指针不是 ATGTIX-2 起点；在当前 `data/attlib.dat` 中，实际起始页为 **1920**（数据区相对页号），沿页切换/结束标记解析得到 1384 条 noun → (page, slot) 记录。
- 典型校验：`PIPE` (0x000CA439) → page 1751 offset 328；`SITE` (0x0009D65A) → page 1820 offset 350；所有记录已导出至 `docs/属性解析/python/atgtix2.csv`。
- 解析脚本：`docs/属性解析/python/parse_atgtix2.py`（纯标准库，自动扫描起始页并导出 CSV），运行示例：  
  `python docs/属性解析/python/parse_atgtix2.py data/attlib.dat docs/属性解析/python/atgtix2.csv`

### Python 端提取 noun → attributes（需要 DAB）

**重要说明**：当前 `rs-core` 项目环境中**不包含标准的 PDMS DAB 运行时数据库文件**：

- `data/desvir.dat` 经分析确认不是 DAB 格式（无 ATGTIX-2 段），可能是项目特定数据文件
- `data/attlib.dat` 仅包含属性定义元数据（ATGTIX/ATGTDF/ATGTSX），不存储 noun 实例的 PRDISP 数据
- PRDISP 真正的属性列表需从运行时 DAB 数据库中读取

**脚本功能**（需标准 DAB 文件）：

- 脚本：`docs/属性解析/python/noun_to_attrs.py`。输入 attlib.dat + DAB 文件，按 ATGTIX-2 → ATGTDF-2 → PRDISP 链路导出 noun 的属性哈希列表。
- 默认假设：页大小 2048、大端、数据区起点 0x1000（可用 `--data-offset` 覆盖），字段哈希默认 PRDISP=0x0E5416D9。
- 运行示例（需真实 DAB 文件）：  
  `python docs/属性解析/python/noun_to_attrs.py --attlib data/attlib.dat --dab /path/to/real_db.dab --out noun_attrs.csv`
- 输出列：`noun_hash`、`noun_name`、`attr_count`、`attr_hashes`（分号分隔 16 进制）、`attr_names`（base27 解码）。若 DAB 中缺失 PRDISP 或格式不同则该 noun 被跳过。

### 若只持有 attlib.dat（无 DAB）的近似方案

- PRDISP 真正的属性列表存储在 DAB，attlib.dat 只能给出“可应用”语法映射（ATGTSX），无法还原实际 PRDISP 顺序与过滤结果。
- 提供基于 ATGTSX 的近似导出脚本：`docs/属性解析/python/atgtsx_noun_attrs.py`。它读取 attlib.dat 的 ATGTSX 段（指针索引 3），输出 noun 可用的属性集合（hash/名称，含 extra_info 忽略）。
- 运行示例：  
  `python docs/属性解析/python/atgtsx_noun_attrs.py data/attlib.dat docs/属性解析/python/atgtsx_noun_attrs.csv`
- 输出列：`noun_hash`、`noun_name`、`attr_hashes`、`attr_names`、`count`。该结果是语法可用集合，非运行时 PRDISP 实际列表。

#### ATGTIX-2 解析流程线框图

**Mermaid 可视化流程图**（适用于支持Mermaid渲染的编辑器）：

```mermaid
flowchart TD
    A[开始解析 attlib.dat] --> B[检查文件参数]
    B --> C[打开二进制文件]
    C --> D[load_pages: 加载所有数据页面]
    
    D --> D1[计算文件大小]
    D1 --> D2[计算数据区域大小]
    D2 --> D3[计算总页面数]
    D3 --> D4[逐页读取2048字节]
    D4 --> D5[解包为512个32位整数]
    D5 --> D6[存储到pages列表]
    
    D6 --> E[find_best_start_page: 寻找最佳起始页]
    
    E --> E1[遍历所有页面]
    E1 --> E2{页面包含有效哈希?}
    E2 -->|否| E1
    E2 -->|是| E3[parse_index_from_page]
    
    E3 --> E4{遇到SEGMENT_END?}
    E4 -->|否| E1
    E4 -->|是| E5{包含PIPE/SITE哈希?}
    E5 -->|否| E1
    E5 -->|是| E6{记录数更多?}
    E6 -->|是| E7[更新最佳页面]
    E6 -->|否| E1
    E7 --> E1
    
    E1 --> E8[遍历完成]
    E8 --> E9{找到有效起始页?}
    E9 -->|否| F[错误: 未找到ATGTIX-2段]
    E9 -->|是| G[parse_index_from_page: 正式解析]
    
    G --> G1[从起始页开始]
    G1 --> G2{页面越界?}
    G2 -->|是| G3[返回记录列表]
    G2 -->|否| G4{索引越界?}
    G4 -->|是| G5[切换到下一页]
    G5 --> G2
    G4 -->|否| G6[读取当前word]
    
    G6 --> G7{word == PAGE_SWITCH?}
    G7 -->|是| G5
    G7 -->|否| G8{word == SEGMENT_END?}
    G8 -->|是| G9[标记结束，跳出循环]
    G9 --> G3
    G8 -->|否| G10{word在哈希范围内?}
    G10 -->|否| G11[跳过，继续下一个]
    G11 --> G4
    G10 -->|是| G12[读取combined值]
    
    G12 --> G13[计算页面号: combined // 512]
    G13 --> G14[计算偏移: combined % 512]
    G14 --> G15[添加记录: (hash, page, offset, combined)]
    G15 --> G4
    
    G3 --> H[输出CSV文件]
    H --> H1[写入表头]
    H1 --> H2[遍历所有记录]
    H2 --> H3[decode_base27: 解码noun名称]
    
    H3 --> H4{hash在有效范围?}
    H4 -->|否| H5[返回空字符串]
    H4 -->|是| H6[计算k = hash - BASE27_OFFSET]
    H6 --> H7[Base27解码循环]
    H7 --> H8{k > 0?}
    H8 -->|否| H9[返回解码结果]
    H8 -->|是| H10[c = k % 27]
    H10 --> H11{c == 0?}
    H11 -->|是| H12[添加空格]
    H11 -->|否| H13[添加chr(c + 64)]
    H12 --> H14[k //= 27]
    H13 --> H14
    H14 --> H8
    
    H9 --> H15[写入CSV行]
    H15 --> H16{还有记录?}
    H16 -->|是| H2
    H16 -->|否| I[完成输出]
    
    F --> J[返回错误码1]
    I --> K[打印统计信息]
    K --> L[返回成功码0]
    
    style A fill:#e1f5fe
    style L fill:#c8e6c9
    style F fill:#ffcdd2
    style J fill:#ffcdd2
```

**说明**：Mermaid流程图提供现代化的可视化效果，在支持Mermaid的编辑器（如GitHub、VS Code等）中会自动渲染为图形。下面的线框图适用于纯文本环境查看。

**解析流程线段线框图**：

```text
═══════════════════════════════════════════════════════════════

1. 初始化阶段
┌─────────────────────────────────────────────────────────────┐
│ 开始 → 检查参数 → 打开attlib.dat文件                           │
└─────────────────────────────────────────────────────────────┘
                                │
                                ▼

2. 页面加载阶段 (load_pages)
┌─────────────────────────────────────────────────────────────┐
│ 计算文件大小 → 计算数据区域大小 → 计算总页数                    │
│     │              │                │                        │
│     ▼              ▼                ▼                        │
│ 逐页读取2048字节 → 解包为512个32位整数 → 存储到pages列表        │
└─────────────────────────────────────────────────────────────┘
                                │
                                ▼

3. 起始页检测阶段 (find_best_start_page)
┌─────────────────────────────────────────────────────────────┐
│ 遍历所有页面                                                     │
│     │                                                          │
│     ├─→ 页面包含有效哈希? ──NO─→ 跳过                           │
│     │     │                                                  │
│     │    YES                                                  │
│     │     │                                                  │
│     ▼     ▼                                                  │
│ parse_index_from_page ──→ 遇到SEGMENT_END? ──NO─→ 继续下一页     │
│     │                    │                                  │
│     │                   YES                                  │
│     │                    │                                  │
│     ▼                    ▼                                  │
│ 包含PIPE/SITE哈希? ──NO─→ 继续下一页                           │
│     │                                                          │
│    YES                                                          │
│     │                                                          │
│     ▼                                                          │
│ 记录数更多? ──YES─→ 更新最佳页面 ──→ 继续遍历                    │
│     │                                                          │
│    NO                                                          │
│     │                                                          │
│     ▼                                                          │
│ 继续下一页                                                     │
└─────────────────────────────────────────────────────────────┘
                                │
                                ▼

4. 正式解析阶段 (parse_index_from_page)
┌─────────────────────────────────────────────────────────────┐
│ 从起始页开始                                                     │
│     │                                                          │
│     ├─→ 页面越界? ──YES─→ 返回记录列表                          │
│     │     │                                                  │
│     │    NO                                                   │
│     │     │                                                  │
│     ▼     ▼                                                  │
│ 索引越界? ──YES─→ 切换到下一页 ──→ 页面越界检查                   │
│     │                                                          │
│    NO                                                           │
│     │                                                          │
│     ▼                                                          │
│ 读取当前word                                                   │
│     │                                                          │
│     ├─→ word == PAGE_SWITCH? ──YES─→ 切换页面                  │
│     │     │                                                  │
│     │    NO                                                   │
│     │     │                                                  │
│     ▼     ▼                                                  │
│ word == SEGMENT_END? ──YES─→ 标记结束 → 返回记录列表             │
│     │                                                          │
│    NO                                                           │
│     │     │                                                  │
│     ▼     ▼                                                  │
│ word在哈希范围内? ──NO─→ 跳过 → 继续下一个word                  │
│     │                                                          │
│    YES                                                          │
│     │                                                          │
│     ▼                                                          │
│ 读取combined值                                                 │
│     │                                                          │
│     ▼                                                          │
│ 计算页面号 (combined // 512)                                   │
│     │                                                          │
│     ▼                                                          │
│ 计算偏移 (combined % 512)                                      │
│     │                                                          │
│     ▼                                                          │
│ 添加记录 (hash, page, offset, combined)                       │
│     │                                                          │
│     ▼                                                          │
│ 继续下一个word                                                 │
└─────────────────────────────────────────────────────────────┘
                                │
                                ▼

5. Base27解码阶段 (decode_base27)
┌─────────────────────────────────────────────────────────────┐
│ hash在有效范围? ──NO─→ 返回空字符串                            │
│     │                                                          │
│    YES                                                          │
│     │                                                          │
│     ▼                                                          │
│ k = hash - BASE27_OFFSET                                      │
│     │                                                          │
│     ▼                                                          │
│ Base27解码循环:                                                │
│     │                                                          │
│     ├─→ k > 0? ──NO─→ 返回解码结果                             │
│     │     │                                                  │
│     │    YES                                                  │
│     │     │                                                  │
│     ▼     ▼                                                  │
│ c = k % 27                                                     │
│     │                                                          │
│     ├─→ c == 0? ──YES─→ 添加空格                              │
│     │     │                                                  │
│     │    NO                                                   │
│     │     │                                                  │
│     ▼     ▼                                                  │
│ 添加chr(c + 64)                                               │
│     │                                                          │
│     ▼                                                          │
│ k //= 27 ──→ 继续循环                                          │
└─────────────────────────────────────────────────────────────┘
                                │
                                ▼

6. 输出阶段
┌─────────────────────────────────────────────────────────────┐
│ 写入CSV表头 → 遍历所有记录 → Base27解码 → 写入CSV行             │
│     │              │              │           │               │
│     └──────────────┴──────────────┴───────────┘               │
│                                │                              │
│                                ▼                              │
│ 完成输出 → 打印统计信息 → 返回成功码0                           │
└─────────────────────────────────────────────────────────────┘
```

**核心数据结构线框图**：

```
核心数据结构线框图
═══════════════════════════════════════════════════════════════

页面结构 (Page Structure)
┌─────────────────────────────────────────────────────────────┐
│ 页面大小: 2048字节                                            │
│ ┌─────────────────────────────────────────────────────────┐ │
│ │ 每512个32位整数 = 2048字节                              │ │
│ │ [0][1][2]...[511]                                      │ │
│ └─────────────────────────────────────────────────────────┘ │
│ 数据区域: 从0x1000偏移开始                                   │
└─────────────────────────────────────────────────────────────┘

记录格式 (Record Format)
┌─────────────────────────────────────────────────────────────┐
│ 每条记录 = (hash, page, offset, combined)                    │
│ ┌─────────────┬─────────────┬─────────────┬─────────────┐   │
│ │ noun_hash   │ page        │ offset      │ combined    │   │
│ │ (8字节)     │ (4字节)     │ (4字节)     │ (4字节)     │   │
│ └─────────────┴─────────────┴─────────────┴─────────────┘   │
│ combined = page * 512 + offset                              │
└─────────────────────────────────────────────────────────────┘

特殊标记 (Special Markers)
┌─────────────────────────────────────────────────────────────┐
│ PAGE_SWITCH = 0x00000000    ←── 页面切换信号                  │
│ SEGMENT_END = 0xFFFFFFFF    ←── 段结束信号                   │
│ MIN_HASH = 0x81BF2          ←── 有效哈希下限                  │
│ MAX_HASH = 0x171FAD39       ←── 有效哈希上限                  │
└─────────────────────────────────────────────────────────────┘

Base27解码算法线框
┌─────────────────────────────────────────────────────────────┐
│ 输入: hash值                                                  │
│     │                                                          │
│     ▼                                                          │
│ k = hash - 0x81BF1                                            │
│     │                                                          │
│     ▼                                                          │
│ while k > 0:                                                  │
│     │                                                          │
│     ├─→ c = k % 27                                            │
│     │     │                                                  │
│     │     ├─→ c == 0: 添加空格                                 │
│     │     └─→ c > 0:  添加chr(c + 64)                        │
│     │                                                          │
│     └─→ k //= 27                                              │
│     │                                                          │
│     ▼                                                          │
│ 反转字符串 → 输出noun名称                                      │
└─────────────────────────────────────────────────────────────┘
```

**关键算法特点**：

1. **智能起始页检测** - 通过预筛选包含有效哈希的页面提高效率，使用已知noun (PIPE/SITE) 作为锚点验证，选择记录数最多的页面作为起始点
2. **流式解析机制** - 按页面顺序遍历，支持跨页连续读取，自动处理页面切换和段结束标记，跳过无效哈希值，只处理有效记录
3. **Base27解码算法** - 将数值哈希转换为可读的noun名称，使用27进制: 26个大写字母 + 空格，逆向计算: 从低位到高位逐步解码

**性能优化点**：

- 批量页面加载: 一次性读取所有页面到内存
- 快速预筛选: 只处理包含有效哈希的页面
- 锚点验证: 使用已知noun提高检测准确性
- 流式输出: 边解析边写入CSV，减少内存占用

**解析脚本文件**：

- 流程图: `docs/属性解析/python/atgtix2_flowchart.py` (Mermaid格式)
- 线框图: `docs/属性解析/python/atgtix2_wireframe.py` (线段格式)
- 解析器: `docs/属性解析/python/parse_atgtix2.py`

### 关键函数与全局

| 名称 | 地址 | 作用 |
|------|------|------|
| `DB_Noun::validProperties` | 0x10459240 | 入口，返回属性对象列表 |
| `DB_Noun::internalGetField` (vector) | 0x10457BE0 | 读取字段（含 PRDISP 数组） |
| `sub_1084F7C0` (`ATNAIN`) | 0x1084F7C0 | noun/field 哈希到页面偏移的 Fortran 桥 |
| `ATTLIB_Load_Index_ATGTIX` | 0x10852A64 | 加载 ATGTIX 索引 |
| `ATTLIB_Read_Page_Header_AndCache` | 0x1044FC20 | 页面缓存 + LRU |
| `FHDBRN` | 0x10766040 | 低层文件读 |
| `DB_Attribute_findAttribute_by_id` | 0x1045E5F0 | 根据 hash 返回/创建属性对象 |
| `DB_Attribute::ReadData` | 0x1085001C 等 | 从 attlib.dat 补全属性元数据 |
| `DB_Noun::dictionary_` | 0x10F5ED4C | noun hash → DB_Noun* |
| `DB_Attribute::dictionary_` | 0x10F64464 | attr hash → DB_Attribute* |

### ATTLIB_OpenAndInit 加载的数据（运行时内存镜像）

```c
// === 属性索引 / 定义 / 语法 ===
0x11BC1880  // attr_hash[] (ATGTIX-1)
0x11BC9880  // page_num[]
0x11BD1880  // offset[]
0x11BFA050  // attr_count

0x11BD9880  // field_hash[] (ATGTDF-1)
0x11BD9A10  // field_type[]
0x11BFA054  // field_count

0x11BDA050  // syntax_attr[] (ATGTSX-1)
0x11BE2050  // syntax_noun[]
0x11BEA050  // syntax_extra[]
0x11BFA05C  // syntax_count

// === 页面缓存 ===
0x11C2A860  // page_cache[512 * 1000] (2 MB)
0x10F5C7A0  // cached_page_nums[1000]
0x10F5D9A0  // lru_counts[1000]
```

### ATGTIX/页面结构速览

```
attlib.dat::ATGTIX:
  [hash_i][combined_i], combined_i = page*512 + offset

运行时:
  page_array[i]   = combined_i / 512
  offset_array[i] = combined_i % 512

PRDISP 字段所在页面示意:
  offset(PRDISP): N          // attr 数量
  attr_hash_1
  attr_hash_2
  ...
  attr_hash_N
  offset(other_field) ...
```

### LRU 页面缓存要点

- 最近使用页命中后直接返回缓存槽；否则最多缓存 1000 页，超出后按 LRU 替换。
- 每次访问会更新 `access_count` 和 `lru_counts`，并记录 `last_cache_idx` 以优化连续访问。

## Noun 获取属性“描述信息”的实现方案

1. **算 hash**：用 base27+0x81BF1 算法计算 noun 名（例如 `"ELBO"`）的哈希值。
2. **取属性 ID 列表**：对照运行时 `validProperties()` 的做法，从数据库的 `PRDISP` 字段拿到 `attr_ids`（读取顺序即展示顺序）。
3. **转属性对象并补全元数据**：对每个 `attr_id` 调 `DB_Attribute_findAttribute_by_id()`，若首次出现会调用 `ReadData()`，从 `attlib.dat` 读取 dtype/unit/default/syntax/owner 等描述。
4. **组装描述结构**：把 hash、名称、类型、默认值、单位、语法/范围、owner noun 等字段收集成统一结构，供上层展示。

参考 Rust 侧的伪代码（贴近现有解析器）：

```rust
pub struct AttrDesc {
    pub hash: u32,
    pub name: String,
    pub dtype: AttrType,
    pub default_val: AttrValue,
    pub unit: Option<String>,
    pub syntax: Option<SyntaxRule>,
    pub owner_noun: Option<u32>,
}

pub fn describe_noun_attrs(noun_code: &str, db: &DabDb, attlib: &mut AttlibStore) -> anyhow::Result<Vec<AttrDesc>> {
    let noun_hash = pdms_hash(noun_code);                    // 步骤 1
    let attr_ids = db.read_prdisp(noun_hash)?;               // 步骤 2，等价 internalGetField(PRDISP)

    let mut descs = Vec::with_capacity(attr_ids.len());
    for id in attr_ids {
        let attr = attlib.get_or_load(id)?;                  // 内部执行 findAttribute_by_id + ReadData
        descs.push(AttrDesc {
            hash: id,
            name: attr.name.clone(),
            dtype: attr.dtype,
            default_val: attr.default.clone(),
            unit: attr.unit.clone(),
            syntax: attr.syntax.clone(),
            owner_noun: attr.owner_noun,
        });
    }
    Ok(descs)
}
```

实现要点：

- `read_prdisp` 必须按 `internalGetField(PRDISP)` 同样的 hash → (page, offset) → 缓存 → 读数组的流程来解析 DAB。
- `get_or_load` 需复用 `DB_Attribute::dictionary_` 思路，避免重复解析 `attlib.dat`。
- 全局属性（如 `ATT_NAME`）在 dictionary 中已有实例，仍可复用其 `ReadData()` 结果，无需特别分支。

## 验证与对照

- `PRDISP` **未**出现在 `attlib.dat::ATGTDF`，印证它是数据库字段；但属性定义（数据类型、默认值、语法）仍在 `attlib.dat`，需要 `ReadData()` 才能拿到。
- 以 `ELBO` 为例：`pdms_hash("ELBO") = 0xCA439`，在 DAB 的 `PRDISP` 字段可读出 ~55 个属性 hash；逐个通过 `DB_Attribute_findAttribute_by_id()` + `ReadData()` 即可得到与 `data/ELBO.json` 一致的属性明细。

## 相关常量

```c
#define HASH_MIN         531442        // 0x81BF2
#define HASH_MAX         387951929     // 0x171FAD39
#define PAGE_SIZE        2048          // bytes
#define WORDS_PER_PAGE   512           // dword per page
#define MAX_CACHED_PAGES 1000
#define PRDISP_HASH      240391897     // 0x0E5416D9
```
