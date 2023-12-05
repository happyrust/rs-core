--从节点出发找到根节点，默认层级深度为 9
let $a = array::flatten(object::values(SELECT id as f0, ->pe_owner->(? AS f1)->pe_owner->(? AS f2)->pe_owner->(? AS f3)->pe_owner->(? AS f4)->pe_owner->(? AS f5)->pe_owner->(? AS f6)->pe_owner->(? AS f7)->pe_owner->(? AS f8)->pe_owner->(? AS f9)  from only (type::thing("pe", $refno))));
select value refno.* from $a;