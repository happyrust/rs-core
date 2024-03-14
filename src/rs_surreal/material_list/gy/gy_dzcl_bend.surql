// bend
// let $refno = [pe:24383_84092];

select
id as id,
string::split(string::split(if refno.SPRE.name == NONE { "//:" } else { refno.SPRE.name },'/')[2],':')[0] as code, // 编码
refno.TYPE as type, // 部件
math::fixed((refno.ANGL / 360) * 2 *3.1415 * refno.SPRE.refno.CATR.refno.PARA[1],2) as count // 长度
from $refno