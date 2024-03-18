// 对应根阀编号
DEFINE FUNCTION fn::yk_pipe_num($pe:record) {
        let $zone_name = ($pe->pe_owner.out->pe_owner.out.name)[0];
        let $names = string::split($zone_name,'-');
        let $pipe_name = if array::len(string::split($zone_name,'-')) < 3 { null } else { string::concat($names[0],'-',$names[2],'-','PIPE')};
        return
        if string::contains($zone_name,'TUBE') && $pipe_name != NONE {
            (select name from pe where name = $pipe_name)
        } else {
            ''
        }
};