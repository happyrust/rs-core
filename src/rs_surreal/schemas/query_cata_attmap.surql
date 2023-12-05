--通过传递refno，查询CATR属性值
let $a = array::distinct(select value [refno.CATR.refno.PRTREF.refno.CATR.refno.*, refno.CATR.refno.*, refno.SPRE.refno.CATR.refno.*] from only (type::thing("pe", $refno)));
if $a[0] != none {
    return $a[0];
} else {
    return $a[1];
}