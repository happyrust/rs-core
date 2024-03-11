// 垫片类型
DEFINE FUNCTION fn::tape_gask_size_pre($pe:record) {
    let $pre_desp = select value (select value in.refno.DESP from only ->pe_owner->pe<-pe_owner where order_num = array::first($parent->pe_owner.order_num) - 1 limit 1) from $pe;
    let $bran_4 = array::at(string::split($pe.owner.name,'-'),3);
    return
    select if array::first($pre_desp)[0] == 614823 {
        'D型'
    }
    else if array::first($pre_desp)[0] != 614823 && ($bran_4 == '1' || $bran_4 == '2' || $bran_4 == '4') {
        'A型'
    } else if array::first($pre_desp)[0] != 614823 && ($bran_4 == '3' || $bran_4 == '5') {
        'B型'
    } else {
        ''
    } as size from $pe
};

DEFINE FUNCTION fn::tape_gask_size_next($pe:record) {
    let $pre_desp = select value (select value in.refno.DESP from only ->pe_owner->pe<-pe_owner where order_num = array::first($parent->pe_owner.order_num) - 1 limit 1) from $pe;
    let $next_desp = select value (select value in.refno.DESP from only ->pe_owner->pe<-pe_owner where order_num = array::first($parent->pe_owner.order_num) + 1 limit 1) from $pe;
    let $bran_4 = array::at(string::split($pe.owner.name,'-'),3);
    return
    select if array::first($pre_desp)[0] != 614823 && array::first($next_desp)[0] == 614823{
        'D型'
    }
    else if array::first($pre_desp)[0] == 614823 && array::first($next_desp)[0] != 614823 && ($bran_4 == '1' || $bran_4 == '2' || $bran_4 == '4') {
        'A型'
    } else if array::first($pre_desp)[0] == 614823 && array::first($next_desp)[0] != 614823 && ($bran_4 == '3' || $bran_4 == '5') {
        'B型'
    } else {
        ''
    } as size from $pe
};

DEFINE FUNCTION fn::tape_gask_size($pe:record) {
    return string::concat(array::first(fn::tape_gask_size_pre($pe)).size,',',array::first(fn::tape_gask_size_next($pe)).size);
};


return fn::tape_gask_size(pe:24381_76894)