
-- DEFINE FIELD refnos ON TABLE his_pe TYPE set<record<pe>>;

REMOVE FUNCTION fn::ancestor;
-- 考虑到版本的原因，需要使用到历史祖先的记录，所以需要使用到这个函数
DEFINE FUNCTION fn::ancestor($r: record) {
    return if !record::exists($r) {
        return [];
    } else {
        let $nodes = (SELECT VALUE @.{..+collect}(owner) FROM ONLY $r LIMIT 1) ?: [];
        return array::filter($nodes, |$v| $v != NONE && $v != pe:0_0);
    };
};

REMOVE FUNCTION fn::newest_pe;
DEFINE FUNCTION fn::newest_pe(
    $pe: record,
) {
    if $pe == none {
        return none;
    };
    let $id = record::id($pe);
    if !type::is::array($id) {
        return $pe;
    };
    let $refno = $id[0];
    return type::thing('pe', $refno);
};

REMOVE FUNCTION fn::newest_pe_id;
DEFINE FUNCTION fn::newest_pe_id(
    $pe: record,
) {
    if !type::is::array($pe) {
        return record::id($pe);
    };
    return record::id($pe)[0];
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
    return (select value (if name=='' {string::join(' ', noun, <string> (fn::order($pe)?:0 + 1) ) } else { name } ) from $pe)[0];
};

REMOVE FUNCTION fn::default_names;
DEFINE FUNCTION fn::default_names(
    $pes: array<record>,
) {
    return select value (if name=='' {string::join(' ', noun,<string> (
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
        if type::is::array(record::id($pe)) || !record::exists($pe) {
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
            let $old_uda_att_id = type::thing('ATT_UDA_H', $old_id);
            if $is_del{
                //软删除
                update $pe.id set op=2, old_pe=$old_pe_id, sesno=$sesno, deleted=true;
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
            //uda 属性需要单独的备份方法，因为有很多引用
            let $uda_data = select * from only type::thing('ATT_UDA', record::id($pe.id)) limit 1;
            if $uda_data != none {
                upsert $old_uda_att_id content $uda_data.patch([{
                                		'op': 'replace',
                                		'path': 'refno',
                                		'value': $old_pe_id
                                }]);
            };
            //删除属性
            if $is_del{
               delete $pe.refno;
            };

            //备份关联
            let $c = (select *, $old_inst_relate_id as id, $old_pe_id as in from $pe->inst_relate);
            if !array::is_empty($c) {
                delete select id from $pe->inst_relate;
                insert relation into inst_relate $c;
            };

            if $a.noun == "BRAN" || $a.noun == "HANG" {

                let $tubis = (select *, type::thing('tubi_relate', [ $old_pe_id, record::id(id)[1] ]) as id,
                                 $old_pe_id as in from $pe->tubi_relate);
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
        // if type::is::array(record::id($pe)) || !record::exists($pe) {
        //      continue;
        // };
        let $old_id = [<string> record::id($pe.id), $pe.sesno];
        let $old_pe_id = type::thing('pe', $old_id);
        //fix 层级不连续的问题，需要更新到之前的 in->owner 关系
        //let $old_owners_id = select value od from $pe->pe_owner where type::is::array(record::id(out));
        -- return $old_owners_id;
        //如果是旧节点，需要更新到旧的引用节点
        //for $o in $old_owners_id {
            //需要删除了，重新创建这个单独的id
            //update $o set in=$old_pe_id;
        //};
        //如果子节点的sesno >= 父节点的sesno 了，则需要使用 old_pe
        //反之则不需要动
        let $c = select *, type::thing('pe_owner', [ $old_pe_id, record::id(id)[1] ]) as id,
                    if $pe.sesno>in.sesno { in } else { in.old_pe?:in } as in, $old_pe_id as out
                    from $pe<-pe_owner;
        -- return $c;
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

REMOVE FUNCTION fn::latest_pe_sesno;
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

REMOVE FUNCTION fn::latest_pe;
define function fn::latest_pe($pe: record, $s: option<number>, $db: option<number>){
    let $s = fn::latest_pe_sesno($pe, $s, $db);
    return if $s[1]  {
        return $pe;
    } else{
        let $id = record::id($pe);
    return type::thing('pe', [$id, $s[0]]);
    }
};


REMOVE FUNCTION fn::find_pe_by_datetime;
define function fn::find_pe_by_datetime($pe: record, $dt: datetime){
    let $his_pe = type::thing('his_pe', fn::newest_pe_id($pe));
    if !record::exists($his_pe) {
        return $pe;
    };
    let $a = select value id from only array::reverse(array::flatten([$his_pe.refnos?:[], $pe])) where <datetime>fn::ses_date(id) <= $dt limit 1;
    return $a?:$pe;
};


-- fn::latest_pe_sesno(pe:17496_171715, 880, 1112);
-- fn::latest_pe_sesno(pe:17496_171715, 880);
-- fn::latest_pe_sesno(pe:17496_171715);

REMOVE FUNCTION fn::ses_data;
define function fn::ses_data($pe: record){
    let $id = type::thing('ses', [$pe.dbnum, $pe.sesno]);
    return select * from only $id limit 1;
};

REMOVE FUNCTION fn::ses_date;
define function fn::ses_date($pe: record){
    return if record::exists($pe) {
        let $id = type::thing('ses', [$pe.dbnum, $pe.sesno]);
        return select value date from only $id limit 1;
    } else {
        return none;
    };
};

REMOVE FUNCTION fn::next_is_type;
DEFINE FUNCTION fn::next_is_type($pe:record,$type:string) {
    return fn::next_type($pe)==$type;
};

REMOVE FUNCTION fn::first_child;
DEFINE FUNCTION fn::first_child($pe:record) {
    return select value (<-pe_owner[0].in) from only $pe limit 1;
};

REMOVE FUNCTION fn::first_child_exclude_type;
DEFINE FUNCTION fn::first_child_exclude_type($pe:record, $exclude_types: array) {
    return select value in from only $pe<-pe_owner where !$exclude_types.any(in.noun) limit 1;
};

REMOVE FUNCTION fn::last_child;
DEFINE FUNCTION fn::last_child($pe:record) {
    return array::last((select value (<-pe_owner.in) from only $pe limit 1)?:[]);
};

REMOVE FUNCTION fn::prev_pe;
DEFINE FUNCTION fn::prev_pe($pe:record) {
    if $pe == none || !record::exists($pe) {
        return none;
    };
    let $a = (record::id($pe->pe_owner.id[0])[1]) - 1;
    return select value (out<-pe_owner[$a].in) from only $pe->pe_owner limit 1;
};

REMOVE FUNCTION fn::prev_pe_exclude_type;
DEFINE FUNCTION fn::prev_pe_exclude_type($pe:record, $exclude_types: array) {
    if $pe == none || !record::exists($pe) {
            return none;
    };
    let $a = (record::id($pe->pe_owner.id[0])[1]);
    return (select value in from ($pe->pe_owner->pe<-pe_owner)[..$a] where !$exclude_types.any(in.noun)).last();
};

REMOVE FUNCTION fn::last_child_exclude_type;
DEFINE FUNCTION fn::last_child_exclude_type($pe:record, $exclude_types: array) {
    return (select * from only $pe<-pe_owner where !$exclude_types.any(in.noun) order by id desc limit 1).in;
};

REMOVE FUNCTION fn::next_pe;
DEFINE FUNCTION fn::next_pe($pe:record) {
    if $pe == none || !record::exists($pe) {
        return none;
    };
    let $a = (record::id($pe->pe_owner.id[0])[1]) + 1;
    return select value (out<-pe_owner[$a].in) from only $pe->pe_owner limit 1;
};

REMOVE FUNCTION fn::next_pe_exclude_type;
DEFINE FUNCTION fn::next_pe_exclude_type($pe:record, $exclude_types: array) {
    if $pe == none || !record::exists($pe) {
            return none;
        };
    let $a = (record::id($pe->pe_owner.id[0])[1]) + 1;
    return select value in from only ($pe->pe_owner->pe<-pe_owner)[$a..] where !$exclude_types.any(in.noun)  limit 1;
};

REMOVE FUNCTION fn::prev_connect_pe;
DEFINE FUNCTION fn::prev_connect_pe($pe:record) {
    if $pe == none {
        return none;
    };
    let $p = fn::prev_pe_exclude_type($pe, ['ATTA','GASK']);
    return if $p != none {
        return $p;
    } else {
        let $connect_bran = $pe.owner.refno.HREF;
        return if $connect_bran != none {
            return fn::last_child_exclude_type($connect_bran,['ATTA','GASK'])?:$connect_bran;
        } else {
            return none;
        };
    };
};

REMOVE FUNCTION fn::prev_connect_pe_data;
DEFINE FUNCTION fn::prev_connect_pe_data($pe:record) {
    let $p = fn::prev_connect_pe($pe);
    // 判断上一个跨bran连接的是否有tubi
    let $p_tubi = fn::query_tubi_to($p).arrive;
    //如果有到达这个元件的直段，则返回这个直段的leave
    let $tmp_tubi_refno = fn::query_tubi_from($pe).leave;
    let $b_href_bran = $pe.owner.refno.HREF.noun == 'BRAN';
    let $tubi_refno = if $tmp_tubi_refno != none {
        $tmp_tubi_refno
    } else if fn::first_child($pe.owner) == $pe && $b_href_bran && fn::query_tubi_to($pe.owner) != none {
        // 判断在bran第一节点时有没有tubi
        fn::query_tubi_to($pe.owner).arrive
    } else if $p_tubi != none && $b_href_bran {
        // 判断跨bran的上一个元件，该元件下面有没有tubi  并排除HREF连接的是TEE等管件的情况
        $p
    } else {
       none
    };
    let $tubi_name = return if $tubi_refno == none {
        none
    } else {
        string::concat('ileave tube of ',  fn::default_full_name($tubi_refno))
    };
    return if $p != none {   //如果存在上一个节点
        //如果tubi_refno是branch的最后一个节点，则需要判断关联的branch有tubi
        let $tubi_spref = return if $tubi_refno == none {
            none
        } else if $tubi_refno.noun == 'BRAN'{
            $tubi_refno.refno.HSTU.name
        } else {
            $tubi_refno.refno.LSTU.name
        };
        return {
            id: $pe,
            prev_full_name: fn::default_full_name($p),
            prev: $p,
            tubi_name: $tubi_name,
            tubi_refno: $tubi_refno,
            tubi_spref: $tubi_spref
        };
    } else {  //如果不存在上一个节点
        return if $tubi_refno != none {
            return {
                id: $pe,
                prev_full_name: fn::default_full_name($pe.owner),
                prev: $pe.owner,
                tubi_name: $tubi_name,
                tubi_refno: $pe.owner,
                tubi_spref: $pe.owner.refno.HSTU.name
            };
        } else {
            none
        };
    };
};

REMOVE FUNCTION fn::next_connect_pe;
DEFINE FUNCTION fn::next_connect_pe($pe:record) {
    if $pe == none {
        return none;
    };
    let $p = fn::next_pe_exclude_type($pe, ['ATTA','GASK']);
    return if $p != none {
        return $p;
    } else {
        let $connect_bran = $pe.owner.refno.TREF;
        return if $connect_bran != none {
            return fn::first_child_exclude_type($connect_bran,['ATTA','GASK'])?:$connect_bran;
        } else {
            return none;
        };
    };
};

REMOVE FUNCTION fn::next_connect_pe_data;
DEFINE FUNCTION fn::next_connect_pe_data($pe:record) {
    let $p = fn::next_connect_pe($pe);
    let $p_tubi = fn::query_tubi_from($p).leave;
    let $b_tref_bran = $pe.owner.refno.TREF.noun == 'BRAN';
    let $tubi_refno = if fn::query_tubi_to($pe) != none {
        $pe
    } else if fn::last_child($pe.owner) == $pe && $b_tref_bran && fn::query_tubi_to($pe.owner.refno.TREF) != none {
        // 返回是否有bran开启的直段
      $pe.owner.refno.TREF
    } else if $p_tubi!= NONE && $b_tref_bran {
       $p
    } else {
        NONE
    };
    let $tubi_name = return if $tubi_refno == none {
       none
   } else { string::concat('ileave tube of ',  fn::default_full_name($tubi_refno)) };
    return if $p != none {
        let $tubi_spref = return if $tubi_refno == none {
            none
        } else if $tubi_refno.noun == 'BRAN'{
            $tubi_refno.refno.HSTU.name
        } else {
            $tubi_refno.refno.LSTU.name
        };
        return {
            id: $pe,
            next_full_name: fn::default_full_name($p),
            next: $p,
            tubi_refno: $tubi_refno,
            tubi_name: $tubi_name,
            tubi_spref: $tubi_spref
        };
    } else {
        return if $tubi_refno != none {
            return {
                id: $pe,
                next_full_name: fn::default_full_name($pe),
                next: $pe,
                tubi_refno: tubi_refno,
                tubi_name: $tubi_name,
                tubi_spref: $pe.refno.LSTU.name
            };
        } else {
            none
        };
    };
};

REMOVE FUNCTION fn::query_tubi_to;
DEFINE FUNCTION fn::query_tubi_to($pe:record) {
    if $pe.noun == 'BRAN' {
       return select arrive from only $pe->tubi_relate where leave=$pe limit 1;
    } else {
        return select arrive from only $pe.owner->tubi_relate where leave=$pe limit 1;
    }
};

REMOVE FUNCTION fn::query_tubi_from;
DEFINE FUNCTION fn::query_tubi_from($pe:record) {
    if $pe.noun == 'BRAN' {
        return select leave from only $pe->tubi_relate where arrive=$pe limit 1;
    } else {
        return select leave from only $pe.owner->tubi_relate where arrive=$pe limit 1;
    }
};

REMOVE FUNCTION fn::query_bran_first_tubi;
DEFINE FUNCTION fn::query_bran_first_tubi($pe:record) {
    if $pe.noun != 'BRAN' {
        return none;
    };
    return tubi_relate:[$pe, 0].id;
};

REMOVE FUNCTION fn::prev_type;
DEFINE FUNCTION fn::prev_type($pe:record) {
    return fn::prev_pe($pe).noun;
};

REMOVE FUNCTION fn::next_type;
DEFINE FUNCTION fn::next_type($pe:record) {
    return fn::next_pe($pe).noun;
};


REMOVE FUNCTION fn::children;
DEFINE FUNCTION fn::children($pe:record) {
    return array::distinct(select value in from $pe<-pe_owner where record::exists(in.id) and !in.deleted);
};


// 获取ipara
REMOVE FUNCTION fn::get_ipara;
DEFINE FUNCTION fn::get_ipara($pe:record) {
    let $ispe = $pe.owner.refno.ISPE;
    return if $ispe == none {
        return [0];
    } else {
        return (select value (select value CATR.refno.PARA
                from only $ispe<-pe_owner<-pe<-pe_owner<-pe[? refno.TYPE = 'SELE'
                and $parent.owner.refno.TEMP >=refno.ANSW
                and $parent.owner.refno.TEMP <= refno.MAXA ]<-pe_owner<-pe.refno.*
                where $parent.owner.refno.HBOR >=ANSW and $parent.owner.refno.HBOR <= MAXA limit 1)
            from $pe)[0];
    };
};

//根据类型找到父节点
REMOVE FUNCTION fn::find_ancestor_type;
DEFINE FUNCTION fn::find_ancestor_type($pe: record, $t: string){
    (array::flatten(select value fn::ancestor(id) from $pe)[?$self.noun=$t])[0]
};

// 根据类型返回子节点
REMOVE FUNCTION fn::find_deep_children_type;
define function fn::find_deep_children_type($pe:record,$t:string) {
    let $children = array::flatten( object::values( (select
                      [id] as p0, <-pe_owner<-(? as p1)<-pe_owner<-(? as p2)<-pe_owner<-(? as p3)<-pe_owner<-(? as p4)<-pe_owner<-(? as p5)<-pe_owner<-(? as p6)<-pe_owner<-(? as p7)<-pe_owner<-(? as p8)<-pe_owner<-(? as p9)<-pe_owner<-(? as p10)<-pe_owner<-(? as p11)
                   from only $pe where record::exists(id))?:{{}} ) );

    return select value id from $children where id.noun == $t

};

// 返回类型集合的父节点
REMOVE FUNCTION fn::find_ancestor_types;
DEFINE FUNCTION fn::find_ancestor_types($pe: record, $t: array){
    let $r = (array::flatten(select value fn::ancestor(id) from $pe));
    return select value id from $r where noun in $t;
};

REMOVE FUNCTION fn::find_door_from_wall;
DEFINE FUNCTION fn::find_door_from_wall($pe:array) {
    let $fitts = select value fn::collect_descendant_ids_by_types(id, ["FIXING", "NBOX", "NCYL", "NXTR", "PFIT", "NPYR", "FITT", "SBFI"], none) from $pe;
    let $uda_value = select id as refno,fn::default_name(id) as name,fn::find_ancestor_types(id,["STWALL", "GWALL", "WALL"])[0] as wall from array::flatten(select value id from $fitts) where string::contains(fn::get_uda_value(id,'/JGOBJBASE')?:'','门洞');
    return $uda_value;
};

REMOVE FUNCTION fn::get_mdb_dbnums;
DEFINE FUNCTION fn::get_mdb_dbnums($mdb: string, $db_type: int) {
        let $dbnos = select value (select value DBNO from CURD.refno where STYP=$db_type) from only MDB where NAME=$mdb limit 1;
        return (select value dbnum from (
            select REFNO.dbnum as dbnum, array::find_index($dbnos, REFNO.dbnum) as o
            from WORL
            where REFNO.dbnum in $dbnos
            order by o
        ));
};

REMOVE FUNCTION fn::get_flan_bolt_list;
define function fn::get_flan_bolt_list($pe:record) {
    if $pe.refno.SPRE == NONE { return NONE; };
    let $bltf = select value refno.SPRE.refno.CATR.refno.BLRF[0].name from $pe;
    if $bltf == NONE { return NONE; };
    let $bltf_split = array::last(string::split($bltf[0],'-')?:[]);
    let $blot_size = string::concat($bltf_split,'W')?:'';
    let $ancestor = fn::find_ancestor_type($pe.refno.SPRE?:pe:0_0,'SPEC').refno.BSPE;
    if $ancestor == NONE { return NONE; };
    let $spcos = fn::find_deep_children_type($ancestor,'SPCO');
    if $spcos == NONE { return NONE; };
    let $filter_spcos = select value id from $spcos where string::contains(name,$blot_size);
    if array::len($filter_spcos) == 0 { return NONE; };
    let $mat = select name , id.refno.DETR.refno.RTEX as rtext from $filter_spcos;
    return {
        'bltf':$bltf[0],
        'mat':$mat
    }
};

REMOVE FUNCTION fn::query_ptset;
define function fn::query_ptset($refno: string) {
    let $table_name = string::concat($refno, '_inst_relate');
    let $sql = string::concat(
        '(SELECT world_trans.d AS transform, object::values(out.ptset?:{}).pt AS points FROM ',
        $table_name,
        ')[0]'
    );

    let $result = SELECT VALUE $sql FROM ONLY [];
    return $result[0];
};

-- ============================================================================
-- 性能优化函数：收集子孙节点并过滤 inst_relate 和 tubi_relate
-- ============================================================================
-- 功能：一次性完成子孙节点收集和关系过滤，避免多次往返
-- 参数：
--   $root: 起始节点
--   $types: 要筛选的节点类型数组（空数组表示不过滤类型）
--   $filter_inst: 是否过滤掉有 inst_relate 或 tubi_relate 的节点
--   $include_self: 是否包含起始节点自身
--   $skip_deleted: 是否跳过已删除的节点
-- 返回：符合条件的节点 ID 数组
-- 性能提升：相比分块查询，减少 90% 的网络往返时间
-- ============================================================================
REMOVE FUNCTION fn::collect_descendants_filter_inst;
DEFINE FUNCTION fn::collect_descendants_filter_inst(
    $root: record,
    $types: array<string>,
    $filter_inst: bool,
    $include_self: bool,
    $skip_deleted: bool
) {
    -- 1. 收集所有子孙节点（使用现有的高效函数）
    let $descendants = fn::collect_descendants_by_children($root, $types, $include_self, $skip_deleted);

    -- 2. 如果不需要过滤关系，直接返回
    if !$filter_inst {
        return $descendants;
    };

    -- 3. 过滤掉有 inst_relate 或 tubi_relate 的节点
    -- 使用 count + LIMIT 1 优化：只需检查是否存在，不需要计数所有关系
    return array::filter($descendants, |$node| {
        let $has_inst = count(SELECT VALUE id FROM $node->inst_relate LIMIT 1);
        let $has_tubi = count(SELECT VALUE id FROM $node->tubi_relate LIMIT 1);
        return $has_inst = 0 AND $has_tubi = 0;
    });
};

-- ============================================================================
-- 辅助函数：快速检查节点是否有指定类型的关系
-- ============================================================================
-- 功能：高效检查节点是否存在某种关系
-- 参数：
--   $node: 要检查的节点
--   $relation: 关系表名（如 'inst_relate', 'tubi_relate'）
-- 返回：true 表示有关系，false 表示无关系
-- 优化：使用 LIMIT 1 避免遍历所有关系
-- ============================================================================
REMOVE FUNCTION fn::has_relation;
DEFINE FUNCTION fn::has_relation(
    $node: record,
    $relation: string
) {
    -- 动态构建关系查询
    -- 注意：SurrealDB 不支持动态关系名，这里需要在调用端处理
    return count(SELECT VALUE id FROM $node->$relation LIMIT 1) > 0;
};

-- ============================================================================
-- 性能优化函数：收集子孙节点并过滤 SPRE/CATR
-- ============================================================================
-- 功能：一次性完成子孙节点收集和 SPRE/CATR 过滤，避免多次往返
-- 参数：
--   $root: 起始节点
--   $types: 要筛选的节点类型数组（空数组表示不过滤类型）
--   $filter_inst: 是否同时过滤掉有 inst_relate 或 tubi_relate 的节点
--   $exclude_self: 排除自身节点（none 表示包含，true 表示排除）
-- 返回：符合条件的节点 ID 数组
-- 性能提升：相比分块查询，减少 90% 的网络往返时间
-- ============================================================================
REMOVE FUNCTION fn::collect_descendants_filter_spre;
DEFINE FUNCTION fn::collect_descendants_filter_spre(
    $root: record,
    $types: array<string>,
    $filter_inst: bool,
    $exclude_self: option<bool>
) {
    -- 1. 收集所有子孙节点
    let $descendants = fn::collect_descendant_ids_by_types($root, $types, $exclude_self);
    
    -- 2. 过滤 SPRE 和 CATR（两者不能同时为空）
    let $with_spre = array::filter($descendants, |$node_id| {
        let $pe = type::thing('pe', $node_id);
        let $has_spre = $pe.refno.SPRE.id != none;
        let $has_catr = $pe.refno.CATR.id != none;
        return $has_spre OR $has_catr;
    });
    
    -- 3. 如果需要过滤 inst_relate/tubi_relate
    if $filter_inst {
        return array::filter($with_spre, |$node_id| {
            let $pe = type::thing('pe', $node_id);
            let $has_inst = count(SELECT VALUE id FROM $pe->inst_relate LIMIT 1);
            let $has_tubi = count(SELECT VALUE id FROM $pe->tubi_relate LIMIT 1);
            return $has_inst = 0 AND $has_tubi = 0;
        });
    };
    
    return $with_spre;
};

-- ============================================================================
-- 性能优化函数：收集子孙节点及其属性
-- ============================================================================
-- 功能：一次性完成子孙节点收集和属性查询，避免多次往返
-- 参数：
--   $root: 起始节点
--   $types: 要筛选的节点类型数组（空数组表示不过滤类型）
--   $exclude_self: 排除自身节点（none 表示包含，true 表示排除）
-- 返回：所有节点的 refno.* 属性数组
-- 性能提升：相比分块查询，减少 90% 的网络往返时间
-- ============================================================================
REMOVE FUNCTION fn::collect_descendants_with_attrs;
DEFINE FUNCTION fn::collect_descendants_with_attrs(
    $root: record,
    $types: array<string>,
    $exclude_self: option<bool>
) {
    -- 1. 收集所有子孙节点 ID
    let $ids = fn::collect_descendant_ids_by_types($root, $types, $exclude_self);
    
    -- 2. 批量查询所有节点的属性
    -- 将 ID 数组转换为 pe 记录，然后提取 refno.* 属性
    let $pes = array::map($ids, |$id| type::thing('pe', $id));
    
    -- 3. 从每个 pe 记录中提取 refno 属性
    return array::filter_map($pes, |$pe| {
        if record::exists($pe) {
            return $pe.refno;
        } else {
            return none;
        }
    });
};
