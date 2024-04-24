DEFINE FUNCTION fn::ancestor($r: record) {
    return if $r.id == none {
        return [];
    } else {
        return array::flatten(object::values(SELECT [id] as f0, ->pe_owner->(? AS f1)->pe_owner->(? AS f2)->pe_owner->(? AS f3)->pe_owner->(? AS f4)->pe_owner->(? AS f5)->pe_owner->(? AS f6)->pe_owner->(? AS f7)->pe_owner->(? AS f8)->pe_owner->(? AS f9) from only $r));
    };
};


DEFINE FUNCTION fn::default_name(
    $pe: record,
) {
    return (select value (if name='' {string::join(" ", noun, <string> (array::first(->pe_owner.id[1])+1) ) } else { name } ) from $pe)[0];
};

DEFINE FUNCTION fn::default_names(
    $pes: array<record>,
) {
    return (select value (if name='' {string::join(" ", noun, <string> (array::first(->pe_owner.id[1])+1) ) } else { name } ) from $pes);
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