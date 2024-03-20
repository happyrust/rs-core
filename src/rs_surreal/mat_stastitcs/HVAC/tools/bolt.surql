
DEFINE FUNCTION fn::hvac_bolt_qty($pe: record) {
    let $pd = fn::pre_is_type($pe, "DAMP");
    let $nd = fn::next_is_type($pe, "DAMP");
    let $pm = fn::pre_is_type($pe, "MESH");
    let $nm = fn::pre_is_type($pe, "MESH");
    let $w = fn::hvac_width($pe);
    let $h = fn::hvac_height($pe);
    let $flw = fn::hvac_flw($pe);
    //string((int(([Width] + [FL_Width] * 2 + 75) / 150) + int(([Height] + [FL_Width] * 2 + 75) / 150) - 2) * 4 , 'D0')
    let $v1 = ( <int>(($w + $flw * 2 + 75) / 150) + <int>(($h + $flw * 2 + 75) / 150 ) - 2);
    if $pd
};