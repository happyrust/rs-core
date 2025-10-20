# IDA Pro 验证总结

**验证日期**：2025-10-20  
**验证工具**：IDA Pro MCP + 反编译分析  
**验证对象**：`docs/attlib_parsing_logic.md`  
**验证状态**：✅ 完成

---

## 核心发现

### 🔴 发现 1 个关键错误

**错误位置**：ATGTIX 三元组结构（第 3.3、4.1、10.1、12.2 节）

**错误内容**：
- 文档声称 ATGTIX 包含**三个字**：`attr_hash`、`record_num`、`slot_offset`
- 实际情况：ATGTIX 只包含**两个字**：`attr_hash`、`combined`
- `combined` 字需要分解：`record_num = combined / 512`，`slot_offset = combined % 512`

**IDA Pro 证据**：
```
函数：sub_10852A64 (Attlib_LoadAttrIndex)
地址：0x10852bf6-0x10852c49

汇编代码显示：
1. 读取一个字到 v16
2. 计算 record_num = v16 / 512（使用移位优化）
3. 计算 slot_offset = v16 % 512（使用除法）
```

**影响**：
- ❌ 所有基于三元组的解析代码都会错误
- ❌ Rust 实现会读取错误的字段
- ❌ 跨页数据拼接会出现偏移错误

---

## 验证结果详表

| 函数 | 地址 | 验证项 | 结果 | 说明 |
|------|------|--------|------|------|
| `Attlib_LoadAttrIndex` | 0x10852A64 | ATGTIX 三元组 | ❌ 错误 | 只有两个字，不是三个 |
| `Attlib_LoadAttrIndex` | 0x10852A64 | 哈希范围检查 | ✅ 正确 | 531442-387951929 |
| `Attlib_LoadAttrIndex` | 0x10852A64 | 页切换标记 | ✅ 正确 | 0x00000000 |
| `Attlib_LoadAttrIndex` | 0x10852A64 | 段结束标记 | ✅ 正确 | 0xFFFFFFFF |
| `Attlib_LoadAttrDefinitions` | 0x10852E20 | 默认值解析 | ✅ 正确 | TEXT 先读长度再读数据 |
| `Attlib_LoadAttrDefinitions` | 0x10852E20 | 标量默认值 | ✅ 正确 | 直接读一个字 |
| `Attlib_LoadAttrDefinitions` | 0x10852E20 | 缓冲区管理 | ✅ 正确 | a5 存指针，a6 存数据 |
| `sub_1044FC20` | 0x1044FC20 | FHDBRN 机制 | ✅ 正确 | 分页读取逻辑正确 |

---

## 正确的 ATGTIX 结构

### 二进制布局
```
ATGTIX 段：
┌─────────────────────────────────────────────────┐
│ 记录 1                                          │
├─────────────────────────────────────────────────┤
│ 字 0: attr_hash (u32)                           │
│ 字 1: combined (u32)                            │
│       ├─ record_num = combined / 512            │
│       └─ slot_offset = combined % 512           │
├─────────────────────────────────────────────────┤
│ 记录 2                                          │
├─────────────────────────────────────────────────┤
│ 字 0: attr_hash (u32)                           │
│ 字 1: combined (u32)                            │
│       ├─ record_num = combined / 512            │
│       └─ slot_offset = combined % 512           │
├─────────────────────────────────────────────────┤
│ ...                                             │
├─────────────────────────────────────────────────┤
│ 0x00000000 (页切换标记)                         │
│ 0xFFFFFFFF (段结束标记)                         │
└─────────────────────────────────────────────────┘
```

### Rust 结构
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
```

---

## 修正优先级

| 优先级 | 项目 | 影响 |
|--------|------|------|
| 🔴 高 | ATGTIX 三元组结构 | 核心解析逻辑 |
| 🟡 中 | 文档示例代码 | 实现参考 |
| 🟢 低 | 调试输出格式 | 验证工具 |

---

## 建议行动

1. **立即修正**（第 3.3、4.1、10.1、12.2 节）
   - 更新代码示例
   - 修正结构定义
   - 添加分解说明

2. **验证实现**
   - 运行 `cargo test test_attlib_parsing`
   - 对比 IDA 内存结构
   - 验证跨页数据拼接

3. **文档更新**
   - 在顶部添加"IDA Pro 验证通过"标记
   - 添加验证日期和工具版本
   - 链接到本验证报告

---

## 附录：IDA Pro 函数签名

| 函数 | 地址 | 大小 | 用途 |
|------|------|------|------|
| `sub_10852A64` | 0x10852A64 | 0x3BC | ATGTIX 加载 |
| `sub_10852E20` | 0x10852E20 | 0x594 | ATGTDF 加载 |
| `sub_1044FC20` | 0x1044FC20 | 0x38C | FHDBRN 实现 |

---

**验证完成**  
下一步：等待用户确认修正方案

