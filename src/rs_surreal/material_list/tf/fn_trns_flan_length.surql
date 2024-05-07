// 法兰长度
DEFINE FUNCTION fn::trns_flan_size($pe:record) {
    let $height2 = fn::turn_deap_into_str($pe.refno.DESP[4]);
    let $width = $pe.refno.DESP[1];
    let $width2 = $pe.refno.DESP[3];
    let $height = $pe.refno.DESP[2];
    let $height_str = fn::turn_deap_into_str($height);
    let $fl_width = $pe.refno.DESP[20];
    let $fl_width2 = $pe.refno.DESP[23];

    return if $height2 == 'DIAM' {
        array::join([<string>(($width + $height + $fl_width * 2 ) / 1000),<string>((($width2 + $fl_width2 * 2 ) * 3.14) / 1000)],';')
    } else if $height_str == 'DIAM' {
        array::join([<string>((($width + $fl_width * 2 ) * 3.14) / 1000),<string>((($width2 + $height2 + $fl_width2 * 2 ) * 2) / 1000)],';')
    }
}