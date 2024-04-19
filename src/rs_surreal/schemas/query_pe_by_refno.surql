--通过传递refno，查询属性值
(select * omit id from (type::thing("pe", $refno)))[0];
