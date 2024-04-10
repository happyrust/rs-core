--通过传递refno，查询属性值
select fn::default_name(REFNO) as NAME, * from only (type::thing("pe", $refno)).refno;
select string::concat(string::concat(string::concat(':', if UDNA==none || string::len(UDNA)==0 { DYUDNA } else { UDNA }), '::'), UTYP) as u, DFLT as v from UDA where !UHIDE and (type::thing("pe", $refno)).noun in ELEL;
-- uda 单独做个查询？
select if  u.UDNA==none || string::len( u.UDNA)==0 { u.DYUDNA } else { u.UDNA } as u, v from (type::thing("ATT_UDA", $refno)).udas;
