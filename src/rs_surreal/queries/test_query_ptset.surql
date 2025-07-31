-- 测试 query_ptset 函数
-- 首先定义函数
REMOVE FUNCTION fn::query_ptset;
DEFINE FUNCTION fn::query_ptset($refno: string) {
    let $table_name = string::concat($refno, '_inst_relate');
    let $sql = string::concat(
        '(SELECT world_trans.d AS transform, object::values(out.ptset?:{}).pt AS points FROM ',
        $table_name,
        ')[0]'
    );
    
    let $result = SELECT VALUE $sql FROM ONLY [];
    return $result[0];
};

-- 测试函数调用（使用一个示例参考号）
SELECT fn::query_ptset('test_refno') AS result;