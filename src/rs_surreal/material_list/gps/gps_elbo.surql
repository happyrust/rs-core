// 给排水 大宗材料
// elbo

remove function fn::dzcl_elbo;
define function fn::dzcl_elbo($refnos: array<record>){
    select 
        id,
        fn::code($refno) as code, // 编码
        refno.TYPE as type ,// 部件
        math::fixed(refno.SPRE.refno.CATR.refno.PARA[3]?:0,3) as radius // 外径
    from $refno;
}

-- let $refno = [pe:24383_74103];
-- let $elbo = fn::dzcl_elbo($refno);
-- return $elbo;