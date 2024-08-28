
select
id as refno, // 参考号
string::slice(owner.name, string::len(owner.name) - 4) as bran_seg_code, // 管段编号
if fn::hvac_shape($this.id) == 'RECT' { '方形变截面管' } else if fn::hvac_shape($this.id) == 'CIRC' { '圆形变截面管' } else { '' } as desc,
string::concat(string::split(owner.name,'-')[2],string::split(owner.name,'-')[5],
// array::first(array::first((select value (select value v from only udas.* where u.NAME=='/H-SECTCODE' limit 1) from type::thing("ATT_UDA", record::id(id)))))
) as bran_seg_code_split, // 管段编号
array::first(fn::hvac_material($this.id)).r as mat,// 材质
string::concat(array::at(string::split(owner.name,'-'),4),'P') as pressure_level, // 压力等级
refno.DESP[5] as l, // 长
string::concat(string::slice(<string>refno.DESP[1],0,string::len(<string>refno.DESP[1])-1),',',string::slice(<string>refno.DESP[3],0,string::len(<string>refno.DESP[3])-1)) as w, // 宽
if fn::hvac_shape($this.id) == 'RECT' { string::concat(string::slice(<string>refno.DESP[2],0,string::len(<string>refno.DESP[2])-1),',',string::slice(<string>refno.DESP[4],0,string::len(<string>refno.DESP[4])-1)) } else { '' } as h, // 高
(->inst_relate.world_trans.d.translation[0])[0] as x, // 坐标 x
(->inst_relate.world_trans.d.translation[1])[0] as y, // 坐标 y
(->inst_relate.world_trans.d.translation[2])[0] as z, // 坐标 z
string::slice(<string>refno.DESP[32],0,string::len(<string>refno.DESP[32])-1)  as thickness, // 厚度
string::slice(<string>math::fixed(refno.DESP[80] / 1000000,8),0,string::len(<string>math::fixed(refno.DESP[80] / 1000000,8)) -1 ) as area, // 面积
string::slice(<string>math::fixed(refno.DESP[84],3),0,string::len(<string>math::fixed(refno.DESP[84],3))-1) as weight,  // 总重
'' as steel, // 风管加强 型钢
'0' as hvac_length, // 风管加强 长度
'0' as hvac_weight, //// 风管加强 重量
array::first(fn::strt_flan_size($this.id)).size as flan_steel, // 法兰 型钢
array::first(fn::tape_flan_length($this.id)).length as flan_length,//法兰 长度
string::replace(array::first(fn::tape_flan_weight($this.id)).weight,'f','') as flan_weight, //法兰 总重
fn::tape_gask_size($this.id) as gask_size, //垫片类型
fn::tape_gask_length($this.id) as shim_width, // 垫片 长度
fn::tape_bolt_count($this.id) as bolt_count,  // 螺栓数量
'' as other_material_count, // 其它材料数量
'' as screw, // 螺杆
fn::tape_bolt_count($this.id) as nut, // 螺母
string::replace(string::concat(<string>(<float>string::split(fn::tape_bolt_count($this.id),',')[0] * 2),',',<string>(<float>string::split(fn::tape_bolt_count($this.id),',')[1] * 2)),'f','') as nut_count, // 螺母数量
'' as room_num , // 房间号 todo
array::at(string::split(owner.name,'-'),1) as system_name, // 系统号
'' as material, // 材质 todo
'' as hvac_thick // 风管壁厚
from [pe:24381_76894];

