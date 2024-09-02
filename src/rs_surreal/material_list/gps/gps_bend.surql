// 给排水 大宗材料
// bend

-- 弯头
remove function fn::gps_bend;

define function fn::gps_bend($refnos: array<record>){
    return select 
        id,
        fn::code($refno) as code, // 编码
        refno.TYPE as type ,// 部件
        math::fixed(if refno.SPRE.refno.CATR.refno.PARA == NONE { 0 } else { refno.SPRE.refno.CATR.refno.PARA[3] },3) as radius, // 外径
        math::fixed(if refno.SPRE.refno.CATR.refno.PARA == NONE && refno.ANGL == NONE { 0 }
        else { (refno.ANGL / 360) * 2 *3.1415 * refno.SPRE.refno.CATR.refno.PARA[1] },2) as count // 数量
    from $refnos;
};

// let $refno = [pe:24384_2967];
// let $bend = fn::gps_bend($refno);
// return $bend;


