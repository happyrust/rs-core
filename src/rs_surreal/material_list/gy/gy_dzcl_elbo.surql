// let $refno = [pe:24383_67446,pe:24383_67451]

// elbo
select 
id as id,
string::split(string::split(if refno.SPRE.name == NONE { "//:" } else { refno.SPRE.name },'/')[2],':')[0] as code, // 编码
refno.TYPE as type // 部件
//refno.DESP[1] as count // 数量
from $refno

