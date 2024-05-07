// 法兰长度
DEFINE FUNCTION fn::flex_flan_length($pe:record) {
    return
    select if fn::hvac_shape($pe) == 'RECT' {
        string::replace(<string>(((refno.DESP[1] + refno.DESP[2] + refno.DESP[20] * 2 ) * 4 ) / 1000),'f','')
    } else if fn::hvac_shape($pe) == 'CIRC' {
        string::replace(<string>(((refno.DESP[1] + refno.DESP[20] * 2 ) * 6.28 ) / 1000),'f','')
    } else {
        '0'
    } as length from $pe
};
// return fn::flex_flan_length(pe:24381_88902);