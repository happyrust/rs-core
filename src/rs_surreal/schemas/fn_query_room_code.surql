REMOVE FUNCTION fn::room_code;
DEFINE FUNCTION fn::room_code($pe:record) {
    let $noun = $pe.refno.TYPE;
    let $uda_room = [fn::get_uda_value($pe,'/ROOM_NO')];
    return
    // EQUI可能有两层
    if ($noun == 'EQUI'|| $noun == 'STRU' || $noun == 'REST') && $uda_room == [NONE] {
        return array::distinct(
            (array::flatten([
        array::complement((select value (<-room_relate.room_num)[0] from array::flatten($pe<-pe_owner.in.id)),[NONE]),
        array::complement((select value (<-room_relate.room_num)[0] from array::flatten($pe<-pe_owner.in<-pe_owner.in.id)),[NONE])
        ])))
    } else if  $noun == 'BRAN' && $uda_room == [NONE] {
        return array::complement((select value (<-room_relate.room_num)[0] from $pe<-pe_owner.in.id), [none])
    } else if $noun == 'SUPPO' || $noun == 'ZONE' {
        return array::distinct(
            (array::flatten( array::complement((select value (<-room_relate.room_num)[0] from array::flatten($pe<-pe_owner.in<-pe_owner.in<-pe_owner.in.id)),[NONE])
        )))
    } else if $uda_room == [NONE] {
        return select value (<-room_relate.room_num)[0] from $pe
    } else {
        return $uda_room
    }
};