// 仪控 仪表管道
select id,
name ,
fn::yk_pipe_num($this.id) as pipe_name,
'' as room_code
from pe:24381_147880