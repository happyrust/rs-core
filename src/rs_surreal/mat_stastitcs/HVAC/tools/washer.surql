


DEFINE FUNCTION fn::washer_type($pe: record) {
    let $mat_id = meta::id(fn::hvac_mat_id($pe));
    let $pre_is_fjfd = fn::pre_is_type($pe);
    let $next_is_fjfd = fn::pre_is_type($pe);
    let $type_1 = if $pre_is_fjfd {
        "D型"
    } else {
        ""
    };
    let $type_2 = if $mat_id == 1 || $mat_id == 2 || $mat_id == 4 {
         "A型"
    } else if $mat_id == 3 || $mat_id == 5 {
         "B型"
    } else {
         ""
    };
    return $type_2;
};

DEFINE FUNCTION fn::common_washer_type($pe: record) {
    let $mat_id = meta::id(fn::hvac_mat_id($pe));
    return if $mat_id == 1 || $mat_id == 2 || $mat_id == 4 {
            "A型"
        } else if $mat_id == 3 || $mat_id == 5 {
            "B型"
        } else {
            ""
        };
}

//tape 的垫圈类型处理
DEFINE FUNCTION fn::tape_washer_type($pe: record) {
    let $prev_s_is_fjfd = fn::prev_stype_is_fjfd($pe);
    let $next_s_is_fjfd = fn::next_stype_is_fjfd($pe);
    let $pre_is_damp = fn::pre_is_type($pe, "DAMP");
    let $next_is_damp = fn::next_is_type($pe, "DAMP");

    let $s1 = if $prev_s_is_fjfd {
        if $pre_is_damp {
            "D型"
        } else {
            ""
        }
    } else {
        fn::common_washer_type($pe)
    };
}


//STIF 的垫圈类型处理
DEFINE FUNCTION fn::tape_washer_type($pe: record) {
    return "D型";
}