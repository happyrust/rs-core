//TRNS
DEFINE FUNCTION fn::collect_TRNS($pes: array<record>) {
select
    fn::refno(id) as id,
   '天圆地方' as description,
   fn::hvac_seg_code(id) as seg_code,
   fn::hvac_sub_code(id) as sub_code,
   fn::hvac_mats(id) as material,
   fn::hvac_pressure(id) as pressure,
   fn::hvac_len(id) as length,
   fn::hvac_width_format(id) as width,
   fn::hvac_height_format(id) as height,
   fn::hvac_thks(id, "DAMP") as wall_thk,
   fn::hvac_duct_areas(id, "DAMP") as duct_area,
   fn::hvac_duct_weight_format(id, "DAMP") as duct_weight,
   fn::cal_stif_sctn_str(id) as stif_sctn,
   fn::cal_stif_len_str(id) as stif_len,
   fn::cal_stif_wei_str(id) as stif_wei,
   fn::hvac_fl_name(id) as fl_type,
   fn::hvac_fl_len_TRNS(id) as fl_len,
   fn::hvac_fl_wei(id) as fl_wei,

    fn::washer_type_TRNS(id) as washer_type,
    fn::washer_len_TRNS(id) as washer_len,

    fn::hvac_bolt_qtys_TRNS(id) as bolt_qty,  //螺栓数量

    '-' as other_type,
    '-' as other_qty,
    '-' as stud,

    fn::hvac_bolt_qtys_TRNS(id) as nut_qty,  //螺母数量
    fn::hvac_washer_bolt_qtys_TRNS(id) as washer_qty,  //垫片数量


    fn::hvac_system(id) as system,
    fn::get_room_number(id) as room_no

    from $pes;
}