// 是否阀门支架
DEFINE FUNCTION fn::valv_b_supp($pe:record) {
    // let $pre_b_atta = fn::b_pre_type($pe,'ATTA');
    let $pre = select value (select value in.refno.* from only ->pe_owner->pe<-pe_owner where order_num = array::first($parent->pe_owner.order_num) - 1 limit 1) from $pe limit 1;

    return if array::first($pre.TYPE) == 'ATTA' && array::first($pre.POS[0]) == $pe.refno.POS[0] && array::first($pre.POS[1]) == $pe.refno.POS[1] && array::first($pre.POS[2]) == $pe.refno.POS[2] && array::first($pre.AEXCES) == 0 {
        '是'
    } else {
        '否'
    };
};

// return fn::valv_b_supp(pe:24383_66745)