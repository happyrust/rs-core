// 仪控 仪表管道
select id,
name ,
fn::find_pipe_bran($this.id)[0][0] as pipe_name,
fn::room_code($this.id)[0] as room_code
from pe:24381_147880