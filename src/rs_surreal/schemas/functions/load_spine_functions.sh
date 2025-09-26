#!/bin/bash
# 加载SPINE计算函数到SurrealDB

SURREAL_URL="http://127.0.0.1:5357"
DB_NAME="pdms"
NS_NAME="test"

echo "Loading SPINE calculation functions..."

# 加载SPINE计算函数
curl -X POST \
  -H "Accept: application/json" \
  -H "NS: $NS_NAME" \
  -H "DB: $DB_NAME" \
  --data-binary @spine_calc.surql \
  $SURREAL_URL/sql

echo "SPINE functions loaded successfully"