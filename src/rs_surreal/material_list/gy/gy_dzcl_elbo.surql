select id as id,
string::split(string::split(if SPRE.name == NONE { "//:" } else { SPRE.name },'/')[2],':')[0] as code, // 编码
TYPE as type ,// 部件
DESP[1] as count // 数量
from ELBO