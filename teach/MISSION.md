# Mission: PML `getisolines` 弯头切段 → 对齐 Rust V2

## Why
要在 `BranchCalculator` / `extract_isolines` 里复现 PDMS 的直段切分语义。切错一段，后面 Polar 障碍、尺寸方向、ELBO 归属都会一起歪。学习目标不是「读过 PML」，而是**能对照源码判定 V2 切段对不对**。

## Success looks like
- 给定一条含 ELBO/BEND 的 `branMems` 序列，能手推 `allarr` / `poss` / `ldirs` 的切分结果
- 能指出 V2「按方向角阈值切」与 PML「方向变化 + apos/lpos 几何」的差异点
- 能对一条失败样例写出 suppress/`wronglines` 级原因，而不是猜

## Constraints
- 短课 + 即时小测；一次只钉一个概念
- 知识以仓库内 PML 源码与 `MBD/开发文档` 为权威，不靠记忆猜算法
- 教学材料放在 `teach/`，不改业务代码

## Out of scope
- 暂时不讲 `lindim` 小尺寸、标签避让、前端渲染
- 暂不改 Rust 实现（先建立判定能力，再动手改）
