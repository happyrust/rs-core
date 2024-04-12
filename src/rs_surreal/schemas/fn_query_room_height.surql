DEFINE FUNCTION fn::room_height($pe:record) {
    let $noun = $pe.refno.TYPE;
    return
    // EQUI可能有两层
    if $noun == 'EQUI' {
        return array::distinct([
        select value (<-room_relate.in.id->inst_relate.aabb.d.mins[2])[0] from $pe<-pe_owner.in.id,
        select value (<-room_relate.in.id->inst_relate.aabb.d.mins[2])[0] from $pe<-pe_owner.in<-pe_pwner.id])[0]
    } else if  $noun == 'BRAN' {
        return select value (<-room_relate.in.id->inst_relate.aabb.d.mins[2])[0] from $pe<-pe_owner.in.id
    } else  {
        return select value (<-room_relate.in.id->inst_relate.aabb.d.mins[2])[0] from $pe
    }
};

// return fn::room_height(pe:24384_24804)[0];