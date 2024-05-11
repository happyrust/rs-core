//STRT
DEFINE FUNCTION fn::collect_empty($pes: array<record>) {
    return select
        fn::refno(id) as id,
        '-' as description,
        '-' as seg_code,
        '-' as sub_code,
        '-' as material,
        '-' as pressure,
        '-' as length,
        '-' as width,
        '-' as height,
        '-' as wall_thk,
        '-' as duct_area,
        '-' as duct_weight,
        '-' as stif_sctn,
        '-' as stif_len,
        '-' as stif_wei,
        '-' as fl_type,
        '-' as fl_len,
        '-' as fl_wei,

        '-' as washer_type,
        '-' as washer_len,
        '-' as bolt_qty,

        '-' as other_qty,
        '-' as stud,

        '-' as nut_qty, //螺母数量
        '-' as washer_qty,

        '-' as room_no,
        '-' as system
    from $pes;
}