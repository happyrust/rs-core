# 03 · `isobran.putIntoIsoLine()` — 标注对象归段

> 源码：`MBD/markpipe/object/isobran.pmlobj` L1144–1353（现行实现；L1299 以下为注释掉的旧实现）
> 上游：`getobjects()` L1094–1142；配套课程：`lessons/0003/0047/0048/0069–0071`
> Rust 教学复刻：`teach/p1_split/src/put_into_isoline.rs` + fixture `teach/fixtures/p4-put-into-isoline.json`

## 1. 对象收集顺序 `getobjects()`（L1094）

收集顺序即**绘制优先级**（注释明确说明谁必须在前）：

| 顺序 | 收集方法 | 产物类型 | 备注 |
|---|---|---|---|
| 0 | `getattadatas()` | pipeatta 附件族 | 失败即中止 |
| 1 | `getadjustwelds()` | `ISOADJUSTWELD` | *must be first*（优先级高于除 slope/angle 外所有对象） |
| 2 | `getinstallationangles()` | `ISOMBDANGLE` | *must be second* |
| 3 | `getslopes()` | `ISOSLOPE` | *must be third*（优先级最高） |
| 4 | `getmaterials()` | `ISOMATERIAL` | |
| 5 | `gettags()` | `ISOTAG` | |
| 6 | `getwelds()` | `ISOWELDTEXT` | |
| 7 | `getbranmlabel()`+`getcutparts()` | 切管段 | 仅非仪表管（`isinstpipe.not()`）；branmlabel 必须先于 cutparts |
| 8 | `getdmfarrs()` | DMF | |

`this.objects` 最终拼装顺序：ISOLEG（仅非仪表管，来自 `pipeatta.attaobjects`）→ adjustwelds → isombdangles → isoslopes → materials → tags → weldobjects → cutparts → branmlabel。

**注意**：每段 isoline 构造时已自带主尺寸 `ISODIM`（`isoline.getIsoDims()`，见 06 文档），不经过本方法归段。

## 2. 归段主循环（L1153–1298）

对每个 object 求 `possibleIsolineIndices`，分两条路径：

### 2.1 路径 A：无名对象 / 弯头虚拟腿（L1156–1198）

条件：`object.name.unset()`（如切管点）**或** `objecttype eq 'ISOLEG' and elboname.set()`（弯头上的支腿虚拟段）。

- 普通无名对象：对每段，若 `isoline.line.onprojected(object.pos)`（投影落在段线段范围内）→ 候选；记录 `line.distance(object.pos)`。
- 弯头 ISOLEG：先要求腿方向与段 `pipedir` 平行（`angle < 1°`，>90° 先取补角），再要求 `attapos` 投影在段上。
- **决胜：距离最近**的一段（`tempdiss.sortedindices()[1]`）。
- 无候选：打印 `Cant find good isoline …` 并 `skip`（**对象被丢弃**，不再兜底）。

### 2.2 路径 B：有名对象按 mems 查找（L1200–1233）

- 先算 `samedirs`：非 Head/Tail 时取 `adir`/`ldir`，夹角 < 179° 记 `samedirs = false`（**该成员本身是拐点**，可能出现在两段里）。
- 特例：`ISOWELDTEXT` 且宿主是 `ATTA`（elbopad/elboleg 上的焊缝字）→ 不查 mems，改用**投影**收集所有覆盖段。
- 一般：`isoline.mems.findfirst(name)` 命中即候选；`samedirs` 为 true 时找到第一个就 `break`（唯一归属），false 时继续收集（弯头双候选）。

### 2.3 无候选兜底（L1242–1253）

路径 B 也可能空：打印警告并在对象位置放 `aid text '1'`（图面留痕），**不归段**。

### 2.4 多候选决胜（L1255–1277）

- `ISOWELDTEXT`：取 `line.distance(object.pos)` 最小的段（**离焊缝字最近**）。
- 其它类型：取 `abs(pipedir.angle(U) − 90)` 再 `min(angle, 180−angle)` 最小的段 —— **更接近水平的段优先**（材料标/标签放水平段更好读）。

### 2.5 焊缝字 movedis 平移（L1278–1287）

`ISOWELDTEXT` 且 `movedis ≠ 0`：把 `movedir` 与选中段 `polarsystem.showdir` 对齐（夹角 >90° 取反），再 `pos.offset(movedir, movedis)` —— 焊缝字沿视向让位。

### 2.6 插入次序（L1288–1295）

- `ISOSLOPE / ISOADJUSTWELD / ISOMBDANGLE` → `objects.insert(1, object)`（**插队到段首**，注释：坡度需要所有标注给它让步；后插的排更前）。
- 其余（含 `ISOMATERIAL`）→ `append`。

## 3. 与旧实现的差异（L1299–1352，已 `$( … $)` 注释）

旧版按 mems 首命中直接塞入，焊缝字距离 >5mm 才延迟决胜，材料走 `dealaccordingboltdata`（法兰螺栓合并，现已废弃注释）。新版把「候选收集」与「决胜」拆开，弯头双候选/更水平优先都是新版行为。

## 4. 关键阈值

| 阈值 | 含义 |
|---|---|
| `angle < 1°`（腿方向 vs pipedir） | 弯头 ISOLEG 归段方向门槛 |
| `< 179°`（adir vs ldir） | samedirs 判定（是否可能跨两段） |
| `sortedindices()[1]` | 全部决胜都取最小值首位，无并列处理（稳定排序保先出现者） |

## 5. 易错点

1. 归段失败的对象**静默丢弃**（只打印 + aid text），不会进 wronglines。
2. 焊缝字与其它对象的决胜标准不同（距离 vs 水平度），排查落错段先看对象类型。
3. `insert(1)` 使后收集的高优对象反而更靠前：段内顺序 = slope > angle > adjustweld >（其余按收集序）。
4. movedis 平移发生在**归段后**，且方向依赖该段 showdir —— 段选错则平移方向也错。
