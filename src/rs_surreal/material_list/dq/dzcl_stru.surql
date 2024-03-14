select refno,string::slice(array::at(refno.REFNO->pe_owner.out->pe_owner.out.name,0),1,1) as num, // 机组号
string::slice(string::split(array::at(refno.REFNO->pe_owner.out->pe_owner.out->pe_owner.out.name,0),'-')[0],2) as project_num, //子项号
'支吊架' as major,//专业
'' as room_code, // 房间号 todo
string::slice(refno.NAME,1) as supp_name, // 托盘支吊架名称
'碳钢Q355' as material,  //材质
array::last(string::split(array::first(<-pe_owner.in<-pe_owner[where in.noun='SCTN'].in.refno.SPRE.name),'-')) as size_num, // 规格型号
'2' as count // 数量
from pe:24383_86099