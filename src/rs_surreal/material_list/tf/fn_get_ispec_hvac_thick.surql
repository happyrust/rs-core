// 风管壁厚 2
DEFINE FUNCTION fn::get_ispec_hvac_thick($pe:record) {
    let $ipara = array::first(fn::get_ipara($pe));
    let $bran_4 = array::at(string::split($pe.owner.name,'-'),3);
    return
    if ($bran_4 == '1' || $bran_4 == '2' || $bran_4 == '3' || $bran_4 == '4' || $bran_4 == '5' || $bran_4 == '6') && $ipara[0] > 0 {
        ipara[0] / 2
    }  else {
        ''
    }
};

// return fn::get_ispec_material(pe:17496_170719);