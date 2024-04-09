DEFINE FUNCTION fn::room_code($pe:record) {
    let $uda_room = (select value v from type::thing('ATT_UDA', meta::id($pe)).udas where u.NAME='/ROOM_NO');

    return if array::len($uda_room) == 0 {
        return select value (<-room_relate.room_num)[0] from $pe
    } else {
        return $uda_room
    }
};