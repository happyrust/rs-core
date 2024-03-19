let $pe = [pe:24381_107713];

select refno,
refno.NAME as equi_name,
'' as room_code,
(->inst_relate.world_trans.d.translation[2])[0] as pos,
'' as floor_height
from $pe;