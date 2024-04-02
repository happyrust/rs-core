DEFINE FUNCTION fn::get_world_pos($pe:record) {
    let $noun = $pe.noun;
    return
    if $noun == 'EQUI' {
        return select value world_trans.d.translation from type::thing('cal_equi',meta::id($pe))
    } else {
        return select value ->inst_relate.world_trans.d.translation from $pe
    }
};
