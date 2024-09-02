-- 设备清单

DEFINE FUNCTION fn::eq_dz($refnos: array<record>) {
    select 
        id,
        fn::default_name(id) as name,
        fn::room_code(id)[0]?:'unset' as room_code,
        array::max(<-pe_owner[where in.noun = 'BOX']<-pe->inst_relate.world_trans.d.translation[2]) as pos,
        array::max(array::concat(array::concat(<-pe_owner[where in.noun = 'BOX'].in.refno.XLEN,<-pe_owner[where in.noun = 'BOX'].in.refno.YLEN) ,
            <-pe_owner[where in.noun = 'BOX'].in.refno.ZLEN)) as length
    from $refnos where in.noun == 'EQUI';   
};

-- let $refno = [pe:24381_57828,pe:24383_76416];
-- let $equip = fn::dzcl($refno);
-- return $equip;

