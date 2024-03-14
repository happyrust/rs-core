// 给排水 大宗材料
// bend

let $refno = [pe:24384_2967];

select refno,
string::split(string::split(if refno.SPRE.name == NONE { "//:" } else { refno.SPRE.name },'/')[2],':')[0] as code, // 编码
refno.TYPE as type ,// 部件
math::fixed(refno.SPRE.refno.CATR.refno.PARA[3],3) as radius, // 外径
math::fixed((refno.ANGL / 360) * 2 *3.1415 * refno.SPRE.refno.CATR.refno.PARA[1],2) as count // 数量
from $refno;