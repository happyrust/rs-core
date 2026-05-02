#!/usr/bin/env bash
# MBD V2 样本注册脚本（Phase 7.1）
#
# 功能：
#   1. 从 SurrealDB 查询项目内所有 BRAN 的 refno
#   2. 对每个 BRAN 调用 V2 API 获取基准数据
#   3. 输出 JSON 基准快照（用于后续回归对比）
#
# 用法：
#   ./build-sample-registry.sh                               # 默认项目
#   MBD_V2_BASE_URL=http://host:3100 ./build-sample-registry.sh
#   ./build-sample-registry.sh --max 50                      # 限制样本数
#
# 依赖：curl, jq
#
# 输出文件：
#   - reports/mbd_v2_baseline_YYYYMMDD_HHMMSS.json

set -euo pipefail

BASE_URL="${MBD_V2_BASE_URL:-http://127.0.0.1:3100}"
REPORT_DIR="${MBD_V2_REPORT_DIR:-./reports}"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BASELINE_FILE="${REPORT_DIR}/mbd_v2_baseline_${TIMESTAMP}.json"
MAX_SAMPLES="${1:-100}"

if [ "${1:-}" = "--max" ] && [ -n "${2:-}" ]; then
    MAX_SAMPLES="$2"
fi

mkdir -p "${REPORT_DIR}"

echo "═══════════════════════════════════════════════════════════"
echo "  MBD V2 样本注册"
echo "  API: ${BASE_URL}"
echo "  最大样本数: ${MAX_SAMPLES}"
echo "═══════════════════════════════════════════════════════════"
echo ""

# 步骤 1：从已知样本列表开始（后续可替换为 SurrealDB 动态查询）
# 这些是手动收集的代表性 BRAN refno
KNOWN_SAMPLES=(
    "24381_145712"
)

# 步骤 2：尝试从 API 查询更多 BRAN（如果支持列表接口）
# 这里预留了扩展点，当 API 支持 BRAN 列表查询时可以自动发现样本
discover_bran_list() {
    local RESPONSE
    RESPONSE=$(curl -s --max-time 10 "${BASE_URL}/api/pdms/type-info?type=BRAN" 2>/dev/null || echo "")
    if [ -n "${RESPONSE}" ]; then
        echo "${RESPONSE}" | jq -r '.[].refno // empty' 2>/dev/null | head -n "${MAX_SAMPLES}" || true
    fi
}

echo "尝试自动发现 BRAN 列表..."
DISCOVERED=$(discover_bran_list)
if [ -n "${DISCOVERED}" ]; then
    mapfile -t EXTRA_SAMPLES <<< "${DISCOVERED}"
    echo "  发现 ${#EXTRA_SAMPLES[@]} 个 BRAN"
    ALL_SAMPLES=("${KNOWN_SAMPLES[@]}" "${EXTRA_SAMPLES[@]}")
else
    echo "  未发现额外 BRAN（使用已知样本列表）"
    ALL_SAMPLES=("${KNOWN_SAMPLES[@]}")
fi

# 去重
declare -A SEEN
UNIQUE_SAMPLES=()
for s in "${ALL_SAMPLES[@]}"; do
    [[ -z "$s" ]] && continue
    if [[ -z "${SEEN[$s]:-}" ]]; then
        SEEN[$s]=1
        UNIQUE_SAMPLES+=("$s")
    fi
done

TOTAL=${#UNIQUE_SAMPLES[@]}
echo ""
echo "共 ${TOTAL} 个唯一样本"
echo ""

# 步骤 3：逐个查询 V2 API 并记录基准
BASELINE_ENTRIES=()
SUCCEED=0
FAILED=0

for REFNO in "${UNIQUE_SAMPLES[@]}"; do
    RESPONSE=$(curl -s --max-time 30 "${BASE_URL}/api/mbd/v2/pipe/${REFNO}" 2>/dev/null || echo "")

    if [ -z "${RESPONSE}" ]; then
        echo "⏭  ${REFNO} — 跳过（请求失败）"
        FAILED=$((FAILED + 1))
        continue
    fi

    SUCCESS=$(echo "${RESPONSE}" | jq -r '.success' 2>/dev/null || echo "false")
    if [ "${SUCCESS}" != "true" ]; then
        echo "⏭  ${REFNO} — 跳过（success=false）"
        FAILED=$((FAILED + 1))
        continue
    fi

    ENTRY=$(echo "${RESPONSE}" | jq '{
        refno: .data.input_refno,
        branch_refno: .data.branch_refno,
        primitives_count: (.data.primitives | length),
        kinds: ([.data.primitives[] | .kind] | group_by(.) | map({key: .[0], value: length}) | from_entries),
        dims_by_kind: .data.meta.dims_by_kind,
        segments_count: .data.meta.segments_count,
        welds_count: .data.meta.welds_count,
        issues_count: (.data.issues | length),
        error_issues: ([.data.issues[] | select(.severity == "error")] | length),
        warn_issues: ([.data.issues[] | select(.severity == "warning")] | length),
        generated_at: .data.meta.generated_at
    }' 2>/dev/null || echo "null")

    if [ "${ENTRY}" = "null" ]; then
        echo "⏭  ${REFNO} — 跳过（解析失败）"
        FAILED=$((FAILED + 1))
        continue
    fi

    PRIM_COUNT=$(echo "${ENTRY}" | jq '.primitives_count' 2>/dev/null)
    echo "✅ ${REFNO} — ${PRIM_COUNT} primitives"
    BASELINE_ENTRIES+=("${ENTRY}")
    SUCCEED=$((SUCCEED + 1))
done

# 步骤 4：输出基准文件
echo "${BASELINE_ENTRIES[@]}" | jq -s '{
    version: "mbd_v2_baseline_v1",
    created_at: now | todate,
    api_base: "'"${BASE_URL}"'",
    total_samples: length,
    samples: .
}' > "${BASELINE_FILE}" 2>/dev/null || {
    printf '{\n  "version": "mbd_v2_baseline_v1",\n  "created_at": "%s",\n  "total_samples": %d,\n  "samples": [\n' "${TIMESTAMP}" "${SUCCEED}" > "${BASELINE_FILE}"
    local first=true
    for entry in "${BASELINE_ENTRIES[@]}"; do
        if [ "$first" = true ]; then
            first=false
        else
            echo "," >> "${BASELINE_FILE}"
        fi
        echo "    ${entry}" >> "${BASELINE_FILE}"
    done
    printf '\n  ]\n}\n' >> "${BASELINE_FILE}"
}

echo ""
echo "═══════════════════════════════════════════════════════════"
echo "  基准快照已保存: ${BASELINE_FILE}"
echo "  成功: ${SUCCEED} / 失败: ${FAILED} / 总计: ${TOTAL}"
echo "═══════════════════════════════════════════════════════════"
