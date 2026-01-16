-- 数据库相关的 SurrealDB 函数定义
-- 用于处理数据库、WORLD 和 SITE 节点的查询

-- 通过数据库编号获取对应的 WORLD 参考号
REMOVE FUNCTION fn::get_world;
DEFINE FUNCTION fn::get_world($dbnum: number) {
    -- 直接通过数据库编号查找对应的 WORLD 节点
    let $world = SELECT VALUE REFNO FROM WORL WHERE REFNO.dbnum = $dbnum LIMIT 1;
    return $world;
};

-- 查询指定 WORLD 下的所有 SITE 节点
REMOVE FUNCTION fn::query_sites_of_db;
DEFINE FUNCTION fn::query_sites_of_db($world_refno: record) {
    if $world_refno == none {
        return [];
    };
    
    -- 查询 WORLD 下的所有 SITE 子节点
    if !record::exists($world_refno) {
        return [];
    };

    let $kids = $world_refno.children ?: [];
    return array::filter($kids, |$c: any|
        $c != NONE
        && record::exists($c)
        && !$c.deleted
        && $c.noun == 'SITE'
    );
};

-- 通过数据库编号直接获取所有 SITE 节点
REMOVE FUNCTION fn::get_sites_of_dbnum;
DEFINE FUNCTION fn::get_sites_of_dbnum($dbnum: number) {
    -- 首先获取 WORLD 节点
    let $world = fn::get_world($dbnum);
    if $world == none {
        return [];
    };
    
    -- 然后查询该 WORLD 下的所有 SITE 节点
    return fn::query_sites_of_db($world[0]);
};

-- 测试函数示例：
-- SELECT fn::get_world(1112);  -- 使用数据库编号
-- SELECT fn::query_sites_of_db(pe:123_456);
