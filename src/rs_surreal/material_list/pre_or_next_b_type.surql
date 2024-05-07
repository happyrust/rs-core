// 判断上一个或下一个节点是否为 $type类型
DEFINE FUNCTION fn::pre_or_next_b_type($pe:record,$type:string){
    return
    select value (array::any(select value in.noun==$type from ->pe_owner->pe<-pe_owner where order_num in [array::first($parent->pe_owner.order_num) - 1, array::first($parent->pe_owner.order_num) + 1] )) from $pe
}