-- 查询参考号对应的点集合
-- 功能: 获取世界坐标变换矩阵和点集数据
-- 参数: $refno - 需要查询的参考号 (string)
-- 返回: 包含transform(变换矩阵)和points(点集)的对象
-- 注意: ptset 现在是数组格式，直接提取 pt 字段返回二维数组

REMOVE FUNCTION fn::query_ptset;
DEFINE FUNCTION fn::query_ptset($refno: string) {
    let $table_name = string::concat($refno, '_inst_relate');
    let $sql = string::concat(
        '(SELECT world_trans.d AS transform, out.ptset[*].pt AS points FROM ',
        $table_name,
        ')[0]'
    );
    
    -- 执行动态SQL查询
    let $result = SELECT VALUE $sql FROM ONLY [];
    return $result[0];
};