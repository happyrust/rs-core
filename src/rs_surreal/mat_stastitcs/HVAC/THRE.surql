//THRE
select
    fn::refno(id) as id,
    string::concat(fn::shape_name(id), "三通管") as description,
    fn::hvac_seg_code(id) as seg_code,
    fn::hvac_sub_code(id) as sub_code,
    fn::hvac_mats(id) as material,
    fn::hvac_pressure(id) as pressure,
    fn::hvac_len(id) as length,
    fn::hvac_width_three(id) as width,
    fn::hvac_height_three(id) as height,
    fn::hvac_thks(id) as wall_thk,
    fn::hvac_duct_areas(id, "DAMP") as duct_area,
    fn::hvac_duct_weight_format(id, "DAMP") as duct_weight,
    '-' as stif_sctn,
    0 as stif_len,
    0 as stif_wei,
    fn::hvac_fl_name_THRE(id) as fl_type,
    fn::hvac_fl_len_THRE(id) as fl_len,
    fn::hvac_fl_wei(id) as fl_wei,
    fn::washer_type_THRE(id) as washer_type,
    fn::washer_len_THRE(id) as washer_len,

    '-' as other_qty,
    '-' as stud,

    fn::hvac_bolt_qtys_THRE(id) as nut_qty,  //螺母数量
    fn::hvac_washer_bolt_qtys_THRE(id) as washer_qty,  //垫片数量


    fn::hvac_system(id) as system,
    fn::get_room_number(id) as room_no



from pe:24381_100861;