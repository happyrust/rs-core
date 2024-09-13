
DEFINE FUNCTION fn::washer_type_CAP($pe: record) {
    let $pf = fn::prev_stype_is_fjfd($pe);
    return if $pf {
        return "D型";
    } else {
        let $mat_id = record::id(fn::hvac_mat_id($pe));
        return if $mat_id in [1, 2, 4] {
            return "B型";
        } else {
            return "A型";
        };
    };
};

DEFINE FUNCTION fn::washer_len_CAP($pe: record) {
    let $is_circle = fn::hvac_shape($pe) == 'CIRC';
    let $pd = fn::pre_is_type($pe, 'DAMP');
    let $pm = fn::pre_is_type($pe, 'MESH');

    return if $is_circle {
        return if $pd || $pm {
            return math::fixed( (fn::hvac_fl_len($pe) + fn::hvac_fl_width($pe) * 4.0) / 1000.0, 2 );
        } else if (!$pd) && (!$pm) {
            return math::fixed( ((fn::hvac_fl_len($pe) + fn::hvac_fl_width($pe) * 4.0) / 1000.0 ) / 2.0, 2);
        } ;
    } else {
        return if $pd || $pm {
            return math::fixed( ( ( (fn::hvac_width($pe) + (fn::hvac_fl_width($pe) * 2.0) + 75.0) / 150.0 ) +
                                                                    ( (fn::hvac_height($pe) + (fn::hvac_fl_width($pe) * 2.0) + 75.0) / 150.0 ) - 2 ) * 2, 2);
        } else if (!$pd) && (!$pm) {
            return math::fixed( ( ( (fn::hvac_width($pe) + (fn::hvac_fl_width($pe) * 2.0) + 75.0) / 150.0 ) +
                                                                    ( (fn::hvac_height($pe) + (fn::hvac_fl_width($pe) * 2.0) + 75.0) / 150.0 ) - 2 ), 2);
        } ;
    };
};

DEFINE FUNCTION fn::washer_type_single($check: bool, $mat_id: int) {
    return if $check {
        return "D型";
    } else {
        return if $mat_id in [1, 2, 4] {
            return "B型";
        } else {
            return "A型";
        };
    };
};

DEFINE FUNCTION fn::washer_type_THRE($pe: record) {
    let $pf = fn::prev_stype_is_fjfd($pe);
    let $nf = fn::next_stype_is_fjfd($pe);
    let $cf = fn::c_stype_is_fjfd($pe);
    let $mat_id = record::id(fn::hvac_mat_id($pe));
    let $m1 =  fn::washer_type_single($pf, $mat_id);
    let $m2 =  fn::washer_type_single($nf, $mat_id);
    let $m3 =  fn::washer_type_single($cf, $mat_id);

    return [$m1, $m2, $m3];
};


DEFINE FUNCTION fn::washer_len_THRE($pe: record) {
    let $is_circle = fn::hvac_shape($pe) == 'CIRC';
    let $pd = fn::pre_is_type($pe, 'DAMP');
    let $nd = fn::next_is_type($pe, 'DAMP');
    let $pm = fn::pre_is_type($pe, 'MESH');
    let $nm = fn::next_is_type($pe, 'MESH');
    let $cd = fn::cref_is_type($pe, 'DAMP');
    let $cm = fn::cref_is_type($pe, 'MESH');
    let $len = fn::hvac_fl_len_THRE($pe);

    return if $is_circle {
        let $d1 = if $pd || $pm {
            return ( $len[0] + fn::hvac_fl_width($pe, 1) * 4.0 / 1000.0 );
        } else if (!$pd) && (!$pm) {
            return ( $len[0] + fn::hvac_fl_width($pe, 1) * 4.0 / 1000.0 ) / 2.0;
        };
        let $d2 = if $nd || $nm {
                    return ( $len[1] + fn::hvac_fl_width($pe, 2) * 4.0 / 1000.0 );
                } else if (!$nd) && (!$nm) {
                    return ( $len[1] + fn::hvac_fl_width($pe, 2) * 4.0 / 1000.0 ) / 2.0;
                };
        let $d3 = if $cd || $cm {
                            return ( $len[2] + fn::hvac_fl_width($pe, 3) * 4.0 / 1000.0 );
                        } else if (!$nd) && (!$nm) {
                            return ( $len[2] + fn::hvac_fl_width($pe, 3) * 4.0 / 1000.0 ) / 2.0;
                        };
        return [$d1, $d2, $d3];
    } else {
        let $d1 = if $pd || $pm {
            return $len[0];
        } else if (!$pd) && (!$pm) {
            return $len[0] / 2.0;
        };
        let $d2 = if $nd || $nm {
            return $len[1];
        } else if (!$nd) && (!$nm) {
            return $len[1] / 2.0;
        };
        let $d3 = if $cd || $cm {
            return $len[2];
        } else if (!$nd) && (!$nm) {
            return $len[2] / 2.0;
        };
        return [$d1, $d2, $d3];
    };
};


DEFINE FUNCTION fn::washer_type_TRNS($pe: record) {
    let $pf = fn::prev_stype_is_fjfd($pe);
    let $nf = fn::next_stype_is_fjfd($pe);
    let $mat_id = record::id(fn::hvac_mat_id($pe));
    let $m1 =  fn::washer_type_single($pf, $mat_id);
    let $m2 =  fn::washer_type_single($nf, $mat_id);

    return [$m1, $m2];
};

DEFINE FUNCTION fn::washer_len_TRNS($pe: record) {
    let $pd = fn::pre_is_type($pe, 'DAMP');
    let $nd = fn::next_is_type($pe, 'DAMP');
    let $pm = fn::pre_is_type($pe, 'MESH');
    let $nm = fn::next_is_type($pe, 'MESH');
    let $len = fn::hvac_fl_len_TRNS($pe);

    return if fn::hvac_height2_is_DIAM($pe) {
        let $d1 = if $pd || $pm {
                    return ( $len[0] + fn::hvac_fl_width($pe, 1) * 4.0 / 1000.0 );
                } else if (!$pd) && (!$pm) {
                    return ( $len[0] + fn::hvac_fl_width($pe, 1) * 4.0 / 1000.0 ) / 2.0;
                };
         let $d2 = if $nd || $nm {
                            return ( $len[0]  );
                        } else if (!$nd) && (!$nm) {
                            return ( $len[0] ) / 2.0;
                        };
         return [math::fixed($d1, 2), math::fixed($d2, 2)];
    } else if fn::hvac_height_is_DIAM($pe) {
        let $d1 = if $pd || $pm {
                    return ( $len[1] + fn::hvac_fl_width($pe, 2) * 4.0 / 1000.0 );
                } else if (!$pd) && (!$pm) {
                    return ( $len[1] + fn::hvac_fl_width($pe, 2) * 4.0 / 1000.0 ) / 2.0;
                };
         let $d2 = if $nd || $nm {
                            return ( $len[1]  );
                        } else if (!$nd) && (!$nm) {
                            return ( $len[1] ) / 2.0;
                        };
         return [math::fixed($d1, 2), math::fixed($d2, 2)];
    };
};


