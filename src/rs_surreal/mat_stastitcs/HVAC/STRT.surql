//STRT
select
    fn::refno(id) as id,
    string::concat(fn::shape_name(id), "直管") as description,
    fn::hvac_seg_code(id) as seg_code,
    fn::hvac_sub_code(id) as sub_code,
    fn::hvac_mat(id) as material,
    fn::hvac_pressure(id) as pressure,
    fn::hvac_len(id) as length,
    fn::hvac_width(id) as width,
    fn::hvac_height(id) as height,
    fn::hvac_thk(id, "DAMP") as wall_thk,
    fn::hvac_duct_area_format(id, "DAMP") as duct_area,
    fn::hvac_duct_weight_format(id, "DAMP") as duct_weight,
    fn::cal_stif_sctn_str(id) as stif_sctn,
    fn::cal_stif_len_str(id) as stif_len,
    fn::cal_stif_wei_str(id) as stif_wei,
    fn::hvac_fl_name(id) as fl_type,
    fn::hvac_fl_len(id) as fl_len,
    fn::hvac_fl_wei(id) as fl_wei



from pe:24381_58343;