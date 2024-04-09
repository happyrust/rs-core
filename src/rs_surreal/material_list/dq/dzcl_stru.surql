select refno,
string::slice(array::at(refno.REFNO->pe_owner.out->pe_owner.out.name,0),1,1) as num, // 机组号
string::slice(string::split(array::at(refno.REFNO->pe_owner.out->pe_owner.out.name,0),'-')[0],2) as project_num, //子项号
array::first(refno.REFNO->pe_owner.out->pe_owner.out.refno.DESC) as project_name, //子项名称
'支吊架' as major,//专业
fn::room_code($this.id)[0] as room_code, // 房间号
string::slice(refno.NAME,1) as supp_name, // 托盘支吊架名称
'碳钢Q355' as material,  //材质
if (<-pe_owner.in<-pe_owner[where in.noun='SCTN'|| in.noun ='GENSEC'].in.refno.SPRE.name)[0] == NONE { '' }
else { array::last(string::split((<-pe_owner.in<-pe_owner[where in.noun='SCTN'|| in.noun ='GENSEC'].in.refno.SPRE.name)[0],'-')) } as size_num, // 规格型号
(<-pe_owner.in<-pe_owner[where in.noun='SCTN'|| in.noun ='GENSEC'].in.refno.SPRE.name)[0] as spre,
(<-pe_owner.in<-pe_owner[where in.noun='SCTN'|| in.noun ='GENSEC'].in.refno.SPRE.refno.CATR.refno.NAME)[0] as catr,
'2' as count // 数量
from pe:25688_76282