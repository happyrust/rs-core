//STRT
DEFINE FUNCTION fn::collect_STRT($pes: array<record>) {
    return select
        fn::refno(id) as id,
        string::concat(fn::shape_name(id), "直管") as description,
        fn::hvac_seg_code(id) as seg_code,
        fn::hvac_sub_code(id) as sub_code,
        fn::hvac_mats(id) as material,
        fn::hvac_pressure(id) as pressure,
        fn::hvac_len_format(id) as length,
        fn::hvac_width_format(id) as width,
        fn::hvac_height_format(id) as height,
        fn::hvac_thks(id, "DAMP") as wall_thk,
        fn::hvac_duct_areas(id, "DAMP") as duct_area,
        fn::hvac_duct_weight_format(id, "DAMP") as duct_weight,
        fn::cal_stif_sctn_str(id) as stif_sctn,
        fn::cal_stif_len_str(id) as stif_len,
        fn::cal_stif_wei_str(id) as stif_wei,
        fn::hvac_fl_name(id) as fl_type,
        fn::hvac_fl_len_format(id) as fl_len,
        fn::hvac_fl_wei_format(id) as fl_wei,

        fn::common_washer_types(id) as washer_type,
        fn::common_washer_len(id) as washer_len,
        fn::hvac_bolt_qty(id) as bolt_qty,

        '-' as other_qty,
        '-' as stud,

        fn::hvac_bolt_qty(id) as nut_qty,  //螺母数量
        fn::hvac_bolt_qty(id) * 2 as washer_qty,

        fn::get_room_number(id) as room_no,
        fn::hvac_system(id) as system
    from $pes;
}