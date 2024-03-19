let $pe = [pe:24384_24895];

select id,
fn::default_name(refno.REFNO) as equi_name,
string::slice(if refno.CATR.refno.PRTREF.desc == NONE { '/' } else { refno.CATR.refno.PRTREF.desc },1) as ptre_desc, // 设备名称
string::slice(string::split(array::at(refno.REFNO->pe_owner.out->pe_owner.out.name,0),'-')[0],1,3) as belong_factory, // 所属厂房编号
'' as room_code,
(->inst_relate.world_trans.d.translation[0])[0] as x, // 坐标 x
(->inst_relate.world_trans.d.translation[1])[0] as y, // 坐标 y
(->inst_relate.world_trans.d.translation[2])[0] as z, // 坐标 z
string::slice(refno.CATR.refno.PRTREF.refno.NAME,1) as ptre_name
from $pe