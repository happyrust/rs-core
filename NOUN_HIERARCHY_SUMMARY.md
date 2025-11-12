# DB_Noun 层级关系提取报告

生成时间: 2025年
数据来源: attlib.dat + all_attr_info.json

---

## 📊 统计摘要

- **Noun 类型总数**: 337 个
- **已识别 Noun**: 53 个
- **未识别 Noun**: 284 个
- **标准层级关系**: 6 个父类型，39 个父子关系

---

## 🏗️ PDMS/E3D 标准层级结构

### 完整层级树

```
WORL (世界/数据库)
 └── SITE (站点/工厂)
      ├── ZONE (区域)
      │    ├── EQUI (设备)
      │    │    ├── PIPE (管道)
      │    │    │    ├── ELBO (弯头)
      │    │    │    ├── VALV (阀门)
      │    │    │    ├── FLAN (法兰)
      │    │    │    ├── GASK (垫片)
      │    │    │    ├── TEE (三通)
      │    │    │    ├── REDU (异径管)
      │    │    │    ├── CAP (管帽)
      │    │    │    ├── COUP (管接头)
      │    │    │    ├── OLET (支管台)
      │    │    │    ├── BEND (弯管)
      │    │    │    └── WELD (焊缝)
      │    │    │
      │    │    ├── ELBO (弯头)
      │    │    ├── VALV (阀门)
      │    │    ├── FLAN (法兰)
      │    │    ├── GASK (垫片)
      │    │    ├── TEE (三通)
      │    │    ├── REDU (异径管)
      │    │    ├── CAP (管帽)
      │    │    ├── COUP (管接头)
      │    │    ├── OLET (支管台)
      │    │    └── BEND (弯管)
      │    │
      │    ├── PIPE (管道)
      │    └── SUBZONE (子区域)
      │
      ├── PIPE (管道)
      └── EQUI (设备)

BRAN (分支)
 ├── ELBO (弯头)
 ├── VALV (阀门)
 ├── FLAN (法兰)
 ├── GASK (垫片)
 ├── TEE (三通)
 ├── REDU (异径管)
 ├── CAP (管帽)
 ├── COUP (管接头)
 ├── OLET (支管台)
 └── BEND (弯管)
```

---

## 📦 已识别的 Noun 类型分类

### 1. 核心层级类型 (6个)
| Hash | 名称 | 说明 |
|------|------|------|
| 564937 | WORL | 世界/数据库根节点 |
| 631900 | SITE | 站点/工厂 |
| 724361 | ZONE | 区域 |
| 907462 | EQUI | 设备 |
| 958465 | PIPE | 管道 |
| 900968 | BRAN | 分支 |

### 2. 管道构件 (13个)
| Hash | 名称 | 说明 |
|------|------|------|
| 640493 | ELBO | 弯头 |
| 621502 | VALV | 阀门 |
| 779672 | FLAN | 法兰 |
| 640105 | GASK | 垫片 |
| 862086 | TEE | 三通 |
| 808220 | REDU | 异径管 |
| 890182 | CAP | 管帽 |
| 739306 | COUP | 管接头 |
| 621505 | OLET | 支管台 |
| 821683 | BEND | 弯管 |
| 581519 | WELD | 焊缝 |
| 679463 | ATTA | 附件 |
| 718014 | INST | 仪表 |

### 3. 结构类型 (6个)
| Hash | 名称 | 说明 |
|------|------|------|
| 619079 | STRU | 结构 |
| 897228 | FRMW | 框架 |
| 931840 | PANE | 面板 |
| 10403889 | BEAM | 梁 |
| 559969 | COLU | 柱 |
| 3471220 | SLAB | 板 |

### 4. 设备分类 (10个)
| Hash | 名称 | 说明 |
|------|------|------|
| 912101 | PRES | 压力容器 |
| 549344 | HEAT | 换热器 |
| 713035 | PUMP | 泵 |
| 713316 | CMPR | 压缩机 |
| 661557 | TURB | 涡轮 |
| 7146286 | FILT | 过滤器 |
| 929085 | SEPA | 分离器 |
| 641779 | TANK | 储罐 |
| 620516 | VESS | 容器 |
| 900977 | TOWE | 塔 |

### 5. 电气类型 (5个)
| Hash | 名称 | 说明 |
|------|------|------|
| 643214 | CABL | 电缆 |
| 312510290 | COND | 导管 |
| 897213 | JUNC | 接线盒 |
| 973264 | PANE | 配电盘 |
| 717396 | LIGH | 灯具 |

### 6. HVAC类型 (5个)
| Hash | 名称 | 说明 |
|------|------|------|
| 711154 | DUCT | 风管 |
| 602740 | FITT | 管件 |
| 621602 | DAMP | 风阀 |
| 108608856 | GRILLE | 格栅 |
| 312510247 | DIFF | 散流器 |

### 7. 其他常见类型 (8个)
| Hash | 名称 | 说明 |
|------|------|------|
| 269723131 | SUBS | 子系统 |
| 5177808 | GROU | 组 |
| 833646 | ITEM | 项目 |
| 623975 | SPEC | 规格 |
| 968612 | CATA | 目录 |
| 904406 | TEXT | 文本 |
| 938782 | DRAW | 图纸 |
| 535241 | SYMB | 符号 |

---

## 🔍 层级关系详解

### 父子关系矩阵

| 父类型 | 可包含的子类型 | 数量 |
|--------|---------------|------|
| WORL | SITE | 1 |
| SITE | ZONE, PIPE, EQUI | 3 |
| ZONE | EQUI, PIPE, SUBZONE | 3 |
| EQUI | PIPE, ELBO, VALV, FLAN, GASK, TEE, REDU, CAP, COUP, OLET, BEND | 11 |
| PIPE | ELBO, VALV, FLAN, GASK, TEE, REDU, CAP, COUP, OLET, BEND, WELD | 11 |
| BRAN | ELBO, VALV, FLAN, GASK, TEE, REDU, CAP, COUP, OLET, BEND | 10 |

---

## 💡 使用示例

### 1. 判断层级关系

```python
import json

# 加载层级关系
with open('noun_hierarchy_complete.json') as f:
    hierarchy = json.load(f)

# 检查 ELBO 是否可以在 EQUI 下
standard_hierarchy = hierarchy['standard_hierarchy']
if 'ELBO' in standard_hierarchy.get('EQUI', []):
    print("✅ ELBO 可以作为 EQUI 的子对象")
```

### 2. 遍历完整层级树

```python
def print_hierarchy(parent, hierarchy, level=0):
    """递归打印层级树"""
    indent = "  " * level
    children = hierarchy.get(parent, [])
    
    if children:
        for child in children:
            print(f"{indent}├── {child}")
            print_hierarchy(child, hierarchy, level + 1)

# 从根节点开始遍历
print("WORL")
print_hierarchy('WORL', standard_hierarchy)
```

### 3. 查找 Noun 的所有可能父类型

```python
def find_parents(child_noun, hierarchy):
    """查找指定 Noun 的所有可能父类型"""
    parents = []
    for parent, children in hierarchy.items():
        if child_noun in children:
            parents.append(parent)
    return parents

# 查找 ELBO 的父类型
parents = find_parents('ELBO', standard_hierarchy)
print(f"ELBO 的可能父类型: {', '.join(parents)}")
# 输出: ELBO 的可能父类型: EQUI, PIPE, BRAN
```

---

## 📁 生成的文件

1. **noun_hierarchy_complete.json** - 完整的层级关系数据
   - `noun_definitions`: 所有 Noun 类型的 hash 到名称映射
   - `standard_hierarchy`: 标准的父子关系定义
   - `graph_hierarchy`: 从 noun_graph.json 提取的图结构数据

2. **NOUN_HIERARCHY_SUMMARY.md** - 本摘要文档

---

## 🔄 与 IDA Pro 分析的关系

本报告结合了以下数据源：

1. **IDA Pro 反编译**: 从 core.dll 提取的 DB_Noun 结构定义
   - psOwner (0x98): 指向父对象
   - psNext (0x9C): 指向兄弟对象
   - psFirstMember (0xA0): 指向第一个子对象

2. **attlib.dat**: PDMS 属性库文件
   - Noun 类型定义
   - 属性关系定义

3. **all_attr_info.json**: 属性元数据
   - 337 个 Noun 类型的 hash 值
   - 属性到 Noun 的映射关系

4. **PDMS/E3D 标准**: 工程设计标准层级
   - WORL → SITE → ZONE → EQUI → PIPE 层级
   - 管道构件的包含关系

---

## ⚠️ 注意事项

1. **未识别的 Noun**: 
   - 目前有 284 个 Noun 类型尚未识别名称
   - 这些 Noun 以 `NOUN_<hash>` 格式表示
   - 可通过扩展 `decode_noun_name()` 函数添加更多映射

2. **动态层级**:
   - 实际项目中，Noun 之间的层级关系可能更复杂
   - 某些 Noun 类型可能支持非标准的父子关系
   - 建议结合实际项目数据进行验证

3. **扩展支持**:
   - 可以通过分析 attlib.dat 的其他段获取更多信息
   - 用户自定义类型 (UDET) 可能有特殊的层级规则

---

## 📚 相关文档

- `/Volumes/DPC/reverse/DB_Noun_Analysis_Summary.md` - DB_Noun 结构完整分析
- `docs/attlib_parsing_logic.md` - attlib.dat 文件格式说明
- `noun_graph.json` - 原始的 Noun 图结构数据

---

**提取完成时间**: 2025
**数据版本**: 1.0
**工具**: Python 3 + IDA Pro Analysis
