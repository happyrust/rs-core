// 仪控 仪表管道
select id,
name ,
fn::find_pipe_bran($this.id)[0][0] as pipe_name,
'' as room_code
from pe:24381_147880