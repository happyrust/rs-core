
//根据类型找到父节点
DEFINE FUNCTION fn::find_ancestor_type($pe: record, $t: string){
    return (select value array::flatten([->pe_owner->pe[? noun=$t].id,
        ->pe_owner->pe->pe_owner->pe[? noun=$t].id,
        ->pe_owner->pe->pe_owner->pe->pe_owner->pe[? noun=$t].id,
        ->pe_owner->pe->pe_owner->pe->pe_owner->pe->pe_owner->pe[? noun=$t].id,
        ->pe_owner->pe->pe_owner->pe->pe_owner->pe->pe_owner->pe->pe_owner->pe[? noun=$t].id,
        ->pe_owner->pe->pe_owner->pe->pe_owner->pe->pe_owner->pe->pe_owner->pe->pe_owner->pe[? noun=$t].id,
        ->pe_owner->pe->pe_owner->pe->pe_owner->pe->pe_owner->pe->pe_owner->pe->pe_owner->pe->pe_owner->pe[? noun=$t].id,
        ->pe_owner->pe->pe_owner->pe->pe_owner->pe->pe_owner->pe->pe_owner->pe->pe_owner->pe->pe_owner->pe->pe_owner->pe[? noun=$t].id
    ]) from $pe)[0];
};