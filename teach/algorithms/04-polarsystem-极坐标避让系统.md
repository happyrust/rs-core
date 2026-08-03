# 04 · `polarsystem` — 极坐标空间占用与避让系统

> 源码：`MBD/object/polarsystem/polarsystem.pmlobj`（3.1k 行）
> 元素转换：`MBD/object/polarsystem/getpolarelement.pmlobj`；容器：`polarelement.pmlobj` / `polarcyli.pmlobj` / `polarbox.pmlobj`
> 配套课程：`lessons/0004/0046/0061/0067`；Rust 教学复刻：`teach/p1_split/src/polar_neighbors.rs`

## 1. 模型

每段 isoline 建一个 polarsystem，以**管段轴线为极轴**的圆柱坐标系。任何已占空间/待放标注都归一化成三元组区间（文件头注释 L20–28）：

| 维度 | 含义 |
|---|---|
| `dis` | 沿极轴方向、相对 `pos`（段起点）的距离区间 `[startdis, enddis]` |
| `angle` | 绕轴角度区间 `[startangle, endangle]`（以 `this.ori` 为参考，0–360） |
| `radius` | 离轴半径区间 `[startradius, endradius]` |

成员：`pos/dir/ori`、`basicRadius`（= od/2）、`limitMinDis/limitMaxDis`（= 段长）、`horidir/showdir/mainDimDir`、`elements`（polarelement 数组）+ `objects`（源对象）、`ignoreline`、`referencedirs/lookangle/fromltor/branlookdirection/prehoridir/isinstpipe`。

## 2. 视角确定 `getDetail()`（L151–345）

目标：算出 `horidir`（水平读图方向）→ `showdir`（最佳观察反方向）→ `ori`。

1. `horidir` 初值 = `dir.orthogonal(u)`；管子竖直时（orthogonal 失败）依次回退：`referencedirs` 里第一个与 U 夹角在 30°–150° 的方向的 `orthogonal(u)` → 包围盒中心法 → `n`。
2. **prehoridir 继承**（L227–238）：与上一段 horidir 夹角 <45° 直接沿用判向；>135° 取反 —— 保证相邻段读图方向连续（跳过 fromltor 覆写）。
3. 否则按 `branlookdirection`（夹角 <90° 取反）或 `fromltor`（false 取反）统一整枝方向。
4. `showdir`：管段非竖直（`dir.angle(u)` ∈ (10°,170°)）时，在 `x is horidir, z is ±dir` 的圆上取 `lookangle` 度方向（表单默认仰视角）；竖直段直接用 horidir。
5. `ori = x is showdir and z is dir`。**showdir 的反方向即人看图方向**。

`getMainDimDir()`（L347）：用一次 `getBestPosAndOri(needs=[段长,30°,basicRadius], bestdiss=[段中点])` 的结果反推主尺寸方向 `maindimdir`。

## 3. 元素登记 `add(item[, basic])`（L400–433）

`basic=true` 表示「管道自身/根基障碍」（重绘时不清除；`isobran.draw()` L3025 只删 `basic.not()` 的元素）。

`getpolarelement` 把任意对象转为若干 polarelement（`getpolarelement.pmlobj` 按类型分派）：

| deal 方法 | 输入 | 备注 |
|---|---|---|
| `dealpoint / dealline / dealarc` | 几何 | 线/弧离散化为 `getPointsData`（L223：角度必须在 x–x+180 内） |
| `dealpolarcyli`（L827） | 圆柱障碍 | 邻段管体、TEE 支管、阀体等 |
| `dealpolarbox`（L1589） | 盒状障碍 | VALV 等大件 |
| `dealstring`（L2199） | 文本 | 用 `mbdtextlen` 估宽 |
| `deallindim`（L2334） | 尺寸 | 已画尺寸线 |
| `dealmlabel / dealaidtex / dealaidlin / dealaidpoi / dealaidcir / dealaidarc` | 已画标注 | 绘制后回加，见 05 |

**修剪阈值**（`add`，L418–423）：`startdis > limitMaxDis + 2·basicRadius` 或 `enddis < limitMinDis − 2·basicRadius` 丢弃；`startradius > 15·basicRadius`（工艺管）/ `50·basicRadius`(仪表管) 丢弃。

## 4. 搜索算法 `getBestPosAndOri`（主入口 L1028–1295）

签名（文件头 L31–47 有五种便捷重载）：

```
getBestPosAndOri(disranges, angleranges, radiusranges, needs, bestdiss, bestdirs, bestradiuss, isdim, leadline) → [pos, ori, …]
needs = [需要的轴向长度, 需要的角度(默认30°), 需要的径向厚度(≈cheight)]
isdim  = true 时限制方向使所有尺寸方向一致
leadline = 是否带引线（影响碰撞判定与权重）
```

源码内嵌 Step 注释（L1036–1293），流程：

1. **Step 1** `getnormaldirs()`：常用摆放方向集合。
2. **Step 2** 轴向候选区间：无输入时 `getdisrange(bestdiss, needs[1])`；`splitrange` 切成单元（优先含 bestdis 的单元，其次取中间）。
3. **Step 3** `getanglerange`：现默认 0–360 全开（注释：现在没什么用）。
4. **Step 4** 径向输入区间 `splitrange`。
5. **Step 5** 逐轴向单元 `balance()`：取该单元内障碍（`getdiseles`，径向 20r/50r 内），统计已占角度、给出平衡后的可用角度区间/推荐方向/障碍集/障碍半径集。
6. **Step 6–7** 径向候选：优先用输入区间；再对每个障碍半径 `r` 生成 `[r + needs[3]/10, r + 1.1·needs[3]]` 的「贴外沿」候选（**从内往外一圈圈找空**）。
7. **Step 8–9** 逐 (dis, radius) 单元把障碍分成两组：占用组（径向重叠，容差 0.01）与**引线遮挡组**（`leadline` 时：元素在候选半径以内、伸出 basicRadius 以外 → 引线会穿过它）。
8. **Step 10–12** 对每个径向档位 `getDirAndCha()` 找「与推荐方向夹角差最小」的可用方向，`weightedweight()` 计权重（见 §5）；有引线遮挡的档位额外算一次「引线纳入障碍后」的方案（系数 1 vs 1.3 对比）。
9. **Step 13** 全单元取权重最小者 → `(gooddis, goodangle, goodradius)`。
10. **Step 14** 找不到也必须给位置（注释：*标注元素总要放一个地方*）：取第一个单元中值。
11. `getresult()`（L2420）把三元组换算回世界坐标 pos + 文本 ori（含 isdim 时的方向锁定）。

## 5. 权重函数 `weightedweight(anglecha, minradius, havelead, needangle)`（L1298–1418）

`总权 = (anglew + radiusw) × leadCoefficient`，`leadCoefficient = 1.3`（有引线遮挡）否则 1。
`anglecha` 先按 `needangle/30` 归一。

| anglecha(°) | ≤5 | ≤15 | <20 | <30 | <45 | <60 | <80 | ≥80 |
|---|---|---|---|---|---|---|---|---|
| anglew | 0 | 3 | 5 | 8 | 10 | 12 | 15 | 25 |

`rt = minradius / basicRadius`（basicRadius 为 0 时按 200 计）：

| 工艺管 rt | <2 | <3 | <4 | <5 | <7 | <10 | <15 | <20 | <25 | <30 | <40 | ≥40 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| radiusw | 0 | 1 | 3 | 5 | 6 | 7 | 9 | 11 | 12 | 13 | 14 | 50 |

| 仪表管 rt | <4 | <6 | <8 | <10 | <14 | <20 | <30 | <40 | ≥40 |
|---|---|---|---|---|---|---|---|---|---|
| radiusw | 0 | 1 | 3 | 5 | 6 | 7 | 10 | 15 | 50 |

直觉：**方向差 5° 内免费，半径 2 倍管径内免费；引线穿越障碍打 1.3 倍罚**。

## 6. 与管道主链的接线

| 调用点 | 用途 |
|---|---|
| `isoline.getisolineinfo()` | 建系 + `addpipeselftopolarsystem()`（管自身 obstline 圆柱，basic）+ `addrealods()`（承插/法兰实际外径圆柱） |
| `isobran.getisolines()` L1726–1780 | 邻段/隔段 U 形回折障碍（见 02 文档 §4.1） |
| `isobran.draw()` L3087 | 每段画完后 `addtopreornextpolarsystem(x,isoline,'next')`：把本段**非 basic**元素（已画标注）灌给下一段；`x+2` 段夹角 >160° 且距离 < od×15 时也灌 |
| `isoline.drawdim/drawmaterial/…` | 每画一个对象前 `getBestPosAndOri` 找位，画完 `polarsystem.add(创建的 lindim/mlabel/aid)` 回登记 |
| `polarsystem.getgoodradius(isombdangle,cheight)`（L683） | 安装角标注专用径向搜索 |

## 7. 易错点

1. **basic 与重绘**：`isobran.draw()` 重绘前只清非 basic 元素；忘记回加会导致第二遍绘制互相压字。
2. 修剪阈值（§3）意味着**远障碍被忽略**——半径超 15r/50r 的物体不参与避让。
3. `leadline` 默认 true（L1029–1033：unset 视为 true）。
4. Step 14 兜底会把标注硬放进第一个单元 —— 图面重叠时优先怀疑搜索范围（disranges/radius lanes）给错，而不是权重表。
5. `ignoreline = true`（isobran 表单可设）时 `getdiseles` 跳过长度 <1 的线元素。
