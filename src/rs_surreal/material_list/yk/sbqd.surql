let $pe = [pe:24381_107713];

select refno,
refno.NAME as equi_name,
fn::room_code($this.id)[0] as room_code,
fn::get_world_pos($this.id)[0][2] as pos, // 坐标 z
(->nearest_relate.dist)[0] as floor_height
from $pe;