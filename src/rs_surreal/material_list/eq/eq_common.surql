-- 查找子集
remove function fn::find_group_sube_children;
DEFINE FUNCTION fn::find_group_sube_children($pe:record) {
    let $children = $pe<-pe_owner[where in.noun = 'SUBE'].in;
    return $children<-pe_owner.in.refno;
};

-- let $pe = [pe:24383_83477];
-- return fn::find_group_sube_children($pe);
