#!/bin/bash

# 测试 query_ptset 函数
echo "测试 query_ptset 函数定义和调用..."

# 1. 首先定义函数
echo "1. 定义函数..."
curl -s -X POST "http://127.0.0.1:8009/sql" \
  -H "Content-Type: application/json" \
  -H "NS: 1516" \
  -H "DB: AvevaMarineSample" \
  -u "root:root" \
  -d '{
    "query": "REMOVE FUNCTION fn::query_ptset; DEFINE FUNCTION fn::query_ptset($refno: string) { let $table_name = string::concat($refno, \"_inst_relate\"); let $sql = string::concat(\"(SELECT world_trans.d AS transform, object::values(out.ptset?:{}).pt AS points FROM \", $table_name, \")[0]\"); let $result = SELECT VALUE $sql FROM ONLY []; return $result[0]; };"
  }'

echo ""
echo "2. 测试函数调用..."

# 2. 测试函数调用
curl -s -X POST "http://127.0.0.1:8009/sql" \
  -H "Content-Type: application/json" \
  -H "NS: 1516" \
  -H "DB: AvevaMarineSample" \
  -u "root:root" \
  -d '{
    "query": "SELECT fn::query_ptset(\"test_refno\") AS result;"
  }'

echo ""
echo "测试完成！"