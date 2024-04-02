let $pe = [pe:24384_24895];

select id,
fn::default_name(owner) as equi_name,
string::slice(if refno.CATR.refno.PRTREF.desc == NONE { '/' } else { refno.CATR.refno.PRTREF.desc },1) as ptre_desc, // 设备名称
string::slice(string::split(array::at(refno.REFNO->pe_owner.out->pe_owner.out.name,0),'-')[0],1,3) as belong_factory, // 所属厂房编号
'' as room_code,
fn::get_world_pos($this.id)[0][0] as x, // 坐标 x
fn::get_world_pos($this.id)[0][1] as y, // 坐标 y
fn::get_world_pos($this.id)[0][2] as z, // 坐标 z
string::slice(refno.CATR.refno.PRTREF.refno.NAME,1) as ptre_name
from $pe