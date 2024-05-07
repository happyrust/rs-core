// 给排水 大宗材料
// tee

let $refno = [pe:24383_74117];

select refno,
string::split(string::split(if refno.SPRE.name == NONE { "//:" } else { refno.SPRE.name },'/')[2],':')[0] as code, // 编码
refno.TYPE as type, // 部件
string::replace(array::join([<string>math::fixed(if refno.SPRE.refno.CATR.refno.PARA[6] != NONE { refno.SPRE.refno.CATR.refno.PARA[6] } else { 0 },3),<string>math::fixed(if refno.SPRE.refno.CATR.refno.PARA[7] != NONE { refno.SPRE.refno.CATR.refno.PARA[7] } else { 0 },3)],';'),'f','') as radius // 外径
from $refno;