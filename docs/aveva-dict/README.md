# AVEVA E3D 字典文件格式

本文记录 `attlib.dat` 与 `desvir.dat` 两个文件的二进制布局，以及 `core.dll` 读取它们的调用链。
`rs-core/noun_graph.json` 就是从 `desvir.dat` 导出的；版本升级后要重跑提取流程时看这里。

结论来自对 `AVEVA Everything3D 3.1` 的 `core.dll`（50 MB，保留了完整 MSVC C++ 符号）做静态分析，
并用真实模型 `sam7200_0001` 对账验证。

---

## 1. 两个文件各管什么

| 文件 | 大小 | 内容 |
|---|---|---|
| `attlib.dat` | 5,840,896 B = 2852 页 | 元素类型（noun）与属性（attribute）的**元数据表**：分类旗标、数据类型、默认值、名称缩写 |
| `desvir.dat` | 4,847,616 B = 2367 页 | 设计侧 Dabacon **模板库**：每个元素类型的完整描述符，含合法成员/属主列表 |
| `padvir.dat` | 985,088 B | 制图侧模板库。目录布局与 `desvir.dat` 不同，尚未解析 |

两个文件都是**大端 int32**、**每页 512 个字**（2048 字节）。

---

## 2. core.dll 的调用链

```
DB_Noun::primitive() / geomset() / visible() / graphicsBehaviour() ...
  └─ DB_Noun::internalGetField(hash) ─ ATNLOG ─ ATRDRC ─ FHDBRN ──→ attlib.dat

DB_Noun::eleTypes(ATT_MEMB / ATT_OWNER)
  └─ dab 462 ─ 模板库描述符查找 ────────────────────────────────→ desvir.dat

DB_IteratorCreator::getIterator()
  ├─ pred1  接受谓词：DB_PredicateType / DB_PredicateElementTypeFieldEqual<bool>
  └─ pred2  下探剪枝：DB_Noun::recurseEleTypes(ATT_OWNER) 的传递闭包
DB_Iterator::next()  深度优先，pred2 决定是否下探，pred1 决定是否返回
```

`ATTOPE`（`sub_55F4290`）在启动时打开 `attlib.dat` 并把六个索引段读进内存。

---

## 3. attlib.dat

### 3.1 目录

**第 2 页**（1-based 页号，字节偏移 `(page-1) * 2048`）的前 8 个字是段起始页号。
E3D 3.1 实测值 `[3, 4, 2098, 2123, 2168, 2169, 2830, 2838]`：

| 下标 | 起始页 | 段 |
|---|---|---|
| 0 | 3 | 属性字段定义 |
| 1 | 4 | 属性记录数据（4–2097） |
| 2 | 2098 | 属性索引 |
| 3 | 2123 | 属性名称缩写表 |
| 4 | 2168 | **noun 字段定义** |
| 5 | 2169 | **noun 记录数据**（2169–2829） |
| 6 | 2830 | **noun 索引**（2830–2837） |
| 7 | 2838 | noun 名称缩写表 |

名称与整数键之间是 base-27 哈希：

```python
def db1_hash(name):
    val = 0
    for ch in reversed(name.upper()):
        val = val * 27 + (0 if ch == " " else ord(ch) - 64)
    return val + 27**4          # 0x81BF1
```

合法哈希区间 `[531442, 387951929]`，超出即视为非记录。

### 3.2 索引段（`ATGTIX` 格式，段 2 与段 6）

连续的 `(hash, combined)` 二元组：

```
page_no = combined // 512
offset  = combined %  512      # 页内 1-based 字下标
```

遇 `0` 翻下一页继续，遇 `-1` 段结束。

### 3.3 定义段（`ATGTDF` 格式，段 0 与段 4）

```
(hash, data_type, flag)
  flag == 1  无默认值
  flag == 2  紧跟默认值：
               data_type == 4 → 先一个长度字，再该长度个字
               其他          → 一个标量字
```

**加载顺序就是列号**（1-based）。`ATFIND` 对这个数组做线性查找，列号即数组下标。

### 3.4 取值（`ATNLOG`）

设 noun 索引给出 `(page_no, off)`，字段列号 `col`，`page` 为该页的 512 个字（0-based 数组）：

```python
slot = page[off + col - 2]

if slot == 0:                       # 本类型未定义 → 沿 SYNO（同义词）链找父类型
    syno = page[off + SYNO_col - 2]
    if syno in (0, -1): raise Unset
    parent_hash = page[off + syno - 2]
    return lookup(parent_hash, col)

if slot == -1:                      # 用该字段的全局默认值
    return field_default[col]

return page[off + slot - 2]         # 本类型自己的值
```

`SYNO` 的哈希是 `837586`。布尔字段的编码是 **1 = 真，2 = 假**（不是 1/0）。
`data_type`：`1` 逻辑、`2` 引用、`3` 整数、`4` 文本/数组、`8`/`11` 复合。

### 3.5 noun 字段表（93 列，节选）

E3D 3.1 共 1931 个 noun。几何与遍历相关的：

| 列 | 字段 | 类型 | 为真数量 | 对应 API |
|---|---|---|---|---|
| 5 | `VISI` | 逻辑 | 1813 | `visible()`，`COLLECT` 扫描的默认剪枝依据 |
| 12–14 | `PSOWNR` `PSNEXT` `PSFRST` | 整数 | — | 层级走访用的伪属性号 |
| 26 | `IMAP` | 整数 | — | `spatialMap()` |
| 30 | `VOLDEF` | 逻辑 | 102 | `defaultVolumeQuery()` |
| 32 | `GRAPH` | 整数 | — | `graphicsBehaviour()`，0 = 普通 |
| 33 | `TOPF` | 逻辑 | 211 | `toplevel()` |
| 35 | `PICK` | 逻辑 | 374 | `pickable()` |
| 38–39 | `CLWTHN` `CLRSEC` | 逻辑 | 9 / 30 | 碰撞检查分类 |
| 66 | `PRMF` | 逻辑 | **347** | `primitive()`，设计侧“有几何” |
| 67 | `POPF` | 逻辑 | 44 | `point()` |
| 68 | `XTRF` | 逻辑 | 38 | `extrusion()` |
| 69 | `GORP` | 逻辑 | 44 | `geomset()`，目录侧几何 |
| 89 | `SYNO` | 整数 | — | 继承链 |

两点容易踩错：

- **`GRAPH` 不是“有没有几何”**。`BOX` 和 `EQUI` 都是 0，`SITE`/`ZONE`/`WORL` 是 2，制图类是 1，
  电缆桥架与辅助元素是 3。它是“特殊绘制行为”分类，0 表示普通。
- **设计侧与目录侧是两套旗标**。`PRMF` 是 `BOX`/`CYLI`/`ELBO`/`TUBI`/`NOZZ` 这类；
  `GORP` 是目录库的 `SBOX`/`SCYL`/`SCON`/`SDSH`/`SEXT`/`SREV`/`TUBE`、它们的负体变体（`NBXI`/`NLCY`/`NSEX`…）
  以及 P-point（`PTAX`/`PTCA`/`PTMI`/`PTPOS`）。查“有几何的子节点”时两者不能混。

### 3.6 属性字段表（63 列）

`SIZE DTYP TABLE TKEYLN TDATLN AVAIDB NAME RPTX DESTEX TYPE BASLAT QTXT DEFI QUAL ENUM
QSUPPA VISI DEPEND ITYP UNIT NOCACH PRTCTA QSET DACCHK WDEFI COPY CHANGE WNOEVT WNOCLM
RULEDP DESRUL CATPAR CATRUL IDREF EXPSIZ CASC TUBE ITSATT ITSNPF COORD ITSEL EQUIVA
TABATT MAGICV DTLATT DEFER DTLDFL RECONF DCHC PLCF PNAME LGSPLA CONNEX CATEG MANUAL
SYNO DFDP INVIS PSCODE TBLTYP TRUN PREDIT PRCONV`

共 6376 个属性。**这里没有成员/属主表**——那在模板库里。

---

## 4. desvir.dat

### 4.1 目录

**第 2351–2365 页**，每页 73 条 × 7 个字 = 511 字（恰好填满一页），共 1095 条，按哈希升序供二分查找：

```
[0] noun hash
[1] 描述符起始块号
[2] 描述符长度（字）
[3] 第二段起始块号
[4] 第二段长度
[5] 第三段起始块号
[6] 第三段条数
```

> 目录**按页存放**，跨页不连续。直接线性扫描只能拿到 73 条就断，必须逐页解析。

### 4.2 描述符：链式块读取

每块 512 字，**前 511 字是数据，第 512 个字是下一块块号**（仅在还有剩余数据时存在）：

```python
def read_chain(start_block, total_words):
    data, blk = [], start_block
    while len(data) < total_words:
        w = block(blk)                       # 512 words
        data.extend(w[:min(511, total_words - len(data))])
        if len(data) >= total_words:
            break
        blk = w[511]
    return data
```

### 4.3 描述符布局

```
desc[1]  = 自身 noun hash          # 可用来校验解析是否对齐
desc[9]  = 属性块数量
desc[10] = 成员类型数量            # ATT_MEMB，字典键 "MEM" = 541066
desc[11] = 成员列表在 desc 内的字下标
desc[12] = 属主类型数量            # ATT_OWNER，键 10206636
desc[13] = 属主列表在 desc 内的字下标
desc[14..] = 属性块
```

1095 条目录里有 118 条描述符只有 3 个字，那是属性桩（例如 `NAME`），不是元素类型。
**其余 977 个是真正的元素类型。**

---

## 5. 提取流程

脚本都在本目录，按顺序跑：

```bash
python attlib_dump.py          # → noun_flags.json        1931 noun × 15 旗标
python template_dump.py        # → noun_graph_full.json   977 类型 / 3207 成员边
python noun_closure.py         # → noun_closure.json      祖先闭包与剪枝白名单
python gen_noun_graph_json.py  # → noun_graph.json        petgraph 格式
python validate.py             # 用真实模型对账
python noun_summary.py         # 旗标分布与抽查（可选）

cp noun_graph.json ../../noun_graph.json   # crate 用 include_bytes! 读根目录那份
```

各脚本顶部的常量里写着 E3D 安装路径与测试模型路径，换版本时改那里。
`noun_model.json` 是旗标表与关系图的合并结果，方便一次性查询，不参与编译。

---

## 6. 验证结果

对 `pdms-io/pdms-test-data/sam7200_0001`（6.9 MB 真实模型）：

```
元素总数                 15,736（144 种类型）
实际出现的 owner→member  203 种 → 推导图覆盖 203 / 203
primitive 元素           5,549 个 → 剪枝白名单全部可达 5,549 / 5,549
剪枝效果                 跳过 2,191 个元素（13.9%）
```

没有真实存在的父子组合落在图外，也没有几何节点被误剪。

图的自身一致性：双向边一致 3174 / 3207。不一致的部分归因于 23 个“只作为成员名字出现、
自己没有描述符”的类型（`CONVEY`、`ASMBLY`、`AIDTEX` 等）。

### 一个实现上的坑

按 B 树从 index root 递归遍历真实模型会撞到子页号越界。改用**按记录签名全盘扫描**后完全稳定：
4 字节对齐，匹配 `[impl_len][ref0][ref1][type_hash][own0][own1]`，要求 `ref0 == own0 == db_num`、
`type_hash` 能解成合法 base-27 名、`0 < impl_len < 2048`。后出现的副本覆盖先出现的，
自然就取到最新 session 的版本。

---

## 7. 图的形状（影响查询规划）

- 977 个节点、3310 条边（含反向补边）。
- 最长的最短路径 **9 跳**。
- **32 个类型可以拥有自己**（`ZONE` 下面还能放 `ZONE`，`TMPL`/`PANE` 互相嵌套）。
  任何按层展开的算法都必须封顶，`rs-core` 里是 `MAX_NOUN_DEPTH = 12`。
- `PRMF` 的下探白名单是 346 / 977 个类型（35.4%），可剪掉 631 个类型的整棵子树。
  单一目标类型更窄：只找 `ELBO` 时祖先闭包只有 22 个类型。

> 求闭包时 owner 关系必须**同时**取自 `owners` 字段和 `members` 字段的反向边。
> 有 23 个类型只作为成员名字出现、自己没有描述符，只用 `owners` 会漏掉它们的属主。
