//风管壁厚
DEFINE FUNCTION fn::bend_hvac_thick($pe:record) {
    let $pre_desp = select value (select value in.refno.DESP from only ->pe_owner->pe<-pe_owner where order_num = array::first($parent->pe_owner.order_num) - 1 limit 1) from $pe;
    let $bran_4 = array::at(string::split($pe.owner.name,'-'),3);
    let $angle = $pe.refno.DESP[7];
    let $height = $pe.refno.DESP[2];
    return
    select if $angle > 30 && $height > 500 && $bran_4 != '6' {
        $pe.refno.DESP[32]
    } else {
        ''
    } as size from $pe
};


//return fn::bend_hvac_thick(pe:24381_58571)