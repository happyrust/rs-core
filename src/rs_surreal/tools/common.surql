
DEFINE FUNCTION fn::refno($pe: record){
    return string::replace(record::id($pe), '_', '/');
};

// 将已知的desp / para 数字转为 word
DEFINE FUNCTION fn::into_str($desp: float) {
    return if $desp == 788296 {
        'DIAM'
    } else if $desp == 531717 {
        'FJ'
    } else if $desp == 531501 {
        'FB'
    } else if $desp = 560742 {
        'FEMA'
    };
};

DEFINE FUNCTION fn::type($pe:record) {
    return $pe.noun;
};

// 判断上一个或下一个节点是否为 $type类型
DEFINE FUNCTION fn::prev_or_next_is_type($pe:record,$type:string){
    return fn::pre_is_type($pe,$type) || fn::next_is_type($pe,$type);
};

DEFINE FUNCTION fn::pre_is_type($pe:record, $type:string) {
    return fn::prev_type($pe)==$type;
};

//判断cref的类型
DEFINE FUNCTION fn::cref_is_type($pe:record, $type:string) {
    return $pe.refno.CREF.TYPE == $type;
};

DEFINE FUNCTION fn::next_is_type($pe:record,$type:string) {
    return fn::next_type($pe)==$type;
};

DEFINE FUNCTION fn::prev_pe($pe:record) {
    let $a = ($pe->pe_owner.id[1])[0] -1;
    return select value (out<-pe_owner[$a].in) from only $pe->pe_owner limit 1;
};

DEFINE FUNCTION fn::next_pe($pe:record) {
    let $a = ($pe->pe_owner.id[1])[0] + 1;
    return select value (out<-pe_owner[$a].in) from only $pe->pe_owner limit 1;
};

DEFINE FUNCTION fn::prev_type($pe:record) {
    return fn::prev_pe($pe).noun;
};

DEFINE FUNCTION fn::next_type($pe:record) {
    return fn::next_pe($pe).noun;
};


DEFINE FUNCTION fn::children($pe:record) {
    return (select value <-pe_owner.in from $pe)[0];;
};


// 获取ipara
DEFINE FUNCTION fn::get_ipara($pe:record) {
    let $ispe = $pe.owner.refno.ISPE;
    return if $ispe == none {
        return [0];
    } else {
        return (select value (select value CATR.refno.PARA from only $ispe<-pe_owner<-pe<-pe_owner<-pe[? refno.TYPE = 'SELE' and $parent.owner.refno.TEMP >=refno.ANSW and $parent.owner.refno.TEMP <= refno.MAXA ]<-pe_owner<-pe.refno.* where $parent.owner.refno.HBOR >=ANSW and $parent.owner.refno.HBOR <= MAXA limit 1) from $pe)[0];
    };
};


// 获取room
DEFINE FUNCTION fn::get_room_number($pe:record) {
    return ($pe<-room_relate.room_num)[0];
}