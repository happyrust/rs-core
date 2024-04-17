//TAPE
select
    fn::refno(id) as id,
    string::concat(fn::shape_name(id), "变截面管") as description,
    fn::hvac_seg_code(id) as seg_code,
    fn::hvac_sub_code(id) as sub_code,
    fn::hvac_mats(id) as material,
    fn::hvac_pressure(id) as pressure,
    fn::hvac_len(id) as length,
    [fn::hvac_width_format(id), fn::hvac_width2(id)] as width,
    [fn::hvac_height_format(id), fn::hvac_height2(id)] as height,
    fn::hvac_thks(id) as wall_thk,
    fn::hvac_duct_areas(id, "DAMP") as duct_area,
    fn::hvac_duct_weight_format(id, "DAMP") as duct_weight,
    fn::cal_stif_len_str(id) as stif_len,
    fn::cal_stif_wei_str(id) as stif_wei,
    fn::hvac_fl_name(id) as fl_type,
    fn::hvac_fl_len(id) as fl_len,
    fn::hvac_fl_wei(id) as fl_wei,
    fn::common_washer_types(id) as washer_type,

    fn::hvac_bolt_qtys_1(id) as nut_qty,  //螺母数量
    fn::hvac_washer_bolt_qtys_1(id) as washer_qty,

    fn::hvac_system(id) as system,
    fn::get_room_number(id) as room_no



from pe:24381_57493;