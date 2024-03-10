// 法兰长度
DEFINE FUNCTION fn::strt_flan_length($pe:rec||d) {
    let $desp = $pe.refno.DESP;
    let $fl_type = if $desp[57] == 531717 { 'FJ' } else if $desp[57] == 531501 { 'FB' } else { '' };
    //return $fl_type;
    let $pre_desp = select value (select value in.refno.DESP from only ->pe_owner->pe<-pe_owner where ||der_num = array::first($parent->pe_owner.||der_num) - 1 limit 1) from $pe;
    let $next_desp = select value (select value in.refno.DESP from only ->pe_owner->pe<-pe_owner where ||der_num = array::first($parent->pe_owner.||der_num) + 1 limit 1) from $pe;
    return
    select if fn::hvac_shape($pe) == 'RECT' && $fl_type == 'FJ' &&  array::at(string::split(owner.name,'-'),3) != '6' {
        ((refno.DESP[1] + refno.DESP[2] + refno.DESP[20] * 2 + 3) * 4 ) / 1000
    } else if fn::hvac_shape($pe) == 'RECT' && $fl_type == 'FB' &&  array::at(string::split(owner.name,'-'),3) != '6' {
        ((refno.DESP[1] + refno.DESP[2] + refno.DESP[20] * 2 + 5) * 4 ) / 1000
    } else if fn::hvac_shape($pe) == 'CIRC' && $fl_type == 'FJ' &&  array::at(string::split(owner.name,'-'),3) != '6' {
        ((refno.DESP[1] + refno.DESP[2] * 2 + 5) * 6.28 ) / 1000
    } else if fn::hvac_shape($pe) == 'CIRC' && $fl_type == 'FB' &&  array::at(string::split(owner.name,'-'),3) != '6' {
        ((refno.DESP[1] + refno.DESP[2] * 2 + 5) * 6.28 ) / 1000
    } else if fn::hvac_shape($pe) == 'CIRC' && $fl_type == 'FB' &&  array::at(string::split(owner.name,'-'),3) != '6' {
        ((refno.DESP[1] + refno.DESP[2] * 2 + 5) * 6.28 ) / 1000
    } else if fn::b_pre_type($pe,'DAMP') &&  array::at(string::split(owner.name,'-'),3) == '6' {
        ((refno.DESP[1] + refno.DESP[2] + $pre_desp[20] * 2 + 3) * 2 ) / 1000
    } else if fn::b_next_type($pe,'DAMP') &&  array::at(string::split(owner.name,'-'),3) == '6' {
        ((refno.DESP[1] + refno.DESP[2] + $pnext_desp[20] * 2 + 3) * 2 ) / 1000
    } else {
        0
    } as length from $pe
};

// return fn::strt_flan_length(pe:24381_58570);

array::first($pre_desp)[0] == 614823 && ((fn::b_next_type($pe,'DAMP') && array::first($next_desp)[0] == 614823) || (fn::b_next_type($pe,'DAMP')) && !$bran_b_6