#!/bin/bash
set -e

# 默认调试目标
TARGET=${1:-"25688/7958"}

echo "🚀 开始调试布尔运算目标: $TARGET"
echo "------------------------------------------------"

# 运行 Rust 调试 example
# 假设在 rs-core 目录下运行，或者根据脚本位置调整
SCRIPT_DIR=$(cd "$(dirname "$0")"; pwd)
cd "$SCRIPT_DIR"

# 确保 DbOption.toml 存在 (如果不存在则警告)
if [ ! -f "DbOption.toml" ]; then
    echo "⚠️  未找到 DbOption.toml，请确保配置文件存在!"
fi

# 运行 cargo run example
cargo run --example debug_boolean_refno -- "$TARGET"
