REMOVE FUNCTION fn::ancestor;
-- 考虑到版本的原因，需要使用到历史祖先的记录，所以需要使用到这个函数
DEFINE FUNCTION fn::ancestor($r: record) {
    return if $r.id == none {
        return [];
    } else {
        let $a = SELECT value [id,
            owner ,
            owner.owner,
            owner.owner.owner,
            owner.owner.owner.owner,
            owner.owner.owner.owner.owner ,
            owner.owner.owner.owner.owner.owner,
            owner.owner.owner.owner.owner.owner.owner,
            owner.owner.owner.owner.owner.owner.owner.owner,
            owner.owner.owner.owner.owner.owner.owner.owner.owner]
            from only $r limit 1;
        return array::complement($a,
            [NONE, pe:0_0]
        );
    };
};

REMOVE FUNCTION fn::order;
DEFINE FUNCTION fn::order(
    $pe: record,
) {
    if $pe.owner == none {
        return none;
    };
    let $a = ($pe.owner<-pe_owner.in)[where noun=$pe.noun];
    return array::find_index($a, $pe);
};

DEFINE FUNCTION fn::default_name(
    $pe: record,
) {
    return (select value (if name='' {string::join(" ", noun, <string> (fn::order($pe) + 1) ) } else { name } ) from $pe)[0];
};

remove function fn::default_names;
DEFINE FUNCTION fn::default_names(
    $pes: array<record>,
) {
    return (select value (if name='' {string::join(" ", noun,<string> (
       fn::order(id) + 1) ) } else { name } ) from $pes);
};


REMOVE FUNCTION fn::default_full_name;
DEFINE FUNCTION fn::default_full_name($r: record) {
    return if !record::exists($r) {
        return '';
    }else if $r.name != '' && $r.name != none {
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

REMOVE FUNCTION fn::backup_atts_and_pes;
//备份数据到版本管理table中去
DEFINE FUNCTION fn::backup_atts_and_pes(
    $pes: array<record>,
) {
    for $pe in $pes {
        let $a = (select * omit id from $pe)[0];
        let $b = (select * omit id from $pe.refno)[0];
        let $c = (select * omit id, in from $pe->inst_relate)[0];
        if $a != none {
            let $id = string::concat(record::id($a.refno), '_', $a.sesno);
            let $att_id = type::thing(string::concat($a.noun, '_history'), $id);
            let $pe_id = type::thing('pe_history', $id);
            update $pe_id content $a;
            update $pe_id set refno=$att_id;
            update $att_id content $b;

            //备份 inst_relate
            if $c.out != none && ($pe_id->inst_relate_history == none || array::len($pe_id->inst_relate_history) == 0) {
                relate $pe_id->inst_relate_history->($c.out) content $c;
                delete type::thing('inst_relate', record::id($a.refno));
            };

            //备份 tubi_relate, 先暂时只删除，备份需要赋予版本号才行
            if $a.noun == "BRAN" || $a.noun == "HANG" {
                //let $t = (select * omit id, in from $pe->tubi_relate)[0];
                delete $pe->tubi_relate;
            }
        }
    }
};

remove function fn::latest_pe_sesno;
define function fn::latest_pe_sesno($pe: record, $s: option<number>, $db: option<number>){
    let $sesno = $pe.sesno?:0;
    let $r = return if $s != none and $db != none {
        #找到对应的时间
        let $t = ses:[$db, $s].date;
        return select value sesno from only pe_ses_h:[record::id($pe), 0]..[record::id($pe)] where ses.date<=$t ORDER BY sesno DESC limit 1;
    } else if $s != none {
        return if $s >= $sesno {
            return $sesno;
        } else{
            return select value sesno from only pe_ses_h:[record::id($pe), 0]..[record::id($pe)] where sesno<=$s ORDER BY sesno DESC limit 1;
        };
    } else {
        return $sesno
    };

    #需要知道是否是 latest，latest 都在 pe 表里存储了
    let $l = return if $r >= $sesno { 1 } else { 0 } ;

    return [$r?:0, $l];
};

remove function fn::latest_pe;
define function fn::latest_pe($pe: record, $s: option<number>, $db: option<number>){
    let $s = fn::latest_pe_sesno($pe, $s, $db);
    return if $s[1]  {
        return $pe;
    } else{
        let $id = record::id($pe);
    return type::thing('pe', [$id, $s[0]]);
    }
};


-- fn::latest_pe_sesno(pe:17496_171715, 880, 1112);
-- fn::latest_pe_sesno(pe:17496_171715, 880);
-- fn::latest_pe_sesno(pe:17496_171715);

remove function fn::ses_data;
define function fn::ses_data($pe: record){
    let $id = type::thing('ses', [$pe.dbnum, $pe.sesno]);
    return select * from only $id limit 1;
};
