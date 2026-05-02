#!/usr/bin/env bash
# MBD V2 扩展批量验收脚本（Phase 7.2）
#
# 在原有 batch-validate-v2.sh 基础上增加：
#   1. NaN / Infinity 坐标检测
#   2. direction 全零检测
#   3. CSV 报告输出
#   4. 按 failure-category 分类汇总
#   5. 支持从文件读取 refno 列表
#
# 用法：
#   ./batch-validate-v2-extended.sh                          # 默认样本
#   ./batch-validate-v2-extended.sh refno1 refno2 ...        # 指定列表
#   ./batch-validate-v2-extended.sh -f refnos.txt            # 从文件读取
#   MBD_V2_BASE_URL=http://192.168.1.100:3100 ./batch-validate-v2-extended.sh

set -euo pipefail

BASE_URL="${MBD_V2_BASE_URL:-http://127.0.0.1:3100}"
REPORT_DIR="${MBD_V2_REPORT_DIR:-./reports}"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
CSV_FILE="${REPORT_DIR}/mbd_v2_batch_${TIMESTAMP}.csv"

DEFAULT_REFNOS=(
    "24381_145712"
)

if [ "${1:-}" = "-f" ] && [ -n "${2:-}" ]; then
    mapfile -t REFNOS < "$2"
elif [ $# -gt 0 ]; then
    REFNOS=("$@")
else
    REFNOS=("${DEFAULT_REFNOS[@]}")
fi

mkdir -p "${REPORT_DIR}"

TOTAL=${#REFNOS[@]}
PASS=0
FAIL=0
WARN=0

declare -A FAIL_REASONS

echo "═══════════════════════════════════════════════════════════"
echo "  MBD V2 扩展批量验收 — ${TOTAL} 个样本"
echo "  API: ${BASE_URL}/api/mbd/v2/pipe/{refno}"
echo "  报告: ${CSV_FILE}"
echo "═══════════════════════════════════════════════════════════"
echo ""

echo "refno,status,primitives,linear_dim,label,leader_line,weld_mark,slope_mark,angle_dim,error_issues,warn_issues,has_nan,has_zero_dir,failure_reason" > "${CSV_FILE}"

validate_refno() {
    local REFNO="$1"

    local RESPONSE
    RESPONSE=$(curl -s --max-time 30 "${BASE_URL}/api/mbd/v2/pipe/${REFNO}" 2>/dev/null || echo "")

    if [ -z "${RESPONSE}" ]; then
        echo "${REFNO},FAIL,0,0,0,0,0,0,0,0,0,false,false,request_failed" >> "${CSV_FILE}"
        echo "❌ ${REFNO} — 请求失败或超时"
        FAIL=$((FAIL + 1))
        FAIL_REASONS["request_failed"]=$(( ${FAIL_REASONS["request_failed"]:-0} + 1 ))
        return
    fi

    local SUCCESS
    SUCCESS=$(echo "${RESPONSE}" | jq -r '.success' 2>/dev/null || echo "false")
    if [ "${SUCCESS}" != "true" ]; then
        local ERROR_MSG
        ERROR_MSG=$(echo "${RESPONSE}" | jq -r '.error_message // "unknown"' 2>/dev/null)
        echo "${REFNO},FAIL,0,0,0,0,0,0,0,0,0,false,false,api_error:${ERROR_MSG}" >> "${CSV_FILE}"
        echo "❌ ${REFNO} — ${ERROR_MSG}"
        FAIL=$((FAIL + 1))
        FAIL_REASONS["api_error"]=$(( ${FAIL_REASONS["api_error"]:-0} + 1 ))
        return
    fi

    local VERSION PRIM_COUNT LINEAR_COUNT LABEL_COUNT LEADER_COUNT WELD_COUNT SLOPE_COUNT ANGLE_COUNT
    local ERROR_ISSUES WARN_ISSUES HAS_NAN HAS_ZERO_DIR

    VERSION=$(echo "${RESPONSE}" | jq -r '.data.version' 2>/dev/null || echo "")
    PRIM_COUNT=$(echo "${RESPONSE}" | jq '.data.primitives | length' 2>/dev/null || echo "0")
    LINEAR_COUNT=$(echo "${RESPONSE}" | jq '[.data.primitives[] | select(.kind == "linear_dim")] | length' 2>/dev/null || echo "0")
    LABEL_COUNT=$(echo "${RESPONSE}" | jq '[.data.primitives[] | select(.kind == "label")] | length' 2>/dev/null || echo "0")
    LEADER_COUNT=$(echo "${RESPONSE}" | jq '[.data.primitives[] | select(.kind == "leader_line")] | length' 2>/dev/null || echo "0")
    WELD_COUNT=$(echo "${RESPONSE}" | jq '[.data.primitives[] | select(.kind == "weld_mark")] | length' 2>/dev/null || echo "0")
    SLOPE_COUNT=$(echo "${RESPONSE}" | jq '[.data.primitives[] | select(.kind == "slope_mark")] | length' 2>/dev/null || echo "0")
    ANGLE_COUNT=$(echo "${RESPONSE}" | jq '[.data.primitives[] | select(.kind == "angle_dim")] | length' 2>/dev/null || echo "0")
    ERROR_ISSUES=$(echo "${RESPONSE}" | jq '[.data.issues[] | select(.severity == "error")] | length' 2>/dev/null || echo "0")
    WARN_ISSUES=$(echo "${RESPONSE}" | jq '[.data.issues[] | select(.severity == "warning")] | length' 2>/dev/null || echo "0")

    HAS_NAN="false"
    if echo "${RESPONSE}" | jq -e '.data.primitives | tostring | test("NaN|Infinity")' > /dev/null 2>&1; then
        HAS_NAN="true"
    fi

    HAS_ZERO_DIR="false"
    local ZERO_DIR_COUNT
    ZERO_DIR_COUNT=$(echo "${RESPONSE}" | jq '[.data.primitives[] | select(.kind == "linear_dim") | select(.text.orientation == [0,0,0])] | length' 2>/dev/null || echo "0")
    if [ "${ZERO_DIR_COUNT}" -gt 0 ]; then
        HAS_ZERO_DIR="true"
    fi

    local STATUS="PASS"
    local REASON=""

    if [ "${VERSION}" != "v2" ]; then
        STATUS="FAIL"; REASON="wrong_version"
    elif [ "${PRIM_COUNT}" -eq 0 ]; then
        STATUS="FAIL"; REASON="empty_primitives"
    elif [ "${ERROR_ISSUES}" -gt 0 ]; then
        STATUS="FAIL"; REASON="error_issues"
    elif [ "${HAS_NAN}" = "true" ]; then
        STATUS="FAIL"; REASON="nan_or_infinity"
    elif [ "${HAS_ZERO_DIR}" = "true" ]; then
        STATUS="WARN"; REASON="zero_direction"
    elif [ "${LINEAR_COUNT}" -eq 0 ]; then
        STATUS="WARN"; REASON="no_linear_dim"
    fi

    echo "${REFNO},${STATUS},${PRIM_COUNT},${LINEAR_COUNT},${LABEL_COUNT},${LEADER_COUNT},${WELD_COUNT},${SLOPE_COUNT},${ANGLE_COUNT},${ERROR_ISSUES},${WARN_ISSUES},${HAS_NAN},${HAS_ZERO_DIR},${REASON}" >> "${CSV_FILE}"

    local KINDS
    KINDS=$(echo "${RESPONSE}" | jq -r '[.data.primitives[] | .kind] | group_by(.) | map("\(.[0]):\(length)") | join(", ")' 2>/dev/null || echo "?")

    case "${STATUS}" in
        PASS)
            echo "✅ ${REFNO} — ${PRIM_COUNT} prims [${KINDS}] — ${WARN_ISSUES}w"
            PASS=$((PASS + 1))
            ;;
        WARN)
            echo "⚠️  ${REFNO} — ${PRIM_COUNT} prims [${KINDS}] — ${REASON}"
            WARN=$((WARN + 1))
            ;;
        FAIL)
            echo "❌ ${REFNO} — ${REASON}"
            FAIL=$((FAIL + 1))
            FAIL_REASONS["${REASON}"]=$(( ${FAIL_REASONS["${REASON}"]:-0} + 1 ))
            ;;
    esac
}

for REFNO in "${REFNOS[@]}"; do
    [[ -z "${REFNO}" || "${REFNO}" == \#* ]] && continue
    validate_refno "${REFNO}"
done

echo ""
echo "═══════════════════════════════════════════════════════════"
echo "  结果：✅ ${PASS} / ⚠️  ${WARN} / ❌ ${FAIL} / 总计 ${TOTAL}"
if [ ${#FAIL_REASONS[@]} -gt 0 ]; then
    echo ""
    echo "  失败分类："
    for reason in "${!FAIL_REASONS[@]}"; do
        echo "    ${reason}: ${FAIL_REASONS[$reason]}"
    done
fi
echo ""
echo "  CSV 报告: ${CSV_FILE}"
PASS_RATE=$(( (PASS * 100) / (TOTAL > 0 ? TOTAL : 1) ))
echo "  通过率: ${PASS_RATE}%"
echo "═══════════════════════════════════════════════════════════"

if [ "${FAIL}" -eq 0 ]; then
    echo "  批量验收通过 ✓"
    exit 0
else
    echo "  批量验收未通过 ✗"
    exit 1
fi
