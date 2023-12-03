--通过传递refno，查询属性值
(select * from (type::thing("pe", $refno)).refno)[0];
