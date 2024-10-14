
DEFINE FIELD refnos ON TABLE his_pe TYPE set<record<pe>>;

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

REMOVE FUNCTION fn::newest_pe;
DEFINE FUNCTION fn::newest_pe(
    $pe: record,
) {
    if type::is::array($pe) {
        return $pe;
    };
    let $refno = record::id($pe)[0];
    return type::thing('pe', $refno);
};


REMOVE FUNCTION fn::order;
DEFINE FUNCTION fn::order(
    $pe: record,
) {
    if $pe.owner == none {
        return none;
    };
    let $a = ($pe.owner<-pe_owner.in)[where noun=$pe.noun];
    if $a == none {
        let $b = (fn::newest_pe($pe).owner<-pe_owner.in)[where noun=$pe.noun];
        return array::find_index($b, $pe);
    } else {
        return array::find_index($a, $pe);
    };
};

REMOVE FUNCTION fn::default_name;
DEFINE FUNCTION fn::default_name(
    $pe: record,
) {
    return (select value (if name='' {string::join(" ", noun, <string> (fn::order($pe)?:0 + 1) ) } else { name } ) from $pe)[0];
};

remove function fn::default_names;
DEFINE FUNCTION fn::default_names(
    $pes: array<record>,
) {
    return select value (if name='' {string::join(" ", noun,<string> (
       fn::order(id)?:0 + 1) ) } else { name } ) from $pes;
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


REMOVE FUNCTION fn::backup_data;
//备份修改的数据，只针对修改的处理
//如果是新增，应该是对pe_owner的备份
DEFINE FUNCTION fn::backup_data(
    $pes: array<record>,
    $is_del: bool,
    $sesno: number,
) {
    for $pe in $pes {
        //只操作最新的 refno
        if type::is::array($pe) || !record::exists($pe) {
            continue;
        };
        if $pe.sesno > $sesno {
            continue;
        };
        let $old_id = [<string> record::id($pe.id), $pe.sesno];
        let $old_pe_id = type::thing('pe', $old_id);

        let $a = (select * omit id from $pe)[0];
        let $b = (select * omit id from $pe.refno)[0];

        if $a != none && $b != none {
            let $old_inst_relate_id = type::thing('inst_relate', $old_id);
            let $old_att_id = type::thing(string::concat($a.noun, '_H'), $old_id);
            if $is_del{
                update $pe.id set op=2, old_pe=$old_pe_id, refno=$old_att_id, sesno=$sesno;
            } else {
                update $pe.id set op=1, old_pe=$old_pe_id, sesno = $sesno;
            };
            upsert type::thing('his_pe', record::id($pe.id)) set refnos+=$old_pe_id;
            upsert $old_pe_id content $a.patch([{
                                		'op': 'replace',
                                		'path': 'refno',
                                		'value': $old_att_id
                                }]);
            //属性数据需要单独的备份方法，因为有很多引用
            upsert $old_att_id content $b.patch([{
                                		'op': 'replace',
                                		'path': 'REFNO',
                                		'value': $old_pe_id
                                }]);

            if $is_del{
                delete $pe.refno;
            };

            let $c = (select *, $old_inst_relate_id as id, $old_pe_id as in from $pe->inst_relate);
            if !array::is_empty($c) {
                delete select id from $pe->inst_relate;
                insert relation into inst_relate $c;
            };

            if $a.noun == "BRAN" || $a.noun == "HANG" {

                let $tubis = (select *, type::thing('tubi_relate', [ $old_pe_id, record::id(id)[1] ]) as id, $old_pe_id as in from $pe->tubi_relate);
                //将 tubi relate 整体备份
                if !array::is_empty($tubis) {
                    //删除之前的，变成历史的
                    delete select id from $pe->tubi_relate;
                    insert relation into tubi_relate $tubis;
                };
            };
        }
    }
};


REMOVE FUNCTION fn::backup_owner_relate;
//备份修改的数据，只针对修改的处理
//如果是新增，应该是对pe_owner的备份
DEFINE FUNCTION fn::backup_owner_relate(
    $pes: array<record>,
    $delete_old: bool,
) {
    for $pe in $pes {
        //只操作最新的 refno
        if type::is::array($pe) || !record::exists($pe) {
            continue;
        };
        let $old_id = [<string> record::id($pe.id), $pe.sesno];
        let $old_pe_id = type::thing('pe', $old_id);
        //fix 层级不连续的问题，需要更新到之前的 in->owner 关系
        let $relate_id = select value id from $pe->pe_owner;
        update $relate_id set in=$old_pe_id;
        //如果子节点的sesno >= 父节点的sesno 了，则需要使用 old_pe
        //反之则不需要动
        let $c = select *, type::thing('pe_owner', [ $old_pe_id, record::id(id)[1] ]) as id,
                    if $pe.sesno>in.sesno { in } else { in.old_pe?:in } as in, $old_pe_id as out
                    from $pe<-pe_owner;
        if !array::is_empty($c) {
            if $delete_old{
                delete select id from $pe<-pe_owner;
            };
            insert relation into pe_owner $c;
        }
    }
};

// let $pe = pe:17496_171606;
// fn::backup_owner_relate([$pe], false);

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

    return [$r?:0, $sesno];
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


remove function fn::find_pe_by_datetime;
define function fn::find_pe_by_datetime($pe: record, $dt: datetime){
    let $a = select id from only array::reverse(array::flatten([$pe.refnos?:[], $pe])) where <datetime> fn::ses_date(id) <= $dt limit 1;
    return $a;
};


-- fn::latest_pe_sesno(pe:17496_171715, 880, 1112);
-- fn::latest_pe_sesno(pe:17496_171715, 880);
-- fn::latest_pe_sesno(pe:17496_171715);

remove function fn::ses_data;
define function fn::ses_data($pe: record){
    let $id = type::thing('ses', [$pe.dbnum, $pe.sesno]);
    return select * from only $id limit 1;
};

remove function fn::ses_date;
define function fn::ses_date($pe: record){
    return if record::exists($pe) {
        let $id = type::thing('ses', [$pe.dbnum, $pe.sesno]);
        return select value date from only $id limit 1;
    } else {
        return none;
    };
};
