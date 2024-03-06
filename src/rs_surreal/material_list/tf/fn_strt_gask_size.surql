// 垫片类型
DEFINE FUNCTION fn::strt_gask_size($pe:record,$flan_width:float) {
    let $pre_desp = select value (select value in.refno.DESP from only ->pe_owner->pe<-pe_owner where order_num = array::first($parent->pe_owner.order_num) - 1 limit 1) from $pe;
    let $next_desp = select value (select value in.refno.DESP from only ->pe_owner->pe<-pe_owner where order_num = array::first($parent->pe_owner.order_num) + 1 limit 1) from $pe;
    let $bran_4 = array::at(string::split($pe.owner.name,'-'),3);
    let $bran_b_6 = array::at(string::split($pe.owner.name,'-'),3) == '6';
    let $hvac_shape = fn::hvac_shape($pe);
    //return $bran_4;
    return
    select if fn::hvac_shape($pe) == 'RECT' && array::first($pre_desp)[0] == 614823 && array::first($next_desp)[0] == 614823 && !$bran_b_6 {
        <string>(array::first(fn::strt_flan_length($this.id)).length * $flan_width * 8) / 1000
    } else if fn::hvac_shape($pe) == 'CIRC' && array::first($pre_desp)[0] == 614823 && array::first($next_desp)[0] == 614823 && !$bran_b_6 {
        <string>array::first(fn::strt_flan_length($this.id)).length
    } else if array::first($pre_desp)[0] == 614823 && array::first($next_desp)[0] == 614823 && !$bran_b_6 {
        'D型'
    }

    else if fn::hvac_shape($pe) == 'RECT' && array::first($pre_desp)[0] == 614823 && array::first($next_desp)[0] != 614823 && !$bran_b_6 {
        <string>(array::first(fn::strt_flan_length($this.id)).length * $flan_width * 8) / 1000
    } else if fn::hvac_shape($pe) == 'CIRC' && array::first($pre_desp)[0] == 614823 && array::first($next_desp)[0] != 614823 && !$bran_b_6 {
        <string>(array::first(fn::strt_flan_length($this.id)).length) /2
    }

    else if array::first($pre_desp)[0] == 614823 && array::first($next_desp)[0] == 614823 && ($bran_4 == '1'|| $bran_4 == '2'||$bran_4 == '4') {
        'A型'
    } else if array::first($pre_desp)[0] == 614823 && array::first($next_desp)[0] == 614823 && ($bran_4 == '3'|| $bran_4 == '5') {
        'B型'
    } else if array::first($pre_desp)[0] == 614823 && array::first($next_desp)[0] == 614823 &&  $bran_4 == '6' {
        'D型'
    } else {
        ''
    } as size from $pe
};

//return fn::strt_gask_size(pe:24381_58570,0)