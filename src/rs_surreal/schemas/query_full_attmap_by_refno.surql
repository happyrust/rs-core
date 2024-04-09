--通过传递refno，查询属性值
select * from only (type::thing("pe", $refno)).refno;
let $dbnos = select value (select value DBNO from CURD.refno where STYP=1) from only MDB where NAME="/ALL" limit 1;
select string::concat(string::concat(string::concat(':', if string::len(UDNA)==0 { DYUDNA } else { UDNA }), '::'), UTYP) as u, DFLT as v from UDA where REFNO.dbnum in $dbnos and (type::thing("pe", $refno)).noun in ELEL;
-- uda 单独做个查询？
select if string::len(u.UDNA)==0 { u.DYUDNA } else { u.UDNA } as u, v from (type::thing("ATT_UDA", $refno)).udas;
