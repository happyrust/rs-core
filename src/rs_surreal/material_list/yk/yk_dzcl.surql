#仪控专业
#定义方法
remove function fn::yk_dzcl;
define function fn::yk_dzcl($refnos: array<record>) {
    select id,
    string::slice(refno.SPRE.name?:'', 1) as name ,
    refno.TYPE as type // 品名
    from $refnos;
}

-- let $refnos = [pe:24383_67652];
-- let $dzcl = fn::dzcl($refnos);
-- return $dzcl;


-- select id,
-- if refno.SPRE.name != NONE { string::slice(refno.SPRE.name,1) } else { ' ' } as name ,// 编码
-- refno.TYPE as type // 品名
-- from [pe:24383_67652];
