select refno,string::slice(array::at(refno.REFNO->pe_owner.out->pe_owner.out->pe_owner.out.name,0),1,1) as num, // 机组号
string::slice(string::split(array::at(refno.REFNO->pe_owner.out->pe_owner.out->pe_owner.out.name,0),'-')[0],2) as project_num, //子项号
if string::contains(array::at(refno.REFNO->pe_owner.out->pe_owner.out.name,0),'MCT') || string::contains(array::at(refno.REFNO->pe_owner.out->pe_owner.out.name,0),'MSUP')  { '主托盘' } else { '次托盘' } as major,//专业
'' as room_code,
string::slice(refno.NAME,1) as name,// 托盘段号
refno.HPOS[2] as pos, // 托盘标高
//<-pe_owner[where in.noun!='ATTA'] order by order_num,

if string::contains(string::uppercase((select value in from only (select * from <-pe_owner[where in.noun != 'ATTA'] order by order_num) limit 1).refno.SPRE.refno.NAME),'LADDER') { '梯架' } else if string::contains(string::uppercase((select value in from only (select * from <-pe_owner[where in.noun != 'ATTA'] order by order_num) limit 1).refno.SPRE.refno.NAME),'TROUGH') { '实底托盘' } else if string::contains(string::uppercase((select value in from only (select * from <-pe_owner[where in.noun != 'ATTA'] order by order_num) limit 1).refno.SPRE.refno.NAME),'VENTILATED') { '带孔托盘' } else if string::contains(string::uppercase((select value in from only (select * from <-pe_owner[where in.noun != 'ATTA'] order by order_num) limit 1).refno.SPRE.refno.NAME),'RISER') { '竖梯' } else if string::contains(string::uppercase((select value in from only (select * from <-pe_owner[where in.noun != 'ATTA'] order by order_num) limit 1).refno.SPRE.refno.NAME),'DIVIDER') { '分隔板' } as children, // 托盘类型
'碳钢Q235' as material, // 材质
array::first(array::first((select value (select value v from only udas.* where u.NAME=='/BranWidth' limit 1) from type::thing("ATT_UDA", meta::id(id))))) as width, // 托盘宽度
array::first(array::first((select value (select value v from only udas.* where u.NAME=='/BranHigh' limit 1) from type::thing("ATT_UDA", meta::id(id))))) as height, // 托盘高度
array::first(array::first((select value (select value v from only udas.* where u.NAME=='/BranWidth' limit 1) from type::thing("ATT_UDA", meta::id(id))))) * array::first(array::first((select value (select value v from only udas.* where u.NAME=='/BranHigh' limit 1) from type::thing("ATT_UDA", meta::id(id))))) as size_num, // 规格型号
'Y' as b_painting, // 是否刷漆
string::split(refno.NAME,'-')[1] as painting_color,
(select value in from only (select * from <-pe_owner[where in.noun != 'ATTA'] order by order_num) limit 1).refno.DESP as b_cover, //有无盖板 todo
if refno.DESC == NONE { '无' } else { '是' } as b_partition, // 有无隔板
refno.DESC as partition_num,
refno.SPRE.refno.NAME as spre,
if string::contains(string::uppercase((select value in from only (select * from <-pe_owner[where in.noun != 'ATTA'] order by order_num) limit 1).refno.SPRE.refno.NAME),'RISER') { '竖向' } else if string::contains(string::uppercase((select value in from only (select * from <-pe_owner[where in.noun != 'ATTA'] order by order_num) limit 1).refno.SPRE.refno.NAME),'DIVIDER') { '竖向' } else { '水平' } as horizontal_or_vertical,
from pe:24383_84088