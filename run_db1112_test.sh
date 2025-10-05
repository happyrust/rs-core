#!/bin/bash

# 1112 数据库测试运行脚本
# 使用 release 模式进行性能测试

set -e

echo "════════════════════════════════════════════════════════"
echo "  1112 数据库 Kuzu 存储测试 (Release 模式)"
echo "════════════════════════════════════════════════════════"
echo ""

# 设置颜色输出
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 步骤 1: 清理旧的测试数据
echo -e "${BLUE}[1/4]${NC} 清理旧的测试数据..."
rm -rf ./test_output/kuzu_1112_*.db
mkdir -p ./test_output
echo -e "${GREEN}✓${NC} 清理完成"
echo ""

# 步骤 2: 编译 release 版本
echo -e "${BLUE}[2/4]${NC} 编译 release 版本 (这可能需要几分钟)..."
echo -e "${YELLOW}提示:${NC} 使用 --features kuzu 进行编译"
echo ""

# 显示编译进度
cargo build --release --example db1112_quick_test --features kuzu 2>&1 | \
while IFS= read -r line; do
    if [[ "$line" =~ "Compiling" ]]; then
        echo -e "  ${line}"
    elif [[ "$line" =~ "Finished" ]]; then
        echo -e "${GREEN}✓${NC} ${line}"
    elif [[ "$line" =~ "error" ]]; then
        echo -e "\033[0;31m✗${NC} ${line}"
    fi
done

if [ $? -ne 0 ]; then
    echo -e "\033[0;31m✗ 编译失败${NC}"
    exit 1
fi

echo -e "${GREEN}✓${NC} 编译完成"
echo ""

# 步骤 3: 运行测试
echo -e "${BLUE}[3/4]${NC} 运行测试..."
echo ""

# 记录开始时间
START_TIME=$(date +%s)

# 运行测试程序
./target/release/examples/db1112_quick_test

# 计算耗时
END_TIME=$(date +%s)
ELAPSED=$((END_TIME - START_TIME))

echo ""
echo -e "${GREEN}✓${NC} 测试运行完成 (总耗时: ${ELAPSED} 秒)"
echo ""

# 步骤 4: 检查结果
echo -e "${BLUE}[4/4]${NC} 检查测试结果..."

if [ -d "./test_output/kuzu_1112_quick.db" ]; then
    DB_SIZE=$(du -sh ./test_output/kuzu_1112_quick.db | cut -f1)
    echo -e "${GREEN}✓${NC} Kuzu 数据库已创建"
    echo -e "   路径: ./test_output/kuzu_1112_quick.db"
    echo -e "   大小: ${DB_SIZE}"
else
    echo -e "\033[0;31m✗${NC} Kuzu 数据库未找到"
fi

echo ""
echo "════════════════════════════════════════════════════════"
echo -e "  ${GREEN}测试完成!${NC}"
echo "════════════════════════════════════════════════════════"
