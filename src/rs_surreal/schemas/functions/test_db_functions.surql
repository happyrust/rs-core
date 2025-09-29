-- 测试数据库相关函数的脚本
-- 用于验证 fn::get_world 和 fn::query_sites_of_db 函数是否正常工作

-- 测试 fn::get_world 函数
-- 替换 1112 为实际的数据库编号
SELECT fn::get_world(1112) AS world_refno;

-- 测试 fn::query_sites_of_db 函数
-- 替换 pe:123_456 为实际的 WORLD 参考号
SELECT fn::query_sites_of_db(pe:123_456) AS site_refnos;

-- 组合测试：通过数据库编号获取 WORLD，然后查询其下的 SITE
LET $world = fn::get_world(1112);
SELECT fn::query_sites_of_db($world) AS sites WHERE $world != none;

-- 查看数据库相关的数据（用于调试）
SELECT * FROM WORL WHERE REFNO.dbnum = 1112 LIMIT 5;

-- 查看 WORLD 表中的数据（用于调试）
SELECT * FROM WORL LIMIT 5;

-- 查看 SITE 相关的数据（用于调试）
SELECT * FROM pe WHERE noun = 'SITE' LIMIT 5;
