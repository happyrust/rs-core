
select id,
fn::default_name($this.id),
fn::room_code($this.id)[0] as room_code,
(->pe_owner.out->pe_owner.in.refno.NAME)[0] as bran_name,
[if refno.DESP[1] == NONE { 0 } else { refno.DESP[1] },if refno.DESP[2] == NONE { 0 } else { refno.DESP[2] },if refno.DESP[5] == NONE { 0 } else { refno.DESP[5] }] as valv_size,
fn::get_valv_material($this.id) as material,
if name == NONE { '' } else { string::slice(name,-3) } as vala_use
from [pe:24381_76849]