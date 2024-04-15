
DEFINE FUNCTION fn::common_washer_types($pe: record) {
    return [ fn::common_washer_type_single($pe, true) , fn::common_washer_type_single($pe, false) ];
};


//单个的垫圈类型获取
DEFINE FUNCTION fn::common_washer_type_single($pe: record, $prev_or_next: bool) {
    let $is_fjfd = if $prev_or_next {
        return fn::prev_stype_is_fjfd($pe);
    } else {
        return fn::next_stype_is_fjfd($pe);
    };

    let $stype_is_damp = if $prev_or_next {
         return fn::prev_stype_is_damp($pe);
     } else {
         return fn::next_stype_is_damp($pe);
     };

    let $mat_id = meta::id(fn::hvac_mat_id($pe));

    return if $is_fjfd && $stype_is_damp {
        return "D型";
    } else if !$is_fjfd {
        return if $mat_id == 1 || $mat_id == 2 || $mat_id == 4  {
            return "A型";
        } else if $mat_id == 3 || $mat_id == 5 {
            return "B型";
        }
    };
};

