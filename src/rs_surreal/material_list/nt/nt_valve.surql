-- 阀门列表
remove function fn::nt_valv;
define function fn::nt_valv($refnos: array<record>) {
    select 
        id,
        fn::default_name(id),
        fn::room_code(id)[0]?:'unset' as room_code,
        (->pe_owner.out->pe_owner.in.refno.NAME)[0]?:'unset' as bran_name,
        [ refno.DESP[1]?:0, refno.DESP[2]?:0, refno.DESP[5]?:0 ] as valv_size,
        fn::get_valv_material(id) as material,
        if string::slice(name?:'', -3) as vala_use
    from $refnos;
};

-- let $refno = [pe:24383_84088];
-- let $valv_list = fn::nt_valv($refno);
-- return $valv_list;



