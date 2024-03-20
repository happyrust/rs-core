
DEFINE FUNCTION fn::refno($pe: record){
    return string::replace(meta::id($pe), '_', '/');
};

// 将已知的desp / para 数字转为 word
DEFINE FUNCTION fn::into_str($desp: float) {
    return if $desp == 788296 {
        'DIAM'
    } else if $desp == 531717 {
        'FJ'
    } else if $desp == 531501 {
        'FB'
    };
};


// 判断上一个或下一个节点是否为 $type类型
DEFINE FUNCTION fn::prev_or_next_is_type($pe:record,$type:string){
    return
    (select value (array::any(select value in.noun==$type from ->pe_owner->pe<-pe_owner where order_num in [array::first($parent->pe_owner.order_num) - 1, array::first($parent->pe_owner.order_num) + 1] )) from $pe)[0]
};

DEFINE FUNCTION fn::pre_is_type($pe:record,$type:string) {
    return (select value (select value in.noun==$type from only ->pe_owner->pe<-pe_owner where order_num = array::first($parent->pe_owner.order_num) - 1 limit 1) from $pe limit 1)[0];
};

DEFINE FUNCTION fn::next_is_type($pe:record,$type:string) {
    return (select value (select value in.noun==$type from only ->pe_owner->pe<-pe_owner where order_num = array::first($parent->pe_owner.order_num) + 1 limit 1) from $pe limit 1)[0];
};

DEFINE FUNCTION fn::prev_pe($pe:record) {
    return (select value in from $pe->pe_owner->pe<-pe_owner where order_num = array::first($pe->pe_owner.order_num) - 1)[0];
};

DEFINE FUNCTION fn::next_pe($pe:record) {
    return (select value in from $pe->pe_owner->pe<-pe_owner where order_num = array::first($pe->pe_owner.order_num) + 1)[0];
};

DEFINE FUNCTION fn::prev_type($pe:record) {
    return (select value in.noun from $pe->pe_owner->pe<-pe_owner where order_num = array::first($pe->pe_owner.order_num) - 1)[0];
};

DEFINE FUNCTION fn::next_type($pe:record) {
    return (select value in.noun from $pe->pe_owner->pe<-pe_owner where order_num = array::first($pe->pe_owner.order_num) - 1)[0];
};


// 获取ipara
DEFINE FUNCTION fn::get_ipara($pe:record) {
    return (select value (select value CATR.refno.PARA from only owner.refno.ISPE<-pe_owner<-pe<-pe_owner<-pe[? refno.TYPE = 'SELE' and $parent.owner.refno.TEMP >=refno.ANSW and $parent.owner.refno.TEMP <= refno.MAXA ]<-pe_owner<-pe.refno.* where $parent.owner.refno.HBOR >=ANSW and $parent.owner.refno.HBOR <= MAXA limit 1) from $pe)[0];
};