// 法兰规格
DEFINE FUNCTION fn::strt_flan_size($pe:record) {
    let $desp = select value (select value in.refno.DESP from only ->pe_owner->pe<-pe_owner where order_num = array::first($parent->pe_owner.order_num) - 1 limit 1) from $pe;
    let $pre_fls = array::first($desp)[67];
    let $fl_type = if array::first($desp)[57] == 531717 { 'FJ' } else if array::first($desp)[57] == 531501 { 'FB' } else { '' };
    return select size from tf_strt_flan_size where fl_type = $fl_type and fl_size = $pre_fls;
};