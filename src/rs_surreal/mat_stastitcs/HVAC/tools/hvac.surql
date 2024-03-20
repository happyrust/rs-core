
DEFINE FUNCTION fn::hvac_stype($pe: record){
    return $pe.refno.DESP[0];
};

DEFINE FUNCTION fn::hvac_mat_number($pe: record){
    return <int>array::at(string::split(($pe->pe_owner->pe.name)[0], '-'), 3);
};

DEFINE FUNCTION fn::hvac_mat_id($pe: record){
    return type::thing('hvac_mat', <int>array::at(string::split(($pe->pe_owner->pe.name)[0], '-'), 3));
};

DEFINE FUNCTION fn::hvac_number($pe: record){
    let $v = (select value <int>v from type::thing('ATT_UDA', meta::id($pe)).udas where u.NAME='/H-SECTCODE')[0];
    return if $v == none {
        return "unset";
    } else {
        return $v;
    };
};

DEFINE FUNCTION fn::hvac_bran_name_splits($pe: record){
    return (select value string::split((->pe_owner->pe.name)[0], '-') from $pe)[0];
};

DEFINE FUNCTION fn::hvac_zone_name_splits($pe: record){
    return (select value string::split((->pe_owner->pe->pe_owner->pe->pe_owner->pe.name)[0], '-') from $pe)[0];
};

DEFINE FUNCTION fn::hvac_seg_code($pe: record){
    let $s = fn::hvac_bran_name_splits($pe);
    return string::join('/', string::concat($s[2], $s[5]), fn::hvac_number($pe));
};

DEFINE FUNCTION fn::hvac_sub_code($pe: record){
    let $s = fn::hvac_zone_name_splits($pe);
    return $s[2];
};

DEFINE FUNCTION fn::hvac_shape($pe: record){
    return if $pe.refno.DESP[29] == 927441 {
        return "RECT";
    } else if $pe.refno.DESP[29] == 603858 {
        return "CIRC";
    } else {
        return "unset";
    };
};

DEFINE FUNCTION fn::shape_name($pe: record){
    return if $pe.refno.DESP[29] == 927441 {
        return "方形";
    } else if $pe.refno.DESP[29] == 603858 {
        return "圆形";
    } else {
        return "unset";
    };
};


DEFINE FUNCTION fn::hvac_width($pe: record){
    return $pe.refno.DESP[1];
};

DEFINE FUNCTION fn::hvac_height($pe: record){
    return $pe.refno.DESP[2];
};

DEFINE FUNCTION fn::hvac_len($pe: record){
    return $pe.refno.DESP[5];
};

DEFINE FUNCTION fn::hvac_thk($pe: record, $chk_type: option<string>){
    return if $chk_type == "DAMP" {
          let $mat = fn::hvac_mat_number($pe);
          let $p_type = fn::prev_type($pe);
          let $n_type = fn::next_type($pe);
          return if $mat == 6 && ( $p_type == "DAMP" || $n_type == "DAMP" ) {
                  return [$pe.refno.DESP[4], '', 3];
          } else {
              return $pe.refno.DESP[4];
          };
    } else {
        return $pe.refno.DESP[4];
    };
};

DEFINE FUNCTION fn::duct_area($pe: record){
    return $pe.refno.DESP[80];
};

DEFINE FUNCTION fn::hvac_duct_area_format($pe: record, $chk_type: option<string>){
    let $area = fn::duct_area($pe) / 1000000;
    return if $chk_type == "DAMP" {
          let $mat = fn::hvac_mat_number($pe);
          let $p_type = fn::prev_type($pe);
          let $n_type = fn::next_type($pe);
          return if $mat == 6 && ( $p_type == "DAMP" || $n_type == "DAMP" ) {
                let $new_area = fn::height($pe) * fn::width($pe) * 150 / 1000000;
                return [$area, '', $new_area];
          } else {
              return $area;
          };
    } else {
        return $area;
    };
};

DEFINE FUNCTION fn::duct_weight($pe: record){
    return $pe.refno.DESP[84];
};

DEFINE FUNCTION fn::hvac_duct_weight_format($pe: record, $chk_type: option<string>){
    let $weight = fn::duct_weight($pe);
    return if $chk_type == "DAMP" {
          let $mat = fn::hvac_mat_number($pe);
          let $p_type = fn::prev_type($pe);
          let $n_type = fn::next_type($pe);
          return if $mat == 6 && ( $p_type == "DAMP" || $n_type == "DAMP" ) {
                let $new_weight = ( fn::height($pe) * fn::width($pe) * fn::hvac_thk($pe) * 150 * 7.85 ) / 1000000;
                return [$weight, '', $new_weight];
          } else {
              return $weight;
          };
    } else {
        return $weight;
    };
};


DEFINE FUNCTION fn::hvac_max_size($pe: record){
    return math::max([$pe.refno.DESP[1], $pe.refno.DESP[2]]);
};

DEFINE FUNCTION fn::hvac_pressure($pe: record){
    return array::at(string::split(($pe->pe_owner->pe.name)[0], '-'), 4);
};

DEFINE FUNCTION fn::hvac_shape_with_pressure($pe: record){
    return string::join('/', fn::hvac_shape($pe), fn::hvac_pressure($pe));
};



insert into hvac_mat [
    {
        "id": 1,
        "code": "GS",
        "desc": "镀锌风管",
    },{
        "id": 2,
        "code": "CS",
        "desc": "碳钢风管",
    },{
        "id": 3,
        "code": "ACS",
        "desc": "气密性碳钢风管",
    },{
        "id": 4,
        "code": "SS",
        "desc": "不锈钢风管",
    },{
        "id": 5,
        "code": "GS",
        "desc": "气密性不锈钢风管",
    },{
        "id": 6,
        "code": "FP",
        "desc": "防火风管",
    }
];

DEFINE FUNCTION fn::hvac_mat($pe: record){
    let $m = (select value code from fn::hvac_mat_id($pe))[0];
    return if $m == none {
        return "unset";
    } else {
        return $m;
    };
};

DEFINE FUNCTION fn::hvac_mat_name($pe: record){
    let $m = (select value desc from fn::hvac_mat_id($pe))[0];
    return if $m == none {
        return "unset";
    } else {
        return $m;
    };
};




//暂时返回数字
DEFINE FUNCTION fn::stype($pe:record) {
    return $pe.refno.DESP[0];
};

DEFINE FUNCTION fn::prev_stype($pe:record) {
    return fn::stype(fn::prev_pe($pe));
};

DEFINE FUNCTION fn::next_stype($pe:record) {
    return fn::stype(fn::next_pe($pe));
};

//cur
DEFINE FUNCTION fn::stype_is_fjfd($pe:record) {
    return fn::stype($pe) == 614823;
};

DEFINE FUNCTION fn::stype_is_damp($pe:record) {
    return fn::stype($pe) == 855877;
};

DEFINE FUNCTION fn::stype_is_mesh($pe:record) {
    return fn::stype($pe) == 702904;
};

//prev
DEFINE FUNCTION fn::prev_stype_is_fjfd($pe:record) {
    return fn::prev_stype($pe) == 614823;
};

DEFINE FUNCTION fn::prev_stype_is_damp($pe:record) {
    return fn::prev_stype($pe) == 855877;
};

DEFINE FUNCTION fn::prev_stype_is_mesh($pe:record) {
    return fn::prev_stype($pe) == 702904;
};

//next
DEFINE FUNCTION fn::stype_is_fjfd($pe:record) {
    return fn::next_stype($pe) == 614823;
};

DEFINE FUNCTION fn::stype_is_damp($pe:record) {
    return fn::next_stype($pe) == 855877;
};

DEFINE FUNCTION fn::stype_is_mesh($pe:record) {
    return fn::next_stype($pe) == 702904;
};

