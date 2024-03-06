// 法兰重量
DEFINE FUNCTION fn::strt_flan_weight($pe:record) {
    let $desp = select value (select value in.refno.DESP from only ->pe_owner->pe<-pe_owner where order_num = array::first($parent->pe_owner.order_num) - 1 limit 1) from $pe;
    let $flan_type = if array::first($desp)[57] == 531717 { 'FJ' } else if array::first($desp)[57] == 531501 { 'FB' } else { '' };
    let $flan_length = array::first(fn::strt_flan_length($pe)).length;
    //return [$flan_type,array::first($desp)[67],$flan_length.length];
    return 
    select if ($flan_type == 'FJ' && array::first($desp)[67] == 25) {
    $flan_length * 1.124 
} else if ($flan_type == 'FJ' && array::first($desp)[67] == 303) {
    $flan_length * 1.373 
} else if ($flan_type == 'FJ' && array::first($desp)[67] == 30) {
    $flan_length * 1.786 
} else if ($flan_type == 'FJ' && array::first($desp)[67] == 35) {
    $flan_length * 1.656 
} else if ($flan_type == 'FJ' && array::first($desp)[67] == 40) {
    $flan_length * 2.422 
} else if ($flan_type == 'FJ' && array::first($desp)[67] == 45) {
    $flan_length * 2.736 
} else if ($flan_type == 'FJ' && array::first($desp)[67] == 50) {
    $flan_length * 3.77 
} else if ($flan_type == 'FJ' && array::first($desp)[67] == 60) {
    $flan_length * 5.369 
} else if ($flan_type == 'FJ' && array::first($desp)[67] == 608) {
    $flan_length * 7.081 
} else if ($flan_type == 'FJ' && array::first($desp)[67] == 6550) {
    $flan_length * 5.134 
} else if ($flan_type == 'FJ' && array::first($desp)[67] == 65) {
    $flan_length * 5.3694 
} else if ($flan_type == 'FJ' && array::first($desp)[67] == 63) {
    $flan_length * 5.721 
} else if ($flan_type == 'FJ' && array::first($desp)[67] == 70) {
    $flan_length * 7.398 
} else if ($flan_type == 'FJ' && array::first($desp)[67] == 7550) {
    $flan_length * 5.699 
} else if ($flan_type == 'FJ' && array::first($desp)[67] == 75) {
    $flan_length * 7.976 
} else if ($flan_type == 'FJ' && array::first($desp)[67] == 8060) {
    $flan_length * 6.311 
} else if ($flan_type == 'FJ' && array::first($desp)[67] == 80) {
    $flan_length * 9.658 
} else if ($flan_type == 'FJ' && array::first($desp)[67] == 8010) {
    $flan_length * 11.874 
} else if ($flan_type == 'FJ' && array::first($desp)[67] == 90) {
    $flan_length * 5.661 
} else if ($flan_type == 'FJ' && array::first($desp)[67] == 9056) {
    $flan_length * 5.661 
} else if ($flan_type == 'FJ' && array::first($desp)[67] == 10065) {
    $flan_length * 9.8596 
} else if ($flan_type == 'FJ' && array::first($desp)[67] == 10080) {
    $flan_length * 10.946 
} else if ($flan_type == 'FJ' && array::first($desp)[67] == 100) {
    $flan_length * 12.276 
} else if ($flan_type == 'FB' && array::first($desp)[67] == 253) {
    $flan_length * 0.59 
} else if ($flan_type == 'FB' && array::first($desp)[67] == 254) {
    $flan_length * 0.78 
} else if ($flan_type == 'FB' && array::first($desp)[67] == 304) {
    $flan_length * 0.94 
} else if ($flan_type == 'FB' && array::first($desp)[67] == 305) {
    $flan_length * 1.18 
} else if ($flan_type == 'FB' && array::first($desp)[67] == 354) {
    $flan_length * 1.10 
} else if ($flan_type == 'FB' && array::first($desp)[67] == 404) {
    $flan_length * 1.26 
} else if ($flan_type == 'FB' && array::first($desp)[67] == 405) {
    $flan_length * 1.57 
} else if ($flan_type == 'FB' && array::first($desp)[67] == 505) {
    $flan_length * 1.96 
} else if ($flan_type == 'FB' && array::first($desp)[67] == 606) {
    $flan_length * 2.826 
} else if ($flan_type == 'FB' && array::first($desp)[67] == 608) {
    $flan_length * 3.77 
} else if ($flan_type == 'FB' && array::first($desp)[67] == 6010) {
    $flan_length * 4.71 
} else if ($flan_type == 'FB' && array::first($desp)[67] == 808) {
    $flan_length * 5.02 
} else if ($flan_type == 'FB' && array::first($desp)[67] == 8010) {
    $flan_length * 6.28 
} else if ($flan_type == 'FB' && array::first($desp)[67] == 8012) {
    $flan_length * 7.54 
} else {
    0
} as weight from $pe
};