
remove function if exists fn::single_room_code;
//目标查找节点的房间好
DEFINE FUNCTION fn::single_room_code($pe:record) {
	let $res = return select * from only $pe<-room_relate limit 1;
    if $res == none {
        return none;
    };
    let $prefix = string::split(string::slice($res.in.owner.name, 1), '-')[0];
    let $code = if string::len($res.room_num) == 4 {
        $res.room_num
    } else if string::len($res.room_num) == 5{
        string::concat(string::slice($res.room_num, 0, 1), string::slice($res.room_num, 2))
    } else {
        none
    };
    return string::concat($prefix, '-', $code);
};

remove function if exists fn::room_code;
DEFINE FUNCTION fn::room_code($pe:record) {
    let $noun = $pe.noun;
    return
    // EQUI可能有两层
    if ($noun == 'EQUI'|| $noun == 'STRU' || $noun == 'REST') {
         for $id in array::flatten([$pe<-pe_owner.in.id, array::flatten($pe<-pe_owner.in<-pe_owner.in.id)]) {
			let $room_code = fn::single_room_code($id);
			if $room_code {
			   return [$room_code];
			}
		};
		return none;
    } else if  $noun == 'BRAN' {
		return array::distinct(array::complement((select value fn::single_room_code(id) from array::flatten($pe<-pe_owner.in.id)), [NONE]));
    } else if $noun == 'SUPPO' || $noun == 'ZONE' {
		for $id in array::flatten(array::flatten(array::flatten($pe<-pe_owner.in<-pe_owner.in<-pe_owner.in.id))) {
			let $room_code = fn::single_room_code($id);
			if $room_code {
			   return [$room_code];
			}
		};
		return none;
    } else  {
        [fn::single_room_code($pe)]
    };
    
};