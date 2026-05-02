#!/usr/bin/env bash
# MBD V2 后端 JSON 验证脚本
#
# 用法：
#   ./validate-v2-response.sh <refno>
#   ./validate-v2-response.sh 24381_145712
#   ./validate-v2-response.sh 24381_145712 http://localhost:3100
#
# 检查项：
#   1. success = true
#   2. data.version = "v2"
#   3. data.primitives 非空
#   4. 至少包含 linear_dim
#   5. issues 不包含 error 级别
#   6. 统计各 primitive kind 的数量

set -euo pipefail

REFNO="${1:?用法: $0 <refno> [base_url]}"
BASE_URL="${2:-http://127.0.0.1:3100}"
API_URL="${BASE_URL}/api/mbd/v2/pipe/${REFNO}?debug=true"

echo "═══════════════════════════════════════════════════════"
echo "  MBD V2 后端验证 — ${REFNO}"
echo "  API: ${API_URL}"
echo "═══════════════════════════════════════════════════════"
echo ""

RESPONSE=$(curl -s "${API_URL}")

if [ -z "${RESPONSE}" ]; then
    echo "❌ 请求失败：无响应"
    exit 1
fi

# 1. success
SUCCESS=$(echo "${RESPONSE}" | jq -r '.success')
if [ "${SUCCESS}" = "true" ]; then
    echo "✅ success = true"
else
    ERROR_MSG=$(echo "${RESPONSE}" | jq -r '.error_message // "unknown"')
    echo "❌ success = false — ${ERROR_MSG}"
    exit 1
fi

# 2. version
VERSION=$(echo "${RESPONSE}" | jq -r '.data.version')
if [ "${VERSION}" = "v2" ]; then
    echo "✅ version = v2"
else
    echo "❌ version = ${VERSION} (expected v2)"
    exit 1
fi

# 3. primitives 非空
PRIM_COUNT=$(echo "${RESPONSE}" | jq '.data.primitives | length')
if [ "${PRIM_COUNT}" -gt 0 ]; then
    echo "✅ primitives: ${PRIM_COUNT} 个"
else
    echo "❌ primitives 为空"
    exit 1
fi

# 4. 至少包含 linear_dim
LINEAR_COUNT=$(echo "${RESPONSE}" | jq '[.data.primitives[] | select(.kind == "linear_dim")] | length')
if [ "${LINEAR_COUNT}" -gt 0 ]; then
    echo "✅ linear_dim: ${LINEAR_COUNT} 个"
else
    echo "⚠️  linear_dim: 0 个（可能缺失尺寸）"
fi

# 5. issues 不包含 error
ERROR_ISSUES=$(echo "${RESPONSE}" | jq '[.data.issues[] | select(.severity == "error")] | length')
if [ "${ERROR_ISSUES}" -eq 0 ]; then
    echo "✅ error issues: 0"
else
    echo "❌ error issues: ${ERROR_ISSUES}"
    echo "${RESPONSE}" | jq '.data.issues[] | select(.severity == "error") | .message'
fi

# 6. 统计
echo ""
echo "── Primitive 统计 ──"
echo "${RESPONSE}" | jq -r '
  [.data.primitives[] | .kind] | group_by(.) | map({kind: .[0], count: length}) | sort_by(-.count)[] |
  "  \(.kind): \(.count)"
'

echo ""
echo "── Meta ──"
echo "${RESPONSE}" | jq '{
  segments_count: .data.meta.segments_count,
  welds_count: .data.meta.welds_count,
  dims_by_kind: .data.meta.dims_by_kind,
  generated_at: .data.meta.generated_at
}'

ISSUE_COUNT=$(echo "${RESPONSE}" | jq '.data.issues | length')
echo ""
echo "── Issues: ${ISSUE_COUNT} ──"
if [ "${ISSUE_COUNT}" -gt 0 ]; then
    echo "${RESPONSE}" | jq -r '.data.issues[] | "  [\(.severity)] \(.category): \(.message)"'
fi

echo ""
echo "── 验收结果 ──"
WARN_COUNT=$(echo "${RESPONSE}" | jq '[.data.issues[] | select(.severity == "warning")] | length')
if [ "${ERROR_ISSUES}" -eq 0 ] && [ "${LINEAR_COUNT}" -gt 0 ]; then
    echo "✅ 通过 — ${PRIM_COUNT} primitives, ${WARN_COUNT} warnings, 0 errors"
else
    echo "❌ 未通过"
fi
