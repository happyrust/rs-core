// 加强筋规格
DEFINE FUNCTION fn::hvac_size($pe:record,$size:string,$pressure:string) {
    return
    select if $size =='RECT' && $pressure == 'L' && math::max([refno.DESP[1],refno.DESP[2]]) <= 800 { '-' } else if
    $size =='RECT' && $pressure == 'L' &&
    math::max([refno.DESP[1],refno.DESP[2]]) > 800 && math::max([refno.DESP[1],refno.DESP[2]]) <= 2000 && array::at(string::split(owner.name,'-'),3) != '6' { 'L40x4'} else if $size =='RECT' && $pressure == 'L' &&
    math::max([refno.DESP[1],refno.DESP[2]]) > 2000 && array::at(string::split(owner.name,'-'),3) != '6' { 'L50x5' }

    else if $size =='RECT' && $pressure == 'M' && math::max([refno.DESP[1],refno.DESP[2]]) <= 800 { '-' }
    else if $size =='RECT' && $pressure == 'M' && math::max([refno.DESP[1],refno.DESP[2]]) > 800 && math::max([refno.DESP[1],refno.DESP[2]]) <= 2000 && array::at(string::split(owner.name,'-'),3) != '6' { 'L40x4'}
    else if $size =='RECT' && $pressure == 'M' &&
    math::max([refno.DESP[1],refno.DESP[2]]) > 2000 { 'L50x5' }

    else if $size =='RECT' && $pressure == 'H' && math::max([refno.DESP[1],refno.DESP[2]]) <= 600 { '-' }
    else if $size =='RECT' && $pressure == 'H' && math::max([refno.DESP[1],refno.DESP[2]]) > 600 && math::max([refno.DESP[1],refno.DESP[2]]) <= 1800 && array::at(string::split(owner.name,'-'),3) != '6' { 'L40x4'}
    else if $size =='RECT' && $pressure == 'H' &&
    math::max([refno.DESP[1],refno.DESP[2]]) > 1800 { 'L50x5' }

    else { '-' } as size from $pe
};