//工艺直段

remove function fn::gy_tubi;
DEFINE FUNCTION fn::gy_tubi($pes: array<record>) {
    return select  
        fn::pe_to_refno(in) as id,
        string::split(
          array::at(string::split((in.refno.HSTU ?: (in.refno.LSTU)).name?:'', '/'), 2) ?: '',
           ':'
        )[0]?:'' as code,
        'TUBI' as noun,
        world_trans.d.scale[2] as length
    from tubi_relate where record::id(id)[0] in $pes;
};

-- let $leave = [pe:24383_68327];
-- return fn::gy_tubi($leave);