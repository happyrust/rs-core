DEFINE FUNCTION fn::default_names(
    $pes: array<record>,
) {
    return (select value (if name='' { string::concat(noun, <string> (array::find_index(select value order_num from ->pe_owner->pe<-pe_owner[where <-pe[where noun=$parent.noun]] order by order_num, ->pe_owner[0].order_num) + 1) ) } else { name } ) from $pes);
};

DEFINE FUNCTION fn::default_name(
    $pe: record,
) {
    return (select value (if name='' { string::concat(noun, <string> (array::find_index(select value order_num from ->pe_owner->pe<-pe_owner[where <-pe[where noun=$parent.noun]] order by order_num, ->pe_owner[0].order_num) + 1) ) } else { name } ) from $pe)[0];
};