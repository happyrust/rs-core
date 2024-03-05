DEFINE FUNCTION fn::b_pre_type($pe:record,$type:string) {
    return select value (select value in.noun==$type from only ->pe_owner->pe<-pe_owner where order_num = array::first($parent->pe_owner.order_num) - 1 limit 1) from $pe limit 1;
};

DEFINE FUNCTION fn::b_next_type($pe:record,$type:string) {
    return select value (select value in.noun==$type from only ->pe_owner->pe<-pe_owner where order_num = array::first($parent->pe_owner.order_num) + 1 limit 1) from $pe limit 1;
};
