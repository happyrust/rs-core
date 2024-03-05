
let $mat_code_map = ['SS', 'GS'	,'CS','ACS','ASS',	'AS', 'FP'];

select
refno.id as refno, // 参考号
//owner.name,
string::slice(owner.name, string::len(owner.name) - 4) as bran_seg_code, // 管段编号
string::concat(string::split(owner.name,'-')[2],string::split(owner.name,'-')[5],array::first(array::first((select value (select value v from only udas.* where u.NAME=='/H-SECTCODE' limit 1) from type::thing("ATT_UDA", meta::id(id)))))) as bran_seg_code_split,
array::at($mat_code_map, <int>string::slice(string::replace(owner.name,'-',''), 5, 1)) as mat, // 材质
string::concat(string::slice(string::replace(owner.name,'-',''), 6, 1), "P") as strt_name, // 压力等级
refno.DESP[5] as l, // 长
refno.DESP[1] as w, // 宽
refno.DESP[3] as h, // 高
(->inst_relate.world_trans.d.translation[0])[0] as x, // 坐标 x
(->inst_relate.world_trans.d.translation[1])[0] as y, // 坐标 y
(->inst_relate.world_trans.d.translation[2])[0] as z, // 坐标 z
math::max([refno.DESP[2],refno.DESP[3]]) as thickness, // 厚度 todo
(refno.DESP[1] + refno.DESP[2]) * 2 * refno.DESP[5] / 1000000 as area, // 面积
math::max([refno.DESP[2],refno.DESP[3]]) * (refno.DESP[1] + refno.DESP[2]) * 2 * refno.DESP[5] / 1000000 * 7.85 as total_weight,  // 总重 todo
math::max([refno.DESP[2],refno.DESP[3]]) as steel, // 风管加强 型钢 todo
math::max([refno.DESP[2],refno.DESP[3]]) as flan_steel, // 法兰 型钢 todo
(refno.DESP[1] + refno.DESP[2]+refno.DESP[3]) *4 as flan_width, // 法兰 长度 todo
(refno.DESP[1] + refno.DESP[2]+refno.DESP[3]) *4 as shim_width // 垫片 长度 todo
from pe:24381_58343
