let $mat_code_map = ['','SS', 'GS'	,'CS','ACS','ASS',	'AS', 'FP'];
// 材质
DEFINE FUNCTION fn::hvac_material($pe:record) {
    return
    select if array::at(string::split(owner.name,'-'),3) != '6' {
        array::at($mat_code_map,<int>array::at(string::split(owner.name,'-'),3))
    } else if fn::pre_or_next_b_type($this.id,'DAMP') { 'CS' } else {
        'FP'
    } as r from $pe
};

select
id as refno, // 参考号
//owner.name,
string::slice(owner.name, string::len(owner.name) - 4) as bran_seg_code, // 管段编号
string::concat(string::split(owner.name,'-')[2],string::split(owner.name,'-')[5],
// array::first(array::first((select value (select value v from only udas.* where u.NAME=='/H-SECTCODE' limit 1) from type::thing("ATT_UDA", record::id(id)))))
) as bran_seg_code_split, // 管段编号
array::first(fn::hvac_material($this.id)).r as mat,// 材质
string::concat(array::at(string::split(owner.name,'-'),4),'P') as pressure_level, // 压力等级
refno.DESP[5] as l, // 长
if fn::hvac_shape($this.id) == 'RECT' { refno.DESP[1] } else if fn::hvac_shape($this.id) == 'CIRC' { string::concat('φ',string::slice(<string>refno.DESP[1],0,string::len(<string>refno.DESP[1])-1)) } else { refno.DESP[1] } as w, // 宽
if fn::hvac_shape($this.id) == 'RECT' { refno.DESP[3] } else if fn::hvac_shape($this.id) == 'CIRC' { '' } else { refno.DESP[3] } as h, // 高
(->inst_relate.world_trans.d.translation[0])[0] as x, // 坐标 x
(->inst_relate.world_trans.d.translation[1])[0] as y, // 坐标 y
(->inst_relate.world_trans.d.translation[2])[0] as z, // 坐标 z
if array::first(fn::pre_or_next_b_type(refno,'DAMP')) { 3 } else { refno.DESP[32] } as thickness, // 厚度
if array::first(fn::pre_or_next_b_type(refno,'DAMP')) { refno.DESP[80] / 1000000 } else { refno.DESP[1] * refno.DESP[2] / 1000000 * 150 } as area, // 面积
if array::first(fn::pre_or_next_b_type(refno,'DAMP')) { (refno.DESP[1] * refno.DESP[2] * 150 * (if array::first(fn::pre_or_next_b_type(refno,'DAMP')) { 3 } else { refno.DESP[32] }) * 7.85) / 1000000 } else { refno.DESP[84] } as weight,  // 总重
array::first(fn::hvac_size($this.id,fn::hvac_shape($this.id),array::at(string::split(owner.name,'-'),4))).size as steel, // 风管加强 型钢
array::first(fn::hvac_length($this.id,fn::hvac_shape($this.id),array::at(string::split(owner.name,'-'),4),refno.DESP[5],refno.DESP[1],refno.DESP[2])) as hvac_length, // 风管加强 长度
array::first(fn::hvac_weight($this.id,fn::hvac_shape($this.id),array::at(string::split(owner.name,'-'),4),refno.DESP[5],refno.DESP[1],refno.DESP[2])) as hvac_weight, //// 风管加强 重量
array::first(fn::strt_flan_size($this.id)).size as flan_steel, // 法兰 型钢
array::first(fn::strt_flan_length($this.id)).length as flan_length,//法兰 长度
array::first(fn::strt_flan_weight($this.id)).weight as flan_weight, //法兰 总重
array::first(fn::strt_gask_size($this.id)).size as gask_size, //垫片类型
array::first(fn::strt_gask_length($this.id,refno.DESP[20])).size as shim_width, // 垫片 长度
array::first(fn::strt_bolt_count($this.id,refno.DESP[20])).count as bolt_count,  // 螺栓数量
'' as other_material_count, // 其它材料数量
'' as screw, // 螺杆
array::first(fn::strt_bolt_count($this.id,refno.DESP[20])).count as nut_count_a, // 螺母数量
array::first(fn::strt_bolt_count($this.id,refno.DESP[20])).count * 2 as nut_count_b, // 螺母数量
'' as room_num , // 房间号 todo
array::at(string::split(owner.name,'-'),1) as system_name, // 系统号
'' as material, // 材质 todo
'' as hvac_thick, // 风管壁厚
refno.DESP[80] / 2 as hvac_area // 风管面积
from [pe:24381_58343,pe:24381_76693];

