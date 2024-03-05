let $mat_code_map = ['','SS', 'GS'	,'CS','ACS','ASS',	'AS', 'FP'];
// 材质
DEFINE FUNCTION fn::hvac_material($pe:record) {
    return
    select if array::at(string::split(owner.name,'-'),3) != '6' {
        array::at($mat_code_map,<int>array::at(string::split(owner.name,'-'),3))
    } else if fn::pre_or_next_b_type($this.id,'DAMP') { 'CS' } else {
        'FP'
    } as r from $pe
};