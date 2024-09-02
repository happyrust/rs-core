// 给排水 大宗材料
// 法兰
remove function fn::dzcl_flan;
define function fn::dzcl_flan($refnos: array<record>){
    select 
        id,
        fn::code($refno) as code, // 编码
        refno.TYPE as type ,// 部件
        string::replace(<string>math::fixed(refno.SPRE.refno.CATR.refno.PARA[6]?:0.0,3),'f','') as radius, // 外径
        string::replace(<string>math::fixed(refno.SPRE.refno.CATR.refno.PARA[4]?:0.0,3),'f','') as thick // 厚度
    from $refnos;
}

-- let $refno = [pe:24383_67888];
-- let $flan = fn::dzcl_flan($refno);
-- return $flan;
