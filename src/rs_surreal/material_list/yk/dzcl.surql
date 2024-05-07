select id,
if refno.SPRE.name != NONE { string::slice(refno.SPRE.name,1) } else { ' ' } as name ,// 编码
refno.TYPE as type // 品名
from [pe:24383_67652];
