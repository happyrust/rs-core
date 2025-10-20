# attlib.dat Python 解析分析总结

## 执行概览

使用 Python 脚本对 `data/attlib.dat` 进行了快速原型化和分析，发现该文件的结构与预期不符。

## 工作成果

### 1. 创建了 Python 版本的解析器
- **文件**：`scripts/parse_attlib.py`
- **功能**：
  - 读取 attlib.dat 文件
  - 解析段指针表
  - 加载 ATGTIX 段（属性索引）
  - 尝试加载 ATGTDF 段（属性定义）
  - 查询特定属性

### 2. 发现的关键问题

#### 问题 1：ATGTDF 段不存在
- **预期**：attlib.dat 应该包含 ATGTIX 和 ATGTDF 两个段
- **实际**：只找到了 ATGTIX 段（15 条记录），ATGTDF 段为空（0 条记录）

#### 问题 2：段指针表指向错误的位置
- **段指针**：[3, 4, 1683, 1704, 1741, 1742, 2236, 2242]
- **页 3 和 4 的内容**：都是文本数据，不是属性定义
- **页 0 的内容**：包含 ATGTIX 段

#### 问题 3：ELBO 属性大部分找不到
- **ELBO.json 中的属性**：55 个
- **在 attlib.dat 中找到的**：2 个（NAME 和 TYPE）
- **未找到的**：53 个（包括 POS、ORI 等）

## 解析结果

### ATGTIX 段中的 15 条属性

```
[0]  hash=0x0009E770 (649072),    combined=0x00000003
[1]  hash=0x000D3372 (865138),    combined=0x00000003
[2]  hash=0x0034482B (3426347),   combined=0x00000001
[3]  hash=0x0C6A3EBC (208289468), combined=0x00000003
[4]  hash=0x0C68B22C (208187948), combined=0x00000003
[5]  hash=0x01E129B2 (31533490),  combined=0x00000008
[6]  hash=0x0009C18E (639374),    combined=0x00000004  ← NAME
[7]  hash=0x000F8BEF (1018863),   combined=0x00000004
[8]  hash=0x14BD9A10 (347970064), combined=0x00000004
[9]  hash=0x0009CCA7 (642215),    combined=0x00000003  ← TYPE
[10] hash=0x112EF91A (288291098), combined=0x00000001
[11] hash=0x000E6432 (943154),    combined=0x00000004
[12] hash=0x000AE18D (713101),    combined=0x00000003
[13] hash=0x000BBBB6 (768950),    combined=0x00000008
[14] hash=0x000C40C4 (803012),    combined=0x00000003
```

### ATGTDF 段
- **状态**：未找到
- **尝试的位置**：
  - 页 0 之后（ATGTIX 段结束后）：0 条记录
  - 段指针 [0] 指向的页 3：0 条记录
  - 段指针 [1] 指向的页 4：0 条记录

## 根本原因分析

### 假设 1：data/attlib.dat 是不完整的文件
- 可能只包含 ATGTIX 段，没有 ATGTDF 段
- 可能是从 E3D 的某个特定版本或配置中提取的

### 假设 2：段指针表的含义不同
- 可能不是指向 ATGTIX/ATGTDF 段的指针
- 可能指向其他类型的数据结构

### 假设 3：ELBO.json 来自其他来源
- 不是来自 attlib.dat
- 可能来自 E3D 的运行时数据库或其他配置文件

## 建议的后续步骤

### 优先级 1：获取完整的 attlib.dat
- 从 E3D 的安装目录中获取完整的属性库文件
- 对比完整文件和当前文件的结构差异

### 优先级 2：分析 ELBO.json 的来源
- 检查 E3D 的数据库结构
- 查看是否有其他文件存储了 ELBO 属性的定义

### 优先级 3：验证 IDA Pro 分析
- 使用 IDA Pro 跟踪 `sub_10852A64` 和 `sub_10852E20` 的实际行为
- 确认段指针是否真的指向 ATGTIX/ATGTDF 段

### 优先级 4：理解 combined 字段
- 当前的 combined 值都很小（0-8）
- 可能不是用来定位 ATGTDF 的
- 需要进一步调查其实际含义

## 生成的文件

1. **scripts/parse_attlib.py** - Python 版本的 attlib 解析器
2. **docs/attlib_analysis_report.md** - 详细的分析报告
3. **docs/ATTLIB_PYTHON_ANALYSIS_SUMMARY.md** - 本文档

## 结论

当前的 `data/attlib.dat` 文件结构不完整或格式特殊，无法直接解析出 ELBO 属性的完整定义。需要获取完整的属性库文件或找到 ELBO 属性定义的其他来源。

