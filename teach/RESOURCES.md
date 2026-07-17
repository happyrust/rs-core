# getisolines / 弯头切段 Resources

## Knowledge

- [Source: `MBD/markpipe/object/isobran.pmlobj` · `.getisolines()`]
  业务真值。弯头切段、段间 Polar 占位都在这里。Use for: 任何「会不会切开」的判定。
- [Source: `MBD/function/pipe/ispipespecialelement.pmlfnc`]
  Z 型 PCOM 特殊件判定；ELBO/BEND 不走此路径。Use for: 对照「同向分支」里谁才算 special。
- [Source: `MBD/function/pipe/isrjpcom.pmlfnc`]
  RJ PCOM 识别。Use for: 与弯头分支并列的另一条大切段路径。
- [Doc: `MBD/开发文档/BRAN尺寸标注算法全景文档.md` §2.2]
  对 `getisolines` 的综述与 V2 简化点说明。Use for: 建立全景后再回源码。
- [Source: `MBD/markpipe/object/isobran.pmlobj` · `.putIntoIsoLine()`]
  对象归段：路径 A 投影 / 路径 B mems，弯头双候选决胜。Use for: 对象落错段排查。
- [Source: `MBD/markpipe/object/isobran.pmlobj` · `getisolines` 邻段 polarcyli]
  相邻/隔段障碍注入。Use for: 弯头附近尺寸压邻管问题。


## Wisdom (Communities)

- 组内对照：拿 `AvevaMarineSample` 等样例 bran，与 PDMS 实测 isoline 数核对（见 BRAN 文档验收入口）。
- 用户偏好：未要求加入外部社区；优先仓库内源码与样例。

- [Reference: `teach/reference/p0-branch-member-geom.html`]
  方案 β P0：BranchMemberGeom 字段、与现有 BranchMember 映射、缺口与缺省策略。Use for: 写 P1 前对齐契约。
