//STIFF
select
    fn::refno(id) as id,
    string::concat(fn::shape_name(id), "加强筋") as stiff_name,
    fn::hvac_seg_code(id) as seg_code,
    fn::hvac_sub_code(id) as sub_code,
    fn::hvac_mat(id) as material,
    fn::hvac_pressure(id) as pressure,
    fn::hvac_len(id) as length,
    fn::hvac_width(id) as width,
    fn::hvac_height(id) as height,
    fn::hvac_thk(id) as wall_thk

from pe:24381_181248;