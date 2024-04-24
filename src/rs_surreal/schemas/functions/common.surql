DEFINE FUNCTION fn::ancestor($r: record) {
    return if $r.id == none {
        return [];
    } else {
        return array::flatten(object::values(SELECT [id] as f0, ->pe_owner->(? AS f1)->pe_owner->(? AS f2)->pe_owner->(? AS f3)->pe_owner->(? AS f4)->pe_owner->(? AS f5)->pe_owner->(? AS f6)->pe_owner->(? AS f7)->pe_owner->(? AS f8)->pe_owner->(? AS f9) from only $r));
    };
};


DEFINE FUNCTION fn::default_full_name($r: record) {
    return if !$r.is_default_name {
        return $r.name;
    } else if !$r.owner.is_default_name {
        return string::concat($r.name, " OF ", $r.owner.noun, ' ', $r.owner.name);
    } else {
        let $a = fn::ancestor($r);
        let $b = array::find_index($a.is_default_name, false);
        let $i = array::slice($a, 0, $b);
        return string::concat(array::join($i.name, " OF "), " OF ", $a[$b].noun, ' ', $a[$b].name);
    };
};