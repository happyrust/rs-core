--通过传递refno，查询属性值
select * from only (type::thing("pe", $refno)).refno;
let $dbnos = select value CURD.refno.DBNO from only MDB where NAME="/ALL" limit 1;
select string::concat(string::concat(string::concat(':', if UDNA == none { DYUDNA } else { UDNA }), '::'), UTYP) as u, DFLT as v from UDA where REFNO.dbnum in $dbnos and (type::thing("pe", $refno)).noun in ELEL;
-- uda 单独做个查询？
select string::concat(':', if u.UDNA == none { u.DYUDNA } else { u.UDNA }) as u, v from (type::thing("ATT_UDA", $refno)).udas;
