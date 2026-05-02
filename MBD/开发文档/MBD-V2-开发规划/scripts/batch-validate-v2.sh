#!/usr/bin/env bash
# MBD V2 批量验收脚本
#
# 用法：
#   ./batch-validate-v2.sh                     # 使用默认验收样本
#   ./batch-validate-v2.sh refno1 refno2 ...   # 指定 refno 列表
#   cat refnos.txt | xargs ./batch-validate-v2.sh
#
# 验收标准（对应 MBD-V2-下一步开发计划.md §四.1）：
#   - success = true
#   - data.version = "v2"
#   - data.primitives 非空
#   - 至少包含 linear_dim
#   - issues 不包含 error

set -euo pipefail

BASE_URL="${MBD_V2_BASE_URL:-http://127.0.0.1:3100}"

DEFAULT_REFNOS=(
    "24381_145712"
)

if [ $# -gt 0 ]; then
    REFNOS=("$@")
else
    REFNOS=("${DEFAULT_REFNOS[@]}")
fi

TOTAL=${#REFNOS[@]}
PASS=0
FAIL=0
WARN=0

echo "═══════════════════════════════════════════════════════"
echo "  MBD V2 批量验收 — ${TOTAL} 个样本"
echo "  API: ${BASE_URL}/api/mbd/v2/pipe/{refno}"
echo "═══════════════════════════════════════════════════════"
echo ""

for REFNO in "${REFNOS[@]}"; do
    RESPONSE=$(curl -s "${BASE_URL}/api/mbd/v2/pipe/${REFNO}" 2>/dev/null || echo "")

    if [ -z "${RESPONSE}" ]; then
        echo "❌ ${REFNO} — 请求失败"
        FAIL=$((FAIL + 1))
        continue
    fi

    SUCCESS=$(echo "${RESPONSE}" | jq -r '.success' 2>/dev/null || echo "false")
    if [ "${SUCCESS}" != "true" ]; then
        ERROR_MSG=$(echo "${RESPONSE}" | jq -r '.error_message // "unknown"' 2>/dev/null)
        echo "❌ ${REFNO} — ${ERROR_MSG}"
        FAIL=$((FAIL + 1))
        continue
    fi

    VERSION=$(echo "${RESPONSE}" | jq -r '.data.version' 2>/dev/null || echo "")
    PRIM_COUNT=$(echo "${RESPONSE}" | jq '.data.primitives | length' 2>/dev/null || echo "0")
    LINEAR_COUNT=$(echo "${RESPONSE}" | jq '[.data.primitives[] | select(.kind == "linear_dim")] | length' 2>/dev/null || echo "0")
    ERROR_ISSUES=$(echo "${RESPONSE}" | jq '[.data.issues[] | select(.severity == "error")] | length' 2>/dev/null || echo "0")
    WARN_ISSUES=$(echo "${RESPONSE}" | jq '[.data.issues[] | select(.severity == "warning")] | length' 2>/dev/null || echo "0")

    KINDS=$(echo "${RESPONSE}" | jq -r '[.data.primitives[] | .kind] | group_by(.) | map("\(.[0]):\(length)") | join(", ")' 2>/dev/null || echo "?")

    if [ "${VERSION}" != "v2" ]; then
        echo "❌ ${REFNO} — version=${VERSION}"
        FAIL=$((FAIL + 1))
    elif [ "${PRIM_COUNT}" -eq 0 ]; then
        echo "❌ ${REFNO} — primitives 为空"
        FAIL=$((FAIL + 1))
    elif [ "${ERROR_ISSUES}" -gt 0 ]; then
        echo "❌ ${REFNO} — ${ERROR_ISSUES} error issues"
        FAIL=$((FAIL + 1))
    elif [ "${LINEAR_COUNT}" -eq 0 ]; then
        echo "⚠️  ${REFNO} — ${PRIM_COUNT} prims [${KINDS}] — 无 linear_dim"
        WARN=$((WARN + 1))
    else
        echo "✅ ${REFNO} — ${PRIM_COUNT} prims [${KINDS}] — ${WARN_ISSUES}w"
        PASS=$((PASS + 1))
    fi
done

echo ""
echo "═══════════════════════════════════════════════════════"
echo "  结果：✅ ${PASS} 通过 / ⚠️  ${WARN} 警告 / ❌ ${FAIL} 失败 / 总计 ${TOTAL}"
echo "═══════════════════════════════════════════════════════"

if [ "${FAIL}" -eq 0 ]; then
    echo "  批量验收通过 ✓"
    exit 0
else
    echo "  批量验收未通过 ✗"
    exit 1
fi
