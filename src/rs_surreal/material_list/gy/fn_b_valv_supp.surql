// 是否阀门支架
DEFINE FUNCTION fn::valv_b_supp($pe:record) {
    // let $pre_b_atta = fn::b_pre_type($pe,'ATTA');
    let $pre = fn::prev_pe($pe);
    return if  $pre.TYPE == 'ATTA' &&  $pre.POS[0] == $pe.refno.POS[0] && $pre.POS[1] == $pe.refno.POS[1] && $pre.POS[2] == $pe.refno.POS[2] {
        '是'
    } else {
        '否'
    };
};

// return fn::valv_b_supp(pe:24383_66745)