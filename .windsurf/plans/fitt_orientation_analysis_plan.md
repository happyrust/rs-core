# FITT 方位计算定位计划
本计划旨在通过 ida-pro-mcp 追踪 FITT 的 ORI/POS 计算链路，定位算法依据并整理可复用的方位计算步骤。

## 1. 调用链定位（ORI/POS 获取入口）
- 从 DB_PseudoAttPlugger::getOriAtt/getPosAtt 追踪到 runFortranPlugger。
- 确认 sub_100B08CA -> sub_103A2294 的调度路径与输入参数。

## 2. Fortran 调度表与算法入口
- 解析 dword_11B29500/dword_11B29640 调度表结构。
- 通过 GFITMD 等标识定位对应的算法函数地址与索引。
- 用 xrefs/bytes 确认 GFITMD 与具体 FORTRAN 例程的绑定关系。

## 3. 参数来源与坐标系
- 追踪 ORI/POS 的 WRT（相对坐标系）处理逻辑。
- 继续定位 DELP/ZDIS/POSL 等参数读取与变换关系。
- 识别与 STWALL 交互的关键参数（法线、墙厚、中心线等）。

## 4. 输出算法依据与指导
- 汇总 ORI 计算所用输入（欧拉角、法向、偏移、坐标系）。
- 产出 FITT 方位计算的步骤说明与可验证公式。
- 标注关键函数地址与调用顺序供复查。
