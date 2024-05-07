DEFINE FUNCTION fn::find_group_sube_children($pe:record) {
    let $children = $pe<-pe_owner[where in.noun = 'SUBE'].in;
    return $children<-pe_owner.in.refno;
};
