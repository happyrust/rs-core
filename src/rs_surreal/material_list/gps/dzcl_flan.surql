// 给排水 大宗材料
// flan

let $refno = [pe:24383_67888];

select refno,
string::split(string::split(if refno.SPRE.name == NONE { "//:" } else { refno.SPRE.name },'/')[2],':')[0] as code, // 编码
refno.TYPE as type ,// 部件
string::replace(<string>math::fixed(if refno.SPRE.refno.CATR.refno.PARA[6] == NONE { 0 } else { refno.SPRE.refno.CATR.refno.PARA[6] },3),'f','') as radius, // 外径
string::replace(<string>math::fixed(if refno.SPRE.refno.CATR.refno.PARA[4] == NONE { 0 } else { refno.SPRE.refno.CATR.refno.PARA[4] },3),'f','') as thick // 厚度
from $refno;