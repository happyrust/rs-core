// 法兰长度
DEFINE FUNCTION fn::tape_flan_length($pe:record) {
    let $pre_desp = select value (select value in.refno.DESP from only ->pe_owner->pe<-pe_owner where order_num = array::first($parent->pe_owner.order_num) - 1 limit 1) from $pe;
    let $next_desp = select value (select value in.refno.DESP from only ->pe_owner->pe<-pe_owner where order_num = array::first($parent->pe_owner.order_num) + 1 limit 1) from $pe;
    return
    select if fn::hvac_shape($pe) == 'RECT' {
        string::concat(string::slice(<string>(((refno.DESP[1] + refno.DESP[2] * 2 ) * 2 ) / 1000),0,string::len((<string>(((refno.DESP[1] + refno.DESP[2] * 2 ) * 2 ) / 1000)))-1)
        , ',' ,string::slice(<string>(((refno.DESP[3] + refno.DESP[23] * 2 ) * 2 ) / 1000),0,string::len(<string>(((refno.DESP[3] + refno.DESP[23] * 2 ) * 2 ) / 1000))-1))
    } else if fn::hvac_shape($pe) == 'CIRC' {
        string::concat(string::slice((<string>((refno.DESP[1] + refno.DESP[20] * 2 ) * 3.14 ) / 1000),0,string::len(<string>((refno.DESP[1] + refno.DESP[20] * 2 ) * 3.14 ) / 1000)-1),
        '',string::slice(<string>(((refno.DESP[3] + refno.DESP[23] * 2 ) * 3.14 ) / 1000),0,string::len(<string>((refno.DESP[3] + refno.DESP[23] * 2 ) * 3.14 ) / 1000)-1))
    } else {
        '0,0'
    } as length from $pe
};
//return fn::tape_flan_length(pe:24381_76894);