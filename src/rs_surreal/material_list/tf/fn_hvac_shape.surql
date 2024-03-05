// 圆管与方形风管
DEFINE FUNCTION fn::hvac_shape($pe:record) {
    return
    if refno.DESP[29] == 927441 && refno.DESP[39] == 927441 {'RECT'} else if refno.DESP[29] == 603858 && refno.DESP[39] == 603858 { 'CIRC' } else { '-' }
}