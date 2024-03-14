
// tubi
// let $refno = [pe:24383_67444];

select
leave as id,
(select value ( if leave.refno.LSTU.refno.NAME != NONE { string::split(array::at(string::split(leave.refno.LSTU.name, '/'), 2), ':')[0] } else if leave.refno.HSTU.refno.NAME != NONE {
  string::split(array::at(string::split(leave.refno.HSTU.name, '/'), 2), ':')[0]
} else { '' }  ) from $self)  as code,
'TUBI' as type,
world_trans.d.scale[2] as length from $refno->tubi_relate;

