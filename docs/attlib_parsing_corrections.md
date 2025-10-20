# attlib_parsing_logic.md 修正方案

基于 IDA Pro 反编译验证，以下是需要修正的具体内容。

---

## 修正 1：第 3.3 节 - 记录解析逻辑

### 当前错误代码
```c
// 从 Attlib_LoadAttrIndex 函数提取
while (1) {
    word = buffer[++index];           // 读取 32 位字
    
    if (word < RANGE_MIN || word > RANGE_MAX)  // 范围检查
        break;
    
    attr_hash = word;                 // 属性哈希值
    record_num = buffer[++index] / 512;    // 记录号 ❌ 错误
    slot_offset = buffer[++index] % 512;   // 页内偏移 ❌ 错误
    
    if (word == 0) {                  // 页切换
        page_num++;
        continue;
    }
    
    if (word == -1) {                 // 段结束
        break;
    }
}
```

### 修正后代码
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

### 说明
- **原错误**：文档误认为 `record_num` 和 `slot_offset` 分别来自两个不同的字
- **实际情况**：两者都来自**同一个字**，通过除法和模运算提取
- **IDA 验证**：汇编代码 0x10852bf6-0x10852c49 明确显示只读取一个字 `v16`

---

## 修正 2：第 4.1 节 - AttlibAttrIndex 结构

### 当前错误表格
| 顺序 | 字段 | 含义 | 备注 |
|------|------|------|------|
| 0 | `attr_hash` | 属性 / Noun 哈希值 | 531 442 – 387 951 929 |
| 1 | `record_num` | 属性定义所在的页号 | 与段指针表同单位 |
| 2 | `slot_offset` | 页内偏移（以 32 位字为单位） | 0–511 |

### 修正后表格
| 顺序 | 字段 | 含义 | 备注 |
|------|------|------|------|
| 0 | `attr_hash` | 属性 / Noun 哈希值 | 531 442 – 387 951 929 |
| 1 | `combined` | 组合字（包含 record_num 和 slot_offset） | 需要分解 |

### 分解方式
```c
uint32_t combined = index_entry[1];
uint32_t record_num = combined / 512;    // 商 = 页号
uint32_t slot_offset = combined % 512;   // 余数 = 页内偏移
```

### 说明
- **原错误**：表格显示三个字段，但实际只有两个字
- **实际情况**：第二个字包含两个信息，需要通过算术运算分解
- **范围**：`combined` 的范围应该是 0 到 (512 * 512 - 1) = 262143

---

## 修正 3：第 4.1 节 - 字节定位算法

### 当前代码
```
byte_offset = record_num * 2048 + slot_offset * 4
```

### 修正后代码
```c
// 方法 1：直接使用 combined 字
uint32_t combined = index_entry[1];
byte_offset = (combined / 512) * 2048 + (combined % 512) * 4;

// 方法 2：等价的简化形式
byte_offset = combined * 4;  // 因为 combined 已经是字索引
```

### 说明
- 第一种方法与原文档一致，但需要先分解 `combined`
- 第二种方法更简洁：`combined` 本身就是全局字索引

---

## 修正 4：第 12.2 节 - 属性解析伪代码

### 当前错误代码
```rust
let def_hash = words[cursor];
let data_type = words[cursor + 1];
let default_flag = words[cursor + 2];
```

### 修正后代码
```rust
let def_hash = words[cursor];
let combined = words[cursor + 1];
let record_num = combined / 512;
let slot_offset = combined % 512;
let data_type = words[cursor + 2];
let default_flag = words[cursor + 3];
```

### 说明
- ATGTIX 段中的属性定义位置由 `combined` 字指定
- 需要分解 `combined` 才能定位到 ATGTDF 段中的实际定义

---

## 修正 5：第 10.1 节 - Rust 结构定义

### 当前代码
```rust
#[repr(C)]
pub struct AttlibAttrIndex {
    pub attr_hash: u32,
    pub record_num: u32,    
    pub slot_offset: u32,
}
```

### 修正后代码
```rust
#[repr(C)]
pub struct AttlibAttrIndex {
    pub attr_hash: u32,
    pub combined: u32,  // 包含 record_num 和 slot_offset
}

impl AttlibAttrIndex {
    pub fn record_num(&self) -> u32 {
        self.combined / 512
    }
    
    pub fn slot_offset(&self) -> u32 {
        self.combined % 512
    }
}
```

---

## 总结

| 项目 | 错误类型 | 影响范围 |
|------|---------|---------|
| ATGTIX 三元组 | 结构误解 | 第 3.3, 4.1, 10.1, 12.2 节 |
| 字段数量 | 文档说三个，实际两个 | 所有相关代码示例 |
| 分解方式 | 需要算术运算 | 所有解析实现 |

**优先级**：🔴 **高** - 这个错误会导致解析器实现完全错误

---

## 验证清单

- [ ] 修正第 3.3 节代码块
- [ ] 修正第 4.1 节表格和说明
- [ ] 修正第 10.1 节 Rust 结构
- [ ] 修正第 12.2 节伪代码
- [ ] 更新所有相关的代码示例
- [ ] 在文档顶部添加"已验证"标记
- [ ] 运行 Rust 解析器测试验证修正

