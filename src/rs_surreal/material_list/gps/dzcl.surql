select refno,
string::split(string::split(if refno.SPRE.name == NONE { "//:" } else { refno.SPRE.name },'/')[2],':')[0] as code, // 编码
TYPE as type ,// 部件
refno.SPRE.refno.CATR.refno.PARA[3] as radius // 外径
from pe:24384_2967