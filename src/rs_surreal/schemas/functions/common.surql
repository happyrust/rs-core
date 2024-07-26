DEFINE FUNCTION fn::ancestor($r: record) {
    return if $r.id == none {
        return [];
    } else {
        return array::flatten(object::values(SELECT [id] as f0, ->pe_owner->(? AS f1)->pe_owner->(? AS f2)->pe_owner->(? AS f3)->pe_owner->(? AS f4)->pe_owner->(? AS f5)->pe_owner->(? AS f6)->pe_owner->(? AS f7)->pe_owner->(? AS f8)->pe_owner->(? AS f9) from only $r));
    };
};



DEFINE FUNCTION fn::order(
    $pe: record,
) {
    return if array::first($pe->pe_owner.id[1]) == none { 0 } else { array::first($pe->pe_owner.id[1]) };
};

DEFINE FUNCTION fn::default_name(
    $pe: record,
) {
    return (select value (if name='' {string::join(" ", noun, <string> (
        fn::order($pe) + 1) ) } else { name } ) from $pe)[0];
};

DEFINE FUNCTION fn::default_names(
    $pes: array<record>,
) {
    return (select value (if name='' {string::join(" ", noun,<string> (
       fn::order(id) + 1) ) } else { name } ) from $pes);
};


DEFINE FUNCTION fn::default_full_name($r: record) {
    return if $r.name != '' && $r.name != none {
        return $r.name;
    } else if $r.owner.name != '' && $r.owner.name != none {
        return string::concat(fn::default_name($r), " OF ", $r.owner.noun, ' ', $r.owner.name);
    } else {
        let $a = fn::ancestor($r);
        let $b = array::len($a) - array::find_index(array::reverse($a).name, '');
        let $i = array::slice($a, 0, $b);
        let $c = $b + 1;
        return string::concat(array::join(fn::default_names($i), " OF "), " OF ", $a[$c].noun, ' ', $a[$c].name);
    };
};


//备份数据到版本管理table中去
DEFINE FUNCTION fn::backup_atts_and_pes(
    $pes: array<record>,
) {
    for $pe in $pes {
        let $a = (select * omit id from $pe)[0];
        let $b = (select * omit id from $pe.refno)[0];
        let $c = (select * omit id, in from $pe->inst_relate)[0];
        if $a != none {
            let $id = string::concat(meta::id($a.refno), '_', $a.sesno);
            let $att_id = type::thing(string::concat($a.noun, '_history'), $id);
            let $pe_id = type::thing('pe_history', $id);
            update $pe_id content $a;
            update $pe_id set refno=$att_id;
            update $att_id content $b;

            //备份 inst_relate
            if $c.out != none && ($pe_id->inst_relate_history == none || array::len($pe_id->inst_relate_history) == 0) {
                relate $pe_id->inst_relate_history->($c.out) content $c;
                delete type::thing('inst_relate', meta::id($a.refno));
            };

            //备份 tubi_relate, 先暂时只删除，备份需要赋予版本号才行
            if $a.noun == "BRAN" || $a.noun == "HANG" {
                //let $t = (select * omit id, in from $pe->tubi_relate)[0];
                delete $pe->tubi_relate;
            }
        }
    }
};

