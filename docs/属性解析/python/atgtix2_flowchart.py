#!/usr/bin/env python3
"""
ATGTIX-2 解析流程线框图生成器
基于 parse_atgtix2.py 的执行逻辑生成 Mermaid 流程图
"""

def generate_flowchart():
    """生成 ATGTIX-2 解析流程的 Mermaid 线框图"""
    
    flowchart = '''```mermaid
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

## 核心数据结构说明

### 页面结构 (Page Structure)
```
页面大小: 2048 字节
每页字数: 512 个 32位整数
数据区域: 从 0x1000 偏移开始
```

### 记录格式 (Record Format)
```
每条记录包含4个字段:
1. noun_hash: noun的哈希值 (用于Base27解码)
2. page: 属性数据所在页面号
3. offset: 页面内的偏移位置
4. combined: page * 512 + offset 的组合值
```

### 特殊标记 (Special Markers)
```
PAGE_SWITCH = 0x00000000    # 页面切换标记
SEGMENT_END = 0xFFFFFFFF    # 段结束标记
MIN_HASH = 0x81BF2          # 最小有效哈希
MAX_HASH = 0x171FAD39       # 最大有效哈希
```

## 关键算法特点

### 1. 智能起始页检测
- 通过预筛选包含有效哈希的页面提高效率
- 使用已知noun (PIPE/SITE) 作为锚点验证
- 选择记录数最多的页面作为起始点

### 2. 流式解析机制
- 按页面顺序遍历，支持跨页连续读取
- 自动处理页面切换和段结束标记
- 跳过无效哈希值，只处理有效记录

### 3. Base27解码算法
- 将数值哈希转换为可读的noun名称
- 使用27进制: 26个大写字母 + 空格
- 逆向计算: 从低位到高位逐步解码

## 性能优化点

1. **批量页面加载**: 一次性读取所有页面到内存
2. **快速预筛选**: 只处理包含有效哈希的页面
3. **锚点验证**: 使用已知noun提高检测准确性
4. **流式输出**: 边解析边写入CSV，减少内存占用
'''
    
    return flowchart

if __name__ == "__main__":
    print(generate_flowchart())
