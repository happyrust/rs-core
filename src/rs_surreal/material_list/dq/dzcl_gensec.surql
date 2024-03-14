// 电气 托盘及接地
// gensec
select 
refno,
string::slice(array::at(refno.REFNO->pe_owner.out->pe_owner.out.name,0),1,1) as num, // 机组号
string::slice(string::split(array::at(refno.REFNO->pe_owner.out->pe_owner.out->pe_owner.out.name,0),'-')[0],2) as project_num, //子项号
'主托盘接地' as major,//专业
'' as room_code, // 房间号 todo
string::slice(refno.NAME,1) as name, // 托盘段号
math::fixed(refno.POS[2],3) as pos,
'裸铜缆' as material,  //材质
refno.SPRE.refno.NAME as spre,
fn::vec3_distance(array::clump(<-pe_owner.in<-pe_owner[where in.noun='POINSP'].in.refno.POS,3)[0],array::clump(<-pe_owner.in<-pe_owner[where in.noun='POINSP'].in.refno.POS,3)[1]) as length
from pe:25688_76336
