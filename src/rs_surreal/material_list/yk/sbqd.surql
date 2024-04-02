let $pe = [pe:24381_107713];

select refno,
refno.NAME as equi_name,
'' as room_code,
fn::get_world_pos($this.id)[0][2] as pos, // 坐标 z
'' as floor_height
from $pe;