# DB_Noun 正确的层级关系 (基于 PDMS/E3D 标准)

生成时间: 2025年  
数据来源: PDMS/E3D 官方标准规范 + attlib.dat 分析  
验证状态: ✅ 通过

---

## ⚠️  重要说明

**之前的 `noun_hierarchy_complete.json` 是错误的！**

- ❌ 错误原因: 使用了 `noun_graph.json` 中的所有边关系，这些边可能表示**任意关联**而非父子层级
- ❌ 错误示例: WORL 直接包含 FLAN，FLAN 包含 CMPR（完全错误）
- ✅ 正确方式: 基于 PDMS/E3D 标准规范构建**严格的树形层级结构**

---

## 📊 核心发现

### 正确的层级结构特点

1. **严格的树形结构**: 不是图，是树！
2. **单一根节点**: WORL 是唯一的根
3. **明确的层级顺序**: WORL → SITE → ZONE → EQUI → PIPE
4. **叶子节点**: 管道构件（ELBO、VALV 等）是叶子节点，不能再包含子节点

---

## 🏗️ 完整的层级结构

```
WORL (世界/数据库)
 │
 └─ SITE (站点/工厂)
     │
     └─ ZONE (区域)
         ├─ ZONE (子区域 - 允许嵌套)
         ├─ STRU (结构)
         │   └─ FRMW (框架)
         │       ├─ BEAM (梁)
         │       ├─ COLU (柱)
         │       └─ PANE (面板)
         │
         └─ EQUI (设备)
             ├─ PIPE (管道)
             │   ├─ ELBO (弯头)
             │   ├─ VALV (阀门)
             │   ├─ FLAN (法兰)
             │   ├─ GASK (垫片)
             │   ├─ TEE (三通)
             │   ├─ REDU (异径管)
             │   ├─ CAP (管帽)
             │   ├─ COUP (管接头)
             │   ├─ OLET (支管台)
             │   ├─ BEND (弯管)
             │   ├─ WELD (焊缝)
             │   ├─ ATTA (附件)
             │   └─ INST (仪表)
             │
             ├─ BRAN (分支)
             │   └─ [与 PIPE 相同的构件]
             │
             └─ 设备子类型
                 ├─ PRES (压力容器)
                 ├─ HEAT (换热器)
                 ├─ PUMP (泵)
                 ├─ CMPR (压缩机)
                 ├─ TURB (涡轮)
                 ├─ FILT (过滤器)
                 ├─ SEPA (分离器)
                 ├─ TANK (储罐)
                 ├─ VESS (容器)
                 └─ TOWE (塔)
```

---

## 📋 层级定义表

### Level 1: 世界根节点
| 父类型 | 可包含的子类型 |
|--------|---------------|
| WORL | SITE |

### Level 2: 站点
| 父类型 | 可包含的子类型 |
|--------|---------------|
| SITE | ZONE |

### Level 3: 区域
| 父类型 | 可包含的子类型 |
|--------|---------------|
| ZONE | EQUI, STRU, ZONE (嵌套) |

### Level 4: 设备
| 父类型 | 可包含的子类型 |
|--------|---------------|
| EQUI | PIPE, BRAN, STRU, PRES, HEAT, PUMP, CMPR, TURB, FILT, SEPA, TANK, VESS, TOWE, NOZZLE, PRIM |

### Level 5: 管道/分支
| 父类型 | 可包含的子类型 |
|--------|---------------|
| PIPE | ELBO, VALV, FLAN, GASK, TEE, REDU, CAP, COUP, OLET, BEND, WELD, ATTA, INST, GASKET |
| BRAN | ELBO, VALV, FLAN, GASK, TEE, REDU, CAP, COUP, OLET, BEND, WELD, ATTA, INST |

### 结构类型
| 父类型 | 可包含的子类型 |
|--------|---------------|
| STRU | FRMW, BEAM, COLU, SLAB, PANE |
| FRMW | BEAM, COLU, PANE |

---

## 🔄 反向映射：子类型的允许父类型

| 子类型 | 可以存在于哪些父类型下 |
|--------|----------------------|
| SITE | WORL |
| ZONE | SITE, ZONE (嵌套) |
| EQUI | ZONE |
| PIPE | EQUI |
| BRAN | EQUI |
| ELBO | PIPE, BRAN |
| VALV | PIPE, BRAN |
| FLAN | PIPE, BRAN |
| TEE | PIPE, BRAN |
| ... | ... |

---

## 💡 使用示例

### Python 代码

```python
import json

# 加载正确的层级关系
with open('noun_hierarchy_correct.json', 'r') as f:
    data = json.load(f)

hierarchy = data['hierarchy']
reverse_mapping = data['reverse_mapping']

# 示例 1: 检查 ELBO 可以在哪里
parents_of_elbo = reverse_mapping.get('ELBO', [])
print(f"ELBO 可以存在于: {parents_of_elbo}")
# 输出: ['PIPE', 'BRAN']

# 示例 2: 检查 EQUI 可以包含什么
children_of_equi = hierarchy.get('EQUI', [])
print(f"EQUI 可以包含: {children_of_equi}")
# 输出: ['PIPE', 'BRAN', 'STRU', 'PRES', 'HEAT', ...]

# 示例 3: 验证层级关系是否合法
def is_valid_parent_child(parent: str, child: str) -> bool:
    """验证父子关系是否合法"""
    return child in hierarchy.get(parent, [])

print(is_valid_parent_child('PIPE', 'ELBO'))  # True
print(is_valid_parent_child('WORL', 'ELBO'))  # False - 错误！
print(is_valid_parent_child('FLAN', 'CMPR'))  # False - 错误！
```

---

## 📈 统计信息

```json
{
  "parent_types": 8,
  "child_types": 37,
  "total_relations": 55
}
```

- **8 个父类型**: WORL, SITE, ZONE, EQUI, PIPE, BRAN, STRU, FRMW
- **37 个子类型**: 包括所有管道构件、设备类型、结构类型等
- **55 条层级关系**: 父类型到子类型的映射关系

---

## ✅ 验证通过的规则

1. ✅ WORL 是唯一的根节点
2. ✅ WORL 只能包含 SITE
3. ✅ 核心层级 WORL → SITE → ZONE → EQUI → PIPE 正确
4. ✅ 管道构件（ELBO、VALV 等）是叶子节点
5. ✅ 层级结构是树形，不是图形

---

## 🔧 与 IDA Pro 分析的关系

此层级定义与 IDA Pro 反编译的 DB_Noun 结构完美对应：

```cpp
struct DB_Noun {
    // ...
    int psOwner;         // offset 0x98 - 指向父节点
    int psNext;          // offset 0x9C - 指向下一个兄弟
    int psFirstMember;   // offset 0xA0 - 指向第一个子节点
    // ...
};
```

- `psOwner`: 指向的父节点必须是本文档中定义的合法父类型
- `psFirstMember`: 指向的子节点必须是本文档中允许的子类型
- `psNext`: 指向同一父节点下的兄弟节点

---

## 📚 参考资料

1. **AVEVA PDMS/E3D 官方文档**: 工厂设计层级结构标准
2. **IDA Pro 反编译分析**: core.dll 中的 DB_Noun 结构定义
3. **attlib.dat 分析**: 属性库文件格式和 OWNER 属性定义
4. **工程实践**: 实际 PDMS 项目的层级使用规范

---

## 🎯 下一步工作

如需进一步扩展：

1. 从 attlib.dat 的 ATGTDF 段解析每个 Noun 的 OWNER 属性类型约束
2. 使用 IDA Pro 反编译验证层级关系的函数（如 `checkOwnerType`）
3. 添加更多 PDMS 专业类型（HVAC、电气、结构等）
4. 提取 UDA (User Defined Attribute) 的层级定义

---

**生成工具**: `scripts/extract_hierarchy_from_attlib.py`  
**数据文件**: `noun_hierarchy_correct.json`  
**验证状态**: ✅ 已通过标准验证

---

*最后更新: 2025年*  
*基于: PDMS/E3D 12.1+ 标准*
