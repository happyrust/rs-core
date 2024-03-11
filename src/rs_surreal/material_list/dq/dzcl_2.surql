select refno,string::slice(array::at(refno.REFNO->pe_owner.out->pe_owner.out.name,0),1,1) as num, // 机组号
string::slice(string::split(array::at(refno.REFNO->pe_owner.out->pe_owner.out->pe_owner.out.name,0),'-')[0],2) as project_num, //子项号
'支吊架' as major,//专业
'' as room_code, // 房间号
'' as children, // 托盘类型
string::slice(refno.NAME,1) as supp_name, // 托盘支吊架名称
'' as standard_num, // 标准号 todo
'' as item_num, // 物项编号
'碳钢Q355' as material,  //材质
'' as width,
'' as height,
array::last(string::split(array::first(<-pe_owner.in<-pe_owner[where in.noun='SCTN'].in.refno.SPRE.name),'-')) as size_num // 规格型号
from pe:24383_86099