let $refno = [pe:24383_83477];

select id,
fn::default_name($this.id) as name,
fn::room_code($this.id)[0] as room_code,
array::max(<-pe_owner[where in.noun = 'BOX']<-pe->inst_relate.world_trans.d.translation[2]) as pos,
array::max(array::concat(array::concat(<-pe_owner[where in.noun = 'BOX'].in.refno.XLEN,<-pe_owner[where in.noun = 'BOX'].in.refno.YLEN) ,
 <-pe_owner[where in.noun = 'BOX'].in.refno.ZLEN)) as length
from $refno