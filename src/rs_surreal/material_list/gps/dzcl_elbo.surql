// 给排水 大宗材料
// elbo

let $refno = [pe:24383_74103];

select refno,
string::split(string::split(if refno.SPRE.name == NONE { "//:" } else { refno.SPRE.name },'/')[2],':')[0] as code, // 编码
refno.TYPE as type ,// 部件
math::fixed(refno.SPRE.refno.CATR.refno.PARA[3],3) as radius // 外径
from $refno;