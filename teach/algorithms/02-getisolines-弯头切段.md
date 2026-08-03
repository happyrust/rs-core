# 02 · `isobran.getisolines()` — 按直管段切分 BRAN（切段算法）

> 源码：`MBD/markpipe/object/isobran.pmlobj` L1470–1792（方法本体）
> 前置阅读：`01-管道标注主链总览.md`；配套课程：`teach/lessons/0001/0002/0014–0021/0033/0039–0045`
> Rust 教学复刻：`teach/p1_split/src/lib.rs`（split_isolines，7 测）

## 1. 职责

把一条 BRAN 的成员序列（`branMems`，含 `'Head'`/`'Tail'` 哨兵）按**直管段**切开，产出：

| 输出 | 含义 |
|---|---|
| `allarr`（→ `this.isolines[i].mems`） | 每段的成员数组。**弯头同时是前一段的最后一个成员和后一段的第一个成员**（L1610 注释） |
| `poss` | 每段起点坐标（拐点坐标） |
| `ldirs` | 每段起点的朝向（后续构造 isoline 用的 `pipedir`） |
| `this.isolines` | 逐段构造出的 `isoline` 对象数组 |

返回 `false` 表示切段失败（反向 BRAN、坏几何等），错误写入 `this.wronglines`。

## 2. 主循环：四类分支

对 `branMems` 顺序遍历（PDMS `forwards` 模式），当前累积段为 `onearr`：

### 2.1 Head / Tail 哨兵（L1481–1505）

- `'Head'`：开新段，`poss` 收 `hpos of bran`，`ldirs` 收 `hdir of bran`。
- `'Tail'`：把当前 `onearr` 收尾进 `allarr`（Tail 也进当前段）。

### 2.2 直通类型直接跳过（L1512）

`ATTA / TUBI / OLET / WELD` 四类**永不触发切段**，只 append 进当前段后 `skip`。

### 2.3 RJ PCOM 大切段（L1516–1572）

`!!isrjpcom(mem)`（`MBD/function/pipe/isrjpcom.pmlfnc`：`type eq 'PCOM'` 且名字以 `RJ` 结尾）走特殊三段式：

- 先校验 `arrive of mem`，若 `eq 2` → `wronglines.append('此Bran方向建反了')`，整体失败。
- **水平布置**（`abs(p1dir.angle(u) − 90) < 10`，L1527）：
  1. 结束当前段；切点 = 过 `p1pos` 沿 `−adir` 的直线上距 `p9pos` 最近点（`templine.near(p9pos)`）；
  2. 新段以该 PCOM 开头，`ldir = u`（竖直向上）。
- **垂直布置**（else，L1540）：
  1. `ldir = p0pos.direction(p3pos)`；
  2. 若 `p1pos` 距上一个拐点 > 1mm，先在 `p1pos` 处收一段；否则**覆盖**上一个拐点/朝向（`poss[last] = p1pos`）；
  3. 再以 `templine(p1pos, ldir).near(p9pos)` 为切点收掉「PCOM 自身段」，下一段 `ldir = u`。
  - 效果：RJ PCOM 独占一小段（p1→p9 投影段），两侧各自成段。

### 2.4 方向变化（弯头/弯管路径，L1573–1631）

进入条件：`dir.angle(ldir) > 0.01`，其中 `dir = ldir of mem`（成员出口方向）、`ldir` 是当前段朝向。

1. **近 180° 头部误差矫正**（L1582–1591）：若 `adir.angle(memldir) > 179.99` 且 `adir.angle(!ldir) > 179`（本质是直通件但当前段朝向记反了），且 `apos` 与上一拐点重合（<1mm），则把 `ldirs[last]` 改成 `adir.opposite()` 后 `skip` —— **贴拐点，不切开**。
2. **真切开**（L1594–1618）：条件 `apos.distance(lpos) > 1`（进出口不重合，即弯头有实体长度）**或** `dir.angle(!ldir) > 10`（注释指 BEND 情形）：
   - 若 `lpos` 距上一拐点 < 0.1mm → 只更新 `ldirs[last] = ldir of mem`（**贴拐点**：把上一段朝向改成出口方向）；
   - 否则：拐点 = `pos of mem`；`allarr` 收掉当前段；新段以该弯头开头（弯头「双含」）；`ldirs` 收 `dir`。
3. **> 100° 强制切**（L1620–1630，2024-12-31 修补）：若 `adir.angle(memldir) > 100`（进出口方向夹角大，弯管本体拐得很急但上面的条件没触发），无条件再切一次。同一个弯头可能因此产生两次切段。

### 2.5 同向 Z 型特殊件（L1632–1674）

方向没变（`angle ≤ 0.01`）时，若 `!!ispipespecialelement(mem)`（`ispipespecialelement.pmlfnc`：PCOM、非 RJ、`adir` 与 `ldir` 夹角 <0.1° 或 >179.9°、但 `lpos` 不在 `apos+adir` 轴线上 —— **Z 型偏置件**）：

- `*JH` 结尾的 PCOM 用 `p2pos/p3pos` 中点当 `apos`（L1636）；
- 若 `lpos` 偏离轴线 > 0.1mm：在进口轴线最近点切一刀、给特殊件自身独立成段（`nearpos1 → nearpos2`，段向 = 两投影点连线方向），出口侧再开新段。即 **一件切成三段**。

## 3. 视角决策 `getfromltor()`（L1794–1958）

切完 `allarr` 后、构造 isoline 前调用。当前实现 `useFromltor = true` 分支：

1. 收集所有「会拐弯的」成员位置 `poss` 与朝向 `ldirs`（跳过 ATTA/TUBI；`adir.angle(ldir) < 177` 视为拐点；BEND 更新朝向）。
2. 以 BRAN 包围盒中心 `cenpos` 为参照：对每对相邻拐点中点，取 `ldir.orthogonal(d)` 与「中点→中心」方向比较，投票 `znum/fnum`。
3. 票差显著（`abs(znum−fnum) > (znum+fnum)/3`）→ `fromltor = true/false`；否则回退到 `useFromltor = false` 分支：改用 `e45n`/`e45s` 两个 45° 方向找**最长展开方向**，再按「多数段中点朝向中心」定 `branlookdirection`。
4. `fromltor` / `branlookdirection` 最终传入每个 isoline 的 polarsystem，决定 `horidir`（读图方向）。

## 4. isoline 构造循环（L1684–1787）

对每个 `allarr[x]`：

- `prehoridir`：第 1 段为空方向，其余取上一段 `polarsystem.horidir`（**读图方向逐段继承**，防止相邻段翻面）。
- 构造 `object isoline(branname, allarr[x], ldirs[x], minslope, maxslope, considerprenextdir, lookangle, fromltor|branlookdirection, isinstpipe, prehoridir)`。任何一段 `wronglines` 非空 → 整体失败。

### 4.1 邻段障碍注入（`usepolarsystem` 分支，L1712–1781）

- **非 polar 模式**（旧路径）：互加 `isoUsedDir('PreIsoLine'/'NextIsoLine', …, ±od/2)`。
- **polar 模式**（现行）：
  - 把**上一段**的 `obstline`（见 05 文档：管自身障碍线）包成 `polarcyli`（直径 = 上段 od）加进**当前段** polarsystem（`basic=true`）；
  - 把**当前段** obstline 包成 polarcyli 加进**上一段**（高度 >1 才加，规避 cap 上 OLET 的零长段）；
  - 若 polarcyli 直径或高 < 0.01 → 打印 `pre isoline is bad , check bore and length` 并整体失败。
  - **隔段 U 形回折**（`x > 2`，L1752–1780）：`isolines[x−2].pipedir` 与当前段夹角 > 160°（近平行反折）且两段轴线距离 `dis < od×15` 时，把 x−2 段以 **1.5× od** 的加粗 polarcyli 注入当前段、当前段同样注入 x−2 段 —— 这是「弯头附近尺寸压到邻管」问题的主要防线（对照 lesson 0004/0046）。

### 4.2 尺寸车道初始化（L1783–1784）

每段固定 `firstdimtimes = 0.5`、`seconddimtimes = 1.7`（供 isoline 内小尺寸/主尺寸分车道，实际 drawdim 传的是 1 或 2，见 05 文档）。

## 5. 关键阈值速查

| 阈值 | 位置 | 含义 |
|---|---|---|
| `0.01°` | L1573 | 触发「方向变化」分支的最小角 |
| `179.99° / 179°` | L1582 | 近 180° 头部误差矫正 |
| `1mm / 10°` | L1594 | 真切开条件（apos-lpos 距离 / BEND 角度） |
| `0.1mm` | L1601 | lpos 贴上一拐点 → 只改朝向不切 |
| `100°` | L1621 | 强制补切 |
| `10°` | L1527 | RJ PCOM 水平/垂直判定（p1dir 相对 U） |
| `160° / od×15 / 1.5×od` | L1760–1774 | 隔段 U 形回折障碍注入 |
| `0.5 / 1.7` | L1783 | first/seconddimtimes |

## 6. 已知易错点

1. **弯头双含**：数段数时弯头两侧各算一段的成员，中点/长度计算注意去重（lesson 0019/0020 的「公共端 E」）。
2. **贴拐点 vs 真切开**：`lpos` 与上一拐点距离 <0.1mm 时只改 `ldirs[last]`，段数不增。
3. **强制切与真切开可叠加**：同一 BEND 可能先走 2.4-2 再走 2.4-3，产生短段。
4. **RJ PCOM 反向**：`arrive eq 2` 直接判「Bran 方向建反」，整枝失败。
5. `getisolines` **只切段不收对象**；对象收集与归段见 `03-putIntoIsoLine-对象归段.md`。
