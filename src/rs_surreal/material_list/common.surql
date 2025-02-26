//string::split(string::split(refno.SPRE.name?:'//:', '/')[2],':')[0]?:'' as code, 
// 编码
remove function fn::code;
define function fn::code($pe: record, $opt: option<number>){
    return if $opt == none {
        string::split(string::split($pe.refno.SPRE.name?:'', '/')[2]?:'', ':')[0]?:''
    } else {
        string::split(string::split($pe.refno.SPRE.name?:'', '/')[2]?:'', ':')[0]?:''
    };
};

//将pe转换为refno
remove function fn::pe_to_refno;
define function fn::pe_to_refno($pe:record){
    return string::replace(record::id($pe), '_', '/');
};

// RETURN fn::normal_name("/exampleName");  -- 返回 "exampleName"
// RETURN fn::normal_name(":anotherName");  -- 返回 "anotherName"
// RETURN fn::normal_name("noPrefixName");  -- 返回 "noPrefixName"
remove function fn::normal_name;
DEFINE FUNCTION fn::normal_name($name: string) {
    RETURN $name
        .replace("^/", "")  -- 去掉以 '/' 开头的前缀
        .replace("^:", ""); -- 去掉以 ':' 开头的前缀
};


// 返回 UDA 的某个属性，这里如果在 ATT_UDA 里如果没有找到，需要去查找默认值
remove FUNCTION fn::get_uda_value;
DEFINE FUNCTION fn::get_uda_value($pe:record, $uda_name:string) {

   let $a = SELECT VALUE (
                SELECT VALUE object::values(v) FROM ONLY udas.* WHERE u.NAME ==  $uda_name LIMIT 1
            )[0] FROM ONLY type::thing("ATT_UDA", record::id($pe)) limit 1;
    return if $a == NONE {
        let $n = fn::normal_name($uda_name);
        return select value DFLT from only UDA
            where !UHIDE and $pe.noun in ELEL and (DYUDNA == $n || UDNA == $n) limit 1;
    } else {
        $a
    };
};



-- let $refno = [pe:24383_84092];
-- let $code = fn::code($refno);
-- return $code;

// 判断上一个或下一个节点是否为 $type类型
remove function fn::pre_or_next_b_type;
DEFINE FUNCTION fn::pre_or_next_b_type($pe:record,$type:string){
    return
    select value fn::prev_pe($pe).noun == $type || fn::next_pe($pe) == $type from $pe
};

remove function fn::get_world_pos;
DEFINE FUNCTION fn::get_world_pos($pe:record) {
    let $noun = $pe.noun;
    return
    if $noun == 'EQUI' {
        return select value world_trans.d.translation from type::thing('cal_equi',record::id($pe))
    } else {
        return select value ->inst_relate.world_trans.d.translation from $pe
    }
};

remove function fn::refno;
DEFINE FUNCTION fn::refno($pe: record){
    return string::replace(record::id($pe), '_', '/');
};

remove function fn::type;
DEFINE FUNCTION fn::type($pe:record) {
    return $pe.noun;
};

// 判断上一个或下一个节点是否为 $type类型
remove function fn::prev_or_next_is_type;
DEFINE FUNCTION fn::prev_or_next_is_type($pe:record,$type:string){
    return fn::pre_is_type($pe,$type) || fn::next_is_type($pe,$type);
};

remove function fn::pre_is_type;
DEFINE FUNCTION fn::pre_is_type($pe:record, $type:string) {
    return fn::prev_type($pe)==$type;
};

//判断cref的类型
remove function fn::cref_is_type;
DEFINE FUNCTION fn::cref_is_type($pe:record, $type:string) {
    return $pe.refno.CREF.TYPE == $type;
};

remove function fn::next_is_type;
DEFINE FUNCTION fn::next_is_type($pe:record,$type:string) {
    return fn::next_type($pe)==$type;
};

remove function fn::prev_pe;
DEFINE FUNCTION fn::prev_pe($pe:record) {
    let $a = (record::id($pe->pe_owner.id[0])[1]) -1;
    return select value (out<-pe_owner[$a].in) from only $pe->pe_owner limit 1;
};

remove function fn::next_pe;
DEFINE FUNCTION fn::next_pe($pe:record) {
    let $a = (record::id($pe->pe_owner.id[0])[1]) + 1;
    return select value (out<-pe_owner[$a].in) from only $pe->pe_owner limit 1;
};

remove function fn::prev_type;
DEFINE FUNCTION fn::prev_type($pe:record) {
    return fn::prev_pe($pe).noun;
};

remove function fn::next_type;
DEFINE FUNCTION fn::next_type($pe:record) {
    return fn::next_pe($pe).noun;
};

remove function fn::children;
DEFINE FUNCTION fn::children($pe:record) {
    return (select value <-pe_owner.in from $pe)[0];;
};


// 获取ipara
remove function fn::get_ipara;
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


// 获取room
remove function fn::get_room_number;
DEFINE FUNCTION fn::get_room_number($pe:record) {
    return ($pe<-room_relate.room_num)[0];
};

remove function fn::vec3_distance;
DEFINE FUNCTION fn::vec3_distance($x:array, $y:array) {
    return
    if array::len($x) < 3 || array::len($y) < 3 {
        0
    } else {
        math::sqrt(($x[0] - $y[0]) * ($x[0] - $y[0]) + ($x[1] - $y[1]) * ($x[1] - $y[1]) + ($x[2] - $y[2]) * ($x[2] - $y[2]))
    }
};

remove function fn::array_distance;
DEFINE FUNCTION fn::array_distance($x: array<array>) {
    return vector::distance::euclidean($x[0], $x[1]);
};

-- let $x = [[1,2,3],[4,5,6]];
-- return fn::array_distance($x);