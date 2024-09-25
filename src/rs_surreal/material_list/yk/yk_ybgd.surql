// 仪控 仪表管道
remove function fn::yk_ybgd;
define function fn::yk_ybgd($refnos: array<record>) {
    return select 
        fn::pe_to_refno(id) as id,
        name ,
        fn::find_gy_bran($this.id)[0][0]?:'' as pipe_name,
        fn::room_code($this.id)[0] as room_code
    from $refnos;
};

