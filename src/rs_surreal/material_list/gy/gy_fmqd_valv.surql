select
id,
string::slice(fn::default_name(REFNO),1) as valv_name, // 阀门位号
'' as room_code, // 房间号 todo
string::split(string::slice(array::at(REFNO->pe_owner.out.name,0),1),'-')[0] as valv_belong, // 阀门归属
if SPRE.refno.CATR.refno.PARA[1] == NONE { 0 } else { SPRE.refno.CATR.refno.PARA[1] } * 2 as valv_length, // 阀门长度
if string::slice(SPRE.refno.CATR.name,4,1) == "R" { SPRE.refno.CATR.refno.PARA[14] } else { SPRE.refno.CATR.refno.PARA[10] } as vala_weight, // 阀门重量
if string::slice(SPRE.refno.CATR.name,4,1) == "R" { SPRE.refno.CATR.refno.PARA[11] } else { SPRE.refno.CATR.refno.PARA[7] } as vala_x, // 阀门重心X
if string::slice(SPRE.refno.CATR.name,4,1) == "R" { SPRE.refno.CATR.refno.PARA[12] } else { SPRE.refno.CATR.refno.PARA[8] } as vala_y, // 阀门重心Y
if string::slice(SPRE.refno.CATR.name,4,1) == "R" { SPRE.refno.CATR.refno.PARA[13] } else { SPRE.refno.CATR.refno.PARA[9] } as vala_z, // 阀门重心Z
'' as valv_supp // 阀门支架 todo
from VALV limit 5