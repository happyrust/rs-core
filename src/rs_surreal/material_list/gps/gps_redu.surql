// 给排水 大宗材料
// redu

remove function fn::dzcl_redu;

define function fn::dzcl_redu($refnos: array<record>){
    select 
        id,
        fn::code($refno) as code, // 编码
        refno.TYPE as type ,// 部件
        string::replace(<string>array::join([math::fixed(refno.SPRE.refno.CATR.refno.PARA[5]?:0.0,3),
           math::fixed(refno.SPRE.refno.CATR.refno.PARA[6]?:0.0,3)],';'),'f','') as radius, // 外径 
        string::replace(<string>math::fixed(refno.SPRE.refno.CATR.refno.PARA[3]?:0.0,3),'f','') as length // 长度
    from $refnos;
}

-- let $refno = [pe:24383_67889];
-- let $redu = fn::dzcl_redu($refno);
-- return $redu;
