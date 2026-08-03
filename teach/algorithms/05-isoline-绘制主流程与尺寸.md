# 05 · `isoline` — 段级绘制主流程与尺寸标注（drawdim / dimtimes）

> 源码：`MBD/markpipe/object/isoline.pmlobj`（4.9k 行）
> 配套课程：`lessons/0005/0036/0049/0052`；上游：`02-getisolines`，下游：`06-isodim`、`07-lindim`、`08-附属标注`

## 1. 构造与段几何 `getisolineinfo()`（L113–280）

- 段起点 `pos`：Head 用 `hpos`；RJ PCOM 用 p1/p9 投影（同切段规则）；Z 型特殊件投影到 `lpos+ldir` 轴线；普通件用 `pos of mem`。
- `od`：全段成员取 `getod(mem)` 最大值。`getod`（L318–386）回退链：`aod of mem` → first/last tubi 的 aod → first mem → CAP 特判用 `lod` → PCOM 用 abore/lbore → `min(bore×1.1, bore+10)` 估算。
- `line`：`pos` 沿 `pipedir` 到「成员投影最远点」（`maxdis`；为 0 时强制 1mm 防零长线，L252 修补）。
- polar 模式（现行恒 `usepolarsystem = true`，L134）：
  - `considerprenextdir` 开时 `getpreandnextdir()`（L282）取头两个「adir 取反（流向）/ldir 中与 pipedir 更垂直者」当 referencedirs；
  - 构造 polarsystem（fromltor 或 branlookdirection 二选一，prehoridir 继承）；
  - `addpipeselftopolarsystem()`（L652–980）：
    - 收集本段 ATTA 中的**弯头虚拟腿**（p1/p2 夹角 <170 的 atta → pipeatta → elbo ISOLEG），把腿高端点并进 `diss`，得到 **obstline**（管自身障碍线，可能比 line 长）；
    - `polarcyli(obstline 中点, ori, od, 长度)` 以 basic 加入；
    - 逐成员加障碍：TEE/OLET 按 p3（支管圆柱，SW/EUF 承插口加长 od 或 3·od，其它 0.5·od）或 p1/p2（支管在轴线上时）；其余「有手柄」件（p3 偏轴 → VALV/INST/PCOM 等）用 `getpolarbox(mem)` 盒障碍（旧的逐点圆柱算法已废弃，含 VALV 体积缩放 hack）。
  - `addrealods()`（L530–650）：对承插/螺纹/EU 连接（aconn/lconn 匹配 `SW SC EU`）与 FLAN/FBLI，按 `!!getpipemaxdia`（spref 缓存）把**实际承插外径**段加为 basic 圆柱；TEE 另加支管向圆柱。
- `getIsoDims()`（L388–432）：构造主 `isoDim`（`mainDim = true`），首尾点距 <1mm 的段不加尺寸；`getAttaDims(isodim)` 生成附件小尺寸（复制点做 `mainDim=false` 的子尺寸）。
- 非 polar 旧路径：`getIsoUsedDirs()`（L439–528）为带手柄件登记 `isoUsedDir(…, 'Handle', 60°)`。

## 2. 绘制入口 `draw()`（L1676–1763）

1. 开关：`addelenum = false`（零部件序号，弃用）、`drawPforInst = false`。
2. **cheight 公式**（L1689–1691）：

```
tempval  = od − 50
tempbili = cheightbili × (1 − 0.5 × tempval / (|tempval| + 100))
cheight  = int(od × tempbili)
```

   od=50 时不缩放；od 越大字高比越低（防大管字爆炸）。`cheightbili` 来自表单（isobran.draw 逐段下发，L3068）。
3. `changecheightauto()`（L1822–1834）：开关开时对每个 ISODIM 用 `!!getgoodcheight(poss, pipedir, 0.7)`（文本总长 ≤ 0.7×段长的最大字高）压低 cheight，但不低于 `cheight × changecheightautobili`。
4. polar 模式：`polarsystem.getmaindimdir()` + 下发 `ignoreline`。
5. `mergeshowtext()`（L1253）：合并材料/螺栓展示文本（法兰对螺栓合并计数，`mergebolttexttoshowitem`）。
6. `getelbolegdims()`（L1035）：把弯头 ISOLEG 转成 elbo isoDim（见 06 §4）。
7. 逐对象 `drawoneobject(index)` → `drawOneObjectNew`（L4165 分派表）：

| objecttype | 方法 | dimtimes |
|---|---|---|
| ISODIM(mainDim) | `drawdim(object, twoormoredims ? 2 : 1)` | 主尺寸在外圈(2)或单圈(1) |
| ISODIM(子尺寸) | `drawdim(object, 1)` | 附件尺寸贴管(1) |
| ISOSLOPE | `drawslope` | — |
| ISOMATERIAL | `drawmaterial` | — |
| ISOWELDTEXT | `drawweld` | — |
| ISOTAG | `drawtag` | — |
| ISOADJUSTWELD | `drawadjustweld` | — |
| ISOMBDANGLE | `drawangle` | — |
| ISOLEG(普通) | `drawleg` | — |

8. 收尾 `drawteeolet()`（L2593）：TEE/OLET 支管示意线。

`draw(type)`（L1809）：单类型重绘（表单按钮 draw(type) 路径，isobran.deleteType 先删同类）。

## 3. 尺寸绘制 `drawdim(isodim, dimtimes)`（L1836–1962）

1. 轴向范围：`disarr = [suidirdis(polar.pos, pipedir, poss[1]) + 0.1, suidirdis(…, poss.last()) − 0.1]`；跨度 <1mm 直接放弃。
2. **径向车道**（L1868–1873）：

```
radiusarr[1] = object.od + cheight × 1.2 × (dimtimes − 1)
radiusarr[2] = radiusarr[1] + cheight
```

   dimtimes=1 贴管（od 起），dimtimes=2 在外一圈（od + 1.2·cheight 起）—— 这就是「车道」。
3. `needs = [跨度, 30°, cheight]`；`bestdiss/bestdirs/bestradiuss` 留空（尺寸方向统一交给 isdim=true 处理）。
4. 非「双尺寸的外圈」时先 `dimOneMemSlope(poss[1], poss.last())`（斜段单件坡度处理）。
5. `polarsystem.getBestPosAndOri(…, isdim=true, leadline=true)` → `dimpos/textori`。
6. 构造 lindim（07 文档）：`temparr = [changeCheightAuto, changeCheightAutobili, sepSmallDim, plusstringcheightbili]`；
   - 普通尺寸传 `this.objects`（lindim 从中摘 ISOADJUSTWELD 做 +N 附加长度）；
   - elbo isoleg 尺寸（`maxslope.unset()`）传 fake ISOADJUSTWELD（`elboisolegadlongth`）。
7. `polarsystem.add(lindim.createNames)` 把画出的尺寸回登记为障碍。
8. `lindim.plus` 为真 → `plusAdjustLenInDim(lindim,'调整焊附加长度')`（L2070：在对应分段数字两侧补 `+N` 说明）。
9. 非外圈 & 非仪表管：`drawflow(dimpos, poss, cheight, textori)`（L2153：流向箭头）+ `tspec eq 'Y'` 时 `drawtspec`（L1964：伴热标记）。

**twoormoredims**：isobran 依据段内对象密度设定；主尺寸走外圈(2)、子尺寸走内圈(1)，对照 lesson 0036 的「车道」。

## 4. 方向类辅助

- `sethorishowdir(ps)`（L2526）：材料标注用的水平视向微调。
- `getbestdir(item)`（L3653）/`getdisarr(textlen,bestdis,item)`（L3589）：标签/焊缝字的优选方向与轴向候选。
- `tagori(dimdir)`（L4879）：由尺寸方向导出标签文本方位。
- `getusedDirs(mindis,maxdis)`（L4891）/`getminMax(object)`（L4914）：非 polar 旧路径的方向占用查询。
- 非 polar 旧路径分派：`drawOneObject`（L4210–4730）配合 `!!isoGetHandleDimDir/!!isoGetDimDir/!!isoGetBestDir`（`markpipe/function/`）从手柄方向推尺寸方向；现行 polar 模式不走。

## 5. 关键阈值

| 值 | 位置 | 含义 |
|---|---|---|
| `od−50 / |…|+100` | L1689 | cheight 缩放核 |
| `0.7` | L1827 | changeCheightAuto 文本占比 |
| `±0.1mm` | L1840 | 尺寸轴向内缩 |
| `1.2 × cheight` | L1869 | 车道间距系数 |
| `30°` | needs[2] | 默认所需角空间 |
| `0.5 / 1.7` | isobran L1783 | first/seconddimtimes（isoline 成员，当前 drawdim 实际传 1/2） |

## 6. 易错点

1. `cheight` 是**段级**属性，主尺寸 changeCheightAuto 压低后全段共用（L4294 注释：only main dim decide the cheight of ISOline）。
2. `drawdim` 的 disarr 基于 `polarsystem.pos`（= 段起点），与 isodim.poss 首尾并不总一致（RJ/特殊件投影差）。
3. 画完必须 `polarsystem.add(createNames)`，否则后画对象不知道尺寸占位 —— 排查压字先确认回登记链。
4. elbo isoleg 尺寸靠 `maxslope.unset()` 区分，构造顺序错了会把普通尺寸当腿尺寸。
